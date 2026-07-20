use crate::protocol::api_gateway::*;
use crate::protocol::pagination::TimeseriesPagination;
use crate::protocol::{ErrorResponse, HealthResponse};
use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Utc};
use log::{debug, trace};
use reqwest;
use serde::{Serialize, de::DeserializeOwned};
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

    pub async fn leaderboard(&self, request: LeaderboardRequest) -> Result<LeaderboardResponse> {
        self.request(reqwest::Method::GET, "leaderboard", Some(request), true)
            .await
    }

    // Balance & Transaction endpoints

    /// Balances for the connection's default (primary) account.
    pub async fn get_balances(&self) -> Result<GetBalancesResponse> {
        self.get_balances_inner(None).await
    }

    /// Balances for a specific account the authenticated user is authorized for.
    pub async fn get_balances_for_account(
        &self,
        account_id: impl Into<String>,
    ) -> Result<GetBalancesResponse> {
        self.get_balances_inner(Some(account_id.into())).await
    }

    async fn get_balances_inner(&self, account_id: Option<String>) -> Result<GetBalancesResponse> {
        let query = GetBalancesRequest { account_id };
        self.request(reqwest::Method::GET, "balances", Some(query), true)
            .await
    }

    /// Positions for the connection's default (primary) account.
    pub async fn get_positions(&self) -> Result<GetPositionsResponse> {
        self.get_positions_inner(None).await
    }

    /// Positions for a specific account the authenticated user is authorized for.
    pub async fn get_positions_for_account(
        &self,
        account_id: impl Into<String>,
    ) -> Result<GetPositionsResponse> {
        self.get_positions_inner(Some(account_id.into())).await
    }

    async fn get_positions_inner(
        &self,
        account_id: Option<String>,
    ) -> Result<GetPositionsResponse> {
        let query = GetPositionsRequest { account_id };
        self.request(reqwest::Method::GET, "positions", Some(query), true)
            .await
    }

    /// Transactions for the connection's default (primary) account. An optional
    /// time range may be supplied via `timeseries`; when both `start` and `end`
    /// bounds are given, `end` must be greater than `start`.
    pub async fn get_transactions(
        &self,
        request: GetTransactionsRequest,
        timeseries: TimeseriesPagination,
    ) -> Result<GetTransactionsResponse> {
        self.get_transactions_inner(request, timeseries, None).await
    }

    /// Transactions for a specific account the authenticated user is authorized
    /// for, over the given time range.
    pub async fn get_transactions_for_account(
        &self,
        request: GetTransactionsRequest,
        timeseries: TimeseriesPagination,
        account_id: impl Into<String>,
    ) -> Result<GetTransactionsResponse> {
        self.get_transactions_inner(request, timeseries, Some(account_id.into()))
            .await
    }

    async fn get_transactions_inner(
        &self,
        request: GetTransactionsRequest,
        timeseries: TimeseriesPagination,
        account_id: Option<String>,
    ) -> Result<GetTransactionsResponse> {
        let query = GetTransactionsQueryParams {
            request,
            timeseries,
            account_id,
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

    pub async fn get_tickers_with_params(
        &self,
        request: GetTickersQueryParams,
    ) -> Result<GetTickersResponse> {
        self.request(reqwest::Method::GET, "tickers", Some(request), true)
            .await
    }

    pub async fn get_book(&self, request: GetBookRequest) -> Result<GetBookResponse> {
        self.request(reqwest::Method::GET, "book", Some(request), true)
            .await
    }

    pub async fn get_fills(&self, request: GetFillsRequest) -> Result<GetFillsResponse> {
        self.request(reqwest::Method::GET, "fills", Some(request), true)
            .await
    }

    // Market data endpoints

    /// Ticker for a single symbol.
    pub async fn get_ticker(&self, symbol: &str) -> Result<GetTickerResponse> {
        let query = GetTickerRequest {
            symbol: symbol.to_string(),
        };
        self.request(reqwest::Method::GET, "ticker", Some(query), true)
            .await
    }

    /// Recent trades for a symbol; `limit` defaults server-side (max 100).
    pub async fn get_trades(&self, symbol: &str, limit: Option<u32>) -> Result<GetTradesResponse> {
        let query = GetTradesRequest {
            symbol: symbol.to_string(),
            limit,
        };
        self.request(reqwest::Method::GET, "trades", Some(query), true)
            .await
    }

    /// Historical candles for a symbol over a time range.
    pub async fn get_candles(&self, request: GetCandlesRequest) -> Result<GetCandlesResponse> {
        self.request(reqwest::Method::GET, "candles", Some(request), true)
            .await
    }

    /// The last completed candle for a symbol at the given width.
    pub async fn get_last_candle(
        &self,
        symbol: &str,
        candle_width: &str,
    ) -> Result<GetCandleResponse> {
        let query = GetCandleRequest {
            symbol: symbol.to_string(),
            candle_width: candle_width.to_string(),
        };
        self.request(reqwest::Method::GET, "candles/last", Some(query), true)
            .await
    }

    /// The current (in-progress) candle for a symbol at the given width.
    pub async fn get_current_candle(
        &self,
        symbol: &str,
        candle_width: &str,
    ) -> Result<GetCandleResponse> {
        let query = GetCandleRequest {
            symbol: symbol.to_string(),
            candle_width: candle_width.to_string(),
        };
        self.request(reqwest::Method::GET, "candles/current", Some(query), true)
            .await
    }

    /// Historical best-bid/offer candles for a symbol over a time range.
    pub async fn get_bbo_candles(
        &self,
        request: GetBboCandlesRequest,
    ) -> Result<GetBboCandlesResponse> {
        self.request(reqwest::Method::GET, "bbo-candles", Some(request), true)
            .await
    }

    /// The last completed BBO candle for a symbol at the given width.
    pub async fn get_last_bbo_candle(
        &self,
        symbol: &str,
        candle_width: &str,
    ) -> Result<GetBboCandleResponse> {
        let query = GetBboCandleRequest {
            symbol: symbol.to_string(),
            candle_width: candle_width.to_string(),
        };
        self.request(reqwest::Method::GET, "bbo-candles/last", Some(query), true)
            .await
    }

    /// The current (in-progress) BBO candle for a symbol at the given width.
    pub async fn get_current_bbo_candle(
        &self,
        symbol: &str,
        candle_width: &str,
    ) -> Result<GetBboCandleResponse> {
        let query = GetBboCandleRequest {
            symbol: symbol.to_string(),
            candle_width: candle_width.to_string(),
        };
        self.request(
            reqwest::Method::GET,
            "bbo-candles/current",
            Some(query),
            true,
        )
        .await
    }

    /// Historical funding rates for a symbol over a time range.
    pub async fn get_funding_rates(
        &self,
        request: GetFundingRatesRequest,
    ) -> Result<GetFundingRatesResponse> {
        self.request(reqwest::Method::GET, "funding-rates", Some(request), true)
            .await
    }

    /// Live estimated funding rate for a symbol.
    pub async fn get_estimated_funding_rate(
        &self,
        symbol: &str,
    ) -> Result<GetEstimatedFundingRateResponse> {
        let query = GetEstimatedFundingRateRequest {
            symbol: symbol.to_string(),
        };
        self.request(
            reqwest::Method::GET,
            "estimated-funding-rate",
            Some(query),
            true,
        )
        .await
    }

    /// Funding slots for a symbol on a given trading date.
    pub async fn get_funding_slots(
        &self,
        request: GetFundingSlotsRequest,
    ) -> Result<GetFundingSlotsResponse> {
        self.request(reqwest::Method::GET, "funding-slots", Some(request), true)
            .await
    }

    /// Account equity history over a time range at the given resolution.
    pub async fn get_account_equity_history(
        &self,
        request: GetAccountEquityHistoryRequest,
    ) -> Result<GetAccountEquityHistoryResponse> {
        self.request(
            reqwest::Method::GET,
            "account-equity-history",
            Some(request),
            true,
        )
        .await
    }

    /// Traded volume over a time range for the user (optionally a specific
    /// account).
    pub async fn get_volume(&self, request: GetVolumeRequest) -> Result<GetVolumeResponse> {
        self.request(
            reqwest::Method::GET,
            "user/stats/volume",
            Some(request),
            true,
        )
        .await
    }

    /// Historical underlying prices for a symbol over a time range.
    pub async fn get_underlying_prices(
        &self,
        request: GetUnderlyingPricesRequest,
    ) -> Result<GetUnderlyingPricesResponse> {
        self.request(
            reqwest::Method::GET,
            "underlying-prices",
            Some(request),
            true,
        )
        .await
    }

    /// Risk snapshot for the connection's default (primary) account.
    pub async fn get_risk_snapshot(&self) -> Result<GetRiskSnapshotResponse> {
        self.get_risk_snapshot_inner(None).await
    }

    /// Risk snapshot for a specific account the authenticated user is authorized for.
    pub async fn get_risk_snapshot_for_account(
        &self,
        account_id: impl Into<String>,
    ) -> Result<GetRiskSnapshotResponse> {
        self.get_risk_snapshot_inner(Some(account_id.into())).await
    }

    async fn get_risk_snapshot_inner(
        &self,
        account_id: Option<String>,
    ) -> Result<GetRiskSnapshotResponse> {
        let query = GetRiskSnapshotRequest { account_id };
        self.request(reqwest::Method::GET, "risk-snapshot", Some(query), true)
            .await
    }
}
