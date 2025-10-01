use crate::protocol::api_gateway::*;
use crate::protocol::{ErrorResponse, HealthResponse};
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use log::{debug, error};
use reqwest;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use url::Url;

pub struct ApiGatewayRestClient {
    client: reqwest::Client,
    base_url: Url,
    token: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
}

impl ApiGatewayRestClient {
    pub fn new(base_url: Url) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            base_url,
            token: None,
            token_expires_at: None,
        })
    }

    /// Set the authentication token and its expiration time
    pub fn set_token(&mut self, token: String, expires_at: DateTime<Utc>) {
        self.token = Some(token);
        self.token_expires_at = Some(expires_at);
    }

    /// Helper method to get current token
    fn token(&self) -> Result<&str> {
        if let Some(token) = &self.token {
            if self.token_expires_at.is_some_and(|exp| Utc::now() > exp) {
                bail!("token expired")
            }
            return Ok(token);
        } else {
            bail!("token not available")
        }
    }

    /// Helper method to make HTTP requests with optional authentication
    async fn request<T: Serialize, R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<T>,
        auth: bool,
    ) -> Result<R> {
        let url = self.base_url.join(path)?;
        debug!("{} {}", method, url);

        let mut request = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json");

        if auth {
            let token = self.token()?;
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            match response.json::<ErrorResponse>().await {
                Ok(error_response) => {
                    error!("API error: {}", error_response.error);
                    Err(anyhow!(error_response.error))
                }
                Err(e) => Err(anyhow!("failed to parse error response: {:?}", e)),
            }
        }
    }

    // Public endpoints (no auth required)

    pub async fn health(&self) -> Result<HealthResponse> {
        self.request::<(), HealthResponse>(reqwest::Method::GET, "health", None, false)
            .await
    }

    pub async fn get_user_token(
        &self,
        request: GetUserTokenRequest,
    ) -> Result<GetUserTokenResponse> {
        self.request(
            reqwest::Method::POST,
            "get_user_token",
            Some(request),
            false,
        )
        .await
    }

    pub async fn get_instruments(&self) -> Result<GetInstrumentsResponse> {
        self.request::<(), GetInstrumentsResponse>(reqwest::Method::GET, "instruments", None, false)
            .await
    }

    pub async fn get_instrument(&self, symbol: &str) -> Result<GetInstrumentResponse> {
        let path = format!("instruments/{}", symbol);
        self.request::<(), GetInstrumentResponse>(reqwest::Method::GET, &path, None, false)
            .await
    }

    // Authenticated endpoints

    pub async fn change_password(
        &self,
        request: ChangePasswordRequest,
    ) -> Result<ChangePasswordResponse> {
        self.request(
            reqwest::Method::POST,
            "change_password",
            Some(request),
            true,
        )
        .await
    }

    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        self.request(reqwest::Method::POST, "create_api_key", Some(request), true)
            .await
    }

    pub async fn get_api_keys(&self, request: GetApiKeysRequest) -> Result<GetApiKeysResponse> {
        self.request(reqwest::Method::POST, "get_api_keys", Some(request), true)
            .await
    }

    pub async fn revoke_api_key(
        &self,
        request: RevokeApiKeyRequest,
    ) -> Result<RevokeApiKeyResponse> {
        self.request(reqwest::Method::POST, "revoke_api_key", Some(request), true)
            .await
    }

    pub async fn whoami(&self) -> Result<WhoAmIResponse> {
        self.request::<(), WhoAmIResponse>(reqwest::Method::GET, "whoami", None, true)
            .await
    }

    pub async fn setup_2fa(&self) -> Result<Setup2faResponse> {
        self.request::<(), Setup2faResponse>(reqwest::Method::POST, "setup_2fa", None, true)
            .await
    }

    pub async fn confirm_2fa(&self, request: Confirm2faRequest) -> Result<Confirm2faResponse> {
        self.request(reqwest::Method::POST, "confirm_2fa", Some(request), true)
            .await
    }

    pub async fn disable_2fa(&self) -> Result<Disable2faResponse> {
        self.request::<(), Disable2faResponse>(reqwest::Method::POST, "disable_2fa", None, true)
            .await
    }

    // Admin endpoints

    pub async fn create_user(&self, request: CreateUserRequest) -> Result<CreateUserResponse> {
        self.request(reqwest::Method::POST, "create_user", Some(request), true)
            .await
    }

    pub async fn get_users(&self) -> Result<GetUsersResponse> {
        self.request::<(), GetUsersResponse>(reqwest::Method::GET, "users", None, true)
            .await
    }

    pub async fn get_user(&self, username: &str) -> Result<GetUserResponse> {
        let path = format!("users/{}", username);
        self.request::<(), GetUserResponse>(reqwest::Method::GET, &path, None, true)
            .await
    }

    pub async fn revoke_user_token(
        &self,
        request: RevokeUserTokenRequest,
    ) -> Result<RevokeUserTokenResponse> {
        self.request(
            reqwest::Method::POST,
            "revoke_user_token",
            Some(request),
            true,
        )
        .await
    }

    pub async fn decode_token(&self, request: DecodeTokenRequest) -> Result<DecodeTokenResponse> {
        self.request(reqwest::Method::POST, "decode_token", Some(request), true)
            .await
    }
}
