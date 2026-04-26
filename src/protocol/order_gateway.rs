use crate::{
    protocol::{
        common::{Fill, Timestamp},
        pagination::{LimitOffsetPage, LimitOffsetPagination},
        ws,
    },
    trading::TimeInForce,
    types::{Order, OrderId, OrderRejectReason, OrderState, Side},
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Query parameters for the order gateway WebSocket endpoint (`/orders/ws`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct WsQueryParams {
    /// When true, all orders placed on this connection will be cancelled on disconnect.
    #[serde(default)]
    pub cancel_on_disconnect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "t")]
pub enum OrderGatewayRequest {
    #[serde(rename = "X")]
    CancelAllOrders(CancelAllOrdersRequest),
    #[serde(rename = "x")]
    CancelOrder(CancelOrderRequest),
    #[serde(rename = "o")]
    GetOpenOrders(GetOpenOrdersRequest),
    #[serde(rename = "p")]
    PlaceOrder(PlaceOrderRequest),
    #[serde(rename = "r")]
    ReplaceOrder(ReplaceOrderRequest),
}

/// Request types for the admin firehose websocket endpoint (/admin/ws)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(tag = "t")]
pub enum AdminFirehoseRequest {
    #[serde(rename = "s")]
    Subscribe(AdminSubscribeRequest),
}

#[derive(Debug)]
#[repr(u8)]
pub enum OrderGatewayRequestType {
    CancelAllOrders,
    CancelOrder,
    GetOpenOrders,
    PlaceOrder,
    ReplaceOrder,
}

/// Expected response types from the order gateway.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum OrderGatewayResponse {
    CancelAllOrdersResponse(CancelAllOrdersResponse),
    CancelOrderResponse(CancelOrderResponse),
    GetOpenOrdersResponse(GetOpenOrdersResponse),
    LoginResponse(LoginResponse),
    PlaceOrderResponse(PlaceOrderResponse),
    ReplaceOrderResponse(ReplaceOrderResponse),
}

/// Expected response types from the admin firehose endpoint.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum AdminFirehoseResponse {
    AdminLoginResponse(AdminLoginResponse),
    AdminSubscribeResponse(AdminSubscribeResponse),
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

