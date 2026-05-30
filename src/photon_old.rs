use crate::{
    error::Result,
    event_codes::EventCode,
    extracted_packet::{ExtractedPacket, MarketPlaceNotification},
    models::{
        AlbionLocation, AlbionMail, CachedOrder, ChatChannel, MailInfoMetadata, OperationType,
        PlayerState, TradeType, WorldMap,
    },
    names,
    operation_codes::OperationCode,
    packet::{DecodedEvent, DecodedOperation, DecodedPacket, DecodedUnknown},
    protocol18::Protocol18Deserializer,
    requests::{
        auction_buy_offer::AuctionBuyOffer, auction_get_offers::AuctionGetOffers,
        auction_get_requests::AuctionGetRequests,
        auction_sell_specific_item::AuctionSellSpecificItem as AuctionSellSpecificItemRequest,
    },
    responses::{
        AuctionGetOffersResult, AuctionGetRequestsResult, AuctionTrade, AuctionTradeResponse,
        ChatMessage, GetMailInfos, JoinResponse, JoinedChatChannel, LeftChatChannel, ReadMail,
    },
    util::{params_to_json, read_i32_be, to_signed_short, value_i64},
};
use chrono::Utc;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

const COMMAND_DISCONNECT: u8 = 4;
const COMMAND_SEND_RELIABLE: u8 = 6;
const COMMAND_SEND_UNRELIABLE: u8 = 7;
const COMMAND_SEND_FRAGMENT: u8 = 8;

const MESSAGE_OPERATION_REQUEST: u8 = 2;
const MESSAGE_OPERATION_RESPONSE: u8 = 3;
const MESSAGE_EVENT: u8 = 4;
const ITEM_NAME_MAPPINGS_URL: &str =
    "https://cdn.albionfreemarket.com/AlbionFormattedItemsParser/us_name_mappings.json";

struct PendingSegment {
    payload: Vec<u8>,
    written: usize,
    total_length: usize,
}

struct CodeParseError {
    raw_code: Option<i32>,
    reason: &'static str,
    message: String,
}

fn parse_operation_code(
    params: &BTreeMap<u8, Value>,
    debug: bool,
) -> std::result::Result<OperationCode, CodeParseError> {
    let Some(value) = params.get(&253).and_then(value_i64) else {
        // If it fails, output the full json
        if debug {
            for (code, value) in params.into_iter() {
                println!("{}: {}", code, value.to_string());
            }
        }

        return Err(CodeParseError {
            raw_code: None,
            reason: "missing_operation_code",
            message: "Operation code parameter 253 is missing".to_string(),
        });
    };
    let code = to_signed_short(value);
    names::operation(code).ok_or_else(|| CodeParseError {
        raw_code: Some(code),
        reason: "unknown_operation_code",
        message: format!("Unknown operation code in parameter 253: {code}"),
    })
}

fn parse_event_code(
    params: &BTreeMap<u8, Value>,
) -> std::result::Result<EventCode, CodeParseError> {
    let Some(value) = params.get(&252).and_then(value_i64) else {
        return Err(CodeParseError {
            raw_code: None,
            reason: "missing_event_code",
            message: "Event code parameter 252 is missing".to_string(),
        });
    };
    let code = to_signed_short(value);
    if let Some(event_code) = names::event(code) {
        return Ok(event_code);
    }
    let unsigned_value = (code as i64 & 0xffff) as i32;
    let shifted = unsigned_value >> 4;
    if (unsigned_value & 0x0f) == 0x01 {
        if let Some(event_code) = names::event(shifted) {
            return Ok(event_code);
        }
    }
    Err(CodeParseError {
        raw_code: Some(code),
        reason: "unknown_event_code",
        message: format!("Unknown event code in parameter 252: {code}"),
    })
}

fn operation_from_cached_order(
    cached_order: Option<&CachedOrder>,
    trade_type: &TradeType,
) -> OperationType {
    cached_order
        .map(|order| OperationType::from_auction_type(&order.auction_type, trade_type))
        .unwrap_or_else(|| OperationType::Unknown("missing_cached_order".to_string()))
}

fn silver_amount(amount: Option<i64>, cached_order: Option<&CachedOrder>) -> Option<i64> {
    let amount = amount?;
    let order = cached_order?;
    Some(
        (((order.unit_price_silver * amount) - order.distance_fee) as f64 / 10_000.0).floor()
            as i64,
    )
}

