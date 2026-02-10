//! Order ID Type
//!
//! This module contains the OrderId newtype for type safety.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;

const REGULAR_PREFIX: &str = "O-";
const LIQUIDATION_PREFIX: &str = "L-";

/// Strong type for Order IDs to prevent mixing with other string values
///
/// Order IDs are ULIDs with a prefix:
///
/// - Regular orders: O-<ULID>
/// - Liquidation orders: L-<ULID>
#[derive(
    Debug,
    derive_more::Display,
    derive_more::AsRef,
    derive_more::Into,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct OrderId(String);

impl OrderId {
    /// Create a new OrderId from a string with validation
    pub fn new(id: impl Into<String>) -> Result<Self> {
        let t = Self::new_unchecked(id);
        t.validate()?;
        Ok(t)
    }

    /// Create a new OrderId from a string without validation (use with caution)
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn validate(&self) -> Result<()> {
        if !self.0.starts_with(REGULAR_PREFIX) && !self.0.starts_with(LIQUIDATION_PREFIX) {
            bail!("invalid order ID format");
        }
        // INVARIANT: we have either "O-" or "L-" prefix
        if self
            .0
            .strip_prefix(REGULAR_PREFIX)
            .or_else(|| self.0.strip_prefix(LIQUIDATION_PREFIX))
            .is_none_or(|ulid| ulid::Ulid::from_string(ulid).is_err())
        {
            bail!("invalid ULID in order ID");
        }
        Ok(())
    }

    /// Generate a new regular order ID (O-<ULID>)
    pub fn generate_regular() -> Self {
        let ulid = ulid::Ulid::new();
        Self(format!("{}{}", REGULAR_PREFIX, ulid))
    }

    /// Generate a new liquidation order ID (L-<ULID>)
    pub fn generate_liquidation() -> Self {
        let ulid = ulid::Ulid::new();
        Self(format!("{}{}", LIQUIDATION_PREFIX, ulid))
    }

    /// Generate a new order ID based on the liquidation flag
    pub fn generate(is_liquidation: bool) -> Self {
        if is_liquidation {
            Self::generate_liquidation()
        } else {
            Self::generate_regular()
        }
    }

    /// Check if this is a regular order ID (O- prefix)
    pub fn is_regular(&self) -> bool {
        self.0.starts_with(REGULAR_PREFIX)
    }

    /// Check if this is a liquidation order ID (L- prefix)
    pub fn is_liquidation(&self) -> bool {
        self.0.starts_with(LIQUIDATION_PREFIX)
    }

    /// Extract the ULID from a validated OrderId.
    pub fn ulid(&self) -> Result<ulid::Ulid> {
        let raw = self
            .0
            .strip_prefix(REGULAR_PREFIX)
            .or_else(|| self.0.strip_prefix(LIQUIDATION_PREFIX))
            .ok_or_else(|| anyhow!("invalid order ID format"))?;
        ulid::Ulid::from_string(raw).map_err(|e| anyhow!("invalid ULID: {e}"))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for OrderId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for OrderId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_regular() {
        let order_id = OrderId::generate_regular();
        assert!(order_id.is_regular());
        assert!(!order_id.is_liquidation());
        assert!(order_id.as_str().starts_with("O-"));
    }

    #[test]
    fn test_generate_liquidation() {
        let order_id = OrderId::generate_liquidation();
        assert!(order_id.is_liquidation());
        assert!(!order_id.is_regular());
        assert!(order_id.as_str().starts_with("L-"));
    }

    #[test]
    fn test_generate_with_flag() {
        let regular = OrderId::generate(false);
        assert!(regular.is_regular());

        let liquidation = OrderId::generate(true);
        assert!(liquidation.is_liquidation());
    }

    #[test]
    fn test_validation() {
        // Valid regular order ID
        let regular_ulid = ulid::Ulid::new();
        let regular_id = format!("O-{}", regular_ulid);
        let order_id = OrderId::new(regular_id.clone()).unwrap();
        assert_eq!(order_id.as_str(), regular_id);
        assert!(order_id.is_regular());

        // Valid liquidation order ID
        let liq_ulid = ulid::Ulid::new();
        let liq_id = format!("L-{}", liq_ulid);
        let order_id = OrderId::new(liq_id.clone()).unwrap();
        assert_eq!(order_id.as_str(), liq_id);
        assert!(order_id.is_liquidation());

        // Invalid prefix
        assert!(OrderId::new("X-01234567890123456789012345").is_err());

        // Invalid ULID
        assert!(OrderId::new("O-invalid").is_err());
        assert!(OrderId::new("L-invalid").is_err());

        // Missing prefix
        assert!(OrderId::new("01234567890123456789012345").is_err());
    }

    #[test]
    fn test_serde() {
        let order_id = OrderId::new("O-01KA7S36VM6HBEEAE3EN9ZRHEA").unwrap();
        let json = serde_json::to_string(&order_id).unwrap();
        assert_eq!(json, r#""O-01KA7S36VM6HBEEAE3EN9ZRHEA""#);
        let order_id2: OrderId = serde_json::from_str(&json).unwrap();
        assert_eq!(order_id, order_id2);
    }
}
