use crate::db::DB;
use tokio::sync::mpsc;
use async_channel;
use chrono:: Utc;
use cache::Cache;

pub use alarm::{Alarm, AlarmSeverity, AlarmState, AlarmAck, AlarmTrigger, DigitalAlarm};

#[derive(Debug)]
pub struct AlarmHandler {
    rx_trg: async_channel::Receiver<String>,
    tx_publisher: mpsc::Sender<Alarm>,
    db: DB,
    cache: Cache,
}

impl AlarmHandler {
    pub fn new(rx_trg: async_channel::Receiver<String>, tx_publisher: mpsc::Sender<Alarm>, db: DB, cache: Cache) -> Self {
        Self {
            rx_trg,
            tx_publisher,
            db,
            cache,
        }
    }

    pub async fn run(&mut self) {

        while let Ok(value) = self.rx_trg.recv().await {
            let alm_trg: AlarmTrigger = serde_json::from_str(&value).unwrap();
            let digi_alm = self.cache.get_alm_config(&alm_trg.alarm).await.unwrap();

            if alm_trg.input == digi_alm.set {
                let status = Alarm {
                    name: digi_alm.name.clone(),
                    timestamp: Utc::now(),
                    value: alm_trg.input,
                    state: AlarmState::Set,
                    severity: digi_alm.severity,
                    ack: AlarmAck::NotAck,
                };
                Self::send_event(&self.tx_publisher, status.clone()).await;
                self.db.insert_alm(status).await;
            } else if alm_trg.input == digi_alm.reset {
                let alm = self
                    .db
                    .get_latest_alm(digi_alm.name.clone())
                    .await;
                match alm {
                    Some(alm) => {
                        if alm.state != AlarmState::Reset {
                            let status = Alarm {
                                name: digi_alm.name.clone(),
                                timestamp: Utc::now(),
                                value: digi_alm.reset,
                                state: AlarmState::Reset,
                                severity: digi_alm.severity,
                                ack: if alm.ack == AlarmAck::Ack {
                                    alm.ack
                                } else {
                                    AlarmAck::NotAck
                                },
                            };
                            Self::send_event(&self.tx_publisher, status.clone())
                                .await;
                            self.db.insert_alm(status).await;
                        }
                    }
                    _ => {
                        eprintln!("Error reading the last status of {}", digi_alm.name);
                    }
                };
            }
        }
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
