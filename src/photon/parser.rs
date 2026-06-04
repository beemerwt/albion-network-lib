use crate::{
    Endpoint,
    error::Result,
    packet::DecodedPacket,
    packet::{OperationPacketKind, PacketMetadata},
    photon::{
        PhotonParserConfig,
        command::{PhotonCommand, parse_command},
        fragment::FragmentReassembler,
        message::PhotonMessage,
    },
    protocol::Protocol18Deserializer,
};
use crate::{
    albion::{AlbionExtractor, AlbionMail, PlayerState},
    photon::recorder::PacketRecorder,
};

use std::{collections::HashMap, sync::Arc};

pub struct PhotonParser {
    source_name: String,
    debug: bool,
    deserializer: Protocol18Deserializer,
    fragments: FragmentReassembler,
    recorder: PacketRecorder,
    extractor: AlbionExtractor,
}

impl PhotonParser {
    pub fn new(config: PhotonParserConfig) -> Self {
        let capture_unknown_packets = config.capture_unknown_packets
            || std::env::var("ALBION_NETWORK_DEBUG").as_deref() == Ok("1");
        Self {
            source_name: config.source_name,
            debug: config.debug,
            deserializer: Protocol18Deserializer,
            fragments: FragmentReassembler::new(),
            recorder: PacketRecorder::new(capture_unknown_packets),
            extractor: AlbionExtractor::new(Arc::new(config.world_map), config.item_names),
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
        source: Endpoint,
        destination: Endpoint,
    ) -> Result<&'static str> {
        let metadata =
            PacketMetadata::new(self.source_name.clone(), packet_number, source, destination);

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
            let result = self.handle_command(payload, offset, command_index, &metadata)?;
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
        offset: usize,
        command_index: u8,
        metadata: &PacketMetadata,
    ) -> Result<(&'static str, usize)> {
        let (command, header) = parse_command(data, offset)?;

        if self.debug {
            let packet_number = metadata.packet_number;
            let command_type = header.command_type;
            let command_length = header.command_length;
            let sequence_number = header.sequence_number;
            eprintln!(
                "DEBUG:albion:packet={packet_number} command={command_index} type={command_type} sequence={sequence_number} payload_length={command_length}"
            );
        }

        let status = match command {
            PhotonCommand::Disconnect => "DisconnectCommand",

            PhotonCommand::SendReliable { payload } | PhotonCommand::SendUnreliable { payload } => {
                let (status, _) = self.handle_message_payload(payload, metadata)?;
                status
            }

            PhotonCommand::Fragment { header, payload } => {
                if let Some(total_payload) = self.fragments.push_fragment(
                    header.start_sequence_number,
                    header.total_length,
                    header.fragment_offset,
                    payload,
                )? {
                    let (status, _) = self.handle_message_payload(&total_payload, metadata)?;
                    status
                } else {
                    "Success"
                }
            }

            PhotonCommand::Unknown => "Undefined",
        };

        Ok((status, header.next_offset))
    }

    fn handle_message_payload(
        &mut self,
        payload: &[u8],
        metadata: &PacketMetadata,
    ) -> Result<(&'static str, usize)> {
        let message = PhotonMessage::parse(payload)?;

        match message {
            PhotonMessage::OperationRequest(operation_payload) => {
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
                )?
            }

            PhotonMessage::OperationResponse(operation_payload) => {
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
                )?
            }

            PhotonMessage::Event(event_payload) => {
                let (event_code, mut params) =
                    self.deserializer.deserialize_event_data(event_payload)?;
                self.recorder.record_event(
                    &mut self.extractor,
                    metadata.clone(),
                    event_code,
                    &mut params,
                )?
            }

            PhotonMessage::Encrypted => {
                self.extractor.mark_encrypted_data_seen();
                return Ok(("Encrypted", payload.len()));
            }

            PhotonMessage::Unknown => {}
        }

        Ok(("Success", payload.len()))
    }
}
