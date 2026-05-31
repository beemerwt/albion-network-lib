use crate::albion::CachedOrder;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionBuyOffer {
    pub amount: Option<i64>,
    pub cached_order: Option<CachedOrder>,
    pub order_id: Option<i64>,
}
