use crate::{albion::CachedOrder, packet::RawParameters, util::value_i64};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionBuyOffer {
    pub amount: Option<i64>,
    pub cached_order: Option<CachedOrder>,
    pub order_id: Option<i64>,
}

impl AuctionBuyOffer {
    pub fn from_params(parameters: &RawParameters) -> Self {
        let amount = value_i64(parameters, 4);
        let order_id = value_i64(parameters, 1);
        Self {
            amount,
            cached_order: None,
            order_id,
        }
    }
}
