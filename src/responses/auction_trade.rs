use crate::models::{CachedOrder, OperationType, TradeType};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionTrade {
    pub amount: Option<i64>,
    pub silver_amount: Option<i64>,
    pub operation: OperationType,
    pub trade_type: TradeType,
    pub order: Option<CachedOrder>,
    pub timestamp: i64,
    pub id: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuctionTradeResponse {
    pub confirmed_trade: Option<AuctionTrade>,
    pub success: bool,
}
