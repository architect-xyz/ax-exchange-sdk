use std::time::Duration;
/// Default base URL for the Architect production environment.
pub const DEFAULT_BASE_URL: &str = "https://gateway.architect.exchange";

/// Base URL for the Architect sandbox environment.
pub const SANDBOX_BASE_URL: &str = "https://gateway.sandbox.architect.exchange";

pub const READ_TIMEOUT: Duration = Duration::from_secs(6);
