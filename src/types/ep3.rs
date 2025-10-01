//! EP3 Types
//!
//! This module contains strong type wrappers for EP3-specific data.

// TODO: don't expose EP3 types to the public

use serde::{Deserialize, Serialize};
use std::fmt;

/// Strong type for EP3 Account to prevent mixing with other string values
#[derive(
    Default,
    Debug,
    derive_more::Display,
    derive_more::AsRef,
    derive_more::From,
    derive_more::FromStr,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
#[as_ref(forward)]
pub struct Ep3Account(String);

impl Ep3Account {
    /// Create a new Ep3Account from a string
    pub fn new(account: impl Into<String>) -> Self {
        Self(account.into())
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

impl From<&str> for Ep3Account {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// EP3 Username that can handle both structured and simple formats
/// This is the main EP3 username type used throughout the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Ep3Username {
    /// Username with firm and user components (e.g., firms/CHI/users/ADX.USER.971)
    WithFirm {
        firm_id: String, // e.g., "CHI"
        user_id: String, // e.g., "ADX.USER.971"
    },
    /// Simple username without firm (e.g., "admin")
    NoFirm(String),
}

impl Ep3Username {
    /// Create username with firm from components (alias for with_firm for compatibility)
    pub fn new(firm_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self::with_firm(firm_id, user_id)
    }

    /// Create username with firm from components
    pub fn with_firm(firm_id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self::WithFirm {
            firm_id: firm_id.into(),
            user_id: user_id.into(),
        }
    }

    /// Create simple username without firm
    pub fn simple(username: impl Into<String>) -> Self {
        Self::NoFirm(username.into())
    }

    /// Create from full EP3 username path or simple username
    pub fn from_full_path(full_path: impl AsRef<str>) -> Result<Self, String> {
        let path = full_path.as_ref();
        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() == 4 && parts[0] == "firms" && parts[2] == "users" {
            // WithFirm format: firms/CHI/users/ADX.USER.971
            Ok(Self::new(parts[1], parts[3]))
        } else if parts.len() == 1 {
            // NoFirm format: admin
            Ok(Self::simple(path))
        } else {
            Err(format!("Invalid EP3 username format: {}", path))
        }
    }

    /// Get the firm ID (if WithFirm, or None for NoFirm)
    pub fn firm_id(&self) -> Option<&str> {
        match self {
            Self::WithFirm { firm_id, .. } => Some(firm_id),
            Self::NoFirm(_) => None,
        }
    }

    /// Get the user ID (if WithFirm) or the simple username (if NoFirm)
    pub fn user_id(&self) -> &str {
        match self {
            Self::WithFirm { user_id, .. } => user_id,
            Self::NoFirm(username) => username,
        }
    }

    // TODO: dogshit method; pull default into config/env then feed the arg.
    /// Extract firm as Ep3Firm (uses default firm for NoFirm usernames)
    pub fn extract_firm(&self) -> Ep3Firm {
        match self {
            Self::WithFirm { firm_id, .. } => Ep3Firm::new(firm_id.clone()),
            Self::NoFirm(_) => {
                // For NoFirm usernames like "admin", use a default firm or extract from environment
                let default_firm =
                    std::env::var("EP3_PARTICIPANT_FIRM_ID").unwrap_or_else(|_| "CHI".to_string());
                Ep3Firm::new(default_firm)
            }
        }
    }

    /// Check if this username has firm information
    pub fn has_firm(&self) -> bool {
        matches!(self, Self::WithFirm { .. })
    }

    /// Check if this is a simple username without firm
    pub fn is_simple(&self) -> bool {
        matches!(self, Self::NoFirm(_))
    }
}

impl fmt::Display for Ep3Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WithFirm { firm_id, user_id } => write!(f, "firms/{firm_id}/users/{user_id}"),
            Self::NoFirm(username) => write!(f, "{username}"),
        }
    }
}

impl From<String> for Ep3Username {
    fn from(s: String) -> Self {
        Self::from_full_path(s).expect("Invalid EP3 username format")
    }
}

impl From<&str> for Ep3Username {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

/// EP3 firm names; format is always firms/<firm_id>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ep3Firm {
    pub firm_id: String, // e.g., "CHI"
}

impl Ep3Firm {
    /// Create from firm ID (e.g., "CHI")
    pub fn new(firm_id: impl Into<String>) -> Self {
        Self {
            firm_id: firm_id.into(),
        }
    }
}

impl fmt::Display for Ep3Firm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "firms/{}", self.firm_id)
    }
}

impl std::str::FromStr for Ep3Firm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let just_the_firm_id = s.trim_start_matches("firms/");
        if just_the_firm_id.is_empty() {
            Err(anyhow::anyhow!("invalid firm ID"))
        } else {
            Ok(Self::new(just_the_firm_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_firm_from_ep3_username() {
        let ep3_username = Ep3Username::from("firms/CHI/users/ADX.DEMO.01K6B0ANZY2W4ZBMM4RFJTFTCF");
        let firm = ep3_username.extract_firm();
        let firm_chi = Ep3Firm::new("CHI");
        assert_eq!(firm, firm_chi);
    }
}
