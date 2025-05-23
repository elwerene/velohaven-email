use crate::{cleverreach::Member, config::CONFIG, nextcloud::NextcloudData};
use anyhow::{Context, Result};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Email {
    from: String,
    #[serde(default)]
    to_overwrite: Option<String>,
    host: String,
    username: String,
    password: String,
}

impl Email {
    pub async fn send_emails(
        &self,
        nextcloud_data: NextcloudData,
        members: Vec<Member>,
    ) -> Result<()> {
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)
            .context("Could not create async smtp transport")?
            .credentials(Credentials::new(
                self.username.clone(),
                self.password.clone(),
            ))
            .build();
        if !mailer
            .test_connection()
            .await
            .context("Could not connect to SMTP server")?
        {
            anyhow::bail!("Could not connect to SMTP server");
        }

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
        let today = chrono::Utc::now().date_naive();
        for member in members {
            log::info!("Member: {member:?}");

            if nextcloud_data.unsubscribed.contains(&member.email) {
                log::info!("Member {} is unsubscribed", member.email);
                continue;
            }

            let Some(to) = to_overwrite.clone().or_else(|| member.email.parse().ok()) else {
                log::error!("Could not parse email address: {}", member.email);
                continue;
            };

            let start_at = member.added_at.max(CONFIG.min_date);

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
                        Err(err) => anyhow::bail!("Could not send email: {err:?}"),
                    }
                }
            }
        }

        Ok(())
    }
}
