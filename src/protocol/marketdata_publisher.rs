use crate::{
    InstrumentState,
    protocol::api_gateway::GetEstimatedFundingRateResponse,
    protocol::common::Timestamp,
    types::trading::{BboCandle, Candle, CandleWidth},
};
use enumflags2::bitflags;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MarketdataRequest<'a> {
    /// Subscribe to market data on a symbol at a level. A book level
    /// (`LEVEL_1`/`LEVEL_2`/`LEVEL_3`) delivers book updates, ticker, and
    /// trades; `TRADES` delivers only trade prints.
    ///
    /// For a book-level subscription, `trades` and `ticker` (both default
    /// `true`) independently suppress trade or ticker delivery — e.g. set
    /// `trades: false` for a book-only feed, or `ticker: false` for book and
    /// trades without the periodic ticker. They have no effect on a `TRADES`
    /// subscription.
    Subscribe {
        symbol: &'a str,
        level: SubscriptionLevel,
        #[serde(default = "default_true", skip_serializing_if = "is_true")]
        trades: bool,
        #[serde(default = "default_true", skip_serializing_if = "is_true")]
        ticker: bool,
    },
    /// Unsubscribe from ticker and trade updates on a symbol.
    Unsubscribe { symbol: &'a str },
    /// Subscribe to candle updates on a symbol.
    #[serde(rename = "subscribe_candles")]
    SubscribeCandles { symbol: &'a str, width: CandleWidth },
    /// Unsubscribe from candle updates on a pair of symbol and width.
    #[serde(rename = "unsubscribe_candles")]
    UnsubscribeCandles { symbol: &'a str, width: CandleWidth },
    /// Subscribe to BBO (mid-price) candle updates on a symbol.
    #[serde(rename = "subscribe_bbo_candles")]
    SubscribeBboCandles { symbol: &'a str, width: CandleWidth },
    /// Unsubscribe from BBO candle updates on a pair of symbol and width.
    #[serde(rename = "unsubscribe_bbo_candles")]
    UnsubscribeBboCandles { symbol: &'a str, width: CandleWidth },
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
    /// Receive only trade prints, with no order book or ticker updates.
    #[serde(rename = "TRADES")]
    Trades = 0b1000,
}

impl SubscriptionLevel {
    /// The book levels (`LEVEL_1`/`LEVEL_2`/`LEVEL_3`), i.e. every level that
    /// delivers order book and ticker updates as opposed to trades only.
    pub fn book_levels() -> enumflags2::BitFlags<SubscriptionLevel> {
        SubscriptionLevel::Level1 | SubscriptionLevel::Level2 | SubscriptionLevel::Level3
    }
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
    #[serde(rename = "bc")]
    BboCandle(BboCandle),
}

/// Low frequency (e.g. ~1s or ~5s) stats update for a symbol.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Ticker {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    /// Instrument symbol; e.g. XAU-PERP, EURUSD-PERP
    #[serde(rename = "s")]
    pub symbol: String,
    /// Last trade price in USD
    #[serde(rename = "p")]
    pub last_trade_price: Option<Decimal>,
    /// Last trade quantity in contracts
    #[serde(rename = "q")]
    pub last_trade_quantity: u64,
    /// Session open price in USD
    #[serde(rename = "o")]
    pub session_open_price: Option<Decimal>,
    /// Session low price in USD
    #[serde(rename = "l")]
    pub session_low_price: Option<Decimal>,
    /// Session high price in USD
    #[serde(rename = "h")]
    pub session_high_price: Option<Decimal>,
    /// Total 24h volume in contracts (quantity traded, not notional value)
    #[serde(rename = "v")]
    pub total_volume: u64,
    /// Open interest in contracts
    #[serde(rename = "oi")]
    pub open_interest: u64,
    /// Instrument state
    #[serde(rename = "i")]
    #[serde(default)]
    pub instrument_state: InstrumentState,
    #[serde(rename = "m")]
    pub mark_price: Decimal,
    #[serde(rename = "bp")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_price: Option<Decimal>,
    #[serde(rename = "ap")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_price: Option<Decimal>,
    /// Price band lower limit in USD: the absolute lower bound of the price band that orders are checked against
    #[serde(rename = "pl")]
    pub price_band_lower_limit: Option<Decimal>,
    /// Price band upper limit in USD: the absolute upper bound of the price band that orders are checked against
    #[serde(rename = "pu")]
    pub price_band_upper_limit: Option<Decimal>,
    /// Last settlement price in USD
    #[serde(rename = "lsp")]
    #[serde(default)]
    pub last_settlement_price: Option<Decimal>,
    /// Last settlement time as epoch seconds
    #[serde(rename = "lst")]
    #[serde(default)]
    pub last_settlement_time: Option<i32>,
    /// Live estimated funding rate, when available for this symbol.
    #[serde(rename = "ef")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_funding_rate: Option<GetEstimatedFundingRateResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookUpdateData<Snapshot = (), Level = L2BookLevel> {
    #[serde(flatten)]
    pub timestamp: Timestamp,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bids: Vec<Level>,
    #[serde(rename = "a")]
    pub asks: Vec<Level>,
    #[serde(flatten)]
    pub snapshot: Snapshot,
}

pub type L1BookUpdate = BookUpdateData<()>;
pub type L2BookUpdate = BookUpdateData<SnapshotFlag>;
pub type L3BookUpdate = BookUpdateData<SnapshotFlag, L3BookLevel>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFlag {
    #[serde(rename = "st")]
    pub is_snapshot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct L3BookLevel {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: u64,
    #[serde(rename = "o")]
    pub order_quantities: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Trade {
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub quantity: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "d")]
    pub taker_side: crate::types::trading::Side,
    #[serde(flatten)]
    pub timestamp: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::api_gateway::EstimatedFundingRateStatus;
    use chrono::{DateTime, Utc};
    use serde_json::json;

    fn ticker(estimated_funding_rate: Option<GetEstimatedFundingRateResponse>) -> Ticker {
        Ticker {
            timestamp: Timestamp { ts: 1, tn: 2 },
            symbol: "EURUSD-PERP".to_string(),
            last_trade_price: None,
            last_trade_quantity: 0,
            session_open_price: None,
            session_low_price: None,
            session_high_price: None,
            total_volume: 0,
            open_interest: 0,
            instrument_state: InstrumentState::default(),
            mark_price: Decimal::new(100, 0),
            bid_price: None,
            ask_price: None,
            price_band_lower_limit: None,
            price_band_upper_limit: None,
            last_settlement_price: None,
            last_settlement_time: None,
            estimated_funding_rate,
        }
    }

    #[test]
    fn ticker_omits_estimated_funding_when_absent() {
        let value = serde_json::to_value(ticker(None)).unwrap();

        assert!(value.get("ef").is_none());
    }

    #[test]
    fn ticker_serializes_estimated_funding_payload() {
        let timestamp = "2026-05-05T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let value = serde_json::to_value(ticker(Some(GetEstimatedFundingRateResponse {
            symbol: "EURUSD-PERP".to_string(),
            status: EstimatedFundingRateStatus::Ready,
            reason: None,
            funding_rate: Some(Decimal::new(5, 3)),
            funding_amount: Some(Decimal::new(5, 1)),
            benchmark_price: Some(Decimal::new(10025, 2)),
            settlement_price: Some(Decimal::new(10075, 2)),
            timestamp,
        })))
        .unwrap();

        assert_eq!(
            value.get("ef").unwrap(),
            &json!({
                "symbol": "EURUSD-PERP",
                "status": "ready",
                "funding_rate": "0.005",
                "funding_amount": "0.5",
                "benchmark_price": "100.25",
                "settlement_price": "100.75",
                "timestamp": "2026-05-05T12:00:00Z"
            })
        );

        let round_trip: Ticker = serde_json::from_value(value).unwrap();
        assert!(matches!(
            round_trip.estimated_funding_rate.unwrap().status,
            EstimatedFundingRateStatus::Ready
        ));
    }
}
