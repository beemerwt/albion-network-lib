
use crate::{
    event_codes::EventCode, extracted_packet::ExtractedPacket, operation_codes::OperationCode,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use crate::metadata::PacketMetadata;
use crate::parameters::SerializableParameters;

#[derive(Clone, Serialize)]
pub struct DecodedOperation {
    pub metadata: PacketMetadata,
    pub file: String,
    pub message_type: String,
    pub code: OperationCode,
    pub name: String,
    pub return_code: Option<i16>,
    pub debug_message: String,
    pub parameters: SerializableParameters,
    pub extracted: Option<ExtractedPacket>,
}

#[derive(Clone, Serialize)]
pub struct DecodedEvent {
    pub metadata: PacketMetadata,
    pub file: String,
    pub message_type: String,
    pub code: EventCode,
    pub name: String,
    pub return_code: Option<i16>,
    pub debug_message: String,
    pub parameters: SerializableParameters,
    pub extracted: Option<ExtractedPacket>,
}

#[derive(Clone, Serialize)]
pub struct DecodedUnknown {
    pub metadata: PacketMetadata,
    pub file: String,
    pub message_type: String,
    pub kind: String,
    pub code_parameter: u8,
    pub raw_code: Option<i32>,
    pub reason: String,
    pub return_code: Option<i16>,
    pub debug_message: String,
    pub parameters: SerializableParameters,
}

#[derive(Clone, Serialize)]
pub enum DecodedPacket {
    Operation(DecodedOperation),
    Event(DecodedEvent),
    Unknown(DecodedUnknown),
}

impl DecodedPacket {
    pub fn extracted_json(&self) -> Option<Value> {
        match self {
            Self::Operation(packet) => packet.extracted.as_ref(),
            Self::Event(packet) => packet.extracted.as_ref(),
            Self::Unknown(_) => None,
        }
        .map(ExtractedPacket::to_json)
    }

    pub fn into_extracted_json(self) -> Option<Value> {
        match self {
            Self::Operation(packet) => packet.extracted,
            Self::Event(packet) => packet.extracted,
            Self::Unknown(_) => None,
        }
        .map(ExtractedPacket::into_json)
    }
}