//! Business Logic Types
//!
//! This module contains core business types for trading operations.

use crate::OrderId;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::VariantArray;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentV0 {
    pub symbol: String,
    pub tick_size: Decimal,
    pub base_currency: String,
    pub multiplier: i32,
    pub minimum_trade_quantity: u64,
    pub description: String,
    pub product_id: String,
    pub state: String,
    pub price_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Instrument {
    pub symbol: String,
    // Programmatic specification fields
    pub multiplier: Decimal,
    pub price_scale: i64,
    pub minimum_order_size: Decimal,
    pub tick_size: Decimal,
    pub quote_currency: String,
    pub price_band_lower_deviation_pct: Option<Decimal>,
    pub price_band_upper_deviation_pct: Option<Decimal>,
    pub funding_settlement_currency: String,
    pub funding_rate_cap_upper_pct: Option<Decimal>,
    pub funding_rate_cap_lower_pct: Option<Decimal>,
    pub maintenance_margin_pct: Decimal,
    pub initial_margin_pct: Decimal,
    pub category: InstrumentCategory,
    // English language specification fields
    pub description: Option<String>,
    pub underlying_benchmark_price: Option<String>,
    pub contract_mark_price: Option<String>,
    pub contract_size: Option<String>,
    pub price_quotation: Option<String>,
    pub price_bands: Option<String>,
    pub funding_frequency: Option<String>,
    pub funding_calendar_schedule: Option<String>,
    pub trading_schedule: Option<TradingSchedule>,
}

#[derive(
    Copy, Clone, Debug, Eq, PartialEq, strum::Display, strum::EnumString, Serialize, Deserialize,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum InstrumentCategory {
    Fx,
    Equities,
    Metals,
}

/// Trading schedule for an instrument, containing multiple trading hour segments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TradingSchedule {
    pub segments: Vec<TradingHoursSegment>,
}

/// A single trading hours segment with specific days, times, and state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TradingHoursSegment {
    /// Days of the week (1=Monday, 2=Tuesday, ..., 7=Sunday)
    pub days_of_week: Vec<u8>,
    /// Time of day when this segment starts
    pub time_of_day: TimeOfDay,
    /// Duration of this segment in seconds
    pub duration_seconds: u64,
    /// Trading state during this segment
    pub state: InstrumentState,
    /// Whether to hide market data during this segment
    pub hide_market_data: bool,
    /// Whether to expire all orders during this segment
    pub expire_all_orders: bool,
}

/// Time of day representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TimeOfDay {
    pub hours: u8,
    pub minutes: u8,
    #[serde(default)]
    pub seconds: u8,
}

#[derive(Default, Debug, strum::Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentState {
    /// The instrument is closed. No trading can occur, nor can orders
    /// even be cancelled in this state.
    ClosedFrozen,

    /// The instrument is available to place orders and modify them
    /// before the opening, but no matching will occur until the open.
    ///
    /// At the open, crossing orders will be matched via Dutch auction.
    PreOpen,

    /// The instrument is open and is available for full trading.
    Open,

    /// The instrument has suspended trading. In this state, no orders
    /// can be placed or modified, but they can be cancelled.
    Closed,

    /// The instrument has been delisted.  This state is terminal.
    Delisted,

    /// The instrument has halted trading. This state is similar to the suspended state in that no orders can be placed or modified, but orders cannot be cancelled unlike the suspended state which allows cancellation.
    Halted,

    /// The instrument is available to place orders and modify them just as in pre open, but no matching will occur until this state has been exited.
    MatchAndCloseAuction,

    /// The instrument status is unknown.
    #[default]
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PlaceOrder {
    pub symbol: String,
    pub side: Side,
    pub quantity: u64,
    pub price: Decimal,
    pub time_in_force: String,
    pub post_only: bool,
    pub tag: Option<String>,
    pub clord_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: OrderId,
    pub user_id: String,
    pub symbol: String,
    pub side: Side,
    pub quantity: u64,
    pub price: Decimal,
    pub time_in_force: String,
    pub tag: Option<String>,
    pub clord_id: Option<u64>,
    /// Timestamp when the order was received by the order gateway
    pub timestamp: DateTime<Utc>,
    pub order_state: OrderState,
    pub filled_quantity: u64,
    pub remaining_quantity: u64,
    /// Timestamp when the order state became terminal
    pub completion_time: Option<DateTime<Utc>>,
    /// Reason for rejection if order_state is Rejected
    pub reject_reason: Option<OrderRejectReason>,
    /// Additional message for rejection if order_state is Rejected
    pub reject_message: Option<String>,
}

