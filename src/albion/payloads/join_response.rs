use crate::albion::Guid;
use crate::packet::RawParameters;
use crate::util::{value_i32, value_i64};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JoinResponse {
    pub player_location: String,
    pub player_name: Option<String>,
    pub user_object_id: Option<i32>,
    pub user_guid: Option<Guid>,
    pub global_multiplier: Option<f64>,
}

impl JoinResponse {
    pub fn from_params(params: &RawParameters) -> Self {
        Self {
            player_location: params
                .get(8)
                .and_then(Value::as_str)
                .unwrap_or("Unknown")
                .to_string(),
            player_name: params.get(2).and_then(Value::as_str).map(str::to_string),
            user_object_id: value_i32(params, 0),
            user_guid: params.get(1).and_then(Guid::from_value),
            global_multiplier: value_i64(params, 84).map(|value| value as f64 / 10000.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JoinResponse;
    use crate::{albion::Guid, packet::RawParameters};
    use serde_json::json;

    #[test]
    fn parses_full_join_response() {
        let mut params = RawParameters::empty();
        params.insert(0, json!(42));
        params.insert(
            1,
            json!({
                "type_code": 0,
                "data_hex": "785634123412785690abcdef12345678"
            }),
        );
        params.insert(2, json!("TestPlayer"));
        params.insert(8, json!("Bridgewatch"));
        params.insert(84, json!(15000));

        let response = JoinResponse::from_params(&params);

        assert_eq!(response.player_location, "Bridgewatch");
        assert_eq!(response.player_name.as_deref(), Some("TestPlayer"));
        assert_eq!(response.user_object_id, Some(42));
        assert_eq!(
            response.user_guid,
            Some(Guid {
                data1: 0x12345678,
                data2: 0x1234,
                data3: 0x5678,
                data4: [0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78],
            })
        );
        assert_eq!(response.global_multiplier, Some(1.5));
    }

    #[test]
    fn missing_params_use_optional_fields_and_unknown_location() {
        let response = JoinResponse::from_params(&RawParameters::empty());

        assert_eq!(response.player_location, "Unknown");
        assert_eq!(response.player_name, None);
        assert_eq!(response.user_object_id, None);
        assert_eq!(response.user_guid, None);
        assert_eq!(response.global_multiplier, None);
    }

    #[test]
    fn object_id_accepts_supported_json_numeric_shapes() {
        for (value, expected) in [
            (json!(7), Some(7)),
            (json!(u32::MAX), Some(-1)),
            (json!("123"), Some(123)),
            (json!(true), Some(1)),
            (json!({ "unexpected": true }), None),
        ] {
            let mut params = RawParameters::empty();
            params.insert(0, value);

            assert_eq!(JoinResponse::from_params(&params).user_object_id, expected);
        }
    }

    #[test]
    fn parses_guid_from_string() {
        let mut params = RawParameters::empty();
        params.insert(1, json!("12345678-1234-5678-90ab-cdef12345678"));

        assert_eq!(
            JoinResponse::from_params(&params).user_guid,
            Some(Guid {
                data1: 0x12345678,
                data2: 0x1234,
                data3: 0x5678,
                data4: [0x90, 0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78],
            })
        );
    }

    #[test]
    fn malformed_multiplier_is_ignored() {
        let mut params = RawParameters::empty();
        params.insert(84, json!({ "unexpected": true }));

        assert_eq!(JoinResponse::from_params(&params).global_multiplier, None);
    }
}
