use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::{models::ChatChannel, util::value_i64};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatMessage {
    pub channel_id: i64,
    pub channel_type: ChatChannel,
    pub player_name: String,
    pub message: String,
    pub timestamp: i64,
}

impl ChatMessage {
    pub fn from_params(parameters: &BTreeMap<u8, Value>) -> Self {
        Self::from_params_with_channel_type(parameters, None)
    }

    pub fn from_params_with_channel_type(
        parameters: &BTreeMap<u8, Value>,
        channel_type: Option<ChatChannel>,
    ) -> Self {
        let channel_id = parameters.get(&0).and_then(value_i64).unwrap_or_default();
        let player_name = parameters
            .get(&1)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let message = parameters
            .get(&2)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let timestamp = Utc::now().timestamp_millis();
        Self {
            channel_id,
            channel_type: channel_type.unwrap_or_else(|| ChatChannel::from_i64(channel_id)),
            player_name,
            message,
            timestamp,
        }
    }

    pub fn from_say_params(parameters: &BTreeMap<u8, Value>) -> Self {
        let player_name = parameters
            .get(&0)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let message = parameters
            .get(&1)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let timestamp = Utc::now().timestamp_millis();
        Self {
            channel_id: 0,
            channel_type: ChatChannel::Say,
            player_name,
            message,
            timestamp,
        }
    }
}
