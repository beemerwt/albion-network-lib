use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    pub fn from_auction_type(auction_type: &AuctionType) -> Self {
        match auction_type {
            AuctionType::Offer => Self::Buy,
            AuctionType::Request => Self::Sell,
            AuctionType::Unknown(value) => Self::Unknown(value.clone()),
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

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum MailInfoType {
    Unknown = 0,
    MarketPlaceSellOrderFinishedSummary = 1,
    MarketPlaceBuyOrderFinishedSummary = 2,
    MarketPlaceBuyOrderExpiredSummary = 3,
    MarketPlaceSellOrderExpiredSummary = 4,
    BlackMarketSellOrderExpiredSummary = 5,
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
            OperationType::from_auction_type(&AuctionType::Offer),
            OperationType::Buy
        );
        assert_eq!(
            OperationType::from_auction_type(&AuctionType::Request),
            OperationType::Sell
        );
        assert_eq!(
            OperationType::from_auction_type(&AuctionType::Unknown("weird".to_string())),
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
