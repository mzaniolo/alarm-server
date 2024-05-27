use alarm_server::{alarm, broker::Broker, config, server::Server};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or("examples/server_config.toml".to_string());

    println!("Using config '{config_path}'");

    let config = config::read_config(&config_path);
    run(config).await
}

async fn run(config: config::Config) {
    let alms = alarm::create_alarms(&config.alarm.path);

    let mut broker = Broker::new(config.broker);
    if let Err(e) = broker.connect().await {
        eprint!("Couldn't connect to rabbitMQ, {e}");
        return;
    }

    let mut reader = broker.create_reader().await.unwrap();

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
