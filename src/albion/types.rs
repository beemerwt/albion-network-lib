use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuctionType {
    Offer,
    Request,
    Unknown(String),
}

impl AuctionType {
    pub fn from_str(value: &str) -> Self {
        match value {
            value if value.eq_ignore_ascii_case("offer") => Self::Offer,
            value if value.eq_ignore_ascii_case("request") => Self::Request,
            value => Self::Unknown(value.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Offer => "offer",
            Self::Request => "request",
            Self::Unknown(value) => value,
        }
    }
}

impl<'de> Deserialize<'de> for AuctionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_str(&value))
    }
}

impl Serialize for AuctionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationType {
    Buy,
    Sell,
    Unknown(String),
}

impl OperationType {
    // TODO: Offer and Request are reversed when is_instant
    pub fn from_auction_type(auction_type: &AuctionType, trade_type: &TradeType) -> Self {
        match trade_type {
            TradeType::Instant => match auction_type {
                AuctionType::Offer => Self::Buy,
                AuctionType::Request => Self::Sell,
                AuctionType::Unknown(value) => Self::Unknown(value.clone()),
            },
            TradeType::Order => match auction_type {
                AuctionType::Offer => Self::Sell,
                AuctionType::Request => Self::Buy,
                AuctionType::Unknown(value) => Self::Unknown(value.clone()),
            },
            TradeType::Unknown(value) => Self::Unknown(value.clone()),
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            value if value.eq_ignore_ascii_case("buy") => Self::Buy,
            value if value.eq_ignore_ascii_case("sell") => Self::Sell,
            value => Self::Unknown(value.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Unknown(value) => value,
        }
    }
}

impl<'de> Deserialize<'de> for OperationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_str(&value))
    }
}

impl Serialize for OperationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TradeType {
    Instant,
    Order,
    Unknown(String),
}

impl TradeType {
    pub fn from_str(value: &str) -> Self {
        match value {
            value if value.eq_ignore_ascii_case("instant") => Self::Instant,
            value if value.eq_ignore_ascii_case("order") => Self::Order,
            value => Self::Unknown(value.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Instant => "instant",
            Self::Order => "order",
            Self::Unknown(value) => value,
        }
    }
}

impl<'de> Deserialize<'de> for TradeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::from_str(&value))
    }
}

impl Serialize for TradeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum MailInfoType {
    Unknown = 0,
    MarketPlaceSellOrderFinishedSummary = 1,
    MarketPlaceBuyOrderFinishedSummary = 2,
    MarketPlaceBuyOrderExpiredSummary = 3,
    MarketPlaceSellOrderExpiredSummary = 4,
    BlackMarketSellOrderExpiredSummary = 5,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailInfoMetadata {
    pub mail_id: i64,
    pub location_id: String,
    pub info_type: MailInfoType,
    pub received: i64,
}

impl MailInfoType {
    pub fn from_i64(value: i64) -> Self {
        match value {
            1 => Self::MarketPlaceSellOrderFinishedSummary,
            2 => Self::MarketPlaceBuyOrderFinishedSummary,
            3 => Self::MarketPlaceBuyOrderExpiredSummary,
            4 => Self::MarketPlaceSellOrderExpiredSummary,
            5 => Self::BlackMarketSellOrderExpiredSummary,
            _ => Self::Unknown,
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            value if value.eq_ignore_ascii_case("MARKETPLACE_SELLORDER_FINISHED_SUMMARY") => {
                Self::MarketPlaceSellOrderFinishedSummary
            }
            value if value.eq_ignore_ascii_case("MARKETPLACE_BUYORDER_FINISHED_SUMMARY") => {
                Self::MarketPlaceBuyOrderFinishedSummary
            }
            value if value.eq_ignore_ascii_case("MARKETPLACE_BUYORDER_EXPIRED_SUMMARY") => {
                Self::MarketPlaceBuyOrderExpiredSummary
            }
            value if value.eq_ignore_ascii_case("MARKETPLACE_SELLORDER_EXPIRED_SUMMARY") => {
                Self::MarketPlaceSellOrderExpiredSummary
            }
            value if value.eq_ignore_ascii_case("BLACKMARKET_SELLORDER_EXPIRED_SUMMARY") => {
                Self::BlackMarketSellOrderExpiredSummary
            }
            _ => Self::Unknown,
        }
    }

