//! Type definitions for the ArchitectX SDK
//!
//! This module contains all type definitions organized into logical submodules.

pub mod auth;
pub mod days_of_week;
pub mod environment;
pub mod funding_rate_schedule;
pub mod order_id;
pub mod orderbook;
pub mod symbol;
pub mod tag;
pub mod trading;

// Re-export commonly used types for convenience
pub use auth::{ApiKey, ApiKeyType, Password, Token, Username};
pub use days_of_week::DaysOfWeek;
pub use funding_rate_schedule::{FundingException, FundingRateSchedule, FundingTime};
pub use order_id::OrderId;
pub use orderbook::{Orderbook, OrderbookLevel};
pub use symbol::Symbol;
pub use tag::Tag;
pub use trading::{
    Balance, BboCandle, Candle, DepositRecord, FundingHistory, Instrument, InstrumentState,
    InstrumentV0, OpenInterest, OpenInterestData, Order, OrderRejectReason, OrderState, PlaceOrder,
    Position, Side, TimeOfDay, TradingHoursSegment, TradingSchedule, WithdrawalRecord,
};
