use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;

const CHANNEL_SIZE: usize = 5;

struct Client {
    addr: SocketAddr,
    tx: mpsc::Sender<String>,
}

pub struct Publisher {
    addr: String,
    port: u16,

    listener: Option<TcpListener>,
    clients: Vec<Client>,
}

impl Publisher {
    pub async fn new(addr: Option<String>, port: Option<u16>) -> Self {
        Self {
            addr: addr.unwrap_or(String::from("127.0.0.1")),
            port: port.unwrap_or(8080),
            listener: None,
            clients: Vec::new(),
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
            self.clients.push(Client{addr, tx});
            tokio::spawn(Self::handle_client(stream, addr, rx));
        }
        println!("Server started");
    }

    async fn handle_client(
        raw_stream: TcpStream,
        addr: SocketAddr,
        mut rx: mpsc::Receiver<String>,
    ) {
        println!("Incoming TCP connection from: {}", addr);

        let ws_stream = tokio_tungstenite::accept_async(raw_stream)
            .await
            .expect("Error during the websocket handshake occurred");
        println!("WebSocket connection established: {}", addr);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        loop {
            tokio::select! {
                msg = ws_read.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg.unwrap();
                            if msg.is_text() ||msg.is_binary() {
                                println!("got from {addr} the message: {}", msg.to_string());
                            } else if msg.is_close() {
                                eprintln!("Connection closed by client {addr}");
                            }
                        }
                        _ => {
                            eprintln!("Something went wrong with client {addr}");
                        }
                    }

                }
                val = rx.recv() => {
                    if let Some(val) = val{
                        println!("notifying client of val {val}");
                        ws_write.send(Message::Text(val)).await;
                    }
                }
            }
        }

        // Ok(())
    }
}
