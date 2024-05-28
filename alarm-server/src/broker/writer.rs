use amqprs::{
    channel::{BasicPublishArguments, Channel, ExchangeDeclareArguments},
    BasicProperties,
};

const EXCHANGE_NAME: &str = "alarms";

pub struct Writer {
    channel: Channel,
    exchange_name: String,
    publish_args: BasicPublishArguments,
}

impl Writer {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel: channel,
            exchange_name: EXCHANGE_NAME.to_string(),
            publish_args: BasicPublishArguments::new(EXCHANGE_NAME, ""),
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let x_type = "direct";
        let x_args = ExchangeDeclareArguments::new(&self.exchange_name, x_type)
            .durable(true)
            .finish();
        self.channel.exchange_declare(x_args).await?;
        Ok(())
    }

    pub async fn write(&self, msg: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.channel
            .basic_publish(BasicProperties::default(), msg, self.publish_args.clone())
            .await
            .unwrap();

        Ok(())
    }
}
