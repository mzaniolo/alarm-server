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
    rx: Option<broadcast::Receiver<i64>>,
    tx: Option<mpsc::Sender<AlarmStatus>>,
}

impl Alarm {
    pub fn new(path: String, set: i64, reset: i64, severity: AlarmSeverity, meas: String) -> Self {
        Self {
            path,
            set,
            reset,
            severity,
            meas,
            rx: None,
            tx: None,
        }
    }

    pub async fn run(&mut self) {
        let rx = self.rx.as_mut().unwrap();
        while let Ok(value) = rx.recv().await {
            if value == self.set {
                // println!(
                //     "alarm {} is set with severity '{:?}' - value: {value}!",
                //     self.path, self.severity
                // );
                let _ = self
                    .tx
                    .as_ref()
                    .unwrap()
                    .send(AlarmStatus {
                        status: AlarmState::Set(value),
                        severity: self.severity.clone(),
                        meas: self.path.clone(),
                    })
                    .await;
            } else if value == self.reset {
                // println!(
                //     "alarm {} is reset with severity '{:?}' - value: {value}!",
                //     self.path, self.severity
                // );
                let _ = self
                    .tx
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
        self.rx = Some(rx);
    }

    pub fn get_meas(&self) -> &str {
        &self.meas
    }
    pub fn set_notifier(&mut self, tx: mpsc::Sender<AlarmStatus>) {
        self.tx = Some(tx);
    }
}
