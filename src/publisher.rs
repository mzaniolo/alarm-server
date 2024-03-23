use crate::alarm;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::{client, Message};

const CHANNEL_SIZE: usize = 5;

type Subscriptions = Arc<Mutex<HashMap<String, Vec<Client>>>>;

#[derive(Debug, Clone)]
pub struct Client {
    addr: SocketAddr,
    tx: mpsc::Sender<String>,
    // subscriptions: Vec<String>,
}

pub struct Publisher {
    addr: String,
    port: u16,

    clients: Vec<Client>,
    subscriptions: Subscriptions,
}

impl Publisher {
    pub fn new(addr: Option<String>, port: Option<u16>) -> Self {
        Self {
            addr: addr.unwrap_or(String::from("127.0.0.1")),
            port: port.unwrap_or(8080),
            clients: Vec::new(),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect(&mut self) {
        let full_addr = std::format!("{}:{}", self.addr, self.port);

        let try_socket = TcpListener::bind(&full_addr).await;
        let listener = try_socket.expect("Failed to bind");
        println!("Listening on: {}", full_addr);

        // Let's spawn the handling of each connection in a separate task.
        while let Ok((stream, addr)) = listener.accept().await {
            let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
            let client = Client { addr, tx };
            self.clients.push(client.clone());
            tokio::spawn(Self::handle_client(
                stream,
                client,
                rx,
                Arc::clone(&self.subscriptions),
            ));
        }
        println!("Server started");
    }

    async fn subscribe(alm: &str, subscriptions: &Subscriptions, client: Client) {
        subscriptions
            .lock()
            .await
            .entry(String::from(alm))
            .or_default()
            .push(client);
    }

    async fn handle_client(
        raw_stream: TcpStream,
        client: Client,
        mut rx: mpsc::Receiver<String>,
        subscriptions: Subscriptions,
    ) {
        println!("Incoming TCP connection from: {}", client.addr);

        let ws_stream = tokio_tungstenite::accept_async(raw_stream)
            .await
            .expect("Error during the websocket handshake occurred");
        println!("WebSocket connection established: {}", client.addr);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        Publisher::subscribe("sub1/alarm1", &subscriptions, client.clone()).await;
        Publisher::subscribe("sub1/alarm2", &subscriptions, client.clone()).await;
        Publisher::subscribe("sub2/alarm1", &subscriptions, client.clone()).await;
        Publisher::subscribe("sub2/alarm2", &subscriptions, client.clone()).await;

        loop {
            tokio::select! {
                msg = ws_read.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg.unwrap();
                            if msg.is_text() ||msg.is_binary() {
                                println!("got from {} the message: {}", client.addr, msg.to_string());
                            } else if msg.is_close() {
                                eprintln!("Connection closed by client {}", client.addr);
                            }
                        }
                        _ => {
                            eprintln!("Something went wrong with client {}", client.addr);
                        }
                    }

                }
                val = rx.recv() => {
                    if let Some(val) = val{
                        println!("notifying client of val {val}");
                        let _ = ws_write.send(Message::Text(val)).await;
                    }
                }
            }
        }
    }

    pub async fn listen_alarms(
        mut rx: mpsc::Receiver<alarm::AlarmStatus>,
        subscriptions: Subscriptions,
    ) {
        while let Some(alm) = rx.recv().await {
            println!("got notified by alarm: {:?}", alm);
            if let Some(clients) = subscriptions.lock().await.get(&alm.meas) {
                for client in clients.iter() {
                    let _ = client.tx.send(std::format!("{:#?}", alm)).await;
                }
            }
        }
    }

    pub fn get_subscriptions(&self) -> Subscriptions {
        Arc::clone(&self.subscriptions)
    }
}
