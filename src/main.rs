

fn main() {
    println!("Hello, world!");
    println!("config: {:?}", alarm_server::load_config("example/config.yaml"));
}
