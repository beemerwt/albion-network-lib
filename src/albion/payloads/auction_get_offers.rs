use crate::albion::CachedOrder;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionGetOffers {
    pub market_order_count: usize,
    pub market_orders: Vec<CachedOrder>,
}


#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionGetOffersResult {
    pub market_order_count: usize,
    pub market_orders: Vec<CachedOrder>,
}