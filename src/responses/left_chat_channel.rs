use crate::util::value_i64;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LeftChatChannel {
    pub channel_id: i64,
}

impl LeftChatChannel {
    pub fn from_params(parameters: &BTreeMap<u8, Value>) -> Self {
        Self {
            channel_id: parameters.get(&0).and_then(value_i64).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LeftChatChannel;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn parses_full_left_chat_channel_response() {
        let mut params = BTreeMap::new();
        params.insert(0, json!("1856"));

        let response = LeftChatChannel::from_params(&params);

        assert_eq!(response.channel_id, 1856);
    }

    #[test]
    fn missing_or_malformed_params_use_defaults() {
        let mut params = BTreeMap::new();
        params.insert(0, json!({"unexpected": true}));

        let response = LeftChatChannel::from_params(&params);

        assert_eq!(response.channel_id, 0);
    }
}
