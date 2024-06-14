use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, Channel, ExchangeDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
    BasicProperties,
};
use std::{str, thread, time};
use tokio::io::Error as TError;

#[tokio::main]
async fn main() -> Result<(), Box<TError>> {
    let conn_args = OpenConnectionArguments::new("localhost", 5672, "guest", "guest");

    let conn = Connection::open(&conn_args).await.unwrap();
    conn.register_callback(DefaultConnectionCallback)
        .await
        .unwrap();

    let ch = conn.open_channel(None).await.unwrap();
    ch.register_callback(DefaultChannelCallback).await.unwrap();

    let x_name_meas = "meas_exchange";
    let x_type = "direct";
    let x_args = ExchangeDeclareArguments::new(x_name_meas, x_type)
        .durable(true)
        .finish();
    ch.exchange_declare(x_args).await.unwrap();

    let x_name_ack = "ack_exchange";
    let x_type = "direct";
    let x_args = ExchangeDeclareArguments::new(x_name_ack, x_type)
        .durable(true)
        .finish();
    ch.exchange_declare(x_args).await.unwrap();

    let routing_keys: Vec<_> = vec!["my_path.my_meas", "my_path2.my_meas", "my_path.my_meas"];

    let mut value: i64 = 0;
    let mut flag = false;

    let alms = vec!["sub1/alarm1", "sub1/alarm2", "sub2/alarm1", "sub2/alarm2"];
    let mut alms_iter = alms.iter().cycle();

    let sleep_time = time::Duration::from_millis(800);
    loop {
        for meas in routing_keys.iter() {
            value += 1;
            if value & 0b100 == 0b100 {
                value += 1;
            }
            flag = !flag;

            let sign = if flag { "-" } else { "" };
            let payload = std::format!("{}{}", sign, value & 0b11);

            let publish_args = BasicPublishArguments::new(x_name_meas, meas);
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
        send_ack(&ch, x_name_ack, alms_iter.next().as_ref().unwrap()).await;
        thread::sleep(sleep_time);
    }

    // ch.close().await.unwrap();
    // conn.close().await.unwrap();

    Ok(())
}

async fn send_ack(ch: &Channel, x_name: &str, alm: &str) {
    let publish_args = BasicPublishArguments::new(x_name, "ack");
    // publish messages as persistent
    let props = BasicProperties::default().with_delivery_mode(2).finish();
    ch.basic_publish(props, alm.to_string().into_bytes(), publish_args)
        .await
        .unwrap();

    println!(" [x] Sent ack to {}", alm);
}
