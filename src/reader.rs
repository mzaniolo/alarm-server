use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicConsumeArguments, Channel, ExchangeDeclareArguments,
        QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
    error,
};
use std::collections::HashMap;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: u16 = 10;

pub struct Reader {
    host: String,
    port: u16,
    username: String,
    password: String,

    connection: Option<Connection>,
    channel: Option<Channel>,
    exchange_name: String,
    queue_name: String,

    map: HashMap<String, broadcast::Sender<i64>>,
}

impl Reader {
    pub fn new(
        host: Option<&str>,
        port: Option<u16>,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Self {
        Self {
            host: host.unwrap_or("localhost").to_owned(),
            port: port.unwrap_or(5672),
            username: username.unwrap_or("guest").to_owned(),
            password: password.unwrap_or("guest").to_owned(),
            connection: None,
            channel: None,
            exchange_name: String::new(),
            queue_name: String::new(),
            map: HashMap::new(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), error::Error> {
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

        // open a channel on the connection
        self.channel = Some(self.connection.as_ref().unwrap().open_channel(None).await?);
        self.channel
            .as_ref()
            .unwrap()
            .register_callback(DefaultChannelCallback)
            .await?;

        self.exchange_name = String::from("meas_exchange");
        let x_type = "direct";
        let x_args = ExchangeDeclareArguments::new(&self.exchange_name, x_type)
            .durable(true)
            .finish();
        self.channel
            .as_ref()
            .unwrap()
            .exchange_declare(x_args)
            .await
            .unwrap();

        let q_args = QueueDeclareArguments::new("")
            .durable(false)
            .exclusive(true)
            .finish();
        (self.queue_name, _, _) = self
            .channel
            .as_ref()
            .unwrap()
            .queue_declare(q_args)
            .await
            .unwrap()
            .unwrap();

        Ok(())
    }

    pub async fn subscribe(&mut self, meas: &str) -> broadcast::Receiver<i64> {
        if let Some(tx) = self.map.get(meas) {
            return tx.subscribe();
        }

        println!("creating the route. meas: {meas}");
        println!(
            "conn open: {}",
            self.channel.as_ref().unwrap().is_connection_open()
        );
        println!("channel open: {}", self.channel.as_ref().unwrap().is_open());

        // Every meas path will be a route key. This way the receiver can select only the
        //  meas that are needed. If this scales a lot this may turn out to be a bad idea.
        // Needs a test to see how well this thing would scale.
        self.channel
            .as_ref()
            .unwrap()
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
        let (_ctag, mut rx) = self
            .channel
            .as_ref()
            .unwrap()
            .basic_consume_rx(consumer_args)
            .await
            .unwrap();

        println!("waiting on data");
        while let Some(msg) = rx.recv().await {
            if let Some(payload) = msg.content {
                println!(" [x] Received {:?}", std::str::from_utf8(&payload).unwrap());
                println!("msg basic prop: {:?}", msg.basic_properties.unwrap());
                self.channel
                    .as_ref()
                    .unwrap()
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
