use crate::config::BrokerConfig;
use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    connection::{Connection, OpenConnectionArguments},
};

pub mod reader;
pub use crate::broker::reader::Reader;

pub struct Broker {
    host: String,
    port: u16,
    username: String,
    password: String,
    connection: Option<Connection>,
}

impl Broker {
    pub fn new(config: BrokerConfig) -> Self {
        Self {
            host: config.ip,
            port: config.port,
            username: config.username,
            password: config.password,
            connection: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connection = Some(
            Connection::open(&OpenConnectionArguments::new(
                &self.host,
                self.port,
                &self.username,
                &self.password,
            ))
            .await?,
        );

        self.connection
            .as_ref()
            .unwrap()
            .register_callback(DefaultConnectionCallback)
            .await?;

        Ok(())
    }

    pub async fn create_reader(&self) -> Result<Reader, Box<dyn std::error::Error>> {
        // open a channel on the connection
        let channel = self.connection.as_ref().unwrap().open_channel(None).await?;
        channel
            .register_callback(DefaultChannelCallback)
            .await?;
        Ok(Reader::new(channel))
    }
}
