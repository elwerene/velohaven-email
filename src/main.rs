use config::CONFIG;
use env_logger::Env;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};

mod config;
mod nextcloud;

#[tokio::main]
async fn main() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        std::process::exit(1);
    }));

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&CONFIG.email.host)
        .expect("Could not create SMTP transport")
        .credentials(Credentials::new(
            CONFIG.email.username.clone(),
            CONFIG.email.password.clone(),
        ))
        .build();
    let to_overwrite: Option<Mailbox> = CONFIG
        .email
        .to_overwrite
        .as_ref()
        .map(|email| email.parse().expect("Could not parse to_overwrite email"));
    let from: Mailbox = CONFIG
        .email
        .from
        .parse()
        .expect("Could not parse from email");
    let today = chrono::Utc::now().date_naive();
    let data = nextcloud::get().await.expect("Failed to get data");
    for member in &data.members {
        log::info!("Member: {member:?}");

        let Some(to) = to_overwrite.clone().or_else(|| member.email.parse().ok()) else {
            log::error!("Could not parse email address: {}", member.email);
            continue;
        };

        let start_at = member.added_at.max(CONFIG.min_date);

        for template in &data.templates {
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
                    Err(err) => log::error!("Could not send email: {err:?}"),
                }
            }
        }
    }
}
