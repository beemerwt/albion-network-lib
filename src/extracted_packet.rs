use crate::{
    requests::{
        auction_buy_offer::AuctionBuyOffer, auction_get_offers::AuctionGetOffers,
        auction_get_requests::AuctionGetRequests,
        auction_sell_specific_item::AuctionSellSpecificItem,
    },
    responses::{
        auction_get_offers::AuctionGetOffersResult, auction_get_requests::AuctionGetRequestsResult,
        auction_trade::AuctionTradeResponse, join_response::JoinResponse,
    },
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ExtractedPacket {
    AuctionGetOffersRequest(AuctionGetOffers),
    AuctionGetRequestsRequest(AuctionGetRequests),
    AuctionGetOffersResponse(AuctionGetOffersResult),
    AuctionGetRequestsResponse(AuctionGetRequestsResult),
    AuctionBuyOfferRequest(AuctionBuyOffer),
    AuctionSellSpecificItemRequest(AuctionSellSpecificItem),
    AuctionTradeResponse(AuctionTradeResponse),
    JoinResponse(JoinResponse),
    MarketPlaceNotification(MarketPlaceNotification),
}

impl ExtractedPacket {
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }

    pub fn into_json(self) -> Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarketPlaceNotification {
    pub notification: Value,
}
