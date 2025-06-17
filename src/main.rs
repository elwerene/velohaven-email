use std::time::Duration;

use anyhow::Context;
use config::CONFIG;
use env_logger::Env;

mod cleverreach;
mod config;
mod email;
mod nextcloud;

#[tokio::main]
async fn main() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        std::process::exit(1);
    }));
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    match collect_data_and_send_emails().await {
        Ok(count) => {
            log::info!(
                "Data collection and email sending completed successfully. Sent {} emails.",
                count
            );
            CONFIG
                .email
                .send_digest(Ok(count))
                .await
                .expect("Failed to send digest email");
        }
        Err(e) => {
            log::error!("An error occurred: {}", e);
            CONFIG
                .email
                .send_digest(Err(e))
                .await
                .expect("Failed to send digest email");
            std::process::exit(1);
        }
    }
}

async fn collect_data_and_send_emails() -> anyhow::Result<usize> {
    let mut tries = 0;
    let members = loop {
        tries += 1;
        match CONFIG.cleverreach.get_members().await {
            Ok(members) => {
                break members;
            }
            Err(e) => {
                if tries >= 100 {
                    anyhow::bail!(
                        "Failed to get members from CleverReach after 100 attempts: {}",
                        e
                    );
                }
                log::error!("Failed to get members from CleverReach: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    };

    let mut tries = 0;
    let nextcloud_data = loop {
        tries += 1;
        match CONFIG.nextcloud.get_data().await {
            Ok(data) => {
                break data;
            }
            Err(e) => {
                if tries >= 100 {
                    log::error!(
                        "Failed to get data from Nextcloud after 100 attempts: {}",
                        e
                    );
                    std::process::exit(1);
                }
                log::error!("Failed to get data from Nextcloud: {}", e);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    };

    CONFIG
        .email
        .send_emails(nextcloud_data, members)
        .await
        .context("Failed to send emails")
}
