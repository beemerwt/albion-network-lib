use crate::{models::MailInfoType, util::value_i64};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GetMailInfos {
    pub mail_ids: Vec<i64>,
    pub location_ids: Vec<String>,
    pub types: Vec<MailInfoType>,
    pub received: Vec<i64>,
}

impl GetMailInfos {
    pub fn from_params(parameters: &BTreeMap<u8, Value>) -> Self {
        Self {
            mail_ids: i64_array(parameters.get(&3)),
            location_ids: string_array(parameters.get(&7)),
            types: parameters
                .get(&11)
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(MailInfoType::from_str)
                        .collect()
                })
                .unwrap_or_default(),
            received: i64_array(parameters.get(&12)),
        }
    }
}

fn i64_array(value: Option<&Value>) -> Vec<i64> {
    value
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(value_i64).collect())
        .unwrap_or_default()
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::GetMailInfos;
    use crate::models::MailInfoType;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn parses_full_get_mail_infos_response() {
        let mut params = BTreeMap::new();
        params.insert(3, json!([101, "102", true, {"ignored": true}]));
        params.insert(7, json!(["2000", "BLACKBANK-2310"]));
        params.insert(
            11,
            json!([
                "MARKETPLACE_SELLORDER_FINISHED_SUMMARY",
                "MARKETPLACE_BUYORDER_FINISHED_SUMMARY",
                "unexpected"
            ]),
        );
        params.insert(12, json!([1_717_171_717_i64, "1717171718"]));

        let response = GetMailInfos::from_params(&params);

        assert_eq!(response.mail_ids, vec![101, 102, 1]);
        assert_eq!(
            response.location_ids,
            vec!["2000".to_string(), "BLACKBANK-2310".to_string()]
        );
        assert_eq!(
            response.types,
            vec![
                MailInfoType::MarketPlaceSellOrderFinishedSummary,
                MailInfoType::MarketPlaceBuyOrderFinishedSummary,
                MailInfoType::Unknown,
            ]
        );
        assert_eq!(response.received, vec![1_717_171_717, 1_717_171_718]);
    }

    #[test]
    fn missing_or_malformed_params_default_to_empty_vectors() {
        let mut params = BTreeMap::new();
        params.insert(3, json!({"unexpected": true}));
        params.insert(7, json!(null));
        params.insert(11, json!("unexpected"));
        params.insert(12, json!([{"ignored": true}]));

        let response = GetMailInfos::from_params(&params);

        assert!(response.mail_ids.is_empty());
        assert!(response.location_ids.is_empty());
        assert!(response.types.is_empty());
        assert!(response.received.is_empty());
    }
}
