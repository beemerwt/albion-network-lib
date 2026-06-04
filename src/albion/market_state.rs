use crate::albion::payloads::{
    AuctionBuyOffer, AuctionSellSpecificItem, AuctionTrade, AuctionTradeResponse,
};
use crate::albion::{CachedOrder, OperationType, TradeType};
use crate::packet::RawParameters;
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

    pub fn cache_order(&mut self, order: CachedOrder) {
        self.orders_by_id.insert(order.id, order);
    }

    pub fn cache_orders_from_slice(&mut self, orders: &[CachedOrder]) {
        for order in orders {
            self.cache_order(order.clone());
        }
    }

    pub fn get_order(&self, order_id: i64) -> Option<&CachedOrder> {
        self.orders_by_id.get(&order_id)
    }

    pub fn get_order_cloned(&self, order_id: i64) -> Option<CachedOrder> {
        self.orders_by_id.get(&order_id).cloned()
    }

    pub fn begin_buy_order_request(&mut self, parameters: &RawParameters) -> AuctionBuyOffer {
        let mut buy_item = AuctionBuyOffer::from_params(parameters);
        let cached_order = self.begin_instant_trade(buy_item.order_id, buy_item.amount);
        buy_item.cached_order = cached_order;
        buy_item
    }

    pub fn begin_sell_specific_item_request(
        &mut self,
        parameters: &RawParameters,
    ) -> AuctionSellSpecificItem {
        let mut sell_item = AuctionSellSpecificItem::from_params(parameters);
        let cached_order = self.begin_instant_trade(sell_item.order_id, sell_item.amount);
        sell_item.cached_order = cached_order;
        sell_item
    }

    fn begin_instant_trade(
        &mut self,
        order_id: Option<i64>,
        amount: Option<i64>,
    ) -> Option<CachedOrder> {
        let cached_order = order_id.and_then(|id| self.get_order_cloned(id));

        self.unconfirmed_trade = order_id.map(|id| AuctionTrade {
            id,
            amount,
            silver_amount: silver_amount(amount, cached_order.as_ref()),
            operation: operation_from_cached_order(cached_order.as_ref(), &TradeType::Instant),
            timestamp: Utc::now().timestamp_millis(),
            trade_type: TradeType::Instant,
            order: cached_order.clone(),
        });

        cached_order
    }

    pub fn finish_instant_trade_response(
        &mut self,
        return_code: Option<i16>,
    ) -> AuctionTradeResponse {
        let success = return_code == Some(0);

        AuctionTradeResponse {
            confirmed_trade: success.then(|| self.unconfirmed_trade.take()).flatten(),
            success,
        }
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
