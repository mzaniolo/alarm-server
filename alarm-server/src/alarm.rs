use tokio::sync::{broadcast, mpsc};

#[derive(PartialEq, Debug, Clone)]
pub enum AlarmSeverity {
    High,
    Medium,
    Low,
}

#[derive(PartialEq, Debug, Clone)]
pub enum AlarmState {
    Set,
    Reset,
}

#[derive(Debug, Clone)]
pub struct AlarmStatus {
    pub id: u32,
    pub name: String,
    pub state: AlarmState,
    pub severity: AlarmSeverity,
    pub ack: bool,
}

#[derive(Debug)]
pub struct Alarm {
    id: u32,
    path: String,
    set: i64,
    reset: i64,
    meas: String,
    rx_meas: Option<broadcast::Receiver<i64>>,
    rx_ack: Option<mpsc::Receiver<bool>>,
    tx_publisher: Option<mpsc::Sender<AlarmStatus>>,

    status: AlarmStatus,
}

impl Alarm {
    pub fn new(
        id: u32,
        path: String,
        set: i64,
        reset: i64,
        severity: AlarmSeverity,
        meas: String,
    ) -> Self {
        Self {
            id,
            path: path.clone(),
            set,
            reset,
            meas,
            rx_meas: None,
            rx_ack: None,
            tx_publisher: None,
            status: AlarmStatus {
                id,
                name: path,
                state: AlarmState::Reset,
                severity,
                ack: false,
            },
        }
    }

    pub async fn run(&mut self) {
        let rx = self.rx_meas.as_mut().unwrap();
        let rx_ack = self.rx_ack.as_mut().unwrap();
        loop {
            tokio::select! {
                Ok(value) = rx.recv() => {
                    if value == self.set {
                        self.status.state = AlarmState::Set;
                        self.status.ack = false;
                    } else if value == self.reset {
                        self.status.state = AlarmState::Reset;
                    }
                    let _ = self
                            .tx_publisher
                            .as_ref()
                            .unwrap()
                            .send(self.status.clone())
                            .await;
                },
                Some(_) = rx_ack.recv() => {
                    self.status.ack = true;
                    let _ = self
                            .tx_publisher
                            .as_ref()
                            .unwrap()
                            .send(self.status.clone())
                            .await;
                }
            }
        }
    }

    pub fn subscribe(&mut self, rx: broadcast::Receiver<i64>) {
        self.rx_meas = Some(rx);
    }

    pub fn get_meas(&self) -> &str {
        &self.meas
    }
    pub fn set_notifier(&mut self, tx: mpsc::Sender<AlarmStatus>) {
        self.tx_publisher = Some(tx);
    }
    pub fn set_ack_listener(&mut self, rx: mpsc::Receiver<bool>) {
        self.rx_ack = Some(rx);
    }
    pub fn get_path(&self) -> &str {
        &self.path
    }
}
