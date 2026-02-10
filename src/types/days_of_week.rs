use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Days of week using ISO 8601 numbering (1=Monday, 7=Sunday).
///
/// This type ensures that all day values are in the valid range [1, 7].
/// It is serialized as a JSON array of numbers for API compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(try_from = "Vec<u8>", into = "Vec<u8>")]
pub struct DaysOfWeek(Vec<u8>);

impl DaysOfWeek {
    /// Create a new DaysOfWeek, validating that all days are in range [1, 7].
    ///
    /// # Errors
    /// Returns an error if any day is not in the range 1-7 (ISO 8601).
    pub fn new(days: impl Into<Vec<u8>>) -> Result<Self> {
        let days = days.into();
        for day in &days {
            if !(1..=7).contains(day) {
                bail!(
                    "invalid day of week: {} (must be 1-7, ISO 8601: 1=Monday, 7=Sunday)",
                    day
                );
            }
        }
        Ok(Self(days))
    }

    /// Get the inner Vec<u8> of days.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Check if the given day is in this set of days.
    pub fn contains(&self, day: u8) -> bool {
        self.0.contains(&day)
    }

    /// Monday through Friday (1-5).
    pub fn weekdays() -> Self {
        Self(vec![1, 2, 3, 4, 5])
    }

    /// Saturday and Sunday (6-7).
    pub fn weekends() -> Self {
        Self(vec![6, 7])
    }

    /// All days of the week (1-7).
    pub fn all() -> Self {
        Self(vec![1, 2, 3, 4, 5, 6, 7])
    }
}

impl TryFrom<Vec<u8>> for DaysOfWeek {
    type Error = anyhow::Error;

    fn try_from(days: Vec<u8>) -> Result<Self> {
        Self::new(days)
    }
}

impl From<DaysOfWeek> for Vec<u8> {
    fn from(days: DaysOfWeek) -> Self {
        days.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_of_week_validation_invalid() {
        let result = DaysOfWeek::new(vec![0]);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid day of week: 0"));

        let result = DaysOfWeek::new(vec![8]);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid day of week: 8"));

        let result = DaysOfWeek::new(vec![1, 2, 3, 8]);
        assert!(result.is_err());
    }
}
