use crate::{
    DecodedPacket, Endpoint,
    error::Result,
    packet::{OperationPacketKind, PacketMetadata},
    protocol::Protocol18Deserializer,
    util::{read_i32_be},
};
use crate::{
    albion::{
        AlbionExtractor, AlbionMail, CachedOrder,
        OperationType,
        PlayerState, TradeType,
    },
    photon::recorder::PacketRecorder,
};

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};

const COMMAND_DISCONNECT: u8 = 4;
const COMMAND_SEND_RELIABLE: u8 = 6;
const COMMAND_SEND_UNRELIABLE: u8 = 7;
const COMMAND_SEND_FRAGMENT: u8 = 8;

const MESSAGE_OPERATION_REQUEST: u8 = 2;
const MESSAGE_OPERATION_RESPONSE: u8 = 3;
const MESSAGE_EVENT: u8 = 4;

struct PendingSegment {
    payload: Vec<u8>,
    written: usize,
    total_length: usize,
}

pub struct PhotonParser {
    file_name: String,
    debug: bool,
    deserializer: Protocol18Deserializer,
    pending_segments: HashMap<i32, PendingSegment>,
    recorder: PacketRecorder,
    extractor: AlbionExtractor,
}

impl PhotonParser {
    pub fn new(file_name: String, debug: bool) -> Self {
        let capture_unknown_packets = std::env::var("ALBION_NETWORK_DEBUG").as_deref() == Ok("1");
        Self {
            file_name,
            debug,
            deserializer: Protocol18Deserializer,
            pending_segments: HashMap::new(),
            recorder: PacketRecorder::new(capture_unknown_packets),
            extractor: AlbionExtractor::with_defaults(),
        }
    }

    pub fn decoded_packets(&self) -> &[DecodedPacket] {
        &self.recorder.decoded_packets()
    }

    pub fn market_order_count(&self) -> usize {
        self.extractor.market_order_count()
    }

    pub fn player_state(&self) -> &PlayerState {
        self.extractor.player_state()
    }

    pub fn albion_mails(&self) -> &HashMap<i64, AlbionMail> {
        self.extractor.albion_mails()
    }

    pub fn into_decoded_packets(self) -> Vec<DecodedPacket> {
        self.recorder.into_decoded_packets()
    }

    pub fn receive_packet(
        &mut self,
        payload: &[u8],
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<&'static str> {
        if payload.len() < 12 {
            return Ok("InvalidHeader");
        }

        let flags = payload[2];
        let command_count = payload[3];

        if flags == 1 {
            self.extractor
                .player_state_mut()
                .set_has_encrypted_data(true);
            return Ok("Encrypted");
        }

        let mut offset = 12;
        let mut status = "Undefined";
        for command_index in 0..command_count {
            if payload.len().saturating_sub(offset) < 12 {
                return Ok("InvalidHeader");
            }
            let result = self.handle_command(
                payload,
                offset,
                packet_number,
                command_index,
                source,
                destination,
            )?;
            status = result.0;
            offset = result.1;
            if status == "InvalidHeader" {
                return Ok(status);
            }
        }
        Ok(status)
    }

    fn handle_command(
        &mut self,
        data: &[u8],
        mut offset: usize,
        packet_number: usize,
        command_index: u8,
        source: &str,
        destination: &str,
    ) -> Result<(&'static str, usize)> {
        let command_type = data[offset];
        let command_length = read_i32_be(data, offset + 4)? - 12;
        let sequence_number = read_i32_be(data, offset + 8)?;
        offset += 12;
        if self.debug {
            eprintln!(
                "DEBUG:albion:packet={packet_number} command={command_index} type={command_type} sequence={sequence_number} payload_length={command_length}"
            );
        }
        if command_length < 0 || data.len().saturating_sub(offset) < command_length as usize {
            return Ok(("InvalidHeader", offset));
        }
        let command_length = command_length as usize;
        match command_type {
            COMMAND_DISCONNECT => Ok(("DisconnectCommand", offset + command_length)),
            COMMAND_SEND_UNRELIABLE => {
                if command_length < 4 {
                    return Ok(("InvalidHeader", offset));
                }
                self.handle_send_reliable(
                    data,
                    offset + 4,
                    command_length - 4,
                    packet_number,
                    source,
                    destination,
                )
            }
            COMMAND_SEND_RELIABLE => self.handle_send_reliable(
                data,
                offset,
                command_length,
                packet_number,
                source,
                destination,
            ),
            COMMAND_SEND_FRAGMENT => self.handle_send_fragment(
                data,
                offset,
                command_length,
                packet_number,
                source,
                destination,
            ),
            _ => Ok(("Undefined", offset + command_length)),
        }
    }

