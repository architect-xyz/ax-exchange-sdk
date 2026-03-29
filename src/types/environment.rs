use std::str::FromStr;

use url::Url;

use crate::constants::{DEFAULT_BASE_URL, SANDBOX_BASE_URL};

#[derive(Debug, Clone)]
pub enum Environment {
    Production,
    Sandbox,
    Custom(String),
}
impl Environment {
    pub fn base_url(&self) -> Url {
        match self {
            Environment::Production => Url::parse(DEFAULT_BASE_URL).unwrap(),
            Environment::Sandbox => Url::parse(SANDBOX_BASE_URL).unwrap(),
            Environment::Custom(url) => Url::parse(url).unwrap(),
        }
    }
}
impl FromStr for Environment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "production" | "prod" => Ok(Environment::Production),
            "sandbox" | "test" => Ok(Environment::Sandbox),
            url => Ok(Environment::Custom(url.to_string())), // Accepts any other string as custom URL
        }
    }
}
