use crate::models::AuctionType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CachedOrder {
    pub amount: i64,
    pub auction_type: AuctionType,
    pub buyer_character_id: Option<String>,
    pub buyer_name: Option<String>,
    pub distance_fee: i64,
    pub enchantment_level: i64,
    pub expires: String,
    pub has_buyer_fetched: bool,
    pub has_seller_fetched: bool,
    pub id: i64,
    pub is_finished: bool,
    pub item_group_type_id: String,
    pub item_type_id: String,
    pub location_id: Option<String>,
    pub location_name: Option<String>,
    pub friendly_location_name: Option<String>,
    pub quality_level: i64,
    pub reference_id: String,
    pub seller_character_id: Option<String>,
    pub seller_name: Option<String>,
    pub tier: i64,
    pub total_price_silver: i64,
    pub unit_price_silver: i64,
}

#[cfg(test)]
mod tests {
    use super::CachedOrder;
    use crate::models::AuctionType;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Deserialize)]
    struct CachedOrderFixture {
        cached_order: CachedOrder,
    }

    #[derive(Deserialize)]
    struct MarketOrdersFixture {
        market_orders: Vec<CachedOrder>,
    }

    #[test]
    fn parses_cached_order_request_examples() {
        let buy_offer: CachedOrderFixture = serde_json::from_str(include_str!(
            "../../examples/auction_buy_offer_request.json"
        ))
        .unwrap();
        let sell_specific_item: CachedOrderFixture = serde_json::from_str(include_str!(
            "../../examples/auction_sell_specific_item_request.json"
        ))
        .unwrap();

        assert_eq!(buy_offer.cached_order.id, 14978117778);
        assert_eq!(buy_offer.cached_order.auction_type, AuctionType::Offer);
        assert_eq!(sell_specific_item.cached_order.id, 14977174637);
        assert_eq!(
            sell_specific_item.cached_order.auction_type,
            AuctionType::Request
        );
    }

    #[test]
    fn parses_market_orders_response_example() {
        let get_requests: MarketOrdersFixture = serde_json::from_str(include_str!(
            "../../examples/auction_get_requests_response.json"
        ))
        .unwrap();

        assert_eq!(get_requests.market_orders.len(), 1);
        assert_eq!(get_requests.market_orders[0].id, 14977174637);
        assert_eq!(
            get_requests.market_orders[0].auction_type,
            AuctionType::Request
        );
    }

    #[test]
    fn preserves_unknown_auction_type() {
        let mut value = json!({
            "Amount": 1,
            "AuctionType": "custom",
            "BuyerCharacterId": null,
            "BuyerName": null,
            "DistanceFee": 0,
            "EnchantmentLevel": 0,
            "Expires": "2026-06-25T07:55:20.513833",
            "HasBuyerFetched": false,
            "HasSellerFetched": false,
            "Id": 14990497605_i64,
            "IsFinished": false,
            "ItemGroupTypeId": "T1_HIDE",
            "ItemTypeId": "T1_HIDE",
            "LocationId": null,
            "QualityLevel": 1,
            "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec287",
            "SellerCharacterId": null,
            "SellerName": null,
            "Tier": 1,
            "TotalPriceSilver": 500000,
            "UnitPriceSilver": 50000
        });
        value["LocationName"] = json!(null);
        value["FriendlyLocationName"] = json!(null);

        let order: CachedOrder = serde_json::from_value(value).unwrap();

        assert_eq!(
            order.auction_type,
            AuctionType::Unknown("custom".to_string())
        );
    }

    #[test]
    fn parses_location_id_as_index_string() {
        let value = json!({
            "Amount": 1,
            "AuctionType": "offer",
            "BuyerCharacterId": null,
            "BuyerName": null,
            "DistanceFee": 0,
            "EnchantmentLevel": 0,
            "Expires": "2026-06-25T07:55:20.513833",
            "HasBuyerFetched": false,
            "HasSellerFetched": false,
            "Id": 14990497605_i64,
            "IsFinished": false,
            "ItemGroupTypeId": "T1_HIDE",
            "ItemTypeId": "T1_HIDE",
            "LocationId": "3008",
            "QualityLevel": 1,
            "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec287",
            "SellerCharacterId": null,
            "SellerName": null,
            "Tier": 1,
            "TotalPriceSilver": 500000,
            "UnitPriceSilver": 50000
        });

        let order: CachedOrder = serde_json::from_value(value).unwrap();

        assert_eq!(order.location_id.as_deref(), Some("3008"));
    }
}
