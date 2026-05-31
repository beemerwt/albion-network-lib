use crate::util::value_i64;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JoinedChatChannel {
    pub chat_index: u8,
    pub channel_id: i64,
}

impl JoinedChatChannel {
    pub fn from_params(parameters: &BTreeMap<u8, Value>) -> Self {
        Self {
            chat_index: parameters
                .get(&0)
                .and_then(value_i64)
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or_default(),
            channel_id: parameters.get(&1).and_then(value_i64).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JoinedChatChannel;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn parses_full_joined_chat_channel_response() {
        let mut params = BTreeMap::new();
        params.insert(0, json!("2"));
        params.insert(1, json!(42));

        let response = JoinedChatChannel::from_params(&params);

        assert_eq!(response.chat_index, 2);
        assert_eq!(response.channel_id, 42);
    }

    #[test]
    fn missing_or_malformed_params_use_defaults() {
        let mut params = BTreeMap::new();
        params.insert(0, json!(300));
        params.insert(1, json!({"unexpected": true}));

        let response = JoinedChatChannel::from_params(&params);

        assert_eq!(response.chat_index, 0);
        assert_eq!(response.channel_id, 0);
    }
}
