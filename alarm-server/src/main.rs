use alarm_server::{alarm, broker::Broker, config, db};
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
    let _ = reader.connect().await;

    let mut writer = broker.create_writer().await.unwrap();
    let _ = writer.connect().await;

    let db = db::DB::new(config.db);

    let (alm_tx, alm_rx) = mpsc::channel(100);

    let mut tasks: Vec<tokio::task::JoinHandle<_>> = Vec::new();

    for mut alm in alms.into_iter() {
        alm.subscribe(reader.subscribe(alm.get_meas()).await);
        alm.set_notifier(alm_tx.clone());
        alm.set_db(db.clone());

        tasks.push(tokio::spawn(async move {
            alm.run().await;
        }));
    }

    writer.set_channel(alm_rx);

    let (ack_tx, ack_rx) = mpsc::channel(100);
    tokio::spawn(async move {
        alarm::process_ack(ack_rx, alm_tx, db).await;
    });
    reader.set_ack_channel(ack_tx);

    println!("=== Set reader to receive ===");
    tokio::spawn(async move {
        reader.receive().await;
    });

    println!("=== Running writer ===");
    writer.write().await;
}
