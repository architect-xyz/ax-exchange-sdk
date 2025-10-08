use crate::protocol::api_gateway::*;
use crate::protocol::{ErrorResponse, HealthResponse};
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use log::{debug, trace};
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
        params: Option<T>,
        auth: bool,
    ) -> Result<R> {
        let url = self.base_url.join(path)?;
        debug!("=> {} {}", method, url);

        let mut req = self
            .client
            .request(method.clone(), url.clone())
            .header("Content-Type", "application/json");

        if auth {
            let token = self.token()?;
            req = req.header("Authorization", format!("{}", token));
        }

        if let Some(params) = params {
            if method == reqwest::Method::POST
                || method == reqwest::Method::PUT
                || method == reqwest::Method::PATCH
            {
                req = req.json(&params);
            } else {
                req = req.query(&params);
            }
        }

        let res = req.send().await?;
        let res_status = res.status();
        let res_text = res.text().await?;
        trace!("<= {method} {url}: {res_status}");
        trace!("<= {res_text}");

        if res_status.is_success() {
            Ok(serde_json::from_str(&res_text)?)
        } else {
            match serde_json::from_str::<ErrorResponse>(&res_text) {
                Ok(error_response) => Err(anyhow!(error_response.error)),
                Err(e) => Err(anyhow!("while parsing error response: {e:?}")),
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

    pub async fn update_user_status(
        &self,
        username: &str,
        request: UpdateUserStatusRequest,
    ) -> Result<GetUserResponse> {
        let path = format!("users/{}/status", username);
        self.request(reqwest::Method::PUT, &path, Some(request), true)
            .await
    }

    pub async fn get_user_risk_profiles(&self) -> Result<GetUserRiskProfilesResponse> {
        self.request::<(), GetUserRiskProfilesResponse>(
            reqwest::Method::GET,
            "user_risk_profiles",
            None,
            true,
        )
        .await
    }

    pub async fn get_user_risk_profile(
        &self,
        username: &str,
    ) -> Result<GetUserRiskProfileResponse> {
        let path = format!("user_risk_profiles/{}", username);
        self.request::<(), GetUserRiskProfileResponse>(reqwest::Method::GET, &path, None, true)
            .await
    }

    // Balance & Transaction endpoints

    pub async fn get_balances(&self) -> Result<GetBalancesResponse> {
        self.request::<(), GetBalancesResponse>(reqwest::Method::GET, "balances", None, true)
            .await
    }

    pub async fn get_positions(&self) -> Result<GetPositionsResponse> {
        self.request::<(), GetPositionsResponse>(reqwest::Method::GET, "positions", None, true)
            .await
    }

    pub async fn get_transactions(
        &self,
        request: GetTransactionsRequest,
    ) -> Result<GetTransactionsResponse> {
        self.request(reqwest::Method::GET, "transactions", Some(request), true)
            .await
    }

    pub async fn sandbox_deposit(
        &self,
        request: SandboxDepositRequest,
    ) -> Result<GetBalancesResponse> {
        self.request(
            reqwest::Method::POST,
            "sandbox/deposit",
            Some(request),
            true,
        )
        .await
    }

    pub async fn sandbox_withdrawal(
        &self,
        request: SandboxWithdrawalRequest,
    ) -> Result<GetBalancesResponse> {
        self.request(
            reqwest::Method::POST,
            "sandbox/withdrawal",
            Some(request),
            true,
        )
        .await
    }
}
