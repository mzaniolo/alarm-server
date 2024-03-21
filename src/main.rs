use alarm_server::reader::Reader;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let config = alarm_server::load_config("examples/config.yaml");
    // println!("config: {:?}", config);

    let alms = alarm_server::create_alarms(config);

    // println!("alarms: {:?}", alms);

    let mut reader = Reader::new(None, None, None, None);
    if let Err(e) = reader.connect().await {
        panic!("Couldn't connect to rabbitMQ, {e}")
    }

    let mut tasks: Vec<tokio::task::JoinHandle<_>> = Vec::new();

    for mut alm in alms.into_iter() {
        alm.subscribe(reader.subscribe(alm.get_meas()).await);
        tasks.push(tokio::spawn(async move {
            alm.run().await;
        }));
    }

    println!("=== Set reader to receive ===");
    tokio::spawn(async move {
        reader.receive().await;
    });

    println!("=== wait tasks ===");

    let guard = Notify::new();
    guard.notified().await;
}
