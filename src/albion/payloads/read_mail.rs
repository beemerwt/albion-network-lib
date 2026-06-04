use crate::{packet::RawParameters, util::value_i64};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadMail {
    pub mail_id: i64,
    pub mail_string: String,
}

impl ReadMail {
    pub fn from_params(parameters: &RawParameters) -> Self {
        Self {
            mail_id: value_i64(parameters, 0).unwrap_or_default(),
            mail_string: parameters
                .get(1)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::RawParameters;

    use super::ReadMail;
    use serde_json::json;

    #[test]
    fn parses_full_read_mail_response() {
        let mut params = RawParameters::empty();
        params.insert(0, json!("42"));
        params.insert(1, json!("mail body"));

        let response = ReadMail::from_params(&params);

        assert_eq!(response.mail_id, 42);
        assert_eq!(response.mail_string, "mail body");
    }

    #[test]
    fn missing_params_use_defaults() {
        let response = ReadMail::from_params(&RawParameters::empty());

        assert_eq!(response.mail_id, 0);
        assert_eq!(response.mail_string, "");
    }

    #[test]
    fn malformed_params_use_defaults() {
        let mut params = RawParameters::empty();
        params.insert(0, json!({"unexpected": true}));
        params.insert(1, json!(123));

        let response = ReadMail::from_params(&params);

        assert_eq!(response.mail_id, 0);
        assert_eq!(response.mail_string, "");
    }
}
