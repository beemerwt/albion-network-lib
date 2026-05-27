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
        mail_string: &str,
        location: Option<AlbionLocation>,
    ) -> Self {
        let mut mail = Self {
            id,
            location_id,
            player_name,
            info_type,
            auction_type: info_type.auction_type(),
            received,
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
        };
        mail.set_data(mail_string);
        mail
    }

    fn set_data(&mut self, mail_string: &str) {
        let data = self.get_data(mail_string);
        self.partial_amount = data.partial_amount;
        self.total_amount = data.total_amount;
        self.item_id = data.item_id;
        self.total_silver = data.total_silver;
        self.unit_silver = data.unit_silver;
        self.total_taxes = data.total_taxes;
        self.is_set = true;
    }

    fn get_data(&self, mail_string: &str) -> MailData {
        self.parse_mail_data(mail_string).unwrap_or_default()
    }

    fn parse_mail_data(&self, mail_string: &str) -> Option<MailData> {
        let parts: Vec<&str> = mail_string.split('|').collect();
        match self.info_type {
            MailInfoType::MarketPlaceSellOrderFinishedSummary
            | MailInfoType::MarketPlaceBuyOrderFinishedSummary => {
                let partial_amount = parse_i32(parts.first()?)?;
                let item_id = parts.get(1)?.to_string();
                let total_silver = parse_i64(parts.get(2)?)? / 10_000;
                let unit_silver =
                    normalize_unit_silver(parse_i64(parts.get(3)?)? as f64 / 10_000.0);
                Some(MailData {
                    partial_amount,
                    total_amount: partial_amount,
                    item_id,
                    total_silver,
                    unit_silver,
                    total_taxes: 0,
                })
            }
            MailInfoType::MarketPlaceBuyOrderExpiredSummary => {
                let partial_amount = parse_i32(parts.first()?)?;
                let total_amount = parse_i32(parts.get(1)?)?;
                let total_refund = parse_i64(parts.get(2)?)? as f64 / 10_000.0;
                let item_id = parts.get(3)?.to_string();
                let remaining_amount = total_amount - partial_amount;
                let unit_silver = normalize_unit_silver(if remaining_amount > 0 {
                    total_refund / remaining_amount as f64
                } else {
                    0.0
                });
                let total_silver = (unit_silver * partial_amount as f64).round() as i64;
                Some(MailData {
                    partial_amount,
                    total_amount,
                    item_id,
                    total_silver,
                    unit_silver,
                    total_taxes: 0,
                })
            }
            MailInfoType::MarketPlaceSellOrderExpiredSummary
            | MailInfoType::BlackMarketSellOrderExpiredSummary => {
                let partial_amount = parse_i32(parts.first()?)?;
                let total_amount = parse_i32(parts.get(1)?)?;
                let total_silver = parse_i64(parts.get(2)?)? / 10_000;
                let item_id = parts.get(3)?.to_string();
                let unit_silver = normalize_unit_silver(if partial_amount == 0 {
                    0.0
                } else {
                    total_silver as f64 / partial_amount as f64
                });
                Some(MailData {
                    partial_amount,
                    total_amount,
                    item_id,
                    total_silver,
                    unit_silver,
                    total_taxes: 0,
                })
            }
            MailInfoType::Unknown => Some(MailData::default()),
        }
    }
}

#[derive(Default)]
struct MailData {
    partial_amount: i32,
    total_amount: i32,
    item_id: String,
    total_silver: i64,
    unit_silver: f64,
    total_taxes: i64,
}

fn parse_i32(value: &str) -> Option<i32> {
    value.parse().ok()
}

fn parse_i64(value: &str) -> Option<i64> {
    value.parse().ok()
}

fn normalize_unit_silver(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
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
            "2|T4_HIDE|100000|50000",
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
        assert_eq!(mail.partial_amount, 2);
        assert_eq!(mail.total_amount, 2);
        assert_eq!(mail.item_id, "T4_HIDE");
        assert_eq!(mail.total_silver, 10);
        assert_eq!(mail.unit_silver, 5.0);
        assert_eq!(mail.total_taxes, 0);
        assert!(mail.is_set);
        assert!(!mail.deleted);
        assert_eq!(
            mail.location,
            Some(AlbionLocation::Known {
                index: "2000".to_string(),
                unique_name: "Bridgewatch".to_string(),
            })
        );
    }

    #[test]
    fn parses_buy_order_finished_summary() {
        let mail = mail(
            MailInfoType::MarketPlaceBuyOrderFinishedSummary,
            "3|T5_ORE|210000|70000",
        );

        assert_eq!(mail.partial_amount, 3);
        assert_eq!(mail.total_amount, 3);
        assert_eq!(mail.item_id, "T5_ORE");
        assert_eq!(mail.total_silver, 21);
        assert_eq!(mail.unit_silver, 7.0);
        assert!(mail.is_set);
    }

    #[test]
    fn parses_buy_order_expired_summary() {
        let mail = mail(
            MailInfoType::MarketPlaceBuyOrderExpiredSummary,
            "2|5|900000|T6_FIBER",
        );

        assert_eq!(mail.partial_amount, 2);
        assert_eq!(mail.total_amount, 5);
        assert_eq!(mail.item_id, "T6_FIBER");
        assert_eq!(mail.unit_silver, 30.0);
        assert_eq!(mail.total_silver, 60);
        assert!(mail.is_set);
    }

    #[test]
    fn normalizes_unit_silver_to_two_decimals_away_from_zero() {
        let mail = mail(
            MailInfoType::MarketPlaceBuyOrderExpiredSummary,
            "2|5|1000050|T6_FIBER",
        );

        assert_eq!(mail.unit_silver, 33.34);
        assert_eq!(mail.total_silver, 67);
    }

    #[test]
    fn parses_sell_order_expired_summary() {
        let mail = mail(
            MailInfoType::MarketPlaceSellOrderExpiredSummary,
            "4|10|1200000|T7_WOOD",
        );

        assert_eq!(mail.partial_amount, 4);
        assert_eq!(mail.total_amount, 10);
        assert_eq!(mail.item_id, "T7_WOOD");
        assert_eq!(mail.total_silver, 120);
        assert_eq!(mail.unit_silver, 30.0);
        assert!(mail.is_set);
    }

    #[test]
    fn parses_black_market_sell_order_expired_summary() {
        let mail = mail(
            MailInfoType::BlackMarketSellOrderExpiredSummary,
            "0|10|1200000|T7_WOOD",
        );

        assert_eq!(mail.partial_amount, 0);
        assert_eq!(mail.total_amount, 10);
        assert_eq!(mail.item_id, "T7_WOOD");
        assert_eq!(mail.total_silver, 120);
        assert_eq!(mail.unit_silver, 0.0);
        assert!(mail.is_set);
    }

    #[test]
    fn malformed_mail_string_sets_default_data() {
        let mail = mail(
            MailInfoType::MarketPlaceSellOrderFinishedSummary,
            "not|enough",
        );

        assert_eq!(mail.partial_amount, 0);
        assert_eq!(mail.total_amount, 0);
        assert_eq!(mail.item_id, "");
        assert_eq!(mail.total_silver, 0);
        assert_eq!(mail.unit_silver, 0.0);
        assert_eq!(mail.total_taxes, 0);
        assert!(mail.is_set);
    }

    fn mail(info_type: MailInfoType, mail_string: &str) -> AlbionMail {
        AlbionMail::from_correlated(
            42,
            "2000".to_string(),
            "PlayerOne".to_string(),
            info_type,
            1_717_171_717,
            mail_string,
            None,
        )
    }
}
