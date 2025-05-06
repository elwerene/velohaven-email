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
    pub server: String,
    pub username: String,
    pub min_date: NaiveDate,
    pub email: EmailConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct EmailConfig {
    pub from: String,
    #[serde(default)]
    pub to_overwrite: Option<String>,
    pub host: String,
    pub username: String,
    pub password: String,
}
