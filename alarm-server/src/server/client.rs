use crate::server::{MapAck, MapAlmStatus, Server, Subscriptions};
use futures_util::{SinkExt, StreamExt};
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Error, Message,
};

const PROTOCOL_VERSION: &'static str = include_str!("../../../protocol_version");

enum Commands<'a> {
    KeepAlive,
    Ack(&'a str),
    Subscribe(&'a str),
    GetAll,

    Unknown,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub addr: SocketAddr,
    pub tx: mpsc::Sender<String>,
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

impl Client {
    pub async fn handle_client(
        raw_stream: TcpStream,
        client: Client,
        mut rx: mpsc::Receiver<String>,
        subscriptions: Subscriptions,
        map_ack: MapAck,
        map_alm_status: MapAlmStatus,
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

        let mut list_subscriptions: Vec<String> = Vec::new();

        loop {
            tokio::select! {
                msg = ws_read.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg.unwrap();
                            if msg.is_text(){
                                let msg: String = msg.to_string();
                                match Self::parse_command(&msg) {
                                    Commands::Subscribe(alm) => {
                                        println!("subscribing to {}", alm);
                                        if Server::subscribe(alm, &subscriptions, client.clone()).await{
                                            list_subscriptions.push(String::from(alm));
                                        }
                                    },
                                    Commands::Ack(alm) => {
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
                                    },
                                    Commands::GetAll => {
                                        let all_alms: Vec<_> = Server::get_all_subscribed(&list_subscriptions, &map_alm_status).await;

                                        let mut ret = String::new();

                                        for alm in all_alms.iter(){
                                            ret.push_str(&std::format!("{:?}\n", alm));
                                        }
                                        let _ = ws_write.send(Message::Text(ret)).await;
                                    }
                                    Commands::KeepAlive => {},
                                    Commands::Unknown => {
                                        eprint!("Got unknown command '{msg}' from {:?}", client.addr)
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

    fn parse_command(msg: &str) -> Commands {
        if msg == "::ka::" {
            return Commands::KeepAlive;
        } else if msg.starts_with("::subscribe::") {
            let alm = &msg[13..];
            return Commands::Subscribe(alm);
        } else if msg.starts_with("::ack::") {
            let alm = &msg[7..];
            return Commands::Ack(alm);
        } else if msg == "::ga::" {
            return Commands::GetAll;
        }

        Commands::Unknown
    }
}
