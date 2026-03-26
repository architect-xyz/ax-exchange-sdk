use crate::protocol::api_gateway::*;
use crate::protocol::pagination::TimeseriesPagination;
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

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Set the authentication token and its expiration time
    pub fn set_token(&mut self, token: String, expires_at: DateTime<Utc>) {
        self.token = Some(token);
        self.token_expires_at = Some(expires_at);
    }

    /// Get the current authentication token, if valid
    pub fn token(&self) -> Result<&str> {
        if let Some(token) = &self.token {
            if self.token_expires_at.is_some_and(|exp| Utc::now() > exp) {
                bail!("token expired")
            }
            Ok(token)
        } else {
            bail!("token not available")
        }
    }

    /// Helper method to make HTTP requests with optional authentication
    pub async fn request<T: Serialize, R: DeserializeOwned>(
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
            req = req.header("Authorization", token.to_string());
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
            log::error!("error: {} {} returned {}", method, url, res_status);
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

    pub async fn authenticate(&self, request: AuthenticateRequest) -> Result<AuthenticateResponse> {
        self.request(reqwest::Method::POST, "authenticate", Some(request), false)
            .await
    }

    pub async fn get_instruments(&self) -> Result<GetInstrumentsResponse> {
        self.request::<(), GetInstrumentsResponse>(reqwest::Method::GET, "instruments", None, false)
            .await
    }

    pub async fn get_instrument(&self, symbol: &str) -> Result<GetInstrumentResponse> {
        let path = format!("instrument?symbol={}", symbol);
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
            "change-password",
            Some(request),
            true,
        )
        .await
    }

    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        self.request(reqwest::Method::POST, "api-keys", Some(request), true)
            .await
    }

    pub async fn get_api_keys(&self) -> Result<GetApiKeysResponse> {
        self.request::<(), GetApiKeysResponse>(reqwest::Method::GET, "api-keys", None, true)
            .await
    }

    pub async fn revoke_api_key(
        &self,
        request: RevokeApiKeyRequest,
    ) -> Result<RevokeApiKeyResponse> {
        self.request(reqwest::Method::DELETE, "api-keys", Some(request), true)
            .await
    }

    pub async fn whoami(&self) -> Result<WhoAmIResponse> {
        self.request::<(), WhoAmIResponse>(reqwest::Method::GET, "whoami", None, true)
            .await
    }

    pub async fn setup_2fa(&self) -> Result<Setup2faResponse> {
        self.request::<(), Setup2faResponse>(reqwest::Method::POST, "mfa/setup", None, true)
            .await
    }

    pub async fn confirm_2fa(&self, request: Confirm2faRequest) -> Result<Confirm2faResponse> {
        self.request(reqwest::Method::POST, "mfa/confirm", Some(request), true)
            .await
    }

    pub async fn disable_2fa(&self) -> Result<Disable2faResponse> {
        self.request::<(), Disable2faResponse>(reqwest::Method::POST, "mfa/disable", None, true)
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
        let query = GetTransactionsQueryParams {
            request,
            timeseries: TimeseriesPagination::default(),
        };
        self.request(reqwest::Method::GET, "transactions", Some(query), true)
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
            "sandbox/withdraw",
            Some(request),
            true,
        )
        .await
    }

    pub async fn get_tickers(&self) -> Result<GetTickersResponse> {
        self.request::<(), GetTickersResponse>(reqwest::Method::GET, "tickers", None, true)
            .await
    }

    pub async fn get_book(&self, request: GetBookRequest) -> Result<GetBookResponse> {
        self.request(reqwest::Method::GET, "book", Some(request), true)
            .await
    }

    pub async fn get_fills(&self) -> Result<GetFillsResponse> {
        self.request::<(), GetFillsResponse>(reqwest::Method::GET, "fills", None, true)
            .await
    }

    pub async fn get_risk_snapshot(&self) -> Result<GetRiskSnapshotResponse> {
        self.request::<(), GetRiskSnapshotResponse>(
            reqwest::Method::GET,
            "risk-snapshot",
            None,
            true,
        )
        .await
    }
}
