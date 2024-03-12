use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let config = alarm_server::load_config("example/config.yaml");
    // println!("config: {:?}", config);

    let mut alms = alarm_server::create_alarms(config);

    // println!("alarms: {:?}", alms);

    let (tx, _) = broadcast::channel(20);

    let mut tasks: Vec<tokio::task::JoinHandle<_>> = Vec::new();

    for mut alm in alms.into_iter() {
        alm.subscribe(tx.subscribe());
        tasks.push(tokio::spawn(async move {
            alm.run().await;
        }));
    }

    println!("=== Send meas ===");

    tx.send(10);
    tx.send(0);
    tx.send(-1);
    tx.send(1);

    println!("=== wait tasks ===");

    for task in tasks.into_iter() {
        task.await;
    }
}
