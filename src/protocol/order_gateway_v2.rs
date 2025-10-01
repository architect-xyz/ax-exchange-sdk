//! Order Gateway V2 Types
//!
//! This module contains types used for Order Gateway V2 requests and responses.

use crate::types::trading::{Order, OrderState, Side};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertOrderRequest {
    pub username: String,
    pub symbol: String,
    pub side: Side,
    pub quantity: i32,
    pub price: Decimal,
    pub time_in_force: String,
    pub post_only: bool,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertOrderResponse {
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    pub username: String,
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderResponse {
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersRequest {
    pub username: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersResponse {
    pub orders: Vec<GetOpenOrdersResponseOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOpenOrdersResponseOrder {
    pub order_id: String,
    pub symbol: String,
    pub side: Side,
    pub quantity: i32,
    pub price: Decimal,
    pub filled_quantity: i32,
    pub remaining_quantity: i32,
    pub order_state: OrderState,
    pub time_in_force: String,
    pub timestamp: DateTime<Utc>,
    pub tag: Option<String>,
}

impl From<Order> for GetOpenOrdersResponseOrder {
    fn from(order: Order) -> Self {
        Self {
            order_id: order.order_id,
            symbol: order.symbol,
            side: order.side,
            quantity: order.quantity,
            price: order.price,
            filled_quantity: order.filled_quantity,
            remaining_quantity: order.remaining_quantity,
            order_state: order.order_state,
            time_in_force: order.time_in_force,
            timestamp: order.timestamp,
            tag: order.tag,
        }
    }
}

impl From<GetOpenOrdersResponseOrder> for Order {
    fn from(order: GetOpenOrdersResponseOrder) -> Self {
        Self {
            order_id: order.order_id,
            user_id: Uuid::nil(), // Not available in response
            symbol: order.symbol,
            price: order.price,
            quantity: order.quantity,
            filled_quantity: order.filled_quantity,
            remaining_quantity: order.remaining_quantity,
            order_state: order.order_state,
            side: order.side,
            time_in_force: order.time_in_force,
            timestamp: order.timestamp,
            tag: order.tag,
            completion_time: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAllRequest {
    pub username: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAllResponse {
    pub canceled_orders: Vec<String>,
}
