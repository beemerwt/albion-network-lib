// src/packet/metadata.rs
use crate::capture::Endpoint;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PacketMetadata {
    pub source_name: String,
    pub packet_number: usize,
    pub direction: PacketDirection,
    pub source: Endpoint,
    pub destination: Endpoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketDirection {
    ClientToServer,
    ServerToClient,
    Unknown,
}

impl PacketDirection {
    pub fn from_endpoints(source: &Endpoint, destination: &Endpoint) -> Self {
        if source.is_albion_server_port() {
            Self::ServerToClient
        } else if destination.is_albion_server_port() {
            Self::ClientToServer
        } else {
            Self::Unknown
        }
    }
}