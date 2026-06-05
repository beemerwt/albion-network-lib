use crate::{
    albion::MailInfoType,
    packet::RawParameters,
    util::{dotnet_ticks_to_unix_millis, i64_array, string_array},
};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GetMailInfos {
    pub mail_ids: Vec<i64>,
    pub location_ids: Vec<String>,
    pub types: Vec<MailInfoType>,
    pub received: Vec<i64>,
}

impl GetMailInfos {
    pub fn from_params(parameters: &RawParameters) -> Self {
        Self {
            mail_ids: i64_array(parameters, 3),
            location_ids: string_array(parameters, 7),
            types: string_array(parameters, 11)
                .into_iter()
                .map(MailInfoType::from_str)
                .collect(),
            received: i64_array(parameters, 12)
                .into_iter()
                .map(dotnet_ticks_to_unix_millis)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GetMailInfos;
    use crate::{albion::MailInfoType, packet::RawParameters};
    use serde_json::json;

    #[test]
    fn parses_full_get_mail_infos_response() {
        let mut params = RawParameters::empty();
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
        let mut params = RawParameters::empty();
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
