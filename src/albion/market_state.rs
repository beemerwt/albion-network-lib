use crate::albion::{
    CachedOrder,
    OperationType,
    TradeType,
};
use crate::responses::AuctionTrade;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct MarketState {
    orders_by_id: HashMap<i64, CachedOrder>,
    unconfirmed_trade: Option<AuctionTrade>,
}

impl MarketState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn order_count(&self) -> usize {
        self.orders_by_id.len()
    }

    pub fn cache_orders<I>(&mut self, orders: I)
    where
        I: IntoIterator<Item = CachedOrder>,
    {
        for order in orders {
            self.orders_by_id.insert(order.id, order);
        }
    }

    pub fn get_order(&self, order_id: i64) -> Option<&CachedOrder> {
        self.orders_by_id.get(&order_id)
    }

    pub fn get_order_cloned(&self, order_id: i64) -> Option<CachedOrder> {
        self.orders_by_id.get(&order_id).cloned()
    }

    pub fn begin_instant_trade(
        &mut self,
        order_id: Option<i64>,
        amount: Option<i64>,
        trade_type: TradeType,
    ) {
        self.unconfirmed_trade = order_id.map(|id| {
            let cached_order = self.get_order_cloned(id);

            AuctionTrade {
                id,
                amount,
                silver_amount: silver_amount(amount, cached_order.as_ref()),
                operation: operation_from_cached_order(cached_order.as_ref(), &trade_type),
                timestamp: Utc::now().timestamp_millis(),
                trade_type,
                order: cached_order,
            }
        });
    }

    pub fn take_unconfirmed_trade(&mut self) -> Option<AuctionTrade> {
        self.unconfirmed_trade.take()
    }

    pub fn clear_unconfirmed_trade(&mut self) {
        self.unconfirmed_trade = None;
    }
}

fn operation_from_cached_order(
    cached_order: Option<&CachedOrder>,
    trade_type: &TradeType,
) -> OperationType {
    cached_order
        .map(|order| OperationType::from_auction_type(&order.auction_type, trade_type))
        .unwrap_or_else(|| OperationType::Unknown("missing_cached_order".to_string()))
}

fn silver_amount(amount: Option<i64>, cached_order: Option<&CachedOrder>) -> Option<i64> {
    let amount = amount?;
    let order = cached_order?;

    Some(
        (((order.unit_price_silver * amount) - order.distance_fee) as f64 / 10_000.0).floor()
            as i64,
    )
}