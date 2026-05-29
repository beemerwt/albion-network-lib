use crate::{
    models::AlbionMail,
    requests::{AuctionBuyOffer, AuctionGetOffers, AuctionGetRequests, AuctionSellSpecificItem},
    responses::{
        AuctionGetOffersResult, AuctionGetRequestsResult, AuctionTradeResponse, ChatMessage,
        GetMailInfos, JoinResponse, JoinedChatChannel, LeftChatChannel,
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
    ChatMessage(ChatMessage),
    JoinedChatChannel(JoinedChatChannel),
    LeftChatChannel(LeftChatChannel),
    GetMailInfos(GetMailInfos),
    AlbionMail(AlbionMail),
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
