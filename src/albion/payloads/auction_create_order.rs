use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum AuctionCreateOrder {
    BuyOrder(AuctionBuyOrder),
    SellOrder(AuctionSellOrder),
}

// OperationCode 80
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuctionBuyOrder {
    pub market_id: i32, // arg 0
    // arg 2 ( also some kind of amount? )
    pub amount: i32, // arg 3 (could be confused with amount leftover for the order, conflating this to the above)
    pub item_id: i32, // arg 4 720 meant T1_HIDE
    pub price: i32,  // arg 5
}

// OperationCode 79
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuctionSellOrder {
    pub market_id: i32, // arg 0
    pub unk_1: i32,     // arg 1
    pub expiry: i32, // arg 2 (assumed to be the amount of time relative to the time the order was created in timestamp form)
    pub price: i32,  // arg 3
    pub item_id: i32, // arg 4 720 meant T1_HIDE
}
