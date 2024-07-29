use alarm_server::{alarm::{self, AlarmHandler}, broker::Broker, config, db};
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

    let (alm_tx, alm_rx) = mpsc::channel(100);
    let (trg_tx, trg_rx) = async_channel::bounded(100);

    let mut broker = Broker::new(config.broker);
    if let Err(e) = broker.connect().await {
        eprint!("Couldn't connect to rabbitMQ, {e}");
        return;
    }

    let mut reader = broker.create_reader().await.unwrap();
    let _ = reader.connect().await;
    reader.set_alm_channel(trg_tx);

    let mut writer = broker.create_writer().await.unwrap();
    let _ = writer.connect().await;
    writer.set_channel(alm_rx);

    let db = db::DB::new(config.db);
    db.try_create_table().await;

    let cache = cache::Cache::new().await;

    let mut tasks: Vec<tokio::task::JoinHandle<_>> = Vec::new();

    for _ in 0..10 {
        let mut alm = AlarmHandler::new(
            trg_rx.clone(),
            alm_tx.clone(),
            db.clone(),
            cache.clone()
        );

        tasks.push(tokio::spawn(async move {
            alm.run().await;
        }));
    }


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
