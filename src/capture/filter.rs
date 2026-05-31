// src/capture/filter.rs

use crate::capture::{Endpoint, UdpPacket, HostFilter};

#[derive(Clone, Debug)]
pub struct CaptureFilter {
    ports: Vec<u16>,
    host_filter: Option<HostFilter>,
}

impl Default for CaptureFilter {
    fn default() -> Self {
        Self {
            ports: vec![5056, 4535],
            host_filter: None,
        }
    }
}

impl CaptureFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_ports<I>(ports: I) -> Self
    where
        I: IntoIterator<Item = u16>,
    {
        Self {
            ports: ports.into_iter().collect(),
            host_filter: None,
        }
    }

    pub fn any_port() -> Self {
        Self {
            ports: Vec::new(),
            host_filter: None,
        }
    }

    pub fn set_host_filter(&mut self, host_filter: HostFilter) {
        self.host_filter = Some(host_filter);
    }

    pub fn with_host_filter(mut self, host_filter: HostFilter) -> Self {
        self.host_filter = Some(host_filter);
        self
    }

    pub fn matches_udp_packet(&self, packet: &UdpPacket<'_>) -> bool {
        self.matches_endpoint(&packet.source) || self.matches_endpoint(&packet.destination)
    }

    pub fn matches_endpoint(&self, endpoint: &Endpoint) -> bool {
        let port_matches = self.ports.is_empty() || self.ports.contains(&endpoint.port);

        let host_matches = self
            .host_filter
            .as_ref()
            .map(|filter| filter.contains(endpoint.ip))
            .unwrap_or(true);

        port_matches && host_matches
    }

    pub fn ports(&self) -> &[u16] {
        &self.ports
    }

    pub fn host_filter(&self) -> Option<&HostFilter> {
        self.host_filter.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::CaptureFilter;
    use crate::capture::{Endpoint, HostFilter, UdpPacket};
    use std::net::{IpAddr, Ipv4Addr};

    fn endpoint(ip: [u8; 4], port: u16) -> Endpoint {
        Endpoint {
            ip: IpAddr::V4(Ipv4Addr::from(ip)),
            port,
        }
    }

    fn udp_packet(source: Endpoint, destination: Endpoint) -> UdpPacket<'static> {
        UdpPacket {
            source,
            destination,
            payload: &[],
        }
    }

    #[test]
    fn default_filter_matches_port_5056() {
        let filter = CaptureFilter::new();

        assert!(filter.matches_endpoint(&endpoint([1, 2, 3, 4], 5056)));
    }

    #[test]
    fn default_filter_matches_port_4535() {
        let filter = CaptureFilter::new();

        assert!(filter.matches_endpoint(&endpoint([1, 2, 3, 4], 4535)));
    }

    #[test]
    fn default_filter_rejects_unrelated_ports() {
        let filter = CaptureFilter::new();

        assert!(!filter.matches_endpoint(&endpoint([1, 2, 3, 4], 9999)));
    }

    #[test]
    fn any_port_matches_unrelated_ports() {
        let filter = CaptureFilter::any_port();

        assert!(filter.matches_endpoint(&endpoint([1, 2, 3, 4], 9999)));
    }

    #[test]
    fn with_ports_restricts_to_provided_list() {
        let filter = CaptureFilter::with_ports([6000, 7000]);

        assert!(filter.matches_endpoint(&endpoint([1, 2, 3, 4], 6000)));
        assert!(filter.matches_endpoint(&endpoint([1, 2, 3, 4], 7000)));
        assert!(!filter.matches_endpoint(&endpoint([1, 2, 3, 4], 5056)));
    }

    #[test]
    fn matches_udp_packet_matches_either_source_or_destination() {
        let filter = CaptureFilter::with_ports([5056]);
        let packet = udp_packet(endpoint([10, 0, 0, 1], 1234), endpoint([10, 0, 0, 2], 5056));

        assert!(filter.matches_udp_packet(&packet));

        let packet = udp_packet(endpoint([10, 0, 0, 1], 5056), endpoint([10, 0, 0, 2], 1234));

        assert!(filter.matches_udp_packet(&packet));
    }

    #[test]
    fn host_filter_restricts_ips_when_present() {
        let host_filter = HostFilter::from_cidrs(["192.168.1.0/24"]).unwrap();
        let filter = CaptureFilter::new().with_host_filter(host_filter);

        assert!(filter.matches_endpoint(&endpoint([192, 168, 1, 10], 5056)));
        assert!(!filter.matches_endpoint(&endpoint([10, 0, 0, 10], 5056)));
    }

    #[test]
    fn empty_ports_plus_host_filter_means_any_port_on_matching_hosts() {
        let host_filter = HostFilter::from_cidrs(["192.168.1.0/24"]).unwrap();
        let filter = CaptureFilter::any_port().with_host_filter(host_filter);

        assert!(filter.matches_endpoint(&endpoint([192, 168, 1, 10], 9999)));
        assert!(!filter.matches_endpoint(&endpoint([10, 0, 0, 10], 9999)));
    }
}
