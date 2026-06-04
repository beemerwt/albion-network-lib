use std::{fmt, net::IpAddr};

use serde::Serialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Endpoint {
    pub ip: IpAddr,
    pub port: u16,
}

impl Endpoint {
    pub fn is_albion_port(&self) -> bool {
        matches!(self.port, 5056 | 4535)
    }

    pub fn from_str(ip_str: &str) -> Self {
        let (ip, port) = ip_str.split_once(':').expect("invalid endpoint format");
        Self {
            ip: ip.parse().expect("invalid IP address"),
            port: port.parse().expect("invalid port"),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.ip, self.port)
    }
}
