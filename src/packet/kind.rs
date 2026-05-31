// src/packet/kind.rs

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPacketKind {
    Request,
    Response,
}

impl OperationPacketKind {
    pub fn message_type(self) -> &'static str {
        match self {
            Self::Request => "operation_request",
            Self::Response => "operation_response",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
        }
    }
}
