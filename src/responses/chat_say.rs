use chrono::Local;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatSay {
    pub player_name: String,
    pub message: String,
    pub timestamp: i64,
}

impl ChatSay {
    pub fn from_parameters(parameters: &BTreeMap<u8, Value>) -> Self {
        let player_name = parameters
            .get(&0)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let message = parameters
            .get(&1)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let timestamp = Local::now().timestamp_millis();
        Self {
            player_name,
            message,
            timestamp,
        }
    }
}
