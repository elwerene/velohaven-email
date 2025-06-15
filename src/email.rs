use crate::{cleverreach::Member, config::CONFIG, nextcloud::NextcloudData};
use anyhow::{Context, Result};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Email {
    from: String,
    #[serde(default)]
    to_overwrite: Option<String>,
    host: String,
    port: Option<u16>,
    username: String,
    password: String,
}

impl Email {
    pub async fn send_emails(
        &self,
        nextcloud_data: NextcloudData,
        members: Vec<Member>,
    ) -> Result<()> {
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
            .context("Could not create async smtp transport")?
            .credentials(Credentials::new(
                self.username.clone(),
                self.password.clone(),
            ));
        if let Some(port) = self.port {
            builder = builder.port(port).tls(Tls::Required(
                TlsParameters::new(self.host.clone()).context("Could not create tls parameters")?,
            ));
        }
        let mailer = builder.build();
        log::info!("Testing connection to SMTP server");
        if !mailer
            .test_connection()
            .await
            .context("Could not connect to SMTP server")?
        {
            anyhow::bail!("Could not connect to SMTP server");
        }
        log::info!("Connection to SMTP server successful");

        let to_overwrite: Option<Mailbox> = match self.to_overwrite.as_ref() {
            Some(email) => {
                let email = email
                    .parse()
                    .context("Could not parse to_overwrite email")?;
                log::info!("Using to_overwrite: {email}");
                Some(email)
            }
            None => {
                log::info!("No to_overwrite provided");
                None
            }
        };
        let from: Mailbox = self.from.parse().context("Could not parse from email")?;
        let today = CONFIG
            .now_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive());
        log::info!("Today's date: {today}");
        for member in members {
            if nextcloud_data.unsubscribed.contains(&member.email) {
                log::info!("Member {} is unsubscribed", member.email);
                continue;
            }

            let Some(to) = to_overwrite.clone().or_else(|| member.email.parse().ok()) else {
                log::error!("Could not parse email address: {}", member.email);
                continue;
            };

            let start_at = member.added_at.max(CONFIG.min_date);
            log::info!(
                "Member {} added at {} (start_at: {})",
                member.email,
                member.added_at,
                start_at
            );

            for template in &nextcloud_data.templates {
                if today == start_at + template.duration {
                    log::info!(
                        "Sending email to {} with template \"{}\"",
                        member.email,
                        template.name
                    );

                    let email = match lettre::Message::builder()
                        .header(ContentType::TEXT_HTML)
                        .from(from.clone())
                        .to(to.clone())
                        .subject(template.subject.clone())
                        .body(template.body.clone())
                    {
                        Ok(email) => email,
                        Err(err) => {
                            log::error!("Could not create email: {err:?}");
                            continue;
                        }
                    };

                    match mailer.send(email).await {
                        Ok(_) => log::info!("Email sent to {to}"),
                        Err(err) => {
                            log::error!("Could not send email to {to}: {err:?}");
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
