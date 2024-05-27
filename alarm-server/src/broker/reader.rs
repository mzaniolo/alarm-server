use amqprs::channel::{
    self, BasicAckArguments, BasicConsumeArguments, Channel, ExchangeDeclareArguments,
    QueueBindArguments, QueueDeclareArguments,
};
use std::collections::HashMap;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: u16 = 10;
const EXCHANGE_NAME: &str = "meas_exchange";

pub struct Reader {
    channel: Channel,
    exchange_name: String,
    queue_name: String,

    map: HashMap<String, broadcast::Sender<i64>>,
}

impl Reader {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel: channel,
            exchange_name: String::new(),
            queue_name: String::new(),
            map: HashMap::new(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.exchange_name = String::from(EXCHANGE_NAME);
        let x_type = "direct";
        let x_args = ExchangeDeclareArguments::new(&self.exchange_name, x_type)
            .durable(true)
            .finish();
        self.channel.exchange_declare(x_args).await?;

        let q_args = QueueDeclareArguments::new("")
            .durable(false)
            .exclusive(true)
            .finish();
        (self.queue_name, _, _) = self.channel.queue_declare(q_args).await?.unwrap();

        Ok(())
    }

    pub async fn subscribe(&mut self, meas: &str) -> broadcast::Receiver<i64> {
        if let Some(tx) = self.map.get(meas) {
            return tx.subscribe();
        }

        println!("creating the route. meas: {meas}");
        println!("conn open: {}", self.channel.is_connection_open());
        println!("channel open: {}", self.channel.is_open());

        // Every meas path will be a route key. This way the receiver can select only the
        //  meas that are needed. If this scales a lot this may turn out to be a bad idea.
        // Needs a test to see how well this thing would scale.
        self.channel
            .queue_bind(QueueBindArguments::new(
                &self.queue_name,
                &self.exchange_name,
                meas,
            ))
            .await
            .unwrap();

        let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY.into());
        self.map.insert(meas.to_owned(), tx);

        rx
    }

    pub async fn receive(&self) {
        println!("init receive");
        let consumer_args = BasicConsumeArguments::default()
            .queue(self.queue_name.clone())
            .finish();
        let (_ctag, mut rx) = self.channel.basic_consume_rx(consumer_args).await.unwrap();

        println!("waiting on data");
        while let Some(msg) = rx.recv().await {
            if let Some(payload) = msg.content {
                let mut meas = String::new();
                if let Some(deliver) = msg.deliver.as_ref() {
                    meas = deliver.routing_key().clone();
                }

                let value: i32 = std::str::from_utf8(&payload).unwrap().parse().unwrap();

                // println!(" [x] Received {value} from {meas}",);

                if let Err(e) = self.map.get(&meas).unwrap().send(value.into()) {
                    eprintln!(
                        "Error sending value '{value}' to alarms subscribed to '{meas}' - {e}"
                    )
                }

                self.channel
                    .basic_ack(BasicAckArguments::new(
                        msg.deliver.unwrap().delivery_tag(),
                        false,
                    ))
                    .await
                    .unwrap();
            }
        }
    }
}
