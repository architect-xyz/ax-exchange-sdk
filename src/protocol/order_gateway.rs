use crate::protocol::ws::Timestamp;
use anyhow::anyhow;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "t")]
pub enum OrderGatewayRequest {
    #[serde(rename = "p")]
    PlaceOrder(PlaceOrderRequest),
    #[serde(rename = "x")]
    CancelOrder(CancelOrderRequest),
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "li")]
    pub logged_in: String,
    #[serde(rename = "o")]
    pub open_orders: Vec<OrderDetails>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaceOrderRequest {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "d")]
    pub side: String,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "tif")]
    pub time_in_force: String,
    #[serde(rename = "po")]
    pub post_only: bool,
}

impl From<crate::types::PlaceOrder> for PlaceOrderRequest {
    fn from(value: crate::types::PlaceOrder) -> Self {
        Self {
            symbol: value.symbol,
            side: value.side,
            quantity: value.quantity,
            price: value.price,
            time_in_force: value.time_in_force,
            post_only: value.post_only,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlaceOrderResponse {
    #[serde(rename = "oid")]
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderRequest {
    #[serde(rename = "oid")]
    pub order_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelOrderResponse {
    #[serde(rename = "cxl_rx")]
    pub cancel_request_accepted: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "t")]
pub enum OrderGatewayEvent {
    #[serde(rename = "h")]
    Heartbeat(Timestamp),
    #[serde(rename = "e")]
    CancelRejected(CancelRejected),
    #[serde(rename = "n")]
    OrderAcked(OrderAcked),
    #[serde(rename = "c")]
    OrderCanceled(OrderCanceled),
    #[serde(rename = "r")]
    OrderReplacedOrAmended(OrderReplacedOrAmended),
    #[serde(rename = "j")]
    OrderRejected(OrderRejected),
    #[serde(rename = "x")]
    OrderExpired(OrderExpired),
    #[serde(rename = "d")]
    OrderDoneForDay(OrderDoneForDay),
    #[serde(rename = "p")]
    OrderPartiallyFilled(OrderPartiallyFilled),
    #[serde(rename = "f")]
    OrderFilled(OrderFilled),
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelRejected {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "oid")]
    pub order_id: String,
    #[serde(rename = "r")]
    pub reject_reason: String,
    #[serde(rename = "txt")]
    pub reject_message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderAcked {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderCanceled {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    #[serde(rename = "xr")]
    pub cancel_reason: String,
    #[serde(rename = "txt")]
    pub cancel_message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderReplacedOrAmended {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderRejected {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    #[serde(rename = "r")]
    pub reject_reason: String,
    #[serde(rename = "txt")]
    pub reject_message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderExpired {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderDoneForDay {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderPartiallyFilled {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    #[serde(rename = "xs")]
    pub fill: FillDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderFilled {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    #[serde(rename = "xs")]
    pub fill: FillDetails,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderDetails {
    #[serde(rename = "oid")]
    pub order_id: String,
    #[serde(rename = "u")]
    pub username: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "xq")]
    pub filled_quantity: i32,
    #[serde(rename = "rq")]
    pub remaining_quantity: i32,
    #[serde(rename = "o")]
    pub order_state: String,
    #[serde(rename = "d")]
    pub side: String,
    #[serde(rename = "tif")]
    pub time_in_force: String,
    #[serde(flatten)]
    pub timestamp: Timestamp,
}

impl TryFrom<OrderDetails> for crate::types::Order {
    type Error = anyhow::Error;

    fn try_from(value: OrderDetails) -> Result<Self, Self::Error> {
        Ok(crate::types::Order {
            order_id: value.order_id,
            username: value.username,
            symbol: value.symbol,
            price: value.price,
            quantity: value.quantity,
            filled_quantity: value.filled_quantity,
            remaining_quantity: value.remaining_quantity,
            order_state: value.order_state,
            side: value.side,
            time_in_force: value.time_in_force,
            timestamp: value
                .timestamp
                .as_datetime()
                .ok_or_else(|| anyhow!("invalid timestamp"))?,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FillDetails {
    #[serde(rename = "tid")]
    pub trade_id: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "d")]
    pub side: String,
    #[serde(rename = "agg")]
    pub is_taker: bool,
}