    pub fn auction_type(self) -> AuctionType {
        match self {
            Self::MarketPlaceSellOrderFinishedSummary
            | Self::MarketPlaceSellOrderExpiredSummary
            | Self::BlackMarketSellOrderExpiredSummary => AuctionType::Offer,
            Self::MarketPlaceBuyOrderFinishedSummary | Self::MarketPlaceBuyOrderExpiredSummary => {
                AuctionType::Request
            }
            Self::Unknown => AuctionType::Unknown("unknown_mail_info_type".to_string()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[repr(u8)]
pub enum ChatChannel {
    Say = 27,
    Guild = 24,
    Faction = 29,
}

impl ChatChannel {
    pub fn from_chat_index(value: i64) -> Self {
        match value {
            27 => ChatChannel::Say,
            24 => ChatChannel::Guild,
            29 => ChatChannel::Faction,
            _ => ChatChannel::Say,
        }
    }

    pub fn from_i64(value: i64) -> Self {
        match value {
            0 => ChatChannel::Say,
            3517 => ChatChannel::Guild,
            1868 => ChatChannel::Faction, // Thetford
            1856 => ChatChannel::Faction, // Martlock
            _ => ChatChannel::Say,        // Default to Say for unknown channels
        }
    }
}

impl fmt::Display for ChatChannel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ChatChannel::Say => write!(f, "Local"),
            ChatChannel::Faction => write!(f, "Faction"),
            ChatChannel::Guild => write!(f, "Guild"),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::String(value) => Self::from_str(value),
            Value::Object(map) => map
                .get("data_hex")
                .and_then(Value::as_str)
                .and_then(Self::from_photon_data_hex),
            _ => None,
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        let hex: String = value.chars().filter(|char| *char != '-').collect();
        if hex.len() != 32 || !hex.chars().all(|char| char.is_ascii_hexdigit()) {
            return None;
        }

        Some(Self {
            data1: u32::from_str_radix(&hex[0..8], 16).ok()?,
            data2: u16::from_str_radix(&hex[8..12], 16).ok()?,
            data3: u16::from_str_radix(&hex[12..16], 16).ok()?,
            data4: [
                u8::from_str_radix(&hex[16..18], 16).ok()?,
                u8::from_str_radix(&hex[18..20], 16).ok()?,
                u8::from_str_radix(&hex[20..22], 16).ok()?,
                u8::from_str_radix(&hex[22..24], 16).ok()?,
                u8::from_str_radix(&hex[24..26], 16).ok()?,
                u8::from_str_radix(&hex[26..28], 16).ok()?,
                u8::from_str_radix(&hex[28..30], 16).ok()?,
                u8::from_str_radix(&hex[30..32], 16).ok()?,
            ],
        })
    }

    pub fn from_photon_data_hex(value: &str) -> Option<Self> {
        let hex: String = value
            .chars()
            .filter(|char| !char.is_ascii_whitespace() && *char != '-')
            .collect();
        if hex.len() != 32 || !hex.chars().all(|char| char.is_ascii_hexdigit()) {
            return None;
        }

        let mut bytes = [0; 16];
        for index in 0..16 {
            bytes[index] = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
        }

        Some(Self {
            data1: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            data2: u16::from_le_bytes(bytes[4..6].try_into().ok()?),
            data3: u16::from_le_bytes(bytes[6..8].try_into().ok()?),
            data4: bytes[8..16].try_into().ok()?,
        })
    }
}

impl Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let guid_string = format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        );
        serializer.serialize_str(&guid_string)
    }
}

