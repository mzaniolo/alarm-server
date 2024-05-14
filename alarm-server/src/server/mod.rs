use crate::alarm::{self, AlarmAck, AlarmState};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};

pub mod client;

const CHANNEL_SIZE: usize = 5;

pub type Subscriptions = Arc<Mutex<HashMap<String, HashSet<client::Client>>>>;
pub type MapAck = Arc<Mutex<HashMap<String, mpsc::Sender<bool>>>>;
pub type MapAlmStatus = Arc<Mutex<HashMap<String, alarm::AlarmStatus>>>;

pub struct Server {
    addr: String,
    port: u16,

    subscriptions: Subscriptions,
    map_ack: MapAck,
    map_alm_status: MapAlmStatus,
}

impl Server {
    pub fn new(addr: Option<String>, port: Option<u16>) -> Self {
        Self {
            addr: addr.unwrap_or(String::from("127.0.0.1")),
            port: port.unwrap_or(8080),
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
            tokio::spawn(client::Client::handle_client(
                stream,
                client,
                rx,
                Arc::clone(&self.subscriptions),
                Arc::clone(&self.map_ack),
                Arc::clone(&self.map_alm_status),
            ));
        }
        println!("Server started");
    }

    async fn subscribe(alm: &str, subscriptions: &Subscriptions, client: client::Client) -> bool {
        subscriptions
            .lock()
            .await
            .entry(String::from(alm))
            .or_default()
            .insert(client)
    }

    pub async fn listen_alarms(
        mut rx: mpsc::Receiver<alarm::AlarmStatus>,
        subscriptions: Subscriptions,
        map_alm_status: MapAlmStatus,
    ) {
        while let Some(alm) = rx.recv().await {
            println!("got notified by alarm: {:?}", alm);

            if let Some(clients) = subscriptions.lock().await.get(&alm.name) {
                let alm_json = serde_json::to_string(&alm).unwrap();
                for client in clients.iter() {
                    let _ = client.tx.send(alm_json.clone()).await;
                }
            }

            if alm.state == AlarmState::Reset && alm.ack == AlarmAck::None {
                // If the alarm was reset and ack we just don't care about it anymore
                map_alm_status.lock().await.remove(&alm.name);
            } else {
                map_alm_status
                    .lock()
                    .await
                    .entry(alm.name.clone())
                    .or_insert(alm);
            }
        }
    }

    pub fn get_subscriptions(&self) -> Subscriptions {
        Arc::clone(&self.subscriptions)
    }

    pub fn get_map_ack(&self) -> MapAck {
        Arc::clone(&self.map_ack)
    }

    pub fn get_map_alm(&self) -> MapAlmStatus {
        Arc::clone(&self.map_alm_status)
    }

    async fn get_all_subscribed(
        subscriptions: &Vec<String>,
        map_alm_status: &MapAlmStatus,
    ) -> Vec<alarm::AlarmStatus> {
        let mut ret = Vec::new();
        let map = map_alm_status.lock().await;
        for subs in subscriptions.iter() {
            if let Some(status) = map.get(subs) {
                ret.push(status.clone())
            }
        }

        ret
    }
}