impl Order {
    /// Check if this is a liquidation order
    pub fn is_liquidation(&self) -> bool {
        self.order_id.is_liquidation()
    }
}

#[derive(Debug, derive_more::Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Side {
    #[serde(rename = "B")]
    Buy,
    #[serde(rename = "S")]
    Sell,
}

impl Side {
    pub fn as_char(&self) -> &'static str {
        match self {
            Self::Buy => "B",
            Self::Sell => "S",
        }
    }

    pub fn from_char(s: &str) -> Result<Self> {
        let t = match s {
            "B" => Self::Buy,
            "S" => Self::Sell,
            other => bail!("unknown side: {other}"),
        };
        Ok(t)
    }

    pub fn position_sign(&self) -> i8 {
        match self {
            Self::Buy => 1,
            Self::Sell => -1,
        }
    }

    pub fn flip(&self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum OrderState {
    #[strum(serialize = "PENDING")]
    #[serde(rename = "PENDING")]
    Pending,
    #[strum(serialize = "ACCEPTED")]
    #[serde(rename = "ACCEPTED")]
    Accepted,
    #[strum(serialize = "PARTIALLY_FILLED")]
    #[serde(rename = "PARTIALLY_FILLED")]
    PartiallyFilled,
    #[strum(serialize = "FILLED")]
    #[serde(rename = "FILLED")]
    Filled,
    #[strum(serialize = "CANCELED")]
    #[serde(rename = "CANCELED")]
    Canceled,
    #[strum(serialize = "REJECTED")]
    #[serde(rename = "REJECTED")]
    Rejected,
    #[strum(serialize = "EXPIRED")]
    #[serde(rename = "EXPIRED")]
    Expired,
    #[strum(serialize = "REPLACED")]
    #[serde(rename = "REPLACED")]
    Replaced,
    #[strum(serialize = "DONE_FOR_DAY")]
    #[serde(rename = "DONE_FOR_DAY")]
    DoneForDay,
    #[strum(serialize = "UNKNOWN")]
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl OrderState {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub fn is_open(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Accepted | Self::PartiallyFilled => true,
            _ => false,
        }
    }

    pub fn is_terminal(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Canceled
            | Self::Filled
            | Self::Rejected
            | Self::Replaced
            | Self::DoneForDay
            | Self::Expired => true,
            _ => false,
        }
    }

    pub fn can_transition_to(&self, next_state: &Self) -> bool {
        match self {
            Self::Pending => matches!(
                next_state,
                Self::Pending
                    | Self::Accepted
                    | Self::Rejected
                    | Self::Canceled
                    | Self::Expired
                    | Self::Replaced
                    | Self::DoneForDay
            ),
            Self::Accepted => matches!(
                next_state,
                Self::Accepted
                    | Self::PartiallyFilled
                    | Self::Filled
                    | Self::Canceled
                    | Self::Expired
                    | Self::Replaced
                    | Self::DoneForDay
            ),
            Self::PartiallyFilled => matches!(
                next_state,
                Self::PartiallyFilled
                    | Self::Filled
                    | Self::Canceled
                    | Self::Expired
                    | Self::Replaced
                    | Self::DoneForDay
            ),
            _ => false, // terminal states
        }
    }

    /// Check if the order can be canceled
    pub fn can_be_canceled(&self) -> bool {
        matches!(self, Self::Pending | Self::Accepted | Self::PartiallyFilled)
    }

    /// Check if the order can be replaced
    pub fn can_be_replaced(&self) -> bool {
        matches!(self, Self::Accepted | Self::PartiallyFilled)
    }

    /// Canonical single-character representation
    pub fn as_char(&self) -> &'static str {
        match self {
            Self::Pending => "P",
            Self::Accepted => "A",
            Self::PartiallyFilled => "D",
            Self::Filled => "F",
            Self::Canceled => "X",
            Self::Rejected => "R",
            Self::Expired => "E",
            Self::Replaced => "K",
            Self::DoneForDay => "Z",
            Self::Unknown => "?",
        }
    }

    pub fn from_char(s: &str) -> Result<Self> {
        let t = match s {
            "P" => Self::Pending,
            "A" => Self::Accepted,
            "D" => Self::PartiallyFilled,
            "F" => Self::Filled,
            "X" => Self::Canceled,
            "R" => Self::Rejected,
            "E" => Self::Expired,
            "K" => Self::Replaced,
            "Z" => Self::DoneForDay,
            "?" => Self::Unknown,
            other => bail!("unknown order state: {other}"),
        };
        Ok(t)
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::EnumString,
    strum::Display,
    strum::IntoStaticStr,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderRejectReason {
    /// User in close-only mode attempting non-closing order
    CloseOnly,
    /// Initial margin required exceeds available
    InsufficientMargin,
    /// User has too many open orders
    MaxOpenOrdersExceeded,
    /// Unknown or invalid symbol
    UnknownSymbol,
    /// Exchange is closed
    ExchangeClosed,
    /// Incorrect or invalid quantity
    IncorrectQuantity,
    /// Invalid price increment
    InvalidPriceIncrement,
    /// Incorrect order type
    IncorrectOrderType,
    /// Price is outside allowed bounds
    PriceOutOfBounds,
    /// No liquidity available
    NoLiquidity,
    /// Insufficient credit limit
    InsufficientCreditLimit,
    /// Unknown or unrecognized reject reason
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub currency: String,
    pub available: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub signed_quantity: i64,
    pub average_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub mark_price: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Candle {
    pub symbol: String,
    #[serde(rename = "ts")]
    #[serde_as(as = "serde_with::TimestampSeconds")]
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub buy_volume: u64,
    pub sell_volume: u64,
    pub volume: u64,
    pub width: CandleWidth,
}

// TODO: re-examine the shape of this type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterest {
    pub symbol: String,
    pub data: Vec<OpenInterestData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestData {
    pub timestamp: DateTime<Utc>,
    pub open_interest: Decimal,
}

// TODO: re-examine the name of this type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingHistory {
    pub symbol: String,
    pub funding_amount: Decimal,
    pub net_position: i32,
    pub timestamp: DateTime<Utc>,
    pub funding_rate: Decimal,
}

// TODO: reconsider where this type lives; cash management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRecord {
    pub id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
}

