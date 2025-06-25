use crate::protocol::ws::Timestamp;
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "t")]
pub enum MarketdataEvent {
    #[serde(rename = "h")]
    Heartbeat(Timestamp),
    #[serde(rename = "s")]
    Ticker(Ticker),
    #[serde(rename = "1")]
    L1BookUpdate(L1BookUpdate),
    #[serde(rename = "2")]
    L2BookUpdate(L2BookUpdate),
    #[serde(rename = "3")]
    L3BookUpdate(L3BookUpdate),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ticker {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub last_trade_price: Decimal,
    #[serde(rename = "q")]
    pub last_trade_quantity: i32,
    #[serde(rename = "o")]
    pub session_open_price: Decimal,
    #[serde(rename = "l")]
    pub session_low_price: Decimal,
    #[serde(rename = "h")]
    pub session_high_price: Decimal,
    #[serde(rename = "v")]
    pub total_volume: i32,
}

pub type L1BookUpdate = L2BookUpdate;

#[derive(Debug, Clone, Deserialize)]
pub struct L2BookUpdate {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bids: Vec<L2BookLevel>,
    #[serde(rename = "a")]
    pub asks: Vec<L2BookLevel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct L2BookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct L3BookUpdate {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bids: Vec<L3BookLevel>,
    #[serde(rename = "a")]
    pub asks: Vec<L3BookLevel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct L3BookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "o")]
    pub order_quantities: Vec<i32>,
}
