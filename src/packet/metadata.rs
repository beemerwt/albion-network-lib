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

#[cfg(test)]
mod tests {
    use super::{PacketDirection, PacketMetadata};
    use crate::capture::Endpoint;
    use std::net::{IpAddr, Ipv4Addr};

    fn endpoint(port: u16) -> Endpoint {
        Endpoint {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port,
        }
    }

    #[test]
    fn metadata_direction_is_server_to_client_when_source_is_albion_port() {
        let metadata = PacketMetadata::new("test".to_string(), 1, endpoint(5056), endpoint(9999));

        assert_eq!(metadata.direction, PacketDirection::ServerToClient);
    }

    #[test]
    fn metadata_direction_is_client_to_server_when_destination_is_albion_port() {
        let metadata = PacketMetadata::new("test".to_string(), 1, endpoint(9999), endpoint(4535));

        assert_eq!(metadata.direction, PacketDirection::ClientToServer);
    }

    #[test]
    fn metadata_direction_is_unknown_when_neither_endpoint_is_albion_port() {
        let metadata = PacketMetadata::new("test".to_string(), 1, endpoint(1000), endpoint(2000));

        assert_eq!(metadata.direction, PacketDirection::Unknown);
    }
}
