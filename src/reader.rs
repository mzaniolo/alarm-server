use amqprs::{
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback}, channel::Channel, connection::{Connection, OpenConnectionArguments}
};
struct Reader{
    host: &'static str,
    port: u16,
    username: &'static str,
    password: &'static str,

    channel: Option<Channel>
}


impl Reader{
    pub fn new(host: Option<&'static str>, port: Option<u16>, username: Option<&'static str>, password: Option<&'static str>) -> Reader {
        Reader{
            host: host.unwrap_or("localhost"),
            port: port.unwrap_or(5672),
            username: username.unwrap_or("guest"),
            password: password.unwrap_or("guest"),
            channel: None
        }
    }


    pub async fn connect(&mut self) -> Result<(), _> {
        let connection = Connection::open(&OpenConnectionArguments::new(
            self.host,
            self.port,
            self.username,
            self.password,
        ))
        .await?;

        connection
        .register_callback(DefaultConnectionCallback)
        .await?;

        // open a channel on the connection
        self.channel = Some(connection.open_channel(None).await?);
        self.channel.unwrap()
            .register_callback(DefaultChannelCallback)
            .await?;
    }
}
