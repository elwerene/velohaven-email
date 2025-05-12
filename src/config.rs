use crate::{cleverreach::CleverreachConfig, email::EmailConfig, nextcloud::NextcloudConfig};
use chrono::NaiveDate;
use once_cell::sync::Lazy;
use serde::Deserialize;

pub const CONFIG_FILE: &str = "config.toml";
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    toml::de::from_str(&std::fs::read_to_string(CONFIG_FILE).expect("Could not read config file"))
        .expect("Could not parse config file")
});

#[derive(Deserialize, Debug)]
pub struct Config {
    pub min_date: NaiveDate,
    pub cleverreach: CleverreachConfig,
    pub email: EmailConfig,
    pub nextcloud: NextcloudConfig,
}
