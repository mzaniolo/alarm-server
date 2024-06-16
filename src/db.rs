use crate::alarm::{AlarmAck, AlarmState, Alarm};
use crate::config::DBConfig;
use chrono::{DateTime, Utc};
use reqwest::{Client, Url, Response, Error};

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

    pub async fn send_ack(&self, name: &str) {
        println!("Insert ack to {name}");
        let now: DateTime<Utc> = Utc::now();

        let timestamp = now.to_rfc3339();
        let table = &self.table;

        let query = format! {"insert into {table} \
        select \
        '{timestamp}' timestamp, \
        '{name}' name, \
        state, \
        value, \
        severity, \
        true \
        from {table} \
        where name = '{name}' \
        limit -1;"};
        let _ = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;
    }

    pub async fn insert_alm(&self, alm: Alarm) {
        println!("insert state: {alm:?}");

        let timestamp = alm.timestamp.to_rfc3339();
        let table = &self.table;
        let name = alm.name;
        let state = alm.state;
        let value = alm.value;
        let severity = alm.severity;
        let ack = alm.ack == AlarmAck::Ack;

        let query = format! {"INSERT INTO {table} VALUES (\
        '{timestamp}',\
        '{name}',\
        '{state}',\
        {value},\
        '{severity}',\
        {ack});"};
        let resp = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;
        let _ = Self::get_body(resp).await;
    }

    pub async fn get_latest_alm(&self, name: String) -> Option<Alarm> {
        let table = &self.table;
        
        let query = format! {
        "SELECT name, state, ack FROM {table} \
        WHERE name = '{name}' \
        LIMIT -1"};

        let resp = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;

        let body = match Self::get_body(resp).await{
            Some(body) => body,
            None => return None,
        };

        let json: serde_json::Value = match serde_json::from_str(&body) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Error parsing the body. body: {body} - Error: {e}");
                return None;
            }
        };

        let data = &json["dataset"][0];
        Some(Alarm {
            timestamp: Utc::now(),
            name: data[0].to_string(),
            state: if data[1].is_boolean() && data[1].as_bool().unwrap() {
                AlarmState::Set
            } else {
                AlarmState::Reset
            },
            value: i64::MAX,
            severity: crate::alarm::AlarmSeverity::High,
            ack: if data[2].is_boolean() && data[2].as_bool().unwrap() {
                AlarmAck::Ack
            } else {
                AlarmAck::NotAck
            },
        })
    }

    async fn get_body(req: Result<Response, Error>) -> Option<String>{
        let resp = match req {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error on http request - Error: {e}");
                return None;
            }
        };

        if resp.status() != 200 {
            eprintln!("response: {}", resp.text().await.unwrap());
            return None;
        }

        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error getting the response body - Error: {e}");
                return None;
            }
        };

        Some(body)
    }

    fn build_full_url(url: &str, query: &str) -> String {
        let base = format! {"{url}/exec?query={query}"};
        Url::parse_with_params(&base, &[("query", query)])
            .unwrap()
            .as_str()
            .to_string()
    }

    pub async fn try_create_table(&self) {
        println!("Creating table");
        let table = &self.table;
        let query = format!(
            "CREATE TABLE IF NOT EXISTS '{table}' (\
            timestamp TIMESTAMP,\
            name SYMBOL,\
            state SYMBOL,\
            value SHORT,\
            severity SYMBOL,\
            ack BOOLEAN\
          ) timestamp (timestamp) PARTITION BY MONTH WAL \
          DEDUP UPSERT KEYS (timestamp, name);"
        );
        let resp = self
            .client
            .get(Self::build_full_url(&self.url, &query))
            .send()
            .await;
        match resp {
            Ok(out) => {
                println!("status: {}", out.status());
                let body = out.text().await.unwrap();
                println!("body: {body}");
            }
            Err(e) => {
                eprintln!("error: {e}");
            }
        };
    }
}
