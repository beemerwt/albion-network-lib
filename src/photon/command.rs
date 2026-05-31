// src/photon/command.rs

use crate::{error::Result, util::read_i32_be};

pub const COMMAND_DISCONNECT: u8 = 4;
pub const COMMAND_SEND_RELIABLE: u8 = 6;
pub const COMMAND_SEND_UNRELIABLE: u8 = 7;
pub const COMMAND_SEND_FRAGMENT: u8 = 8;

const COMMAND_HEADER_LEN: usize = 12;
const UNRELIABLE_SEQUENCE_LEN: usize = 4;
const FRAGMENT_HEADER_LEN: usize = 20;

#[derive(Clone, Copy, Debug)]
pub struct PhotonCommandHeader {
    pub command_type: u8,
    pub command_length: usize,
    pub sequence_number: i32,
    pub payload_offset: usize,
    pub next_offset: usize,
}

impl PhotonCommandHeader {
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        if data.len().saturating_sub(offset) < 12 {
            return Err("Invalid Photon command header".into());
        }

        let command_type = data[offset];
        let raw_command_length = read_i32_be(data, offset + 4)? - COMMAND_HEADER_LEN as i32;
        let sequence_number = read_i32_be(data, offset + 8)?;

        if raw_command_length < 0 {
            return Err("Invalid Photon command length".into());
        }

        let command_length = raw_command_length as usize;
        let payload_offset = offset + COMMAND_HEADER_LEN;
        let next_offset = payload_offset + command_length;

        if data.len() < next_offset {
            return Err("Photon command payload exceeds packet length".into());
        }

        Ok(Self {
            command_type,
            command_length,
            sequence_number,
            payload_offset,
            next_offset,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FragmentHeader {
    pub start_sequence_number: i32,
    pub total_length: usize,
    pub fragment_offset: usize,
}

impl FragmentHeader {
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < FRAGMENT_HEADER_LEN {
            return Err("Invalid Photon fragment payload".into());
        }

        let total_length = read_i32_be(payload, 12)?;

        if total_length < 0 {
            return Err("Invalid Photon fragment total length".into());
        }

        let fragment_offset = read_i32_be(payload, 16)?;

        if fragment_offset < 0 {
            return Err("Invalid Photon fragment offset".into());
        }

        Ok(Self {
            start_sequence_number: read_i32_be(payload, 0)?,
            total_length: total_length as usize,
            fragment_offset: fragment_offset as usize,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PhotonCommand<'a> {
    Disconnect,
    SendReliable {
        payload: &'a [u8],
    },
    SendUnreliable {
        payload: &'a [u8],
    },
    Fragment {
        header: FragmentHeader,
        payload: &'a [u8],
    },
    Unknown,
}

pub fn parse_command<'a>(
    data: &'a [u8],
    offset: usize,
) -> Result<(PhotonCommand<'a>, PhotonCommandHeader)> {
    let header = PhotonCommandHeader::parse(data, offset)?;
    let payload = &data[header.payload_offset..header.next_offset];

    let command = match header.command_type {
        COMMAND_DISCONNECT => PhotonCommand::Disconnect,

        COMMAND_SEND_RELIABLE => PhotonCommand::SendReliable { payload },

        COMMAND_SEND_UNRELIABLE => {
            if payload.len() < 4 {
                return Err("Invalid unreliable Photon command payload".into());
            }

            PhotonCommand::SendUnreliable {
                payload: &payload[UNRELIABLE_SEQUENCE_LEN..],
            }
        }

        COMMAND_SEND_FRAGMENT => {
            let fragment_header = FragmentHeader::parse(payload)?;

            PhotonCommand::Fragment {
                header: fragment_header,
                payload: &payload[FRAGMENT_HEADER_LEN..],
            }
        }

        _ => PhotonCommand::Unknown,
    };

    Ok((command, header))
}
