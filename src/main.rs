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

    let members = CONFIG
        .cleverreach
        .get_members()
        .await
        .expect("Failed to get emails from cleverreach");
    let nextcloud_data = CONFIG
        .nextcloud
        .get_data()
        .await
        .expect("Failed to get nextcloud data");
    CONFIG
        .email
        .send_emails(nextcloud_data, members)
        .await
        .expect("Failed to send emails");
}
