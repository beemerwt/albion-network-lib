// src/packet/kind.rs

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPacketKind {
    Request,
    Response,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownPacketKind {
    OperationRequest,
    OperationResponse,
    Event,
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

    pub fn into_unknown_kind(self) -> UnknownPacketKind {
        match self {
            Self::Request => UnknownPacketKind::OperationRequest,
            Self::Response => UnknownPacketKind::OperationResponse,
        }
    }
}
