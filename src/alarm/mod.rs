use crate::db::DB;
use tokio::sync::{broadcast, mpsc};
use chrono:: Utc;

mod create_alarm;
pub use create_alarm::create_alarms;
pub use alarm::{Alarm, AlarmSeverity, AlarmState, AlarmAck};

#[derive(Debug)]
pub struct AlarmHandler {
    path: String,
    severity: AlarmSeverity,
    set: i64,
    reset: i64,
    meas: String,
    rx_meas: Option<broadcast::Receiver<i64>>,
    tx_publisher: Option<mpsc::Sender<Alarm>>,
    db: Option<DB>,
}

impl AlarmHandler {
    pub fn new(path: String, set: i64, reset: i64, severity: AlarmSeverity, meas: String) -> Self {
        Self {
            path: path.clone(),
            severity,
            set,
            reset,
            meas,
            rx_meas: None,
            tx_publisher: None,
            db: None,
        }
    }

    pub async fn run(&mut self) {
        let rx = self.rx_meas.as_mut().unwrap();

        while let Ok(value) = rx.recv().await {
            if value == self.set {
                let status = Alarm {
                    name: self.path.clone(),
                    timestamp: Utc::now(),
                    value: self.set,
                    state: AlarmState::Set,
                    severity: self.severity.clone(),
                    ack: AlarmAck::NotAck,
                };
                Self::send_event(self.tx_publisher.as_ref().unwrap(), status.clone()).await;
                self.db.as_ref().unwrap().insert_alm(status).await;
            } else if value == self.reset {
                let alm = self
                    .db
                    .as_ref()
                    .unwrap()
                    .get_latest_alm(self.path.clone())
                    .await;
                match alm {
                    Some(alm) => {
                        if alm.state != AlarmState::Reset {
                            let status = Alarm {
                                name: self.path.clone(),
                                timestamp: Utc::now(),
                                value: self.reset,
                                state: AlarmState::Reset,
                                severity: self.severity.clone(),
                                ack: if alm.ack == AlarmAck::Ack {
                                    alm.ack
                                } else {
                                    AlarmAck::NotAck
                                },
                            };
                            Self::send_event(self.tx_publisher.as_ref().unwrap(), status.clone())
                                .await;
                            self.db.as_ref().unwrap().insert_alm(status).await;
                        }
                    }
                    _ => {
                        // eprintln!("Error reading the last status of {}", self.path);
                    }
                };
            }
        }
    }

    pub fn subscribe(&mut self, rx: broadcast::Receiver<i64>) {
        self.rx_meas = Some(rx);
    }

    pub fn get_meas(&self) -> &str {
        &self.meas
    }
    pub fn set_notifier(&mut self, tx: mpsc::Sender<Alarm>) {
        self.tx_publisher = Some(tx);
    }
    pub fn set_db(&mut self, db: DB) {
        self.db = Some(db);
    }
    pub fn get_path(&self) -> &str {
        &self.path
    }

    async fn send_event(tx: &mpsc::Sender<Alarm>, status: Alarm) {
        let _ = tx.send(status).await;
    }
}

pub async fn process_ack(
    mut rx_ack: mpsc::Receiver<String>,
    tx_publisher: mpsc::Sender<Alarm>,
    db: DB,
) {
    while let Some(path) = rx_ack.recv().await {
        db.send_ack(&path).await;

        let status = Alarm {
            name: path,
            timestamp: Utc::now(),
            value: i64::MAX,
            state: AlarmState::Reset,
            severity: AlarmSeverity::Low,
            ack: AlarmAck::Ack,
        };
        AlarmHandler::send_event(&tx_publisher, status).await;
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use tokio::time::{sleep, Duration};

    fn alm_setup(
        path: &str,
        set: i64,
        reset: i64,
        severity: &AlarmSeverity,
    ) -> (
        tokio::task::JoinHandle<()>,
        tokio::sync::mpsc::Receiver<Alarm>,
        tokio::sync::broadcast::Sender<i64>,
    ) {
        let mut alm = AlarmHandler::new(
            path.to_string(),
            set,
            reset,
            severity.clone(),
            "my_meas".to_string(),
        );

        let (tx_alm, rx_alm) = mpsc::channel(1);
        let (tx_meas, rx_meas) = broadcast::channel(1);

        alm.set_notifier(tx_alm);
        alm.subscribe(rx_meas);

        let task = tokio::spawn(async move {
            alm.run().await;
        });

        (task, rx_alm, tx_meas)
    }

    async fn try_receive(rx: &mut mpsc::Receiver<Alarm>) -> Option<Alarm> {
        let mut alm = rx.try_recv();
        let mut i = 5;
        while let Err(_) = alm {
            if i < 0 {
                break;
            }
            alm = rx.try_recv();
            i -= 1;
            sleep(Duration::from_millis(100)).await
        }

        if alm.is_ok() {
            return Some(alm.unwrap());
        }

        None
    }

    fn assert_alm(
        alm_status: &Alarm,
        alm_state: &AlarmState,
        alm_sev: &AlarmSeverity,
        alm_name: &str,
        alm_ack: &AlarmAck,
    ) {
        assert_eq!(alm_status.state, *alm_state);
        assert_eq!(alm_status.severity, *alm_sev);
        assert_eq!(alm_status.name, alm_name);
        assert_eq!(alm_status.ack, *alm_ack);
    }

}
