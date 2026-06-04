use crate::albion::{EventCode, OperationCode};
use crate::packet::RawParameters;
use crate::util::{to_signed_short, value_i64};

pub struct CodeParseError {
    pub raw_code: Option<i32>,
    pub reason: &'static str,
    pub message: String,
}

pub fn parse_operation_code(
    params: &RawParameters,
) -> std::result::Result<OperationCode, CodeParseError> {
    let Some(value) = value_i64(params, 253) else {
        return Err(CodeParseError {
            raw_code: None,
            reason: "missing_operation_code",
            message: "Operation code parameter 253 is missing".to_string(),
        });
    };

    let code = to_signed_short(value);
    OperationCode::try_from(code).map_err(|_| CodeParseError {
        raw_code: Some(code),
        reason: "unknown_operation_code",
        message: format!("Unknown operation code in parameter 253: {code}"),
    })
}

pub fn parse_event_code(params: &RawParameters) -> std::result::Result<EventCode, CodeParseError> {
    let Some(value) = value_i64(params, 252) else {
        return Err(CodeParseError {
            raw_code: None,
            reason: "missing_event_code",
            message: "Event code parameter 252 is missing".to_string(),
        });
    };

    let code = to_signed_short(value);
    if let Ok(event_code) = EventCode::try_from(code) {
        return Ok(event_code);
    }

    let unsigned_value = (code as i64 & 0xffff) as i32;
    let shifted = unsigned_value >> 4;
    if (unsigned_value & 0x0f) == 0x01 {
        if let Ok(event_code) = EventCode::try_from(shifted) {
            return Ok(event_code);
        }
    }

    Err(CodeParseError {
        raw_code: Some(code),
        reason: "unknown_event_code",
        message: format!("Unknown event code in parameter 252: {code}"),
    })
}
