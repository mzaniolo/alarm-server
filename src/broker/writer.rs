use crate::alarm::Alarm;
use amqprs::{
    channel::{BasicPublishArguments, Channel, ExchangeDeclareArguments},
    BasicProperties,
};
use tokio::sync::mpsc;

const EXCHANGE_NAME: &str = "alarms";

pub struct Writer {
    channel: Channel,
    exchange_name: String,
    publish_args: BasicPublishArguments,
    rx: Option<mpsc::Receiver<Alarm>>,
}

impl Writer {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel: channel,
            exchange_name: EXCHANGE_NAME.to_string(),
            publish_args: BasicPublishArguments::new(EXCHANGE_NAME, ""),
            rx: None,
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

    pub async fn write(&mut self) {
        while let Some(alm) = self.rx.as_mut().unwrap().recv().await {
            self.channel
                .basic_publish(
                    BasicProperties::default(),
                    serde_json::to_string(&alm).unwrap().into_bytes(),
                    self.publish_args.clone(),
                )
                .await
                .unwrap();
        }
    }

    pub fn set_channel(&mut self, rx: mpsc::Receiver<Alarm>) {
        self.rx = Some(rx);
    }
}