/// Login response for admin firehose websocket endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct AdminLoginResponse {
    #[serde(rename = "li")]
    pub logged_in: String,
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
    pub quantity: u64,
    /// Order price in USD as decimal string; e.g. "1.2345"
    #[serde(rename = "p")]
    pub price: Decimal,
    /// Order time in force; e.g. "GTC", "IOC", "DAY"
    #[serde(rename = "tif")]
    pub time_in_force: TimeInForce,
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
            post_only: self.post_only,
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
    pub order_id: OrderId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct InitialMarginRequirementResponse {
    /// Initial margin percentage for the order symbol
    #[serde(rename = "im_pct")]
    pub initial_margin_percentage: Decimal,
    /// Initial margin requirement for the order; e.g. "1000.00"
    #[serde(rename = "im")]
    pub initial_margin_requirement: Decimal,
    /// Current signed position in the order symbol
    #[serde(rename = "pos")]
    pub signed_position: i64,
    /// Multiplier for the order symbol
    #[serde(rename = "mult")]
    pub contract_multiplier: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PreviewOrderResponse {
    /// Initial margin percentage for the instrument (e.g. 10 means 10% IM)
    #[serde(rename = "im_pct")]
    pub initial_margin_pct_required: Decimal,
    /// Additional initial margin required to place this order; zero if the
    /// order would reduce the overall margin requirement (e.g. a closing trade)
    #[serde(rename = "im")]
    pub initial_margin_required: Decimal,
    /// Current signed position in the symbol before the order fills
    #[serde(rename = "pos_before")]
    pub signed_position_before: i64,
    /// Projected signed position in the symbol after the order fills
    #[serde(rename = "pos_after")]
    pub signed_position_after: i64,
    /// Estimated liquidation price after the order fills, based on current
    /// equity and maintenance margin; None if the resulting position is flat
    #[serde(rename = "liq")]
    pub estimated_liquidation_price: Option<Decimal>,
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ReplaceOrderRequest {
    /// Order ID of the order to replace; e.g. "ORD-1234567890"
    #[serde(rename = "oid")]
    pub order_id: OrderId,
    /// New price for the replacement order (optional, inherits from original if not provided)
    #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    /// New quantity for the replacement order (optional, inherits from original if not provided)
    #[serde(rename = "q", skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u64>,
    /// New time in force for the replacement order (optional, inherits from original if not provided)
    #[serde(rename = "tif", skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    /// Whether the replacement order is post-only (optional, inherits from original if not provided)
    #[serde(rename = "po", skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ReplaceOrderResponse {
    /// Order ID of the new replacement order; e.g. "ORD-1234567890"
    #[serde(rename = "oid")]
    pub order_id: OrderId,
}

/// Request to cancel all orders for the authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct CancelAllOrdersRequest {
    /// Optional symbol filter. If provided, only orders for this symbol will be canceled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// Response for canceling all orders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CancelAllOrdersResponse {}

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

/// Admin-only request to subscribe to firehose events for all users
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AdminSubscribeRequest {
    /// Subscribe to all fills (includes partial fills and full fills)
    #[serde(rename = "f", default)]
    pub fills: bool,
    /// Subscribe to all order state changes (acks, cancels, rejects, expires, etc.)
    #[serde(rename = "o", default)]
    pub orders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AdminSubscribeResponse {
    /// Confirmation message
    #[serde(rename = "msg")]
    pub message: String,
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
            OrderGatewayEvent::OrderReplacedOrAmended(roa) => Some(&roa.replaced_order.order_id),
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
            OrderGatewayEvent::OrderReplacedOrAmended(roa) => Some(&roa.replaced_order.symbol),
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

/// Event emitted when an order is replaced (cancel-replace) or amended.
///
/// The `replaced_order` field contains the **old** (replaced) order in its
/// terminal `Replaced` state.  The `replacement_order_id` contains the ID
/// of the **new** order that supersedes it (if this was a cancel-replace
/// rather than an in-place amend), and `replacement_order` contains its
/// full details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OrderReplacedOrAmended {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "eid")]
    pub execution_id: String,
    /// The old (replaced) order, now in terminal `Replaced` state.
    #[serde(rename = "ro")]
    pub replaced_order: OrderDetails,
    /// The new replacement order's ID, if this was a cancel-replace.
    #[serde(rename = "noid", skip_serializing_if = "Option::is_none")]
    pub replacement_order_id: Option<OrderId>,
    /// The new replacement order's details, if this was a cancel-replace.
    #[serde(rename = "no", skip_serializing_if = "Option::is_none")]
    pub replacement_order: Option<OrderDetails>,
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
    pub quantity: u64,
    #[serde(rename = "xq")]
    pub filled_quantity: u64,
    #[serde(rename = "rq")]
    pub remaining_quantity: u64,
    #[serde(rename = "o")]
    pub order_state: OrderState,
    #[serde(rename = "d")]
    pub side: Side,
    #[serde(rename = "tif")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "cid", skip_serializing_if = "Option::is_none")]
    pub clord_id: Option<u64>,
    #[serde(rename = "tag", skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(rename = "po", default)]
    pub post_only: bool,
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
            post_only: value.post_only,
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
            post_only: value.post_only,
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
    pub quantity: u64,
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
    #[serde(flatten)]
    pub pagination: LimitOffsetPagination,
    /// Optional order state filter
    pub order_state: Option<OrderState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetOrdersResponse {
    pub orders: Vec<OrderDetails>,
    #[serde(flatten)]
    pub page: LimitOffsetPage,
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
pub struct GetOrderStatusRequest {
    /// Order ID to query; e.g. "ORD-1234567890".
    /// Mutually exclusive with client_order_id - exactly one must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<OrderId>,
    /// Client order ID to query; 64 bit integer.
    /// Mutually exclusive with order_id - exactly one must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::IntoParams))]
pub struct OrderStatus {
    pub symbol: String,
    pub order_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clord_id: Option<u64>,
    pub state: OrderState,
    // TODO: should we have default values for these?
    pub filled_quantity: Option<u64>,
    pub remaining_quantity: Option<u64>,
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
            order_id: Some(OrderId::new_unchecked("O-12345")),
            client_order_id: None,
        };
        let request_with_client_id = GetOrderStatusRequest {
            order_id: None,
            client_order_id: Some(42),
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

        let parsed: GetOrderStatusRequest =
            serde_json::from_str(json_order_id).expect("parse with order_id");
        assert_json_snapshot!(parsed, @r#"
        {
          "order_id": "O-12345"
        }
        "#);

        let parsed: GetOrderStatusRequest =
            serde_json::from_str(json_client_id).expect("parse with client_order_id");
        assert_json_snapshot!(parsed, @r#"
        {
          "client_order_id": 42
        }
        "#);
    }

    #[test]
    fn cancel_all_orders_request_serialization() {
        assert_json_snapshot!(
            CancelAllOrdersRequest { symbol: Some("TEST-PERP".to_string()) },
            @r#"
        {
          "symbol": "TEST-PERP"
        }
        "#
        );
        assert_json_snapshot!(
            CancelAllOrdersRequest { symbol: None },
            @"{}"
        );
    }

    #[test]
    fn cancel_all_orders_ws_request_serialization() {
        let wrapped = ws::Request {
            request_id: 7,
            request: OrderGatewayRequest::CancelAllOrders(CancelAllOrdersRequest {
                symbol: Some("EURUSD-PERP".to_string()),
            }),
        };
        assert_json_snapshot!(wrapped, @r#"
        {
          "rid": 7,
          "t": "X",
          "symbol": "EURUSD-PERP"
        }
        "#);

        let wrapped_no_symbol = ws::Request {
            request_id: 8,
            request: OrderGatewayRequest::CancelAllOrders(CancelAllOrdersRequest { symbol: None }),
        };
        assert_json_snapshot!(wrapped_no_symbol, @r#"
        {
          "rid": 8,
          "t": "X"
        }
        "#);
    }

    #[test]
    fn test_get_orders_request_serialization() {
        let request = GetOrdersRequest {
            symbol: Some("BTCUSD-PERP".to_string()),
            start_time: Some("2024-01-01T00:00:00Z".parse().unwrap()),
            end_time: Some("2024-01-31T23:59:59Z".parse().unwrap()),
            pagination: LimitOffsetPagination {
                limit: Some(100),
                offset: Some(0),
            },
            order_state: Some(OrderState::Filled),
        };
        assert_json_snapshot!(request, @r#"
        {
          "symbol": "BTCUSD-PERP",
          "start_time": "2024-01-01T00:00:00Z",
          "end_time": "2024-01-31T23:59:59Z",
          "limit": 100,
          "offset": 0,
          "order_state": "FILLED"
        }
        "#);
    }
}
