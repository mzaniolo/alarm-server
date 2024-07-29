use amqprs::channel::{
    BasicAckArguments, BasicConsumeArguments, Channel, ExchangeDeclareArguments,
    QueueBindArguments, QueueDeclareArguments,
};
use async_channel;
use tokio::sync::mpsc;

const ALM_EXCHANGE: &str = "alm_trg_exchange";
const ACK_EXCHANGE: &str = "ack_exchange";

pub struct Reader {
    channel: Channel,
    alm_exchange: String,
    ack_exchange: String,
    queue_name: String,
    ack_queue: String,
    ack_tx: Option<mpsc::Sender<String>>,
    alm_tx: Option<async_channel::Sender<String>>,
}

impl Reader {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel: channel,
            alm_exchange: String::from(ALM_EXCHANGE),
            ack_exchange: String::from(ACK_EXCHANGE),
            queue_name: String::new(),
            ack_queue: String::new(),
            ack_tx: None,
            alm_tx: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let x_args = ExchangeDeclareArguments::new(&self.alm_exchange, "direct")
            .durable(true)
            .finish();
        self.channel.exchange_declare(x_args).await?;

        let q_args = QueueDeclareArguments::new("")
            .durable(false)
            .finish();
        (self.queue_name, _, _) = self.channel.queue_declare(q_args).await?.unwrap();

        self.channel
            .queue_bind(QueueBindArguments::new(
                &self.queue_name,
                &self.alm_exchange,
                "",
            ))
            .await
            .unwrap();

        self.bind_ack().await;

        Ok(())
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

                        let payload = std::str::from_utf8(&payload).unwrap();
                        if let Err(e) = self.alm_tx.as_ref().unwrap().send(payload.to_string()).await {
                            eprintln!(
                                "Error sending value '{payload}' - {e}"
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

    pub fn set_alm_channel(&mut self, alm_tx: async_channel::Sender<String>) {
        self.alm_tx = Some(alm_tx);
    }
}
