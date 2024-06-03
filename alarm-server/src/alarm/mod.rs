use crate::db::DB;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};

mod create_alarm;
pub use create_alarm::create_alarms;

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum AlarmSeverity {
    High = 3,
    Medium = 2,
    Low = 1,
    Unknown = 0,
}

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum AlarmState {
    Set,
    Reset,
    Unknown,
}

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum AlarmAck {
    Ack,
    NotAck,
    None,
}

#[derive(Serialize, Debug, Clone)]
pub struct AlarmStatus {
    pub name: String,
    pub state: AlarmState,
    pub value: i64,
    pub severity: AlarmSeverity,
    pub ack: AlarmAck,
}

#[derive(Debug)]
pub struct Alarm {
    path: String,
    severity: AlarmSeverity,
    set: i64,
    reset: i64,
    meas: String,
    rx_meas: Option<broadcast::Receiver<i64>>,
    tx_publisher: Option<mpsc::Sender<AlarmStatus>>,
    db: Option<DB>,
}

impl Alarm {
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
                let status = AlarmStatus {
                    name: self.path.clone(),
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
                            let status = AlarmStatus {
                                name: self.path.clone(),
                                value: self.reset,
                                state: AlarmState::Reset,
                                severity: self.severity.clone(),
                                ack: if alm.ack == AlarmAck::Ack {
                                    alm.ack
                                } else {
                                    AlarmAck::None
                                },
                            };
                            Self::send_event(self.tx_publisher.as_ref().unwrap(), status.clone())
                                .await;
                            self.db.as_ref().unwrap().insert_alm(status).await;
                        }
                    }
                    _ => {
                        eprintln!("Error reading the last status of {}", self.path);
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
    pub fn set_notifier(&mut self, tx: mpsc::Sender<AlarmStatus>) {
        self.tx_publisher = Some(tx);
    }
    pub fn set_db(&mut self, db: DB) {
        self.db = Some(db);
    }
    pub fn get_path(&self) -> &str {
        &self.path
    }

    async fn send_event(tx: &mpsc::Sender<AlarmStatus>, status: AlarmStatus) {
        let _ = tx.send(status).await;
    }
}

