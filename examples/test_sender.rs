use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, ExchangeDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
    BasicProperties,
};
use std::{str, thread, time};
use tokio::io::Error as TError;

#[tokio::main]
async fn main() -> Result<(), Box<TError>> {
    let conn = Connection::open(&OpenConnectionArguments::new(
        "localhost",
        5672,
        "guest",
        "guest",
    ))
    .await
    .unwrap();
    conn.register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    let ch = conn.open_channel(None).await.unwrap();
    ch.register_callback(DefaultChannelCallback).await.unwrap();

    let x_name = "meas_exchange";
    let x_type = "direct";
    let x_args = ExchangeDeclareArguments::new(x_name, x_type)
        .durable(true)
        .finish();
    ch.exchange_declare(x_args).await.unwrap();

    let routing_keys: Vec<_> = vec!["my_path.my_meas", "my_path2.my_meas"];

    let mut value: i8 = 0;
    let mut flag = false;

    let sleep_time = time::Duration::from_millis(500);
    loop {
        for meas in routing_keys.iter() {
            value = value.overflowing_add(1).0;
            if value & 0b100 == 0b100 {
                value = value.overflowing_add(1).0;
            }
            flag = !flag;

            let sign = if flag { "-" } else { "" };
            let payload = std::format!("{}{}", sign, value & 0b11);

            let publish_args = BasicPublishArguments::new(x_name, meas);
            // publish messages as persistent
            let props = BasicProperties::default().with_delivery_mode(2).finish();
            ch.basic_publish(props, payload.clone().into_bytes(), publish_args)
                .await
                .unwrap();

            println!(
                " [x] Sent {}:{:?}",
                meas,
                str::from_utf8(payload.as_bytes()).unwrap()
            );

            thread::sleep(sleep_time);
        }
        thread::sleep(sleep_time);
    }

    // ch.close().await.unwrap();
    // conn.close().await.unwrap();

    Ok(())
}
