//! Authentication Types
//!
//! This module contains strong type wrappers for authentication-related data.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    /// Regex for validating username format - alphanumeric, underscore, @, ., +, -
    static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_@.+\\-]+$").expect("Invalid username regex");

    /// Regex for validating token format - basic alphanumeric and common token characters
    static ref TOKEN_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_\\-]+$").expect("Invalid token regex");
}

/// Strong type for Username with validation
#[derive(
    Default,
    Debug,
    derive_more::Display,
    derive_more::AsRef,
    derive_more::FromStr,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct Username(String);

impl<T: AsRef<str>> PartialEq<T> for Username {
    fn eq(&self, other: &T) -> bool {
        self.0 == other.as_ref()
    }
}

impl Username {
    /// Create a new Username with validation
    pub fn new(username: impl Into<String>) -> Result<Self, String> {
        let username = username.into();

        if username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }

        if username.len() > 50 {
            return Err("Username cannot be longer than 50 characters".to_string());
        }

        // Use compiled regex for validation
        if !USERNAME_REGEX.is_match(&username) {
            return Err("Username contains invalid characters".to_string());
        }

        Ok(Self(username))
    }

    /// Create without validation (for internal use)
    pub fn new_unchecked(username: impl Into<String>) -> Self {
        Self(username.into())
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

impl From<String> for Username {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Username {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Strong type for Password with validation
#[derive(Default, derive_more::FromStr, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Password(String);

impl Password {
    /// Create a new Password with validation
    pub fn new(password: impl Into<String>) -> Result<Self, String> {
        let password = password.into();

        if password.is_empty() {
            return Err("Password cannot be empty".to_string());
        }

        if password.len() < 8 {
            return Err("Password must be at least 8 characters long".to_string());
        }

        if password.len() > 128 {
            return Err("Password cannot be longer than 128 characters".to_string());
        }

        Ok(Self(password))
    }

    /// Create without validation (for internal use)
    pub fn new_unchecked(password: impl Into<String>) -> Self {
        Self(password.into())
    }

    /// Expose the secret password value as a string slice
    /// WARNING: This exposes the sensitive password data - use with caution
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Get the length of the password for validation purposes
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the password is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Password").field(&"[REDACTED]").finish()
    }
}

impl std::fmt::Display for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Strong type for Token with validation
#[derive(
    Default,
    derive_more::From,
    derive_more::FromStr,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Token(String);

impl Token {
    /// Create a new Token with validation
    pub fn new(token: impl Into<String>) -> Result<Self, String> {
        let token = token.into();

        if token.is_empty() {
            return Err("Token cannot be empty".to_string());
        }

        if token.len() < 10 {
            return Err("Token must be at least 10 characters long".to_string());
        }

        if token.len() > 256 {
            return Err("Token cannot be longer than 256 characters".to_string());
        }

        // Use compiled regex for validation
        if !TOKEN_REGEX.is_match(&token) {
            return Err("Token contains invalid characters".to_string());
        }

        Ok(Self(token))
    }

    /// Create without validation (for internal use)
    pub fn new_unchecked(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// Expose the secret token value as a string slice
    /// WARNING: This exposes the sensitive token data - use with caution
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Token {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Token").field(&"[REDACTED]").finish()
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_usernames() {
        let valid_usernames = vec![
            "user123",
            "test_user",
            "user@domain.com",
            "user+tag@domain.com",
            "user.name@domain.com",
            "user-name",
            "user_name",
        ];

        for username in valid_usernames {
            assert!(
                Username::new(username).is_ok(),
                "Username '{}' should be valid",
                username
            );
        }
    }

    #[test]
    fn test_invalid_usernames() {
        let too_long = "a".repeat(51);
        let invalid_usernames = vec![
            "",           // empty
            "user space", // contains space
            "user#",      // contains hash
            "user$",      // contains dollar
            &too_long,    // too long (51 characters)
        ];

        for username in invalid_usernames {
            assert!(
                Username::new(username).is_err(),
                "Username '{}' should be invalid",
                username
            );
        }
    }

    #[test]
    fn test_username_length_limit() {
        // Test exactly 50 characters
        let exactly_50_chars = "a".repeat(50);
        assert!(Username::new(&exactly_50_chars).is_ok());

        // Test 51 characters (should fail)
        let too_long = "a".repeat(51);
        assert!(Username::new(&too_long).is_err());
    }

    #[test]
    fn test_validate_username_function() {
        // Test valid cases
        assert!(Username::new("valid_user").is_ok());
        assert!(Username::new("user@domain.com").is_ok());

        // Test invalid cases
        assert!(Username::new("").is_err());
        assert!(Username::new("user space").is_err());
        assert!(Username::new(&"a".repeat(51)).is_err());
    }
}