// TODO: reconsider where this type lives; cash management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalRecord {
    pub id: String,
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
}

#[derive(
    Copy,
    Clone,
    VariantArray,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    derive_more::Display,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum CandleWidth {
    #[serde(rename = "1s")]
    #[display("1s")]
    OneSecond,
    #[serde(rename = "5s")]
    #[display("5s")]
    FiveSecond,
    #[serde(rename = "1m")]
    #[display("1m")]
    OneMinute,
    #[serde(rename = "5m")]
    #[display("5m")]
    FiveMinute,
    #[serde(rename = "15m")]
    #[display("15m")]
    FifteenMinute,
    #[serde(rename = "1h")]
    #[display("1h")]
    OneHour,
    #[serde(rename = "1d")]
    #[display("1d")]
    OneDay,
}

impl std::str::FromStr for CandleWidth {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "1s" => Ok(Self::OneSecond),
            "5s" => Ok(Self::FiveSecond),
            "1m" => Ok(Self::OneMinute),
            "5m" => Ok(Self::FiveMinute),
            "15m" => Ok(Self::FifteenMinute),
            "1h" => Ok(Self::OneHour),
            "1d" => Ok(Self::OneDay),
            _ => Err(format!("unrecognized candle width: '{s}'")),
        }
    }
}

