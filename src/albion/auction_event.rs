use serde::Serialize;

use crate::albion::payloads::{
    AuctionCreateOrder, AuctionGetOffersResult, AuctionGetRequestsResult, AuctionTrade
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum AuctionEvent {
    CreateOrder(AuctionCreateOrder),
    InstantTrade(AuctionTrade),
    GetOffers(AuctionGetOffersResult),
    GetRequests(AuctionGetRequestsResult),
}
