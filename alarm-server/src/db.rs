use crate::alarm::{AlarmAck, AlarmState, AlarmStatus};
use crate::config::DBConfig;
use chrono::{DateTime, Utc};
use reqwest::Client;

#[derive(Clone, Debug)]
pub struct DB {
    url: String,
    table: String,
    client: Client,
}

impl DB {
    pub fn new(config: DBConfig) -> Self {
        Self {
            url: config.url,
            table: config.table,
            client: Client::new(),
        }
    }

    pub async fn send_ack(&self, path: &str) {
        let now: DateTime<Utc> = Utc::now();

        let timestamp = now.to_rfc3339();
        let table = &self.table;

        let query = format! {"insert into {table} \
        select \
        '{timestamp}' timestamp, \
        '{path}' path, \
        state, \
        value, \
        severity, \
        true \
        from {table} \
        where path = '{path}' \
        limit -1;"};
        let _ = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;
    }

    pub async fn insert_alm(&self, alm: AlarmStatus) {
        let now: DateTime<Utc> = Utc::now();

        let timestamp = now.to_rfc3339();
        let table = &self.table;
        let name = alm.name;
        let state = alm.state == AlarmState::Set;
        let value = alm.value;
        let severity = alm.severity as u8;
        let ack = alm.ack == AlarmAck::Ack;

        let query = format! {"INSERT INTO {table} VALUES (\
        '{timestamp}',\
        '{name}',\
        {state},\
        {value},\
        {severity},\
        {ack});"};
        let _ = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;
    }

    pub async fn get_latest_alm(&self, path: String) -> Option<AlarmStatus> {
        let table = &self.table;

        let query = format! {
        "SELECT path, state, ack FROM {table} \
        WHERE path = '{path}' \
        LIMIT -1"};

        let resp = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error on http request - Error: {e}");
                return None;
            }
        };

        if resp.status() != 200 {
            eprintln!("Error getting latest alm for {path}");
            return None;
        }

        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error getting the response body - Error: {e}");
                return None;
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&body) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Error parsing the body. body: {body} - Error: {e}");
                return None;
            }
        };

        Some(AlarmStatus {
            name: json["name"].to_string(),
            state: if json["state"].is_boolean() && json["state"].as_bool().unwrap() {
                AlarmState::Set
            } else {
                AlarmState::Reset
            },
            value: i64::MAX,
            severity: crate::alarm::AlarmSeverity::High,
            ack: if json["ack"].is_boolean() && json["ack"].as_bool().unwrap() {
                AlarmAck::Ack
            } else {
                AlarmAck::NotAck
            },
        })
    }

    fn build_full_url(url: &str, query: &str) -> String {
        format! {"{url}/exec?query={query}"}
    }

    pub async fn try_create_table(&self) {
        let query = "";
        let _ = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;
    }
}