pub async fn process_ack(
    mut rx_ack: mpsc::Receiver<String>,
    tx_publisher: mpsc::Sender<AlarmStatus>,
    db: DB,
) {
    while let Some(path) = rx_ack.recv().await {
        db.send_ack(&path).await;

        let status = AlarmStatus {
            name: path,
            value: i64::MAX,
            state: AlarmState::Unknown,
            severity: AlarmSeverity::Unknown,
            ack: AlarmAck::Ack,
        };
        Alarm::send_event(&tx_publisher, status).await;
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
        tokio::sync::mpsc::Receiver<AlarmStatus>,
        tokio::sync::mpsc::Sender<bool>,
        tokio::sync::broadcast::Sender<i64>,
    ) {
        let mut alm = Alarm::new(
            path.to_string(),
            set,
            reset,
            severity.clone(),
            "my_meas".to_string(),
        );

        let (tx_alm, rx_alm) = mpsc::channel(1);
        let (tx_ack, rx_ack) = mpsc::channel(1);
        let (tx_meas, rx_meas) = broadcast::channel(1);

        alm.set_notifier(tx_alm);
        alm.subscribe(rx_meas);
        alm.set_ack_listener(rx_ack);

        let task = tokio::spawn(async move {
            alm.run().await;
        });

        (task, rx_alm, tx_ack, tx_meas)
    }

    async fn try_receive(rx: &mut mpsc::Receiver<AlarmStatus>) -> Option<AlarmStatus> {
        let mut alm_status = rx.try_recv();
        let mut i = 5;
        while let Err(_) = alm_status {
            if i < 0 {
                break;
            }
            alm_status = rx.try_recv();
            i -= 1;
            sleep(Duration::from_millis(100)).await
        }

        if alm_status.is_ok() {
            return Some(alm_status.unwrap());
        }

        None
    }

    fn assert_alm(
        alm_status: &AlarmStatus,
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

    #[tokio::test]
    async fn test_set() -> Result<(), Box<dyn std::error::Error>> {
        let alm_path = "my_path";
        let alm_set = 1;
        let alm_sev = AlarmSeverity::High;

        let (task, mut rx_alm, _tx_ack, tx_meas) = alm_setup(alm_path, alm_set, 99, &alm_sev);

        tx_meas.send(alm_set)?;
        let alm_status = rx_alm.recv().await;

        assert!(alm_status.is_some());

        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Set,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        task.abort();

        Ok(())
    }

    #[tokio::test]
    async fn test_set_reset() -> Result<(), Box<dyn std::error::Error>> {
        let alm_path = "my_path";
        let alm_set = 1;
        let alm_reset = 2;
        let alm_sev = AlarmSeverity::High;

        let (task, mut rx_alm, _tx_ack, tx_meas) =
            alm_setup(alm_path, alm_set, alm_reset, &alm_sev);

        tx_meas.send(alm_set)?;
        let alm_status = rx_alm.recv().await;

        assert!(alm_status.is_some());

        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Set,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        tx_meas.send(alm_reset)?;
        let alm_status = rx_alm.recv().await;

        assert!(alm_status.is_some());

        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Reset,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        task.abort();

        Ok(())
    }

    #[tokio::test]
    async fn test_reset() -> Result<(), Box<dyn std::error::Error>> {
        let alm_path = "my_path";
        let alm_set = 1;
        let alm_reset = 2;
        let alm_sev = AlarmSeverity::High;

        let (task, mut rx_alm, _tx_ack, tx_meas) =
            alm_setup(alm_path, alm_set, alm_reset, &alm_sev);

        tx_meas.send(99)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());

        tx_meas.send(42)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());

        tx_meas.send(alm_reset)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());

        tx_meas.send(alm_set)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Set,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        task.abort();

        Ok(())
    }

    #[tokio::test]
    async fn test_ack_no_updates() -> Result<(), Box<dyn std::error::Error>> {
        let alm_path = "my_path";
        let alm_set = 1;
        let alm_reset = 2;
        let alm_sev = AlarmSeverity::High;

        let (task, mut rx_alm, tx_ack, tx_meas) = alm_setup(alm_path, alm_set, alm_reset, &alm_sev);

        // Nothing to ack so no updates
        tx_meas.send(99)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());
        tx_ack.send(true).await?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());

        // Nothing to ack so no updates
        tx_meas.send(alm_reset)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_none());

        task.abort();

        Ok(())
    }

    #[tokio::test]
    async fn test_ack() -> Result<(), Box<dyn std::error::Error>> {
        let alm_path = "my_path";
        let alm_set = 1;
        let alm_reset = 2;
        let alm_sev = AlarmSeverity::High;

        let (task, mut rx_alm, tx_ack, tx_meas) = alm_setup(alm_path, alm_set, alm_reset, &alm_sev);

        tx_meas.send(alm_set)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Set,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        tx_ack.send(true).await?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Set,
            &alm_sev,
            alm_path,
            &AlarmAck::Ack,
        );

        tx_meas.send(alm_reset)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Reset,
            &alm_sev,
            alm_path,
            &AlarmAck::None,
        );

        tx_meas.send(alm_set)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Set,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        tx_meas.send(alm_reset)?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Reset,
            &alm_sev,
            alm_path,
            &AlarmAck::NotAck,
        );

        tx_ack.send(true).await?;
        let alm_status = try_receive(&mut rx_alm).await;
        assert!(alm_status.is_some());
        let alm_status = alm_status.unwrap();

        assert_alm(
            &alm_status,
            &AlarmState::Reset,
            &alm_sev,
            alm_path,
            &AlarmAck::None,
        );

        task.abort();

        Ok(())
    }
}
