use alarm_server::{publisher::Publisher, reader::Reader};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let config = alarm_server::load_config("examples/config.yaml");

    let alms = alarm_server::create_alarms(config);

    let mut reader = Reader::new(None, None, None, None);
    if let Err(e) = reader.connect().await {
        eprint!("Couldn't connect to rabbitMQ, {e}");
        return;
    }

    let (tx_alm, rx_alm) = mpsc::channel(100);

    let mut tasks: Vec<tokio::task::JoinHandle<_>> = Vec::new();

    for mut alm in alms.into_iter() {
        alm.subscribe(reader.subscribe(alm.get_meas()).await);
        alm.set_notifier(tx_alm.clone());
        tasks.push(tokio::spawn(async move {
            alm.run().await;
        }));
    }

    println!("=== Set reader to receive ===");
    tokio::spawn(async move {
        reader.receive().await;
    });

    println!("=== wait tasks ===");

    let mut publisher = Publisher::new(None, None);

    let subscriptions = publisher.get_subscriptions();

    tokio::spawn(async move { Publisher::listen_alarms(rx_alm, subscriptions).await });

    publisher.connect().await;
}
