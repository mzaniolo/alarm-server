use amqprs::channel::{
    BasicAckArguments, BasicConsumeArguments, Channel, ExchangeDeclareArguments,
    QueueBindArguments, QueueDeclareArguments,
};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};

const CHANNEL_CAPACITY: u16 = 10;
const MEAS_EXCHANGE: &str = "meas_exchange";
const ACK_EXCHANGE: &str = "ack_exchange";

pub struct Reader {
    channel: Channel,
    meas_exchange: String,
    ack_exchange: String,
    queue_name: String,
    ack_queue: String,

    map: HashMap<String, broadcast::Sender<i64>>,
    ack_tx: Option<mpsc::Sender<String>>,
}

impl Reader {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel: channel,
            meas_exchange: String::from(MEAS_EXCHANGE),
            ack_exchange: String::from(ACK_EXCHANGE),
            queue_name: String::new(),
            ack_queue: String::new(),
            map: HashMap::new(),
            ack_tx: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let x_type = "direct";
        let x_args = ExchangeDeclareArguments::new(&self.meas_exchange, x_type)
            .durable(true)
            .finish();
        self.channel.exchange_declare(x_args).await?;

        let q_args = QueueDeclareArguments::new("")
            .durable(false)
            .exclusive(true)
            .finish();
        (self.queue_name, _, _) = self.channel.queue_declare(q_args).await?.unwrap();

        self.bind_ack().await;

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
                &self.meas_exchange,
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

        let consumer_args = BasicConsumeArguments::default()
            .queue(self.ack_queue.clone())
            .finish();
        let (_ctag, mut ack_rx) = self.channel.basic_consume_rx(consumer_args).await.unwrap();

        println!("waiting on data");
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
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
                },
                Some(msg) = ack_rx.recv() => {
                    if let Some(payload) = msg.content {
                        let payload = std::str::from_utf8(&payload).unwrap();
                        if let Err(e) = self.ack_tx.as_ref().unwrap().send(payload.to_string()).await {
                            eprintln!(
                                "Error sending ack to '{payload}' - {e}"
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
    }

    async fn bind_ack(&mut self) {
        let x_type = "direct";
        let x_args = ExchangeDeclareArguments::new(&self.ack_exchange, x_type)
            .durable(true)
            .finish();
        self.channel.exchange_declare(x_args).await.unwrap();

        println!("conn open: {}", self.channel.is_connection_open());
        println!("channel open: {}", self.channel.is_open());

        let q_args = QueueDeclareArguments::new("")
            .durable(false)
            .exclusive(true)
            .finish();
        (self.ack_queue, _, _) = self.channel.queue_declare(q_args).await.unwrap().unwrap();

        self.channel
            .queue_bind(QueueBindArguments::new(
                &self.ack_queue,
                &self.ack_exchange,
                "ack",
            ))
            .await
            .unwrap();
    }

    pub fn set_ack_channel(&mut self, ack_tx: mpsc::Sender<String>) {
        self.ack_tx = Some(ack_tx);
    }
}
