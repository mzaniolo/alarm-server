extern crate yaml_rust;
use std::fs;
use tokio::sync::broadcast;
use yaml_rust::{Yaml, YamlLoader};

pub fn load_config(path: &str) -> Yaml {
    let source = fs::read_to_string(path).unwrap();
    YamlLoader::load_from_str(&source).unwrap().remove(0)
}

#[derive(Debug)]
pub enum AlarmSevetity {
    High,
    Medium,
    Low,
}

#[derive(Debug)]
pub struct Alarm {
    path: String,
    set: i64,
    reset: i64,
    sevetity: AlarmSevetity,
    meas: String,
    rx: Option<broadcast::Receiver<i64>>,
}

impl Alarm {
    pub async fn run(&mut self) {
        println!("init run");
        let rx = self.rx.as_mut().unwrap();
        println!("waiting on channel!");
        while let Ok(value) = rx.recv().await {
            if value == self.set {
                println!(
                    "alarm {} is set with severity '{:?}' - value: {value}!",
                    self.path, self.sevetity
                );
            } else if value == self.reset {
                println!(
                    "alarm {} is reset with severity '{:?}' - value: {value}!",
                    self.path, self.sevetity
                );
            }
        }
    }

    pub fn subscribe(&mut self, rx: broadcast::Receiver<i64>) {
        self.rx = Some(rx);
    }
}

pub fn create_alarms(config: Yaml) -> Vec<Alarm> {
    let mut alarmes: Vec<Alarm> = Vec::new();

    if let Yaml::Hash(ref h) = config {
        for (area, alm) in h {
            if let Yaml::Hash(h) = alm {
                for (alm, values) in h {
                    alarmes.push(Alarm {
                        path: format!("{}/{}", area.as_str().unwrap(), area.as_str().unwrap()),
                        set: values["set"].as_i64().expect("Alarm need field 'set'"),
                        reset: values["reset"].as_i64().expect("Alarm need field 'reset'"),
                        meas: String::from(
                            values["meas"].as_str().expect("Alarm need field 'meas'"),
                        ),
                        sevetity: match values["sevetity"]
                            .as_i64()
                            .expect("Alarm need field 'sevetity'")
                        {
                            2 => AlarmSevetity::High,
                            1 => AlarmSevetity::Medium,
                            0 => AlarmSevetity::Low,
                            _ => panic!("Invalid sevetity!"),
                        },
                        rx: None,
                    })
                }
            }
        }
    }

    alarmes
}
