extern crate yaml_rust;
use std::fs;
use yaml_rust::{Yaml, YamlLoader};

pub mod alarm;
pub mod server;
pub mod reader;

pub fn load_config(path: &str) -> Yaml {
    let source = fs::read_to_string(path).unwrap();
    YamlLoader::load_from_str(&source).unwrap().remove(0)
}

pub fn create_alarms(config: Yaml) -> Vec<alarm::Alarm> {
    let mut alarmes: Vec<alarm::Alarm> = Vec::new();

    let mut alm_id: u32 = 0;
    let mut overflow;

    if let Yaml::Hash(ref h) = config {
        for (area, alm) in h {
            if let Yaml::Hash(h) = alm {
                for (alm_name, values) in h {
                    (alm_id, overflow) = alm_id.overflowing_add(1);

                    // If this happens in production we need get a
                    //  a better id
                    if overflow {
                        panic!("The maximum number of alarms was reach! The maximum is {}", u32::MAX);
                    }

                    alarmes.push(alarm::Alarm::new(
                        alm_id,
                        format!("{}/{}", area.as_str().unwrap(), alm_name.as_str().unwrap()),
                        values["set"].as_i64().expect("Alarm need field 'set'"),
                        values["reset"].as_i64().expect("Alarm need field 'reset'"),
                        match values["severity"]
                            .as_i64()
                            .expect("Alarm need field 'severity'")
                        {
                            2 => alarm::AlarmSeverity::High,
                            1 => alarm::AlarmSeverity::Medium,
                            0 => alarm::AlarmSeverity::Low,
                            _ => panic!("Invalid severity!"),
                        },
                        String::from(values["meas"].as_str().expect("Alarm need field 'meas'")),
                    ))
                }
            }
        }
    }

    alarmes
}
