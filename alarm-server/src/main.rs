use alarm_server::{alarm, config, reader::Reader, server::Server};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    let config = config::read_config("examples/server_config.toml");

    let alms = alarm::create_alarms(&config.alarm.path);

    let mut reader = Reader::new(config.broker);
    if let Err(e) = reader.connect().await {
        eprint!("Couldn't connect to rabbitMQ, {e}");
        return;
    }

    let (tx_alm, rx_alm) = mpsc::channel(100);

    let mut server = Server::new(config.server);

    let mut tasks: Vec<tokio::task::JoinHandle<_>> = Vec::new();
    let map_ack = server.get_map_ack();
    {
        let mut map_ack = map_ack.lock().await;

        for mut alm in alms.into_iter() {
            alm.subscribe(reader.subscribe(alm.get_meas()).await);
            alm.set_notifier(tx_alm.clone());

            let (tx_ack, rx_ack) = mpsc::channel(2);
            alm.set_ack_listener(rx_ack);
            if let Some(_) = map_ack.insert(String::from(alm.get_path()), tx_ack) {
                panic!("Got duplicated alarm {}", alm.get_path());
            }

            tasks.push(tokio::spawn(async move {
                alm.run().await;
            }));
        }
    }

    println!("=== Set reader to receive ===");
    tokio::spawn(async move {
        reader.receive().await;
    });

    println!("=== wait tasks ===");

    let subscriptions = server.get_subscriptions();
    let map_alm = server.get_map_alm();

    tokio::spawn(async move { Server::listen_alarms(rx_alm, subscriptions, map_alm).await });

    server.connect().await;
}
