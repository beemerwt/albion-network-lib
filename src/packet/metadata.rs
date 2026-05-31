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

impl PacketMetadata {
    pub fn new(
        source_name: String,
        packet_number: usize,
        source: Endpoint,
        destination: Endpoint,
    ) -> Self {
        let direction = PacketDirection::from_endpoints(&source, &destination);
        Self {
            source_name,
            packet_number,
            direction,
            source,
            destination,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PacketDirection {
    ClientToServer,
    ServerToClient,
    Unknown,
}

impl PacketDirection {
    pub fn from_addresses(source: &str, destination: &str) -> Self {
        if source.ends_with(":5056") || source.ends_with(":4535") {
            Self::ServerToClient
        } else if destination.ends_with(":5056") || destination.ends_with(":4535") {
            Self::ClientToServer
        } else {
            Self::Unknown
        }
    }

    pub fn from_endpoints(source: &Endpoint, destination: &Endpoint) -> Self {
        if source.is_albion_port() {
            Self::ServerToClient
        } else if destination.is_albion_port() {
            Self::ClientToServer
        } else {
            Self::Unknown
        }
    }
}