impl CandleWidth {
    /// Get the closed interval of nanosecond timestamps containing `instant`
    /// that form the candle of this width.
    pub fn to_nanosec_window(&self, instant: u64) -> (u64, u64) {
        let ns_in_sec = 1_000_000_000;
        let nanosec = match self {
            CandleWidth::OneSecond => ns_in_sec,
            CandleWidth::FiveSecond => ns_in_sec * 5,
            CandleWidth::OneMinute => ns_in_sec * 60,
            CandleWidth::FiveMinute => ns_in_sec * 60 * 5,
            CandleWidth::FifteenMinute => ns_in_sec * 60 * 15,
            CandleWidth::OneHour => ns_in_sec * 60 * 60,
            CandleWidth::OneDay => ns_in_sec * 60 * 60 * 24,
        };

        let start = instant - (instant % nanosec);
        let end = start + nanosec - 1;

        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 20250916T19:19:39.100Z - Arbitrary time
    const TIME_1: u64 = 1758050379100000000;

    // 20250916T16:29:43.500Z - Arbitrary time
    const TIME_2: u64 = 1758040183500000000;

    // 20250916T00:00:00.000Z - Exact midnight UTC
    const MIDNIGHT_UTC: u64 = 1757980800000000000;

    // 20250916T23:59:59.999999999Z - Last nanosecond of the day
    const END_OF_DAY: u64 = 1758067199999999999;

    // 20250916T12:00:00.000Z - Exact noon
    const NOON: u64 = 1758024000000000000;

    // 20250916T11:59:59.999999999Z - Last nanosecond before noon
    const JUST_BEFORE_NOON: u64 = 1758023999999999999;

    // 20250916T15:30:00.000Z - Exact half hour
    const HALF_HOUR: u64 = 1758036600000000000;

    // 20250916T15:30:05.000Z - Exact 5-second boundary
    const FIVE_SEC_BOUNDARY: u64 = 1758036605000000000;

    // 20250916T15:30:04.999999999Z - Just before 5-second boundary
    const JUST_BEFORE_FIVE_SEC: u64 = 1758036604999999999;

    #[test]
    fn one_second_candle_window() {
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(TIME_1);
        assert_eq!(start, 1758050379000000000);
        assert_eq!(end, 1758050379999999999);
    }

    #[test]
    fn one_second_exact_boundary() {
        // Exactly on the second boundary
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(NOON);
        assert_eq!(start, 1758024000000000000);
        assert_eq!(end, 1758024000999999999);
    }

    #[test]
    fn one_second_last_nanosecond() {
        // Last nanosecond of a second
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758023999000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn five_second_candle_window() {
        // Using TIME_1: 20250916T19:19:39.100Z
        // The 5-second window containing this instant is 19:19:35 to 19:19:39.999999999
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(TIME_1);
        assert_eq!(start, 1758050375000000000);
        assert_eq!(end, 1758050379999999999);
    }

    #[test]
    fn five_second_exact_boundary() {
        // Exactly on a 5-second boundary (15:30:05)
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(FIVE_SEC_BOUNDARY);
        assert_eq!(start, 1758036605000000000);
        assert_eq!(end, 1758036609999999999);
    }

    #[test]
    fn five_second_just_before_boundary() {
        // Just before a 5-second boundary (15:30:04.999999999)
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(JUST_BEFORE_FIVE_SEC);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758036604999999999);
    }

    #[test]
    fn five_second_at_minute_boundary() {
        // At exact minute boundary (should align to :00 to :04.999999999)
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(HALF_HOUR);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758036604999999999);
    }

    #[test]
    fn one_minute_candle_window() {
        // Using TIME_1: 20250916T19:19:39.100Z
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(TIME_1);
        assert_eq!(start, 1758050340000000000);
        assert_eq!(end, 1758050399999999999);
    }

