// src/photon/message.rs

use crate::error::Result;

pub const MESSAGE_OPERATION_REQUEST: u8 = 2;
pub const MESSAGE_OPERATION_RESPONSE: u8 = 3;
pub const MESSAGE_EVENT: u8 = 4;
pub const MESSAGE_ENCRYPTED: u8 = 131;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhotonMessage<'a> {
    OperationRequest(&'a [u8]),
    OperationResponse(&'a [u8]),
    Event(&'a [u8]),
    Encrypted,
    Unknown { message_type: u8, payload: &'a [u8] },
}

impl<'a> PhotonMessage<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < 2 {
            return Err("Invalid Photon message payload".into());
        }

        let message_type = data[1];
        let payload = &data[2..];

        let message = match message_type {
            MESSAGE_OPERATION_REQUEST => Self::OperationRequest(payload),
            MESSAGE_OPERATION_RESPONSE => Self::OperationResponse(payload),
            MESSAGE_EVENT => Self::Event(payload),
            MESSAGE_ENCRYPTED => Self::Encrypted,
            _ => Self::Unknown {
                message_type,
                payload,
            },
        };

        Ok(message)
    }
}
