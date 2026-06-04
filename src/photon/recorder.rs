use crate::{
    albion::{AlbionExtractor, EventCode, parse_event_code, parse_operation_code},
    error::Result,
    packet::{DecodedEvent, DecodedOperation, DecodedPacket, DecodedUnknown},
    packet::{OperationPacketKind, PacketMetadata, RawParameters},
};
use serde_json::json;

pub struct PacketRecorder {
    capture_unknown_packets: bool,
    decoded_packets: Vec<DecodedPacket>,
}

impl PacketRecorder {
    pub fn new(capture_unknown_packets: bool) -> Self {
        Self {
            capture_unknown_packets,
            decoded_packets: Vec::new(),
        }
    }

    pub fn decoded_packets(&self) -> &[DecodedPacket] {
        &self.decoded_packets
    }

    pub fn into_decoded_packets(self) -> Vec<DecodedPacket> {
        self.decoded_packets
    }

    pub fn record_operation(
        &mut self,
        extractor: &mut AlbionExtractor,
        metadata: PacketMetadata,
        packet_kind: OperationPacketKind,
        parameters: &RawParameters,
        return_code: Option<i16>,
        debug_message: &str,
    ) -> Result<()> {
        let debug_message = debug_message.to_string();
        let kind_str = packet_kind.as_str();
        let operation_code = match parse_operation_code(&parameters) {
            Ok(operation_code) => operation_code,
            Err(error) => {
                if self.capture_unknown_packets {
                    self.record_unknown(
                        format!("operation_{kind_str}"),
                        format!("operation_{kind_str}"),
                        253,
                        error.raw_code,
                        error.reason,
                        parameters,
                        return_code,
                        &debug_message,
                        metadata.clone(),
                    );
                    return Ok(());
                }
                return Err(error.message.into());
            }
        };
        let operation_name = operation_code.name();
        let extracted =
            extractor.extract_operation(packet_kind, operation_code, &parameters, return_code);
        self.decoded_packets
            .push(DecodedPacket::Operation(DecodedOperation {
                metadata,
                kind: packet_kind,
                code: operation_code,
                name: operation_name.to_string(),
                return_code,
                debug_message,
                parameters: parameters.to_serializable(),
                extracted,
            }));
        Ok(())
    }

    pub fn record_event(
        &mut self,
        extractor: &mut AlbionExtractor,
        metadata: PacketMetadata,
        photon_event_code: u8,
        parameters: &mut RawParameters,
    ) -> Result<()> {
        if photon_event_code == EventCode::Move as u8 {
            parameters.insert(252, json!(EventCode::Move as i32));
        }
        let event_code = match parse_event_code(parameters) {
            Ok(event_code) => event_code,
            Err(error) => {
                if self.capture_unknown_packets {
                    self.record_unknown(
                        "event".to_string(),
                        "event".to_string(),
                        252,
                        error.raw_code,
                        error.reason,
                        parameters,
                        None,
                        "",
                        metadata.clone(),
                    );
                    return Ok(());
                }
                return Err(error.message.into());
            }
        };
        let event_name = event_code.name();
        let extracted = extractor.extract_event(event_code, parameters);
        self.decoded_packets
            .push(DecodedPacket::Event(DecodedEvent {
                metadata,
                file: String::new(),
                message_type: "event".to_string(),
                code: event_code,
                name: event_name.to_string(),
                return_code: None,
                debug_message: String::new(),
                parameters: parameters.to_serializable(),
                extracted,
            }));
        Ok(())
    }

    pub fn record_unknown(
        &mut self,
        message_type: String,
        kind: String,
        code_parameter: u8,
        raw_code: Option<i32>,
        reason: &'static str,
        parameters: &RawParameters,
        return_code: Option<i16>,
        debug_message: &str,
        metadata: PacketMetadata,
    ) {
        self.decoded_packets
            .push(DecodedPacket::Unknown(DecodedUnknown {
                metadata,
                file: String::new(),
                message_type,
                kind,
                code_parameter,
                raw_code,
                reason: reason.to_string(),
                return_code,
                debug_message: debug_message.to_string(),
                parameters: parameters.to_serializable(),
            }));
    }
}
