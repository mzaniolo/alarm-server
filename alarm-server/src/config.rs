use serde::Deserialize;
use std::fs;
use toml;

pub fn read_config(path: &str) -> Config {
    let source =
        fs::read_to_string(path).expect(&format!("config file not found. Path: '{}'", path));

    toml::from_str(&source).expect("Invalid configuration file")
}

#[derive(Deserialize, Default)]
#[serde(default)]
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

impl Default for AlarmConfig {
    fn default() -> Self {
        Self {
            path: default_path(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port::<8080>(),
        }
    }
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port::<5672>(),
            username: default_cred(),
            password: default_cred(),
        }
    }
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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_example() -> Result<(), Box<dyn std::error::Error>> {
        let config = read_config("examples/server_config.toml");

        assert_eq!(config.alarm.path, "examples/config.yaml");

        assert_eq!(config.server.ip, "127.0.0.1");
        assert_eq!(config.server.port, 8080);

        assert_eq!(config.broker.ip, "127.0.0.1");
        assert_eq!(config.broker.port, 5672);
        assert_eq!(config.broker.username, "guest");
        assert_eq!(config.broker.password, "guest");

        Ok(())
    }

    #[test]
    fn test_empty() -> Result<(), Box<dyn std::error::Error>> {
        let config: Config = toml::from_str("").expect("Invalid configuration file");

        assert_eq!(config.alarm.path, "examples/config.yaml");

        assert_eq!(config.server.ip, "127.0.0.1");
        assert_eq!(config.server.port, 8080);

        assert_eq!(config.broker.ip, "127.0.0.1");
        assert_eq!(config.broker.port, 5672);
        assert_eq!(config.broker.username, "guest");
        assert_eq!(config.broker.password, "guest");

        Ok(())
    }

    #[test]
    fn test_missing_fields() -> Result<(), Box<dyn std::error::Error>> {
        let config = r#"
            [broker]
            port = 5672
            password = "guest"

            [server]
            port = 8080
        "#;

        let config: Config = toml::from_str(config).expect("Invalid configuration file");

        assert_eq!(config.alarm.path, "examples/config.yaml");

        assert_eq!(config.server.ip, "127.0.0.1");
        assert_eq!(config.server.port, 8080);

        assert_eq!(config.broker.ip, "127.0.0.1");
        assert_eq!(config.broker.port, 5672);
        assert_eq!(config.broker.username, "guest");
        assert_eq!(config.broker.password, "guest");

        Ok(())
    }
}
