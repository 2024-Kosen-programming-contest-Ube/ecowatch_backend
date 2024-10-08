use config::ConfigError;
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub listen_address: String,
    pub cookie_domain: String,
    pub cookie_cross: bool,
    pub database_url: String,
    pub sensor_interval: u64, // msec
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()
            .expect("Failed load .env as config.");
        cfg.try_deserialize()
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenvy::dotenv().expect("Failed to read .env file");
    Config::from_env().expect("Failed to load config.")
});
