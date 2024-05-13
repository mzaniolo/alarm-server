use serde::Serialize;
use tokio::sync::{broadcast, mpsc};

mod create_alarm;
pub use create_alarm::create_alarms;

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum AlarmSeverity {
    High,
    Medium,
    Low,
}

#[derive(Serialize, PartialEq, Debug, Clone)]
pub enum AlarmState {
    Set,
    Reset,
}

#[derive(Serialize, Debug, Clone)]
pub struct AlarmStatus {
    pub name: String,
    pub state: AlarmState,
    pub severity: AlarmSeverity,
    pub ack: bool,
}

#[derive(Debug)]
pub struct Alarm {
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
    pub fn new(path: String, set: i64, reset: i64, severity: AlarmSeverity, meas: String) -> Self {
        Self {
            path: path.clone(),
            set,
            reset,
            meas,
            rx_meas: None,
            rx_ack: None,
            tx_publisher: None,
            status: AlarmStatus {
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
                    let mut updated = false;
                    if value == self.set {
                        self.status.state = AlarmState::Set;
                        self.status.ack = false;
                        updated = true;
                    } else if value == self.reset && self.status.state != AlarmState::Reset{
                        self.status.state = AlarmState::Reset;
                        updated = true;
                    }

                    if updated{
                        let _ = self
                            .tx_publisher
                            .as_ref()
                            .unwrap()
                            .send(self.status.clone())
                            .await;
                    }
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

        assert_eq!(alm_status.state, AlarmState::Set);
        assert_eq!(alm_status.severity, alm_sev);
        assert_eq!(alm_status.name, alm_path);
        assert!(!alm_status.ack);

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

        assert_eq!(alm_status.state, AlarmState::Set);
        assert_eq!(alm_status.severity, alm_sev);
        assert_eq!(alm_status.name, alm_path);
        assert!(!alm_status.ack);

        tx_meas.send(alm_reset)?;
        let alm_status = rx_alm.recv().await;

        assert!(alm_status.is_some());

        let alm_status = alm_status.unwrap();

        assert_eq!(alm_status.state, AlarmState::Reset);
        assert_eq!(alm_status.severity, alm_sev);
        assert_eq!(alm_status.name, alm_path);
        assert!(!alm_status.ack);

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

        assert_eq!(alm_status.state, AlarmState::Set);
        assert_eq!(alm_status.severity, alm_sev);
        assert_eq!(alm_status.name, alm_path);
        assert!(!alm_status.ack);

        task.abort();

        Ok(())
    }
}
