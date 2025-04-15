use super::SequenceIdAndNumber;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Get a snapshot of the L2 book.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L2BookSnapshotRequest {
    pub symbol: String,
}

/// Subscribe to a stream of L2 book updates.
///
/// This allows you to build and maintain the state of the
/// orderbook in realtime.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubscribeL2BookRequest {
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L2BookSnapshot {
    #[serde(rename = "ts")]
    #[schemars(title = "timestamp")]
    pub timestamp: i64,
    #[serde(rename = "tn")]
    #[schemars(title = "timestamp_ns")]
    pub timestamp_ns: u32,
    #[serde(flatten)]
    pub sequence: SequenceIdAndNumber,
    #[serde(rename = "b")]
    #[schemars(title = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(rename = "a")]
    #[schemars(title = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
}

impl L2BookSnapshot {
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        chrono::DateTime::from_timestamp(self.timestamp, self.timestamp_ns)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct L2BookDiff {
    #[serde(rename = "ts")]
    #[schemars(title = "timestamp")]
    pub timestamp: i64,
    #[serde(rename = "tn")]
    #[schemars(title = "timestamp_ns")]
    pub timestamp_ns: u32,
    #[serde(flatten)]
    pub sequence: SequenceIdAndNumber,
    /// Set of (price, level) updates. If zero, the price level
    /// has been removed from the book.
    #[serde(rename = "b")]
    #[schemars(title = "bids")]
    pub bids: Vec<(Decimal, Decimal)>,
    /// Set of (price, level) updates. If zero, the price level
    /// has been removed from the book.
    #[serde(rename = "a")]
    #[schemars(title = "asks")]
    pub asks: Vec<(Decimal, Decimal)>,
}

impl L2BookDiff {
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        chrono::DateTime::from_timestamp(self.timestamp, self.timestamp_ns)
    }
}

/// To build a book from a stream of updates, the client should first subscribe to
/// this update stream, which then returns a stream starting with a snapshot and
/// following with diffs.
///
/// Diffs should be applied consecutively to the snapshot in order to reconstruct
/// the state of the book.
///
/// ```rust
/// # use architect_api::marketdata::*;
/// # use std::collections::BTreeMap;
/// # use rust_decimal::Decimal;
/// # use rust_decimal_macros::dec;
///
/// /// Suppose we receive this snapshot from the server:
/// let snapshot: L2BookUpdate = serde_json::from_str(r#"{
///     "t": "s",
///     "ts": 1729700837,
///     "tn": 0,
///     "sid": 123,
///     "sn": 8999,
///     "b": [["99.00", "3"], ["98.78", "2"]],
///     "a": [["100.00", "1"], ["100.10", "2"]]
/// }"#)?;
///
/// /// It corresponds to the following book:
/// let mut book = BTreeMap::new();
/// book.insert(dec!(99.00), 3);
/// book.insert(dec!(98.78), 2);
/// book.insert(dec!(100.00), 1);
/// book.insert(dec!(100.10), 2);
///
/// /// Then we receive this update:
/// let diff: L2BookUpdate = serde_json::from_str(r#"{
///     "t": "d",
///     "ts": 1729700839,
///     "tn": 0,
///     "sid": 123,
///     "sn": 9000,
///     "b": [["99.00", "1"]],
///     "a": []
/// }"#)?;
///
/// /// Verify that the sequence number is correct
/// assert!(diff.sequence().is_next_in_sequence(&snapshot.sequence()));
///
/// /// Apply the update to our book
/// book.insert(dec!(99.00), 1);
///
/// // Suppose we then receive this update:
/// let diff: L2BookUpdate = serde_json::from_str(r#"{
///     "t": "d",
///     "ts": 1729700841,
///     "tn": 0,
///     "sid": 123,
///     "sn": 9005,
///     "b": [],
///     "a": [["103.00", "1"]]
/// }"#)?;
///
/// /// We shouldn't apply this update because it's not next in sequence!
/// assert_eq!(diff.sequence().is_next_in_sequence(&snapshot.sequence()), false);
///
/// /// Or if we had received this update:
/// let diff: L2BookUpdate = serde_json::from_str(r#"{
///     "t": "d",
///     "ts": 1729700841,
///     "tn": 0,
///     "sid": 170,
///     "sn": 9001,
///     "b": [],
///     "a": [["103.00", "1"]]
/// }"#)?;
///
/// /// It appears that the sequence id is changed, signalling a new sequence.
/// /// In this case, we should re-request the snapshot from the server.
/// assert_eq!(diff.sequence().is_next_in_sequence(&snapshot.sequence()), false);
///
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "t")]
/// <!-- py: tag=t -->
pub enum L2BookUpdate {
    #[serde(rename = "s")]
    #[schemars(title = "Snapshot|L2BookSnapshot")]
    Snapshot(L2BookSnapshot),
    #[serde(rename = "d")]
    #[schemars(title = "Diff|L2BookDiff")]
    Diff(L2BookDiff),
}

impl L2BookUpdate {
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::Snapshot(snapshot) => snapshot.timestamp(),
            Self::Diff(diff) => diff.timestamp(),
        }
    }

    pub fn sequence(&self) -> SequenceIdAndNumber {
        match self {
            Self::Snapshot(snapshot) => snapshot.sequence,
            Self::Diff(diff) => diff.sequence,
        }
    }

    pub fn is_snapshot(&self) -> bool {
        match self {
            Self::Snapshot(_) => true,
            Self::Diff(_) => false,
        }
    }
}
