use crate::models::{AlbionLocation, AuctionType, MailInfoType};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AlbionMail {
    pub id: i64,
    pub location_id: String,
    pub player_name: String,
    pub info_type: MailInfoType,
    pub auction_type: AuctionType,
    pub received: i64,
    pub mail_string: String,
    pub server_id: i32,
    pub partial_amount: i32,
    pub total_amount: i32,
    pub item_id: String,
    pub total_silver: i64,
    pub unit_silver: f64,
    pub taxes_percent: f64,
    pub total_taxes: i64,
    pub is_set: bool,
    pub deleted: bool,
    pub item_name: String,
    pub location: Option<AlbionLocation>,
}

impl AlbionMail {
    pub fn from_correlated(
        id: i64,
        location_id: String,
        player_name: String,
        info_type: MailInfoType,
        received: i64,
        mail_string: String,
        location: Option<AlbionLocation>,
    ) -> Self {
        Self {
            id,
            location_id,
            player_name,
            info_type,
            auction_type: info_type.auction_type(),
            received,
            mail_string,
            server_id: 0,
            partial_amount: 0,
            total_amount: 0,
            item_id: String::new(),
            total_silver: 0,
            unit_silver: 0.0,
            taxes_percent: 0.0,
            total_taxes: 0,
            is_set: false,
            deleted: false,
            item_name: String::new(),
            location,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AlbionMail;
    use crate::models::{AlbionLocation, AuctionType, MailInfoType};

    #[test]
    fn builds_first_pass_mail_from_correlated_parts() {
        let mail = AlbionMail::from_correlated(
            42,
            "2000".to_string(),
            "PlayerOne".to_string(),
            MailInfoType::MarketPlaceSellOrderFinishedSummary,
            1_717_171_717,
            "raw body".to_string(),
            Some(AlbionLocation::Known {
                index: "2000".to_string(),
                unique_name: "Bridgewatch".to_string(),
            }),
        );

        assert_eq!(mail.id, 42);
        assert_eq!(mail.location_id, "2000");
        assert_eq!(mail.player_name, "PlayerOne");
        assert_eq!(
            mail.info_type,
            MailInfoType::MarketPlaceSellOrderFinishedSummary
        );
        assert_eq!(mail.auction_type, AuctionType::Offer);
        assert_eq!(mail.received, 1_717_171_717);
        assert_eq!(mail.mail_string, "raw body");
        assert_eq!(mail.total_silver, 0);
        assert!(!mail.is_set);
        assert!(!mail.deleted);
        assert_eq!(
            mail.location,
            Some(AlbionLocation::Known {
                index: "2000".to_string(),
                unique_name: "Bridgewatch".to_string(),
            })
        );
    }
}