#[cfg(test)]
mod tests {
    use super::{AuctionType, MailInfoType, OperationType, TradeType};

    #[test]
    fn auction_type_maps_known_strings_and_preserves_unknowns() {
        assert_eq!(AuctionType::from_str("offer"), AuctionType::Offer);
        assert_eq!(AuctionType::from_str("request"), AuctionType::Request);
        assert_eq!(
            AuctionType::from_str("unexpected"),
            AuctionType::Unknown("unexpected".to_string())
        );
    }

    #[test]
    fn operation_type_correlates_to_auction_type() {
        assert_eq!(
            OperationType::from_auction_type(&AuctionType::Offer, &TradeType::Instant),
            OperationType::Buy
        );
        assert_eq!(
            OperationType::from_auction_type(&AuctionType::Request, &TradeType::Instant),
            OperationType::Sell
        );
        assert_eq!(
            OperationType::from_auction_type(&AuctionType::Offer, &TradeType::Order),
            OperationType::Sell
        );
        assert_eq!(
            OperationType::from_auction_type(&AuctionType::Request, &TradeType::Order),
            OperationType::Buy
        );
        assert_eq!(
            OperationType::from_auction_type(
                &AuctionType::Unknown("weird".to_string()),
                &TradeType::Unknown("weird".to_string())
            ),
            OperationType::Unknown("weird".to_string())
        );
    }

    #[test]
    fn trade_type_maps_known_strings_and_preserves_unknowns() {
        assert_eq!(TradeType::from_str("instant"), TradeType::Instant);
        assert_eq!(TradeType::from_str("order"), TradeType::Order);
        assert_eq!(
            TradeType::from_str("custom"),
            TradeType::Unknown("custom".to_string())
        );
    }

    #[test]
    fn mail_info_type_maps_known_codes_and_defaults_unknowns() {
        assert_eq!(
            MailInfoType::from_i64(1),
            MailInfoType::MarketPlaceSellOrderFinishedSummary
        );
        assert_eq!(
            MailInfoType::from_i64(2),
            MailInfoType::MarketPlaceBuyOrderFinishedSummary
        );
        assert_eq!(MailInfoType::from_i64(99), MailInfoType::Unknown);
    }

    #[test]
    fn mail_info_type_maps_source_strings_and_defaults_unknowns() {
        assert_eq!(
            MailInfoType::from_str("MARKETPLACE_SELLORDER_FINISHED_SUMMARY"),
            MailInfoType::MarketPlaceSellOrderFinishedSummary
        );
        assert_eq!(
            MailInfoType::from_str("marketplace_buyorder_finished_summary"),
            MailInfoType::MarketPlaceBuyOrderFinishedSummary
        );
        assert_eq!(
            MailInfoType::from_str("MARKETPLACE_BUYORDER_EXPIRED_SUMMARY"),
            MailInfoType::MarketPlaceBuyOrderExpiredSummary
        );
        assert_eq!(
            MailInfoType::from_str("MARKETPLACE_SELLORDER_EXPIRED_SUMMARY"),
            MailInfoType::MarketPlaceSellOrderExpiredSummary
        );
        assert_eq!(
            MailInfoType::from_str("BLACKMARKET_SELLORDER_EXPIRED_SUMMARY"),
            MailInfoType::BlackMarketSellOrderExpiredSummary
        );
        assert_eq!(MailInfoType::from_str("unexpected"), MailInfoType::Unknown);
    }

    #[test]
    fn mail_info_type_maps_to_auction_type() {
        assert_eq!(
            MailInfoType::MarketPlaceSellOrderFinishedSummary.auction_type(),
            AuctionType::Offer
        );
        assert_eq!(
            MailInfoType::MarketPlaceBuyOrderFinishedSummary.auction_type(),
            AuctionType::Request
        );
        assert_eq!(
            MailInfoType::Unknown.auction_type(),
            AuctionType::Unknown("unknown_mail_info_type".to_string())
        );
    }
}
