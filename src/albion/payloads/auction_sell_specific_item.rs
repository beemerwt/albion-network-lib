use crate::{albion::CachedOrder, packet::RawParameters};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionSellSpecificItem {
    pub amount: Option<i64>,
    pub order_id: Option<i64>,

    pub cached_order: Option<CachedOrder>,
    pub tax: Option<i64>, // not part of the packet, calculated as 4% or 8% of the price depending on if the player has premium or not
}

impl AuctionSellSpecificItem {
    pub fn from_params(params: &RawParameters) -> Self {
        let order_id = params.get(0).and_then(|v| v.as_i64());
        let amount = params.get(2).and_then(|v| v.as_i64());
        Self {
            amount,
            cached_order: None,
            order_id,
            tax: None,
        }
    }
}
