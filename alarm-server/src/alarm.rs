use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub enum AlarmSeverity {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub enum AlarmState {
    Set(i64),
    Reset(i64),
}

#[derive(Debug, Clone)]
pub struct AlarmStatus {
    pub status: AlarmState,
    pub severity: AlarmSeverity,
    pub meas: String,
}

#[derive(Debug)]
pub struct Alarm {
    path: String,
    set: i64,
    reset: i64,
    severity: AlarmSeverity,
    meas: String,
    rx_meas: Option<broadcast::Receiver<i64>>,
    rx_ack:Option<mpsc::Receiver<bool>>,
    tx_publisher: Option<mpsc::Sender<AlarmStatus>>,
}

impl Alarm {
    pub fn new(path: String, set: i64, reset: i64, severity: AlarmSeverity, meas: String) -> Self {
        Self {
            path,
            set,
            reset,
            severity,
            meas,
            rx_meas: None,
            rx_ack: None,
            tx_publisher: None,
        }
    }

    pub async fn run(&mut self) {
        let rx = self.rx_meas.as_mut().unwrap();
        while let Ok(value) = rx.recv().await {
            if value == self.set {
                let _ = self
                    .tx_publisher
                    .as_ref()
                    .unwrap()
                    .send(AlarmStatus {
                        status: AlarmState::Set(value),
                        severity: self.severity.clone(),
                        meas: self.path.clone(),
                    })
                    .await;
            } else if value == self.reset {
                let _ = self
                    .tx_publisher
                    .as_ref()
                    .unwrap()
                    .send(AlarmStatus {
                        status: AlarmState::Reset(value),
                        severity: self.severity.clone(),
                        meas: self.path.clone(),
                    })
                    .await;
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
}
