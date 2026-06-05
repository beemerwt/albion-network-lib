use serde::Serialize;

use crate::{packet::RawParameters, util::value_i64};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CreateOrderKind {
    Buy,
    Sell,
}

// AuctionCreateRequest is called when creating a Buy Order
// OperationCode 80
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuctionCreateOrder {
    pub kind: CreateOrderKind,

    pub market_id: i64,
    pub item_id: i64,

    pub amount: i64,

    pub silver_total: i64,
    pub silver_per_unit: i64,

    pub expiry: Option<i64>,

    // not part of the packet, calculated as 2.5% of the price
    pub setup_fee: i64,
}

impl AuctionCreateOrder {
    pub fn buy_order_from_params(parameters: &RawParameters) -> Self {
        let silver_total = value_i64(parameters, 5).unwrap_or_default();

        // Amount could be 2 or 3, I think 3 is the "total amount" and  2 is the "amount left"
        let amount = value_i64(parameters, 3).unwrap_or_default();
        let silver_per_unit = if amount > 0 { silver_total / amount } else { 0 };
        let setup_fee = ((silver_total as f64) * 0.025).floor() as i64;

        Self {
            kind: CreateOrderKind::Buy,
            market_id: value_i64(parameters, 0).unwrap_or_default(),
            amount: value_i64(parameters, 3).unwrap_or_default(),
            item_id: value_i64(parameters, 4).unwrap_or_default(),
            silver_total,
            silver_per_unit,
            setup_fee,
            expiry: None,
        }
    }

    pub fn sell_order_from_params(parameters: &RawParameters) -> Self {
        let silver_total = value_i64(parameters, 3).unwrap_or_default();
        let setup_fee = ((silver_total as f64) * 0.025).floor() as i64;

        // assumed to be the amount of time relative to the time the order was created in timestamp form
        let expiry = value_i64(parameters, 2);

        Self {
            kind: CreateOrderKind::Sell,
            amount: 1, // currently assumed to be 1 until packet field for stack size is identified.
            silver_per_unit: silver_total,
            market_id: value_i64(parameters, 0).unwrap_or_default(),
            item_id: value_i64(parameters, 4).unwrap_or_default(),
            silver_total,
            setup_fee,
            expiry,
        }
    }
}
