use crate::protocol::ws::Timestamp;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Ticker {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    /// Instrument symbol; e.g. GBPUSD-PERP, EURUSD-PERP
    #[serde(rename = "s")]
    pub symbol: String,
    /// Last trade price in USD
    #[serde(rename = "p")]
    pub last_trade_price: Option<Decimal>,
    /// Last trade quantity in contracts
    #[serde(rename = "q")]
    pub last_trade_quantity: i32,
    /// Session open price in USD
    #[serde(rename = "o")]
    pub session_open_price: Option<Decimal>,
    /// Session low price in USD
    #[serde(rename = "l")]
    pub session_low_price: Option<Decimal>,
    /// Session high price in USD
    #[serde(rename = "h")]
    pub session_high_price: Option<Decimal>,
    /// Total volume in contracts
    #[serde(rename = "v")]
    pub total_volume: i32,
    /// Open interest in contracts
    #[serde(rename = "oi")]
    pub open_interest: i32,
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