    fn handle_send_reliable(
        &mut self,
        data: &[u8],
        offset: usize,
        command_length: usize,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<(&'static str, usize)> {
        if command_length < 2 || data.len().saturating_sub(offset) < command_length {
            return Ok(("InvalidHeader", offset));
        }
        let message_type = data[offset + 1];
        let operation_payload = &data[offset + 2..offset + command_length];
        let metadata = packet_metadata(&self.file_name, packet_number, source, destination);
        if message_type == 131 {
            self.extractor.player_state_mut().set_has_encrypted_data(true);
            return Ok(("Encrypted", offset + command_length));
        }
        match message_type {
            MESSAGE_OPERATION_REQUEST => {
                let (_, params) = self
                    .deserializer
                    .deserialize_operation_request(operation_payload)?;
                self.recorder.record_operation(
                    &mut self.extractor,
                    metadata.clone(),
                    OperationPacketKind::Request,
                    &params,
                    None,
                    "",
                )?;
            }
            MESSAGE_OPERATION_RESPONSE => {
                let (_, return_code, debug_message, params) = self
                    .deserializer
                    .deserialize_operation_response(operation_payload)?;
                self.recorder.record_operation(
                    &mut self.extractor,
                    metadata.clone(),
                    OperationPacketKind::Response,
                    &params,
                    Some(return_code),
                    &debug_message,
                )?;
            }
            MESSAGE_EVENT => {
                let (event_code, mut params) = self
                    .deserializer
                    .deserialize_event_data(operation_payload)?;
                self.recorder.record_event(
                    &mut self.extractor,
                    metadata.clone(),
                    event_code,
                    &mut params,
                )?;
            }
            _ => {}
        }
        Ok(("Success", offset + command_length))
    }

    fn handle_send_fragment(
        &mut self,
        data: &[u8],
        mut offset: usize,
        command_length: usize,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<(&'static str, usize)> {
        if command_length < 20 || data.len().saturating_sub(offset) < command_length {
            return Ok(("InvalidHeader", offset));
        }
        let start_sequence_number = read_i32_be(data, offset)?;
        let total_length = read_i32_be(data, offset + 12)? as usize;
        let fragment_offset = read_i32_be(data, offset + 16)? as usize;
        offset += 20;
        let fragment_length = command_length - 20;
        let fragment = &data[offset..offset + fragment_length];

        let pending = self
            .pending_segments
            .entry(start_sequence_number)
            .or_insert_with(|| PendingSegment {
                payload: vec![0; total_length],
                written: 0,
                total_length,
            });
        pending.payload[fragment_offset..fragment_offset + fragment_length]
            .copy_from_slice(fragment);
        pending.written += fragment_length;

        if pending.written >= pending.total_length {
            let total_payload = self
                .pending_segments
                .remove(&start_sequence_number)
                .unwrap()
                .payload;
            let (status, _) = self.handle_send_reliable(
                &total_payload,
                0,
                total_payload.len(),
                packet_number,
                source,
                destination,
            )?;
            return Ok((status, offset + fragment_length));
        }

        Ok(("Success", offset + fragment_length))
    }
}

fn direction(source: &str, destination: &str) -> &'static str {
    if source.ends_with(":5056") {
        "server_to_client"
    } else if destination.ends_with(":5056") {
        "client_to_server"
    } else {
        "unknown"
    }
}

fn operation_from_cached_order(
    cached_order: Option<&CachedOrder>,
    trade_type: &TradeType,
) -> OperationType {
    cached_order
        .map(|order| OperationType::from_auction_type(&order.auction_type, trade_type))
        .unwrap_or_else(|| OperationType::Unknown("missing_cached_order".to_string()))
}

fn packet_metadata(
    source_name: &str,
    packet_number: usize,
    source: &str,
    destination: &str,
) -> PacketMetadata {
    let source = parse_endpoint(source);
    let destination = parse_endpoint(destination);

    PacketMetadata {
        source_name: source_name.to_string(),
        packet_number,
        direction: crate::packet::PacketDirection::from_endpoints(&source, &destination),
        source,
        destination,
    }
}

fn parse_endpoint(value: &str) -> Endpoint {
    let (ip_text, port_text) = value.rsplit_once(':').unwrap_or(("0.0.0.0", "0"));
    let ip = ip_text
        .parse::<IpAddr>()
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    let port = port_text.parse::<u16>().unwrap_or(0);

    Endpoint { ip, port }
}