fn direction(source: &str, destination: &str) -> &'static str {
    if source.ends_with(":5056") {
        "server_to_client"
    } else if destination.ends_with(":5056") {
        "client_to_server"
    } else {
        "unknown"
    }
}

fn normalize_mail_location_id(location_id: &str) -> String {
    if location_id == "@BLACK_MARKET" {
        return "3003".to_string();
    }
    location_id
        .split('@')
        .nth(1)
        .unwrap_or(location_id)
        .to_string()
}

fn download_item_names() -> HashMap<String, String> {
    let response = ureq::get(ITEM_NAME_MAPPINGS_URL)
        .call()
        .expect("failed to download item name mappings");
    let text = response
        .into_string()
        .expect("failed to read item name mappings response");
    parse_item_name_mappings(&text).expect("failed to parse item name mappings")
}

fn parse_item_name_mappings(json: &str) -> serde_json::Result<HashMap<String, String>> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AlbionLocation, AuctionType, MailInfoType};
    use serde_json::json;

    fn parser() -> PhotonParser {
        PhotonParser::with_world_map_and_item_names(
            "test".to_string(),
            false,
            WorldMap::from_embedded().unwrap(),
            item_names(),
        )
    }

    fn parser_with_unknown_capture() -> PhotonParser {
        let mut parser = parser();
        parser.capture_unknown_packets = true;
        parser
    }

    fn item_names() -> HashMap<String, String> {
        HashMap::from([
            ("T1_HIDE".to_string(), "Scraps of Hide".to_string()),
            ("T4_HIDE".to_string(), "Adept's Hide".to_string()),
        ])
    }

    #[test]
    fn parses_item_name_mappings() {
        let mappings =
            parse_item_name_mappings(r#"{"T1_HIDE":"Scraps of Hide","T4_HIDE":"Adept's Hide"}"#)
                .unwrap();

        assert_eq!(
            mappings.get("T1_HIDE").map(String::as_str),
            Some("Scraps of Hide")
        );
        assert_eq!(
            mappings.get("T4_HIDE").map(String::as_str),
            Some("Adept's Hide")
        );
    }

    #[test]
    fn invalid_item_name_mappings_return_parse_error() {
        assert!(parse_item_name_mappings("not json").is_err());
    }

    #[test]
    fn unknown_operation_code_errors_when_debug_capture_is_disabled() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(253, json!(32_767));

        let error = parser
            .record_operation("request", params, None, "", 1, "client:1", "server:5056")
            .unwrap_err();

        assert_eq!(error.0, "Unknown operation code in parameter 253: 32767");
        assert!(parser.decoded_packets().is_empty());
    }

    #[test]
    fn unknown_operation_code_is_captured_when_debug_capture_is_enabled() {
        let mut parser = parser_with_unknown_capture();
        let mut params = BTreeMap::new();
        params.insert(1, json!("payload"));
        params.insert(253, json!(32_767));

        parser
            .record_operation("request", params, None, "", 1, "client:1", "server:5056")
            .unwrap();

        let DecodedPacket::Unknown(packet) = &parser.decoded_packets()[0] else {
            panic!("expected unknown packet");
        };

        assert_eq!(packet.message_type, "operation_request");
        assert_eq!(packet.kind, "operation_request");
        assert_eq!(packet.code_parameter, 253);
        assert_eq!(packet.raw_code, Some(32_767));
        assert_eq!(packet.reason, "unknown_operation_code");
        assert_eq!(packet.direction, "client_to_server");
        assert_eq!(packet.parameters.get("1"), Some(&json!("payload")));
    }

    #[test]
    fn unknown_event_code_is_captured_when_debug_capture_is_enabled() {
        let mut parser = parser_with_unknown_capture();
        let mut params = BTreeMap::new();
        params.insert(0, json!("payload"));
        params.insert(252, json!(32_767));

        parser
            .record_event(0, params, 1, "server:5056", "client:1")
            .unwrap();

        let DecodedPacket::Unknown(packet) = &parser.decoded_packets()[0] else {
            panic!("expected unknown packet");
        };

        assert_eq!(packet.message_type, "event");
        assert_eq!(packet.kind, "event");
        assert_eq!(packet.code_parameter, 252);
        assert_eq!(packet.raw_code, Some(32_767));
        assert_eq!(packet.reason, "unknown_event_code");
        assert_eq!(packet.direction, "server_to_client");
        assert_eq!(packet.parameters.get("0"), Some(&json!("payload")));
    }

    #[test]
    fn missing_codes_are_captured_without_raw_code_when_debug_capture_is_enabled() {
        let mut parser = parser_with_unknown_capture();

        parser
            .record_operation(
                "response",
                BTreeMap::new(),
                Some(0),
                "debug",
                1,
                "server:5056",
                "client:1",
            )
            .unwrap();
        parser
            .record_event(0, BTreeMap::new(), 2, "server:5056", "client:1")
            .unwrap();

        let DecodedPacket::Unknown(operation) = &parser.decoded_packets()[0] else {
            panic!("expected unknown operation packet");
        };
        assert_eq!(operation.kind, "operation_response");
        assert_eq!(operation.code_parameter, 253);
        assert_eq!(operation.raw_code, None);
        assert_eq!(operation.reason, "missing_operation_code");
        assert_eq!(operation.return_code, Some(0));
        assert_eq!(operation.debug_message, "debug");

        let DecodedPacket::Unknown(event) = &parser.decoded_packets()[1] else {
            panic!("expected unknown event packet");
        };
        assert_eq!(event.kind, "event");
        assert_eq!(event.code_parameter, 252);
        assert_eq!(event.raw_code, None);
        assert_eq!(event.reason, "missing_event_code");
    }

    #[test]
    fn join_response_updates_player_state() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!(123));
        params.insert(2, json!("PlayerOne"));
        params.insert(8, json!("Bridgewatch"));

        let extracted = parser
            .extract_operation("response", OperationCode::Join, &params, Some(0))
            .unwrap();

        let ExtractedPacket::JoinResponse(response) = extracted else {
            panic!("expected join response");
        };

        assert_eq!(response.player_name.as_deref(), Some("PlayerOne"));
        assert_eq!(parser.player_state().user_object_id(), Some(123));
        assert_eq!(parser.player_state().player_name, "PlayerOne");
        assert_eq!(
            parser.player_state().location.friendly_name(),
            "Bridgewatch"
        );
        assert_eq!(parser.player_state().location.location_id(), Some(2000));
    }

    #[test]
    fn market_orders_inherit_missing_location_id_from_player_state() {
        let mut parser = parser();
        parser.player_state_mut().set_location_raw("Bridgewatch");

        let mut params = BTreeMap::new();
        params.insert(
            0,
            json!([
                {
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
                    "LocationId": null,
                    "QualityLevel": 1,
                    "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec287",
                    "SellerCharacterId": null,
                    "SellerName": null,
                    "Tier": 1,
                    "TotalPriceSilver": 500000,
                    "UnitPriceSilver": 50000
                },
                {
                    "Amount": 1,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-25T07:55:20.513833",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14990497606_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": "9999",
                    "QualityLevel": 1,
                    "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec288",
                    "SellerCharacterId": null,
                    "SellerName": null,
                    "Tier": 1,
                    "TotalPriceSilver": 500000,
                    "UnitPriceSilver": 50000
                }
            ]),
        );

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &params,
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionGetOffersResponse(response) = extracted else {
            panic!("expected auction get offers response");
        };

        assert_eq!(
            response.market_orders[0].location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(
            response.market_orders[1].location,
            AlbionLocation::unknown()
        );
    }

    #[test]
    fn market_orders_resolve_location_names_from_location_id() {
        let mut parser = parser();

        let mut params = market_order_params(14978117778);
        params.insert(
            0,
            json!([
                {
                    "Amount": 2,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-22T03:34:16.096699",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14978117778_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": "3008",
                    "QualityLevel": 1,
                    "ReferenceId": "7ae09894-4883-479b-932e-ff7914c82855",
                    "SellerCharacterId": "07b8fbc0-c512-4054-bc53-12312af94df3",
                    "SellerName": "CoelhoMalvado",
                    "Tier": 1,
                    "TotalPriceSilver": 100000,
                    "UnitPriceSilver": 50000
                }
            ]),
        );

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &params,
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionGetOffersResponse(response) = extracted else {
            panic!("expected auction get offers response");
        };

        assert_eq!(
            response.market_orders[0].location,
            AlbionLocation::with_names("3008", "Martlock Market", "Martlock Market")
        );
        assert_eq!(
            response.market_orders[0].item_name.as_deref(),
            Some("Scraps of Hide")
        );
    }

    #[test]
    fn market_orders_unknown_non_numeric_location_and_unknown_item_become_unknowns() {
        let mut parser = parser();
        let mut params = market_order_params(14978117778);
        params.insert(
            0,
            json!([
                {
                    "Amount": 2,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-22T03:34:16.096699",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14978117778_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "UNKNOWN_ITEM",
                    "ItemTypeId": "UNKNOWN_ITEM",
                    "LocationId": "NOT-A-WORLD-INDEX",
                    "QualityLevel": 1,
                    "ReferenceId": "7ae09894-4883-479b-932e-ff7914c82855",
                    "SellerCharacterId": "07b8fbc0-c512-4054-bc53-12312af94df3",
                    "SellerName": "CoelhoMalvado",
                    "Tier": 1,
                    "TotalPriceSilver": 100000,
                    "UnitPriceSilver": 50000
                }
            ]),
        );

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &params,
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionGetOffersResponse(response) = extracted else {
            panic!("expected auction get offers response");
        };

        assert_eq!(
            response.market_orders[0].location,
            AlbionLocation::unknown()
        );
        assert_eq!(response.market_orders[0].item_name, None);
    }

    #[test]
    fn auction_operations_return_typed_extracted_packets() {
        let mut parser = parser();
        let params = BTreeMap::new();

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionGetOffers, &params, None)
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionGetOffersRequest(_)
        ));

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionGetRequests, &params, None)
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionGetRequestsRequest(_)
        ));

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetRequests,
                &market_order_params(14978117778),
                Some(0),
            )
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionGetRequestsResponse(_)
        ));
    }

    #[test]
    fn buy_offer_request_exposes_cached_order_through_typed_default_flow() {
        let mut parser = parser();
        parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &market_order_params(14978117778),
                Some(0),
            )
            .unwrap();

        let mut params = BTreeMap::new();
        params.insert(1, json!(1));
        params.insert(2, json!(14978117778_i64));

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionBuyOffer, &params, None)
            .unwrap();

        let ExtractedPacket::AuctionBuyOfferRequest(request) = extracted else {
            panic!("expected auction buy offer request");
        };

        assert_eq!(request.amount, Some(1));
        assert_eq!(request.order_id, Some(14978117778));
        assert_eq!(
            request.cached_order.as_ref().map(|order| order.id),
            request.order_id
        );
        assert_eq!(
            parser.unconfirmed_trade.as_ref().map(|trade| trade.id),
            Some(14978117778)
        );
        assert_eq!(
            parser
                .unconfirmed_trade
                .as_ref()
                .map(|trade| trade.operation.clone()),
            Some(OperationType::Buy)
        );
        assert_eq!(
            parser
                .unconfirmed_trade
                .as_ref()
                .and_then(|trade| trade.silver_amount),
            Some(5)
        );
    }

    #[test]
    fn sell_specific_item_and_trade_response_return_typed_packets() {
        let mut parser = parser();
        parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetRequests,
                &market_order_params_with_type(14977174637, "request"),
                Some(0),
            )
            .unwrap();

        let mut params = BTreeMap::new();
        params.insert(1, json!(14977174637_i64));
        params.insert(4, json!(1));

        let extracted = parser
            .extract_operation(
                "request",
                OperationCode::AuctionSellSpecificItem,
                &params,
                None,
            )
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionSellSpecificItemRequest(_)
        ));

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionSellSpecificItem,
                &BTreeMap::new(),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionTradeResponse(response) = extracted else {
            panic!("expected auction trade response");
        };

        assert!(response.success);
        assert_eq!(
            response.confirmed_trade.as_ref().map(|trade| trade.id),
            Some(14977174637)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .map(|trade| trade.operation.clone()),
            Some(OperationType::Sell)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .map(|trade| trade.trade_type.clone()),
            Some(TradeType::Instant)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .and_then(|trade| trade.silver_amount),
            Some(5)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .and_then(|trade| trade.order.as_ref())
                .map(|order| order.id),
            Some(14977174637)
        );
    }

    #[test]
    fn trade_request_without_order_id_does_not_create_unconfirmed_trade() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(1, json!(1));

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionBuyOffer, &params, None)
            .unwrap();

        let ExtractedPacket::AuctionBuyOfferRequest(request) = extracted else {
            panic!("expected auction buy offer request");
        };

        assert_eq!(request.amount, Some(1));
        assert_eq!(request.order_id, None);
        assert_eq!(parser.unconfirmed_trade, None);

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionBuyOffer,
                &BTreeMap::new(),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionTradeResponse(response) = extracted else {
            panic!("expected auction trade response");
        };

        assert!(response.success);
        assert_eq!(response.confirmed_trade, None);
    }

    #[test]
    fn marketplace_notification_returns_typed_packet() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!({"message": "sold"}));

        let extracted = parser
            .extract_event(EventCode::MarketPlaceNotification, &params)
            .unwrap();

        let ExtractedPacket::MarketPlaceNotification(notification) = extracted else {
            panic!("expected marketplace notification");
        };

        assert_eq!(notification.notification, json!({"message": "sold"}));
    }

    #[test]
    fn joined_chat_channel_returns_typed_packet() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!(3));
        params.insert(1, json!("9001"));

        let extracted = parser
            .extract_event(EventCode::JoinedChatChannel, &params)
            .unwrap();

        let ExtractedPacket::JoinedChatChannel(response) = extracted else {
            panic!("expected joined chat channel");
        };

        assert_eq!(response.chat_index, 3);
        assert_eq!(response.channel_id, 9001);
    }

    #[test]
    fn left_chat_channel_returns_typed_packet() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!("1856"));

        let extracted = parser
            .extract_event(EventCode::LeftChatChannel, &params)
            .unwrap();

        let ExtractedPacket::LeftChatChannel(response) = extracted else {
            panic!("expected left chat channel");
        };

        assert_eq!(response.channel_id, 1856);
    }

    #[test]
    fn joined_chat_channel_state_classifies_chat_messages() {
        let mut parser = parser();
        let mut join_params = BTreeMap::new();
        join_params.insert(0, json!(29));
        join_params.insert(1, json!(9001));

        parser
            .extract_event(EventCode::JoinedChatChannel, &join_params)
            .unwrap();

        let mut message_params = BTreeMap::new();
        message_params.insert(0, json!(9001));
        message_params.insert(1, json!("Player"));
        message_params.insert(2, json!("For the faction"));

        let extracted = parser
            .extract_event(EventCode::ChatMessage, &message_params)
            .unwrap();

        let ExtractedPacket::ChatMessage(message) = extracted else {
            panic!("expected chat message");
        };

        assert_eq!(message.channel_id, 9001);
        assert_eq!(message.channel_type, ChatChannel::Faction);
    }

    #[test]
    fn left_chat_channel_state_removes_chat_message_classification() {
        let mut parser = parser();
        let mut join_params = BTreeMap::new();
        join_params.insert(0, json!(29));
        join_params.insert(1, json!(9001));

        parser
            .extract_event(EventCode::JoinedChatChannel, &join_params)
            .unwrap();

        let mut left_params = BTreeMap::new();
        left_params.insert(0, json!(9001));

        parser
            .extract_event(EventCode::LeftChatChannel, &left_params)
            .unwrap();

        let mut message_params = BTreeMap::new();
        message_params.insert(0, json!(9001));
        message_params.insert(1, json!("Player"));
        message_params.insert(2, json!("No tracked channel"));

        let extracted = parser
            .extract_event(EventCode::ChatMessage, &message_params)
            .unwrap();

        let ExtractedPacket::ChatMessage(message) = extracted else {
            panic!("expected chat message");
        };

        assert_eq!(message.channel_id, 9001);
        assert_eq!(message.channel_type, ChatChannel::Say);
    }

    #[test]
    fn trade_silver_amount_accounts_for_scaled_distance_fee() {
        let mut parser = parser();
        parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &market_order_params_with_price(14978117778, "offer", 50_000, 20_000),
                Some(0),
            )
            .unwrap();

        let mut params = BTreeMap::new();
        params.insert(1, json!(3));
        params.insert(2, json!(14978117778_i64));

        parser
            .extract_operation("request", OperationCode::AuctionBuyOffer, &params, None)
            .unwrap();

        assert_eq!(
            parser
                .unconfirmed_trade
                .as_ref()
                .and_then(|trade| trade.silver_amount),
            Some(13)
        );
    }

    #[test]
    fn get_mail_infos_then_read_mail_returns_correlated_albion_mail() {
        let mut parser = parser();
        parser.player_state_mut().set_player_name("PlayerOne");

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::GetMailInfos,
                &mail_info_params(42),
                Some(0),
            )
            .unwrap();
        assert!(matches!(extracted, ExtractedPacket::GetMailInfos(_)));

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::ReadMail,
                &read_mail_params(42, "2|T4_HIDE|100000|50000"),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AlbionMail(mail) = extracted else {
            panic!("expected correlated albion mail");
        };

        assert_eq!(mail.id, 42);
        assert_eq!(
            mail.location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(mail.player_name, "PlayerOne");
        assert_eq!(
            mail.info_type,
            MailInfoType::MarketPlaceSellOrderFinishedSummary
        );
        assert_eq!(mail.auction_type, AuctionType::Offer);
        assert_eq!(mail.received, 1_717_171_717);
        assert_eq!(mail.item_id, "T4_HIDE");
        assert_eq!(mail.item_name.as_deref(), Some("Adept's Hide"));
        assert_eq!(parser.albion_mails().get(&42), Some(&mail));
    }

    #[test]
    fn read_mail_before_get_mail_infos_is_cached_for_later_correlation() {
        let mut parser = parser();

        let extracted = parser.extract_operation(
            "response",
            OperationCode::ReadMail,
            &read_mail_params(42, "mail body"),
            Some(0),
        );
        assert!(extracted.is_none());
        assert!(parser.albion_mails().is_empty());

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::GetMailInfos,
                &mail_info_params(42),
                Some(0),
            )
            .unwrap();
        assert!(matches!(extracted, ExtractedPacket::GetMailInfos(_)));

        let mail = parser.albion_mails().get(&42).unwrap();
        assert_eq!(mail.id, 42);
        assert_eq!(
            mail.location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(mail.item_name, None);
    }

    #[test]
    fn mail_info_location_ids_are_normalized_before_correlation() {
        let mut parser = parser();
        let mut mail_info = mail_info_params(42);
        mail_info.insert(7, json!(["location@2000"]));

        parser
            .extract_operation("response", OperationCode::GetMailInfos, &mail_info, Some(0))
            .unwrap();

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::ReadMail,
                &read_mail_params(42, "mail body"),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AlbionMail(mail) = extracted else {
            panic!("expected correlated albion mail");
        };

        assert_eq!(
            mail.location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
    }

    #[test]
    fn black_market_mail_location_maps_to_caerleon_index() {
        let mut parser = parser();
        let mut mail_info = mail_info_params(42);
        mail_info.insert(7, json!(["@BLACK_MARKET"]));

        parser
            .extract_operation("response", OperationCode::GetMailInfos, &mail_info, Some(0))
            .unwrap();

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::ReadMail,
                &read_mail_params(42, "mail body"),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AlbionMail(mail) = extracted else {
            panic!("expected correlated albion mail");
        };

        assert_eq!(
            mail.location,
            AlbionLocation::with_names("3003", "Caerleon", "Caerleon")
        );
    }

    #[test]
    fn incomplete_get_mail_infos_rows_are_skipped_without_panic() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(3, json!([42, 43]));
        params.insert(7, json!(["2000"]));
        params.insert(11, json!(["MARKETPLACE_SELLORDER_FINISHED_SUMMARY"]));
        params.insert(12, json!([]));

        let extracted = parser
            .extract_operation("response", OperationCode::GetMailInfos, &params, Some(0))
            .unwrap();

        let ExtractedPacket::GetMailInfos(response) = extracted else {
            panic!("expected get mail infos");
        };

        assert_eq!(response.mail_ids, vec![42, 43]);
        assert!(parser.albion_mails().is_empty());
    }

    #[test]
    fn extracted_json_is_explicit_escape_hatch() {
        let packet = DecodedPacket::Operation(DecodedOperation {
            file: "test".to_string(),
            packet_number: 1,
            direction: "client_to_server".to_string(),
            source: "client:1".to_string(),
            destination: "server:5056".to_string(),
            message_type: "operation_request".to_string(),
            code: OperationCode::AuctionBuyOffer,
            name: OperationCode::AuctionBuyOffer.name().to_string(),
            return_code: None,
            debug_message: String::new(),
            parameters: BTreeMap::new(),
            extracted: Some(ExtractedPacket::AuctionBuyOfferRequest(AuctionBuyOffer {
                amount: Some(1),
                cached_order: None,
                order_id: Some(14978117778),
            })),
        });

        assert_eq!(
            packet.extracted_json(),
            Some(json!({
                "amount": 1,
                "cached_order": null,
                "order_id": 14978117778_i64
            }))
        );
    }

    #[test]
    fn event_extracted_json_is_explicit_escape_hatch() {
        let packet = DecodedPacket::Event(DecodedEvent {
            file: "test".to_string(),
            packet_number: 1,
            direction: "server_to_client".to_string(),
            source: "server:5056".to_string(),
            destination: "client:1".to_string(),
            message_type: "event".to_string(),
            code: EventCode::MarketPlaceNotification,
            name: EventCode::MarketPlaceNotification.name().to_string(),
            return_code: None,
            debug_message: String::new(),
            parameters: BTreeMap::new(),
            extracted: Some(ExtractedPacket::MarketPlaceNotification(
                MarketPlaceNotification {
                    notification: json!({"message": "sold"}),
                },
            )),
        });

        assert_eq!(
            packet.extracted_json(),
            Some(json!({
                "notification": {
                    "message": "sold"
                }
            }))
        );
    }

    #[test]
    fn unknown_extracted_json_is_none() {
        let packet = DecodedPacket::Unknown(DecodedUnknown {
            file: "test".to_string(),
            packet_number: 1,
            direction: "server_to_client".to_string(),
            source: "server:5056".to_string(),
            destination: "client:1".to_string(),
            message_type: "event".to_string(),
            kind: "event".to_string(),
            code_parameter: 252,
            raw_code: Some(32_767),
            reason: "unknown_event_code".to_string(),
            return_code: None,
            debug_message: String::new(),
            parameters: BTreeMap::new(),
        });

        assert_eq!(packet.extracted_json(), None);
    }

    fn mail_info_params(mail_id: i64) -> BTreeMap<u8, Value> {
        let mut params = BTreeMap::new();
        params.insert(3, json!([mail_id]));
        params.insert(7, json!(["2000"]));
        params.insert(11, json!(["MARKETPLACE_SELLORDER_FINISHED_SUMMARY"]));
        params.insert(12, json!([1_717_171_717_i64]));
        params
    }

    fn read_mail_params(mail_id: i64, mail_string: &str) -> BTreeMap<u8, Value> {
        let mut params = BTreeMap::new();
        params.insert(0, json!(mail_id));
        params.insert(1, json!(mail_string));
        params
    }

    fn market_order_params(order_id: i64) -> BTreeMap<u8, Value> {
        market_order_params_with_type(order_id, "offer")
    }

    fn market_order_params_with_type(order_id: i64, auction_type: &str) -> BTreeMap<u8, Value> {
        market_order_params_with_price(order_id, auction_type, 50_000, 0)
    }

    fn market_order_params_with_price(
        order_id: i64,
        auction_type: &str,
        unit_price_silver: i64,
        distance_fee: i64,
    ) -> BTreeMap<u8, Value> {
        let mut params = BTreeMap::new();
        params.insert(
            0,
            json!([
                {
                    "Amount": 2,
                    "AuctionType": auction_type,
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": distance_fee,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-22T03:34:16.096699",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": order_id,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": null,
                    "QualityLevel": 1,
                    "ReferenceId": "7ae09894-4883-479b-932e-ff7914c82855",
                    "SellerCharacterId": "07b8fbc0-c512-4054-bc53-12312af94df3",
                    "SellerName": "CoelhoMalvado",
                    "Tier": 1,
                    "TotalPriceSilver": unit_price_silver * 2,
                    "UnitPriceSilver": unit_price_silver
                }
            ]),
        );
        params
    }
}