    #[test]
    fn one_minute_exact_boundary() {
        // Exactly on minute boundary
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(HALF_HOUR);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758036659999999999);
    }

    #[test]
    fn one_minute_last_nanosecond() {
        // Last nanosecond before noon (11:59:59.999999999)
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758023940000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn fifteen_minute_candle_window() {
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(TIME_2);
        assert_eq!(start, 1758039300000000000);
        assert_eq!(end, 1758040199999999999);
    }

    #[test]
    fn fifteen_minute_at_half_hour() {
        // 15:30 should be in the 15:30-15:44:59.999999999 window
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(HALF_HOUR);
        assert_eq!(start, 1758036600000000000);
        assert_eq!(end, 1758037499999999999);
    }

    #[test]
    fn fifteen_minute_at_noon() {
        // Noon should be in the 12:00-12:14:59.999999999 window
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(NOON);
        assert_eq!(start, 1758024000000000000);
        assert_eq!(end, 1758024899999999999);
    }

    #[test]
    fn fifteen_minute_just_before_noon() {
        // 11:59:59.999999999 should be in the 11:45-11:59:59.999999999 window
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758023100000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn one_hour_candle_window() {
        // Using TIME_2: 20250916T16:29:43.500Z
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(TIME_2);
        assert_eq!(start, 1758038400000000000);
        assert_eq!(end, 1758041999999999999);
    }

    #[test]
    fn one_hour_exact_boundary() {
        // Noon should be exactly at hour boundary
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(NOON);
        assert_eq!(start, 1758024000000000000);
        assert_eq!(end, 1758027599999999999);
    }

    #[test]
    fn one_hour_last_nanosecond_before() {
        // Last nanosecond before noon
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(JUST_BEFORE_NOON);
        assert_eq!(start, 1758020400000000000);
        assert_eq!(end, 1758023999999999999);
    }

    #[test]
    fn one_hour_at_midnight() {
        // Midnight UTC
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(MIDNIGHT_UTC);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1757984399999999999);
    }

    #[test]
    fn one_day_candle_window() {
        // Using TIME_1: 20250916T19:19:39.100Z
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(TIME_1);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn one_day_at_midnight() {
        // Exactly at midnight UTC
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(MIDNIGHT_UTC);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn one_day_end_of_day() {
        // Last nanosecond of the day
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(END_OF_DAY);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn one_day_at_noon() {
        // Noon should still be in the same day window
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(NOON);
        assert_eq!(start, 1757980800000000000);
        assert_eq!(end, 1758067199999999999);
    }

    #[test]
    fn boundaries_are_inclusive_and_continuous() {
        // Verify that consecutive windows are continuous with no gaps
        let time = NOON;

        // Check that end of one second + 1 nanosecond = start of next second
        let (_, end1) = CandleWidth::OneSecond.to_nanosec_window(time);
        let (start2, _) = CandleWidth::OneSecond.to_nanosec_window(end1 + 1);
        assert_eq!(end1 + 1, start2);

        // Check that end of one minute + 1 nanosecond = start of next minute
        let (_, end1) = CandleWidth::OneMinute.to_nanosec_window(time);
        let (start2, _) = CandleWidth::OneMinute.to_nanosec_window(end1 + 1);
        assert_eq!(end1 + 1, start2);
    }

    #[test]
    fn window_widths_are_correct() {
        // Verify window widths are exactly what we expect
        let time = NOON;

        // One second = 1_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneSecond.to_nanosec_window(time);
        assert_eq!(end - start + 1, 1_000_000_000);

        // Five seconds = 5_000_000_000 nanoseconds
        let (start, end) = CandleWidth::FiveSecond.to_nanosec_window(time);
        assert_eq!(end - start + 1, 5_000_000_000);

        // One minute = 60_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneMinute.to_nanosec_window(time);
        assert_eq!(end - start + 1, 60_000_000_000);

        // Fifteen minutes = 900_000_000_000 nanoseconds
        let (start, end) = CandleWidth::FifteenMinute.to_nanosec_window(time);
        assert_eq!(end - start + 1, 900_000_000_000);

        // One hour = 3_600_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneHour.to_nanosec_window(time);
        assert_eq!(end - start + 1, 3_600_000_000_000);

        // One day = 86_400_000_000_000 nanoseconds
        let (start, end) = CandleWidth::OneDay.to_nanosec_window(time);
        assert_eq!(end - start + 1, 86_400_000_000_000);
    }

    #[test]
    fn test_trading_schedule_serialization() {
        let schedule = TradingSchedule {
            segments: vec![
                TradingHoursSegment {
                    days_of_week: vec![1, 2, 3, 4, 5],
                    time_of_day: TimeOfDay {
                        hours: 9,
                        minutes: 30,
                        seconds: 0,
                    },
                    duration_seconds: 3600,
                    state: InstrumentState::PreOpen,
                    hide_market_data: false,
                    expire_all_orders: false,
                },
                TradingHoursSegment {
                    days_of_week: vec![1, 2, 3, 4, 5],
                    time_of_day: TimeOfDay {
                        hours: 10,
                        minutes: 30,
                        seconds: 0,
                    },
                    duration_seconds: 21600,
                    state: InstrumentState::Open,
                    hide_market_data: false,
                    expire_all_orders: false,
                },
            ],
        };

        let json = serde_json::to_string(&schedule).unwrap();

        // Verify it contains the expected structure
        assert!(json.contains("\"segments\""));
        assert!(json.contains("\"days_of_week\":[1,2,3,4,5]"));
        assert!(json.contains("\"state\":\"PRE_OPEN\""));
        assert!(json.contains("\"state\":\"OPEN\""));
        assert!(json.contains("\"duration_seconds\":3600"));
        assert!(json.contains("\"duration_seconds\":21600"));
    }

    #[test]
    fn test_trading_schedule_deserialization() {
        let json = r#"{
            "segments": [
                {
                    "days_of_week": [1, 2, 3, 4, 5],
                    "time_of_day": {"hours": 9, "minutes": 30, "seconds": 0},
                    "duration_seconds": 1800,
                    "state": "PRE_OPEN",
                    "hide_market_data": false,
                    "expire_all_orders": false
                },
                {
                    "days_of_week": [1, 2, 3, 4, 5],
                    "time_of_day": {"hours": 10, "minutes": 0, "seconds": 0},
                    "duration_seconds": 21600,
                    "state": "OPEN",
                    "hide_market_data": false,
                    "expire_all_orders": false
                }
            ]
        }"#;

        let schedule: TradingSchedule = serde_json::from_str(json).unwrap();

        assert_eq!(schedule.segments.len(), 2);

        // Check first segment
        let preopen = &schedule.segments[0];
        assert_eq!(preopen.days_of_week, vec![1, 2, 3, 4, 5]);
        assert_eq!(preopen.time_of_day.hours, 9);
        assert_eq!(preopen.time_of_day.minutes, 30);
        assert_eq!(preopen.duration_seconds, 1800);
        assert_eq!(preopen.state, InstrumentState::PreOpen);

        // Check second segment
        let open = &schedule.segments[1];
        assert_eq!(open.time_of_day.hours, 10);
        assert_eq!(open.time_of_day.minutes, 0);
        assert_eq!(open.duration_seconds, 21600);
        assert_eq!(open.state, InstrumentState::Open);
    }

    #[test]
    fn test_instrument_state_serialization() {
        // Test that InstrumentState serializes to expected string values
        assert_eq!(
            serde_json::to_string(&InstrumentState::ClosedFrozen).unwrap(),
            r#""CLOSED_FROZEN""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::PreOpen).unwrap(),
            r#""PRE_OPEN""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Open).unwrap(),
            r#""OPEN""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Closed).unwrap(),
            r#""CLOSED""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Delisted).unwrap(),
            r#""DELISTED""#
        );
        assert_eq!(
            serde_json::to_string(&InstrumentState::Unknown).unwrap(),
            r#""UNKNOWN""#
        );
    }

    #[test]
    fn test_instrument_with_trading_schedule_serialization() {
        let instrument = Instrument {
            symbol: "TEST-PERP".to_string(),
            multiplier: rust_decimal::Decimal::ONE,
            price_scale: 10000,
            minimum_order_size: rust_decimal::Decimal::ONE,
            tick_size: rust_decimal::Decimal::new(1, 4), // 0.0001
            quote_currency: "USD".to_string(),
            price_band_lower_deviation_pct: Some(rust_decimal::Decimal::new(-5, 0)),
            price_band_upper_deviation_pct: Some(rust_decimal::Decimal::new(5, 0)),
            funding_settlement_currency: "USD".to_string(),
            funding_rate_cap_upper_pct: Some(rust_decimal::Decimal::new(1, 0)),
            funding_rate_cap_lower_pct: Some(rust_decimal::Decimal::new(-1, 0)),
            maintenance_margin_pct: rust_decimal::Decimal::new(4, 0),
            initial_margin_pct: rust_decimal::Decimal::new(8, 0),
            category: InstrumentCategory::Fx,
            description: Some("Test Perpetual Future".to_string()),
            underlying_benchmark_price: None,
            contract_mark_price: None,
            contract_size: None,
            price_quotation: None,
            price_bands: None,
            funding_frequency: None,
            funding_calendar_schedule: None,
            trading_schedule: Some(TradingSchedule {
                segments: vec![TradingHoursSegment {
                    days_of_week: vec![1, 2, 3, 4, 5],
                    time_of_day: TimeOfDay {
                        hours: 9,
                        minutes: 30,
                        seconds: 0,
                    },
                    duration_seconds: 1800,
                    state: InstrumentState::PreOpen,
                    hide_market_data: false,
                    expire_all_orders: false,
                }],
            }),
        };

        let json = serde_json::to_string(&instrument).unwrap();

        // Verify trading_schedule is properly included
        assert!(json.contains("\"trading_schedule\":{\"segments\""));
        assert!(json.contains("\"state\":\"PRE_OPEN\""));

        // Verify it can be deserialized back
        let deserialized: Instrument = serde_json::from_str(&json).unwrap();
        assert!(deserialized.trading_schedule.is_some());
        assert_eq!(deserialized.trading_schedule.unwrap().segments.len(), 1);
    }
}
