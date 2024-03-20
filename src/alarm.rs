use tokio::sync::broadcast;

#[derive(Debug)]
pub enum AlarmSeverity {
    High,
    Medium,
    Low,
}

#[derive(Debug)]
pub struct Alarm {
    path: String,
    set: i64,
    reset: i64,
    severity: AlarmSeverity,
    meas: String,
    rx: Option<broadcast::Receiver<i64>>,
}

impl Alarm {
    pub fn new(
        path: String,
        set: i64,
        reset: i64,
        severity: AlarmSeverity,
        meas: String,
        rx: Option<broadcast::Receiver<i64>>,
    ) -> Self {
        Self {
            path: path,
            set: set,
            reset: reset,
            severity: severity,
            meas: meas,
            rx: rx,
        }
    }

    pub async fn run(&mut self) {
        println!("init run");
        let rx = self.rx.as_mut().unwrap();
        println!("waiting on channel!");
        while let Ok(value) = rx.recv().await {
            if value == self.set {
                println!(
                    "alarm {} is set with severity '{:?}' - value: {value}!",
                    self.path, self.severity
                );
            } else if value == self.reset {
                println!(
                    "alarm {} is reset with severity '{:?}' - value: {value}!",
                    self.path, self.severity
                );
            }
        }
    }

    pub fn subscribe(&mut self, rx: broadcast::Receiver<i64>) {
        self.rx = Some(rx);
    }
}
