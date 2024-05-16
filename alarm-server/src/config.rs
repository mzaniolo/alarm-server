use serde::Deserialize;
use std::fs;
use toml;

struct Config {
    pub broker: BrokerConfig,
    pub server: ServerConfig,
    pub alarm: AlarmConfig,
}

#[derive(Deserialize)]
pub struct BrokerConfig {
    #[serde(default = "localhost")]
    pub ip: String,

    #[serde(default = "5672")]
    pub port: u16,

    #[serde(default = "guest")]
    pub username: String,

    #[serde(default = "guest")]
    pub password: String,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    #[serde(default = "127.0.0.1")]
    pub ip: String,

    #[serde(default = "8080")]
    pub port: u16,
}

#[derive(Deserialize)]
pub struct AlarmConfig {
    #[serde(default = "examples/config.yaml")]
    pub path: String,
}

fn read_config(path: &str) -> Config {
    let source = fs::read_to_string(path).expect("config file not found");

    toml::from_str(source).expect("Invalid configuration file")
}
