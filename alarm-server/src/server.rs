use crate::alarm::{self, AlarmState};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

pub mod client;

const CHANNEL_SIZE: usize = 5;

pub type Subscriptions = Arc<Mutex<HashMap<String, HashSet<client::Client>>>>;
pub type MapAck = Arc<Mutex<HashMap<String, mpsc::Sender<bool>>>>;
pub type MapAlmStatus = Arc<Mutex<HashMap<u32, alarm::AlarmStatus>>>;

pub struct Server {
    addr: String,
    port: u16,

    clients: Vec<client::Client>,
    subscriptions: Subscriptions,
    map_ack: MapAck,
    map_alm_status: MapAlmStatus,
}

impl Server {
    pub fn new(addr: Option<String>, port: Option<u16>) -> Self {
        Self {
            addr: addr.unwrap_or(String::from("127.0.0.1")),
            port: port.unwrap_or(8080),
            clients: Vec::new(),
            subscriptions: Subscriptions::default(),
            map_ack: MapAck::default(),
            map_alm_status: MapAlmStatus::default(),
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
            let client = client::Client { addr, tx };
            self.clients.push(client.clone());
            tokio::spawn(client::Client::handle_client(
                stream,
                client,
                rx,
                Arc::clone(&self.subscriptions),
                Arc::clone(&self.map_ack),
            ));
        }
        println!("Server started");
    }

    async fn subscribe(alm: &str, subscriptions: &Subscriptions, client: client::Client) {
        subscriptions
            .lock()
            .await
            .entry(String::from(alm))
            .or_default()
            .insert(client);
    }

    pub async fn listen_alarms(
        mut rx: mpsc::Receiver<alarm::AlarmStatus>,
        subscriptions: Subscriptions,
        map_alm_status: MapAlmStatus,
    ) {
        while let Some(alm) = rx.recv().await {
            println!("got notified by alarm: {:?}", alm);

            if let Some(clients) = subscriptions.lock().await.get(&alm.name) {
                for client in clients.iter() {
                    let _ = client.tx.send(std::format!("{:#?}", alm)).await;
                }
            }

            if alm.state == AlarmState::Reset && alm.ack {
                // If the alarm was reset and ack we just don't care about it anymore
                map_alm_status.lock().await.remove(&alm.id);
            } else {
                map_alm_status
                    .lock()
                    .await
                    .entry(alm.id)
                    .or_insert(alm.clone());
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
