use crate::albion::{CachedOrder, OperationType, TradeType};
use serde::Serialize;

// AuctionTrade is when you purchase or sell an item without an order

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionTrade {
    pub order: CachedOrder,
    pub amount: i64,
    pub silver_amount: i64,
    pub operation: OperationType,
    pub trade_type: TradeType,
    pub timestamp: i64,
    pub id: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionTradeResponse {
    pub confirmed_trade: Option<AuctionTrade>,
    pub success: bool,
}
