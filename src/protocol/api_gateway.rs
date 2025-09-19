use crate::Token;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteUserResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub username: String,
    pub password: String,
    /// Optional 2FA code, if 2FA is enabled/required for the user.
    pub totp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetApiKeysRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetApiKeysResponse {
    pub api_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeApiKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeApiKeyResponse {
    pub message: String,
}

/// Exchange credentials for a bearer token.
///
/// Must provide exactly one of:
///
/// - `username` + `password`
/// - `api_key` + `secret`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetUserTokenRequest {
    #[serde(flatten)]
    pub auth: GetUserTokenAuthMethod,
    pub expiration_seconds: i32,
    /// Optional 2FA code, if 2FA is enabled/required for the user.
    pub totp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetUserTokenAuthMethod {
    UsernamePassword { username: String, password: String },
    ApiKeySecret { api_key: String, secret: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserTokenResponse {
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeUserTokenRequest {
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeUserTokenResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoAmIResponse {
    pub username: String,
    pub enabled_2fa: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeTokenRequest {
    pub username: String,
    pub token: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeTokenResponse {
    pub username: String,
    pub ep3_username: String,
    pub ep3_account: String,
    pub is_admin_token: bool,
    pub can_place_orders: bool,
    pub enabled_2fa: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUsersResponse {
    pub users: Vec<GetUserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUserResponse {
    pub id: Uuid,
    pub username: String,
    // CR alee: consider whether to remove these fields
    pub ep3_username: String,
    pub ep3_account: String,
    /// NB: will be deprecated soon; use is_onboarded, is_close_only, is_frozen instead
    pub is_valid: bool,
    pub is_onboarded: bool,
    pub is_close_only: bool,
    pub is_frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstrumentResponse {
    pub symbol: String,
    pub tick_size: String,
    pub base_currency: String,
    pub multiplier: i32,
    pub minimum_trade_quantity: i32,
    pub description: String,
    pub product_id: String,
    pub state: String,
    pub price_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstrumentsResponse {
    pub instruments: Vec<GetInstrumentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setup2faResponse {
    pub validate_token: String,
    /// The `uri` field contains a provisioning URI following the
    /// Google Authenticator format:
    ///
    /// `otpauth://totp/ADX:username?secret=BASE32SECRET&issuer=ADX&algorithm=SHA1&digits=6&period=30`
    ///
    /// This URI encodes all TOTP parameters and is typically displayed
    /// as a QR code for client apps to scan.
    pub uri: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confirm2faRequest {
    pub validate_token: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confirm2faResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disable2faResponse {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_token_request_serde() {
        let json = r#"
        {
            "username": "testuser",
            "password": "password",
            "expiration_seconds": 3600
        }
        "#;
        let req: GetUserTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req,
            GetUserTokenRequest {
                auth: GetUserTokenAuthMethod::UsernamePassword {
                    username: "testuser".to_string(),
                    password: "password".to_string()
                },
                expiration_seconds: 3600,
                totp: None
            }
        );

        let json = r#"
        {
            "api_key": "testapikey",
            "secret": "testsecret",
            "expiration_seconds": 3600
        }
        "#;
        let req: GetUserTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req,
            GetUserTokenRequest {
                auth: GetUserTokenAuthMethod::ApiKeySecret {
                    api_key: "testapikey".to_string(),
                    secret: "testsecret".to_string()
                },
                expiration_seconds: 3600,
                totp: None
            }
        );
    }
}
