use crate::alarm;
use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Error, Message,
};

const CHANNEL_SIZE: usize = 5;
const PROTOCOL_VERSION: &'static str = include_str!("../../protocol_version");

type Subscriptions = Arc<Mutex<HashMap<String, HashSet<Client>>>>;
type MapAck = Arc<Mutex<HashMap<String, mpsc::Sender<bool>>>>;

#[derive(Debug, Clone)]
pub struct Client {
    addr: SocketAddr,
    tx: mpsc::Sender<String>,
    // subscriptions: Vec<String>,
}

impl Hash for Client {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state)
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl Eq for Client {}

pub struct Publisher {
    addr: String,
    port: u16,

    clients: Vec<Client>,
    subscriptions: Subscriptions,
    map_ack: MapAck,
}

impl Publisher {
    pub fn new(addr: Option<String>, port: Option<u16>) -> Self {
        Self {
            addr: addr.unwrap_or(String::from("127.0.0.1")),
            port: port.unwrap_or(8080),
            clients: Vec::new(),
            subscriptions: Subscriptions::default(),
            map_ack: MapAck::default(),
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
                Arc::clone(&self.map_ack),
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
            .insert(client);
    }

    async fn handle_client(
        raw_stream: TcpStream,
        client: Client,
        mut rx: mpsc::Receiver<String>,
        subscriptions: Subscriptions,
        map_ack: MapAck,
    ) {
        println!("Incoming TCP connection from: {}", client.addr);

        let ws_stream = tokio_tungstenite::accept_async(raw_stream)
            .await
            .expect("Error during the websocket handshake occurred");
        println!("WebSocket connection established: {}", client.addr);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // We first check if both sides have the same protocol version
        if Self::do_handshake(ws_read.next().await) {
            let _ = ws_write
                .send(Message::Text(std::format!(
                    "::protocol_version:: {PROTOCOL_VERSION}"
                )))
                .await;
        } else {
            println!("handshake with {} failed!", client.addr);
            let _ = ws_write
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Protocol,
                    reason: std::borrow::Cow::Borrowed("wrong version"),
                })))
                .await;
            return;
        }

        loop {
            tokio::select! {
                msg = ws_read.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg.unwrap();
                            if msg.is_text() ||msg.is_binary() {
                                println!("got from {} the message: {}", client.addr, msg.to_string());
                                let msg: String = msg.to_string();
                                if msg.starts_with("::"){
                                    // this is a command
                                    if msg.starts_with("::subscribe::") {
                                        let alm = &msg[13..];
                                        println!("subscribing to {}", alm);
                                        Publisher::subscribe(alm, &subscriptions, client.clone()).await;
                                    }
                                    if msg.starts_with("::ack::"){
                                        let alm = &msg[7..];
                                        println!("ack alm {}", alm);
                                        let map = map_ack.lock().await;
                                        match map.get(alm) {
                                            Some(tx) => {
                                                let _ = tx.send(true).await;
                                            },
                                            None => {
                                                eprint!("Couldn't find {alm} to ack");
                                                let _ = ws_write.send(Message::Text(std::format!("Couldn't find {alm} to ack"))).await;
                                            }
                                        }
                                    }
                                }
                            } else if msg.is_close() {
                                eprintln!("Connection closed by client {}", client.addr);
                                return ;
                            }
                        }
                        _ => {
                            eprintln!("Something went wrong with client {}", client.addr);
                            return ;
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

    fn do_handshake(msg: Option<Result<Message, Error>>) -> bool {
        match msg {
            Some(msg) => match msg {
                Ok(msg) => {
                    let protocol_version = msg.to_string();
                    if protocol_version != PROTOCOL_VERSION {
                        eprintln!("Expected protocol version is {PROTOCOL_VERSION}, Received: {protocol_version}");
                        return false;
                    }
                }
                Err(e) => {
                    eprintln!("Got error on handshake: {e}");
                    return false;
                }
            },
            None => {
                eprintln!("Got no message during handshake");
                return false;
            }
        }

        return true;
    }

    pub async fn listen_alarms(
        mut rx: mpsc::Receiver<alarm::AlarmStatus>,
        subscriptions: Subscriptions,
    ) {
        while let Some(alm) = rx.recv().await {
            println!("got notified by alarm: {:?}", alm);
            if let Some(clients) = subscriptions.lock().await.get(&alm.name) {
                for client in clients.iter() {
                    let _ = client.tx.send(std::format!("{:#?}", alm)).await;
                }
            }
        }
    }

    pub fn get_subscriptions(&self) -> Subscriptions {
        Arc::clone(&self.subscriptions)
    }

    pub fn get_map_ack(&self) -> MapAck {
        Arc::clone(&self.map_ack)
    }
}
