use crate::{albion::CachedOrder, packet::RawParameters};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionSellSpecificItem {
    pub amount: Option<i64>,
    pub cached_order: Option<CachedOrder>,
    pub order_id: Option<i64>,
}

impl AuctionSellSpecificItem {
    pub fn from_params(params: &RawParameters) -> Self {
        let order_id = params.get(0).and_then(|v| v.as_i64());
        let amount = params.get(2).and_then(|v| v.as_i64());
        Self {
            amount,
            cached_order: None,
            order_id,
        }
    }
}
