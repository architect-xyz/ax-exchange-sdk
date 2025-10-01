use crate::{
    protocol::{
        common::{DateRangeParams, PaginationParams},
        ws::{self, Timestamp},
    },
    types::{Order, OrderState, Side},
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum OrderGatewayRequest {
    #[serde(rename = "p")]
    PlaceOrder(PlaceOrderRequest),
    #[serde(rename = "x")]
    CancelOrder(CancelOrderRequest),
    #[serde(rename = "o")]
    GetOpenOrders(GetOpenOrdersRequest),
}

#[repr(u8)]
pub enum OrderGatewayRequestType {
    PlaceOrder,
    CancelOrder,
    GetOpenOrders,
}

/// Expected response types from the order gateway.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum OrderGatewayResponse {
    LoginResponse(LoginResponse),
    PlaceOrderResponse(PlaceOrderResponse),
    CancelOrderResponse(CancelOrderResponse),
    GetOpenOrdersResponse(GetOpenOrdersResponse),
}

/// Expected message types from the order gateway.
pub enum OrderGatewayMessage {
    Event(OrderGatewayEvent),
    Response(ws::Response<OrderGatewayResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "li")]
    pub logged_in: String,
    #[serde(rename = "o")]
    pub open_orders: Option<Vec<OrderDetails>>,
}

impl LoginResponse {
    pub fn into_open_orders(self) -> Result<Vec<Order>> {
        let mut oos = vec![];
        if let Some(orders) = self.open_orders {
            for order in orders {
                oos.push(order.try_into()?);
            }
        }
        Ok(oos)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderRequest {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "d")]
    pub side: Side,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "tif")]
    pub time_in_force: String,
    #[serde(rename = "po")]
    pub post_only: bool,
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

impl PlaceOrderRequest {
    /// Convert this place order request into a pending order
    pub fn into_pending_order(self, order_id: String, user_id: Uuid) -> crate::types::Order {
        crate::types::Order {
            order_id,
            user_id,
            symbol: self.symbol,
            side: self.side,
            quantity: self.quantity,
            price: self.price,
            time_in_force: self.time_in_force,
            tag: self.tag,
            timestamp: Utc::now(),
            order_state: OrderState::Pending,
            filled_quantity: 0,
            remaining_quantity: self.quantity,
            completion_time: None,
        }
    }
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
            tag: value.tag,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderResponse {
    #[serde(rename = "oid")]
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    #[serde(rename = "oid")]
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderResponse {
    #[serde(rename = "cxl_rx")]
    pub cancel_request_accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersRequest {}

pub type GetOpenOrdersResponse = Vec<OrderDetails>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum OrderGatewayEvent {
    // TODO: deprecate in favor of WS native ping
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

impl OrderGatewayEvent {
    /// Order ID that the event pertains to, if applicable and singular.
    pub fn order_id(&self) -> Option<&str> {
        match self {
            OrderGatewayEvent::Heartbeat(..) => None,
            OrderGatewayEvent::CancelRejected(rej) => Some(&rej.order_id),
            OrderGatewayEvent::OrderAcked(ack) => Some(&ack.order.order_id),
            OrderGatewayEvent::OrderCanceled(ccl) => Some(&ccl.order.order_id),
            OrderGatewayEvent::OrderReplacedOrAmended(roa) => Some(&roa.order.order_id),
            OrderGatewayEvent::OrderRejected(rej) => Some(&rej.order.order_id),
            OrderGatewayEvent::OrderExpired(exp) => Some(&exp.order.order_id),
            OrderGatewayEvent::OrderDoneForDay(done) => Some(&done.order.order_id),
            OrderGatewayEvent::OrderPartiallyFilled(fill) => Some(&fill.order.order_id),
            OrderGatewayEvent::OrderFilled(fill) => Some(&fill.order.order_id),
        }
    }

    /// Symbol that the event pertains to, if applicable and singular.
    pub fn symbol(&self) -> Option<&str> {
        match self {
            OrderGatewayEvent::Heartbeat(..) => None,
            OrderGatewayEvent::CancelRejected(..) => None,
            OrderGatewayEvent::OrderAcked(ack) => Some(&ack.order.symbol),
            OrderGatewayEvent::OrderCanceled(ccl) => Some(&ccl.order.symbol),
            OrderGatewayEvent::OrderReplacedOrAmended(roa) => Some(&roa.order.symbol),
            OrderGatewayEvent::OrderRejected(rej) => Some(&rej.order.symbol),
            OrderGatewayEvent::OrderExpired(exp) => Some(&exp.order.symbol),
            OrderGatewayEvent::OrderDoneForDay(done) => Some(&done.order.symbol),
            OrderGatewayEvent::OrderPartiallyFilled(fill) => Some(&fill.order.symbol),
            OrderGatewayEvent::OrderFilled(fill) => Some(&fill.order.symbol),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAcked {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReplacedOrAmended {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExpired {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDoneForDay {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetails {
    #[serde(rename = "oid")]
    pub order_id: String,
    #[serde(rename = "u")]
    pub user_id: Uuid,
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
    pub order_state: OrderState,
    #[serde(rename = "d")]
    pub side: Side,
    #[serde(rename = "tif")]
    pub time_in_force: String,
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(flatten)]
    pub timestamp: Timestamp,
}

impl TryFrom<OrderDetails> for crate::types::Order {
    type Error = anyhow::Error;

    fn try_from(value: OrderDetails) -> Result<Self, Self::Error> {
        Ok(crate::types::Order {
            order_id: value.order_id,
            user_id: value.user_id,
            symbol: value.symbol,
            price: value.price,
            quantity: value.quantity,
            filled_quantity: value.filled_quantity,
            remaining_quantity: value.remaining_quantity,
            order_state: value.order_state,
            side: value.side,
            time_in_force: value.time_in_force,
            tag: value.tag,
            timestamp: value
                .timestamp
                .as_datetime()
                .ok_or_else(|| anyhow!("invalid timestamp"))?,
            completion_time: None,
        })
    }
}

impl From<crate::types::Order> for OrderDetails {
    fn from(value: crate::types::Order) -> Self {
        Self {
            order_id: value.order_id,
            user_id: value.user_id,
            symbol: value.symbol,
            price: value.price,
            quantity: value.quantity,
            filled_quantity: value.filled_quantity,
            remaining_quantity: value.remaining_quantity,
            order_state: value.order_state,
            side: value.side,
            time_in_force: value.time_in_force,
            tag: value.tag,
            timestamp: Timestamp {
                ts: value.timestamp.timestamp() as i32,
                tn: value.timestamp.timestamp_subsec_nanos() as u32,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Order history query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderHistoryFilters {
    pub symbol: Option<String>,
    pub side: Option<Side>,
    pub order_state: Option<OrderState>,
    pub status: Option<String>,
    pub order_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub pagination: Option<PaginationParams>,
    pub date_range: Option<DateRangeParams>,
}
