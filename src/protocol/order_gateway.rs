use crate::{
    protocol::{
        common::{Fill, Timestamp},
        ws,
    },
    types::{Order, OrderId, OrderRejectReason, OrderState, Side},
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "t")]
pub enum OrderGatewayRequest {
    #[serde(rename = "p")]
    PlaceOrder(PlaceOrderRequest),
    #[serde(rename = "x")]
    CancelOrder(CancelOrderRequest),
    #[serde(rename = "o")]
    GetOpenOrders(GetOpenOrdersRequest),
}

#[derive(Debug)]
#[repr(u8)]
pub enum OrderGatewayRequestType {
    PlaceOrder,
    CancelOrder,
    GetOpenOrders,
}

/// Expected response types from the order gateway.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum OrderGatewayResponse {
    LoginResponse(LoginResponse),
    PlaceOrderResponse(PlaceOrderResponse),
    CancelOrderResponse(CancelOrderResponse),
    GetOpenOrdersResponse(GetOpenOrdersResponse),
}

/// Expected message types from the order gateway.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum OrderGatewayMessage {
    Event(OrderGatewayEvent),
    Response(ws::Response<OrderGatewayResponse>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PlaceOrderRequest {
    /// Order symbol; e.g. GBPUSD-PERP, EURUSD-PERP
    #[serde(rename = "s")]
    pub symbol: String,
    /// Order side; buying ("B") or selling ("S")
    #[serde(rename = "d")]
    pub side: Side,
    /// Order quantity in contracts; e.g. 100, 1000
    #[serde(rename = "q")]
    pub quantity: i32,
    /// Order price in USD as decimal string; e.g. "1.2345"
    #[serde(rename = "p")]
    pub price: Decimal,
    /// Order time in force; e.g. "GTC", "IOC", "DAY"
    #[serde(rename = "tif")]
    pub time_in_force: String,
    /// Whether the order is post-only (maker-or-cancel)
    #[serde(rename = "po")]
    pub post_only: bool,
    /// Optional order tag; maximum 10 alphanumeric characters
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Optional client order ID; 64 bit integer
    #[serde(rename = "cid", skip_serializing_if = "Option::is_none")]
    pub clord_id: Option<u64>,
}

impl PlaceOrderRequest {
    /// Convert this place order request into a pending order
    pub fn into_pending_order(self, order_id: OrderId, user_id: String) -> crate::types::Order {
        crate::types::Order {
            order_id,
            user_id,
            symbol: self.symbol,
            side: self.side,
            quantity: self.quantity,
            price: self.price,
            time_in_force: self.time_in_force,
            tag: self.tag,
            clord_id: self.clord_id,
            timestamp: Utc::now(),
            order_state: OrderState::Pending,
            filled_quantity: 0,
            remaining_quantity: self.quantity,
            completion_time: None,
            reject_reason: None,
            reject_message: None,
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
            clord_id: value.clord_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PlaceOrderResponse {
    /// Order ID of the placed order; e.g. "ORD-1234567890"
    #[serde(rename = "oid")]
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct InitialMarginRequirementResponse {
    /// Initial margin requirement for the order; e.g. "1000.00"
    #[serde(rename = "im")]
    pub initial_margin_requirement: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CancelOrderRequest {
    /// Order ID to cancel; e.g. "ORD-1234567890"
    #[serde(rename = "oid")]
    pub order_id: OrderId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CancelOrderResponse {
    /// Whether the cancel request has been accepted; e.g. true, false
    #[serde(rename = "cxl_rx")]
    pub cancel_request_accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetOpenOrdersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetOpenOrdersResponse {
    pub orders: Vec<OrderDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
    pub fn order_id(&self) -> Option<&OrderId> {
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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CancelRejected {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "oid")]
    pub order_id: OrderId,
    #[serde(rename = "r")]
    pub reject_reason: String,
    #[serde(rename = "txt")]
    pub reject_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderAcked {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderReplacedOrAmended {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderRejected {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    #[serde(rename = "r")]
    pub reject_reason: Option<OrderRejectReason>,
    #[serde(rename = "txt")]
    pub reject_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderExpired {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderDoneForDay {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderPartiallyFilled {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    // TODO: retag as "x"
    #[serde(rename = "xs")]
    pub fill: FillDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderFilled {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    #[serde(rename = "o")]
    pub order: OrderDetails,
    // TODO: retag as "x"
    #[serde(rename = "xs")]
    pub fill: FillDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct OrderDetails {
    #[serde(rename = "oid")]
    pub order_id: OrderId,
    #[serde(rename = "u")]
    pub user_id: String,
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
    #[serde(rename = "cid", skip_serializing_if = "Option::is_none")]
    pub clord_id: Option<u64>,
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(rename = "r", skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<OrderRejectReason>,
    #[serde(rename = "txt", skip_serializing_if = "Option::is_none")]
    pub reject_message: Option<String>,
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
            clord_id: value.clord_id,
            timestamp: value
                .timestamp
                .as_datetime()
                .ok_or_else(|| anyhow!("invalid timestamp"))?,
            completion_time: None,
            reject_reason: value.reject_reason,
            reject_message: value.reject_message,
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
            clord_id: value.clord_id,
            reject_reason: value.reject_reason,
            reject_message: value.reject_message,
            timestamp: Timestamp {
                ts: value.timestamp.timestamp() as i32,
                tn: value.timestamp.timestamp_subsec_nanos(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetOrdersRequest {
    pub symbol: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetOrdersResponse {
    pub orders: Vec<OrderDetails>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum OrderIdentifier {
    OrderId(OrderId),
    ClientOrderId(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
#[serde(transparent)]
pub struct GetOrderStatusRequest {
    pub order: OrderIdentifier,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct OrderStatus {
    pub symbol: String,
    pub order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clord_id: Option<u64>,
    pub state: OrderState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetOrderStatusResponse {
    pub status: OrderStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetOrderFillsRequest {
    pub order_id: OrderId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct GetOrderFillsResponse {
    pub fills: Vec<Fill>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_json_snapshot;

    #[test]
    fn order_identifier_serialization() {
        assert_json_snapshot!(
            OrderIdentifier::OrderId(OrderId::new_unchecked("ORD-12345")), @r#"
        {
          "order_id": "ORD-12345"
        }
        "#
        );
        assert_json_snapshot!(OrderIdentifier::ClientOrderId(42), @r#"
        {
          "client_order_id": 42
        }
        "#);
    }

    #[test]
    fn order_status_request_serialization() {
        let request_with_order_id = GetOrderStatusRequest {
            order: OrderIdentifier::OrderId(OrderId::new_unchecked("O-12345")),
        };
        let request_with_client_id = GetOrderStatusRequest {
            order: OrderIdentifier::ClientOrderId(42),
        };

        assert_json_snapshot!(request_with_order_id, @r#"
        {
          "order_id": "O-12345"
        }
        "#);
        assert_json_snapshot!(request_with_client_id, @r#"
        {
          "client_order_id": 42
        }
        "#);
    }

    #[test]
    fn order_status_request_deserialization() {
        let json_order_id = r#"{"order_id": "O-12345"}"#;
        let json_client_id = r#"{"client_order_id": 42}"#;

        let parsed: GetOrderStatusRequest = serde_json::from_str(json_order_id).unwrap();
        assert_json_snapshot!(parsed, @r#"
        {
          "order_id": "O-12345"
        }
        "#);

        let parsed: GetOrderStatusRequest = serde_json::from_str(json_client_id).unwrap();
        assert_json_snapshot!(parsed, @r#"
        {
          "client_order_id": 42
        }
        "#);
    }
}
