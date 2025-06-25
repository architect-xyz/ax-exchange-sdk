use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct PlaceOrder {
    pub symbol: String,
    pub side: String,
    pub quantity: i32,
    pub price: Decimal,
    pub time_in_force: String,
    pub post_only: bool,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: String,
    pub username: String,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: i32,
    pub filled_quantity: i32,
    pub remaining_quantity: i32,
    pub order_state: String,
    pub side: String,
    pub time_in_force: String,
    pub timestamp: DateTime<Utc>,
}
