//! Type definitions for the ArchitectX SDK
//!
//! This module contains all type definitions organized into logical submodules.

pub mod auth;
pub mod order_id;
pub mod orderbook;
pub mod symbol;
pub mod tag;
pub mod trading;

// Re-export commonly used types for convenience
pub use auth::{ApiKey, Password, Token, Username};
pub use order_id::OrderId;
pub use orderbook::{Orderbook, OrderbookLevel};
pub use symbol::Symbol;
pub use tag::Tag;
pub use trading::{
    Balance, Candle, DepositRecord, FundingHistory, Instrument, InstrumentState, InstrumentV0,
    OpenInterest, OpenInterestData, Order, OrderRejectReason, OrderState, PlaceOrder, Position,
    Side, TimeOfDay, TradingHoursSegment, TradingSchedule, WithdrawalRecord,
};
