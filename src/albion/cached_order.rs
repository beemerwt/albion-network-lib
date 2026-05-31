use crate::albion::{AuctionType, world::AlbionLocation};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
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
    pub item_id: String,
    pub item_name: Option<String>,
    pub location: AlbionLocation,
    pub quality_level: i64,
    pub reference_id: String,
    pub seller_character_id: Option<String>,
    pub seller_name: Option<String>,
    pub tier: i64,
    pub total_price_silver: i64,
    pub unit_price_silver: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawCachedOrder {
    amount: i64,
    auction_type: AuctionType,
    buyer_character_id: Option<String>,
    buyer_name: Option<String>,
    distance_fee: i64,
    enchantment_level: i64,
    expires: String,
    has_buyer_fetched: bool,
    has_seller_fetched: bool,
    id: i64,
    is_finished: bool,
    item_group_type_id: String,
    item_type_id: String,
    location_id: Option<Value>,
    quality_level: i64,
    reference_id: String,
    seller_character_id: Option<String>,
    seller_name: Option<String>,
    tier: i64,
    total_price_silver: i64,
    unit_price_silver: i64,
}

impl<'de> Deserialize<'de> for CachedOrder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCachedOrder::deserialize(deserializer)?;
        Ok(Self {
            amount: raw.amount,
            auction_type: raw.auction_type,
            buyer_character_id: raw.buyer_character_id,
            buyer_name: raw.buyer_name,
            distance_fee: raw.distance_fee,
            enchantment_level: raw.enchantment_level,
            expires: raw.expires,
            has_buyer_fetched: raw.has_buyer_fetched,
            has_seller_fetched: raw.has_seller_fetched,
            id: raw.id,
            is_finished: raw.is_finished,
            item_group_type_id: raw.item_group_type_id,
            item_id: raw.item_type_id,
            item_name: None,
            location: location_from_value(raw.location_id),
            quality_level: raw.quality_level,
            reference_id: raw.reference_id,
            seller_character_id: raw.seller_character_id,
            seller_name: raw.seller_name,
            tier: raw.tier,
            total_price_silver: raw.total_price_silver,
            unit_price_silver: raw.unit_price_silver,
        })
    }
}

fn location_from_value(value: Option<Value>) -> AlbionLocation {
    match value {
        Some(Value::String(id)) if !id.is_empty() => AlbionLocation::from_id(id),
        _ => AlbionLocation::unknown(),
    }
}

#[cfg(test)]
mod tests {
    use super::CachedOrder;
    use crate::albion::AuctionType;
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
        let value = json!({
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
        let order: CachedOrder = serde_json::from_value(value).unwrap();

        assert_eq!(
            order.auction_type,
            AuctionType::Unknown("custom".to_string())
        );
        assert_eq!(order.item_name, None);
    }

    #[test]
    fn parses_location_id_as_id_only_location() {
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

        assert_eq!(
            order.location,
            crate::albion::world::AlbionLocation::from_id("3008")
        );
    }

    #[test]
    fn missing_or_malformed_location_id_deserializes_as_unknown_location() {
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
            "LocationId": 3008,
            "QualityLevel": 1,
            "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec287",
            "SellerCharacterId": null,
            "SellerName": null,
            "Tier": 1,
            "TotalPriceSilver": 500000,
            "UnitPriceSilver": 50000
        });

        let order: CachedOrder = serde_json::from_value(value).unwrap();

        assert_eq!(
            order.location,
            crate::albion::world::AlbionLocation::unknown()
        );
    }
}
