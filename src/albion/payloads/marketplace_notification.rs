use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarketPlaceNotification {
    pub notification: Value,
}
