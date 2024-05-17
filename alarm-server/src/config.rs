use serde::Deserialize;
use std::fs;
use toml;

pub fn read_config(path: &str) -> Config {
    let source = fs::read_to_string(path).expect("config file not found");

    toml::from_str(&source).expect("Invalid configuration file")
}

#[derive(Deserialize)]
pub struct Config {
    pub broker: BrokerConfig,
    pub server: ServerConfig,
    pub alarm: AlarmConfig,
}

#[derive(Deserialize)]
pub struct BrokerConfig {
    #[serde(default = "default_ip")]
    pub ip: String,

    #[serde(default = "default_port::<5672>")]
    pub port: u16,

    #[serde(default = "default_cred")]
    pub username: String,

    #[serde(default = "default_cred")]
    pub password: String,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_ip")]
    pub ip: String,

    #[serde(default = "default_port::<8080>")]
    pub port: u16,
}

#[derive(Deserialize)]
pub struct AlarmConfig {
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_ip() -> String {
    "127.0.0.1".to_string()
}

fn default_cred() -> String {
    "guest".to_string()
}

fn default_path() -> String {
    "examples/config.yaml".to_string()
}

const fn default_port<const T: u16>() -> u16 {
    T
}
