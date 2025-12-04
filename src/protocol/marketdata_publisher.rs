use crate::{
    protocol::common::Timestamp,
    types::trading::{Candle, CandleWidth},
    InstrumentState,
};
use enumflags2::bitflags;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MarketdataRequest<'a> {
    /// Subscribe to ticker and trade updates on a symbol at a level.
    Subscribe {
        symbol: &'a str,
        level: SubscriptionLevel,
    },
    /// Unsubscribe from ticker and trade updates on a symbol.
    Unsubscribe { symbol: &'a str },
    /// Subscribe to candle updates on a symbol.
    #[serde(rename = "subscribe_candles")]
    SubscribeCandles { symbol: &'a str, width: CandleWidth },
    /// Unsubscribe from candle updates on a pair of symbol and width.
    #[serde(rename = "unsubscribe_candles")]
    UnsubscribeCandles { symbol: &'a str, width: CandleWidth },
}

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SubscriptionLevel {
    /// Receive updates on just the top level of the order book.
    #[serde(rename = "LEVEL_1")]
    Level1 = 0b001,
    /// Receive updates (price and quantity) for all levels of the order book.
    #[serde(rename = "LEVEL_2")]
    Level2 = 0b010,
    /// Receive updates (price, quantity, and distinct orders) for all levels of the order book.
    #[serde(rename = "LEVEL_3")]
    Level3 = 0b100,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(rename = "t")]
    Trade(Trade),
    #[serde(rename = "c")]
    Candle(Candle),
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
    pub open_interest: u32,
    /// Instrument state
    #[serde(rename = "i")]
    #[serde(default)]
    pub instrument_state: InstrumentState,
    #[serde(rename = "m")]
    pub mark_price: Decimal,
    /// Price band lower limit in USD (absolute bound calculated from settlement price and lower deviation percentage)
    #[serde(rename = "pl")]
    pub price_band_lower_limit: Option<Decimal>,
    /// Price band upper limit in USD (absolute bound calculated from settlement price and upper deviation percentage)
    #[serde(rename = "pu")]
    pub price_band_upper_limit: Option<Decimal>,
}

pub type L1BookUpdate = L2BookUpdate;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct L3BookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "o")]
    pub order_quantities: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Trade {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: i32,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "d")]
    pub taker_side: crate::types::trading::Side,
    #[serde(flatten)]
    pub timestamp: Timestamp,
}
