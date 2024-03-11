fn main() {
    println!("Hello, world!");

    let config = alarm_server::load_config("example/config.yaml");
    println!("config: {:?}", config);

    let alms = alarm_server::create_alarms(config);

    println!("alarms: {:?}", alms)
}
