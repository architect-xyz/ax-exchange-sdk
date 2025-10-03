use crate::api_gateway::ApiGatewayRestClient;
use crate::marketdata::MarketdataWsClient;
use crate::order_gateway::*;
use crate::{protocol, types::*};
use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwapOption;
use arcstr::ArcStr;
use chrono::{DateTime, Utc};
use log::{debug, warn};
use reqwest;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use url::Url;

#[derive(Clone)]
pub struct ArchitectX {
    base_url: Url,
    api_gateway_base_url: Url,
    order_gateway_base_url: Url,
    username: Option<String>,
    password: Option<String>,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl ArchitectX {
    // CR alee: deprecate username/password arguments
    pub fn new(
        base_url: Url,
        username: Option<impl AsRef<str>>,
        password: Option<impl AsRef<str>>,
    ) -> Result<Self> {
        Ok(Self {
            base_url: base_url.clone(),
            api_gateway_base_url: base_url.join("api/")?,
            order_gateway_base_url: base_url.join("orders/")?,
            username: username.map(|u| u.as_ref().to_string()),
            password: password.map(|p| p.as_ref().to_string()),
            user_token: Arc::new(ArcSwapOption::const_empty()),
        })
    }

    pub fn set_api_gateway_base_url(&mut self, base_url: Url) {
        self.api_gateway_base_url = base_url;
    }

    pub fn set_order_gateway_base_url(&mut self, base_url: Url) {
        self.order_gateway_base_url = base_url;
    }

    fn username(&self) -> Result<String> {
        self.username
            .as_ref()
            .ok_or_else(|| anyhow!("no username provided"))
            .cloned()
    }

    /// Login with username and password.  If the account has 2FA enabled,
    /// you will also need to provide a TOTP code.
    ///
    /// This method currently exchanges the username and password for a
    /// user token directly.
    pub async fn login(
        &self,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        totp: Option<impl AsRef<str>>,
    ) -> Result<ArcStr> {
        use crate::protocol::api_gateway::{GetUserTokenAuthMethod, GetUserTokenRequest};
        let client = ApiGatewayRestClient::new(self.api_gateway_base_url.clone())?;
        let res = client
            .get_user_token(GetUserTokenRequest {
                auth: GetUserTokenAuthMethod::UsernamePassword {
                    username: username.as_ref().to_string(),
                    password: password.as_ref().to_string(),
                },
                expiration_seconds: 3600,
                totp: totp.map(|t| t.as_ref().to_string()),
            })
            .await?;
        let token: ArcStr = res.token.expose_secret().to_string().into();
        let expires = Utc::now() + chrono::Duration::seconds(3300);
        self.user_token
            .store(Some(Arc::new((token.clone(), expires))));
        Ok(token)
    }

    pub async fn refresh_user_token(&self, force: bool) -> Result<ArcStr> {
        let now = Utc::now();
        let token = self.user_token.load();
        if let Some(stored) = &*token {
            let (token, expires_at) = &**stored;
            if !force && *expires_at > now {
                return Ok(token.clone());
            }
        }
        let username = self
            .username
            .as_ref()
            .ok_or_else(|| anyhow!("no username provided"))?;
        let password = self
            .password
            .as_ref()
            .ok_or_else(|| anyhow!("no password provided"))?;
        self.login(username, password, None::<&str>).await
    }

    pub fn api_gateway(&self) -> Result<ApiGatewayRestClient> {
        let mut client = ApiGatewayRestClient::new(self.api_gateway_base_url.clone())?;
        let auth = self.user_token.load();
        if let Some(token) = &*auth {
            let (token, expires_at) = &**token;
            if *expires_at > Utc::now() {
                client.set_token(token.as_str().to_string(), expires_at.clone());
            } else {
                warn!("while creating api gateway client: token expired");
            }
        }
        Ok(client)
    }

    pub fn order_gateway(&self) -> Result<OrderGatewayRestClient> {
        let mut client = OrderGatewayRestClient::new(self.order_gateway_base_url.clone())?;
        let auth = self.user_token.load();
        if let Some(token) = &*auth {
            let (token, expires_at) = &**token;
            if *expires_at > Utc::now() {
                client.set_token(token.as_str().to_string(), expires_at.clone());
            } else {
                warn!("while creating order gateway client: token expired");
            }
        }
        Ok(client)
    }

    pub async fn order_gateway_ws(&self) -> Result<OrderGatewayWsClient> {
        let token = self.refresh_user_token(false).await?;
        OrderGatewayWsClient::connect(self.base_url.clone(), token).await
    }

    pub async fn marketdata_ws(&self) -> Result<MarketdataWsClient> {
        let username = self.username()?;
        let token = self.refresh_user_token(false).await?;
        MarketdataWsClient::connect(self.base_url.clone(), username, token).await
    }

    /// Get risk manager client
    pub async fn risk_manager_client(&self) -> Result<RiskManagerClient> {
        Ok(RiskManagerClient::new(
            self.base_url.clone(),
            self.user_token.clone(),
        ))
    }

    /// Get settlement gateway client
    pub async fn settlement_gateway_client(&self) -> Result<SettlementGatewayClient> {
        Ok(SettlementGatewayClient::new(
            self.base_url.clone(),
            self.user_token.clone(),
        ))
    }

    /// Get candle server client
    pub async fn candle_server_client(&self) -> Result<CandleServerClient> {
        Ok(CandleServerClient::new(
            self.base_url.clone(),
            self.user_token.clone(),
        ))
    }
}

/// Risk Manager Client for risk snapshots and management
pub struct RiskManagerClient {
    base_url: Url,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl RiskManagerClient {
    pub fn new(base_url: Url, user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>) -> Self {
        Self {
            base_url,
            user_token,
        }
    }

    /// Helper method to get current token
    async fn get_token(&self) -> Result<ArcStr> {
        let token = self.user_token.load();
        if let Some(stored) = &*token {
            let (token, expires_at) = &**stored;
            let now = Utc::now();
            if *expires_at > now {
                return Ok(token.clone());
            }
        }
        bail!("Token expired or not available")
    }

    /// Helper method to make authenticated HTTP requests
    async fn make_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response> {
        let url = self.base_url.join(path)?;
        debug!("{} {}", method, url);

        let token = self.get_token().await?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let mut request = client
            .request(method, url)
            .header("Authorization", token.as_str())
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        Ok(response)
    }

    /// Get risk snapshot for a specific user
    pub async fn get_risk_snapshot(&self, username: &str) -> Result<RiskSnapshot> {
        let path = format!("risk_manager/risk_snapshot?username={}", username);
        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let snapshot: RiskSnapshot = response.json().await?;
            Ok(snapshot)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get risk snapshot failed: {}", error_text)
        }
    }

    /// Get all risk snapshots (admin only)
    pub async fn get_admin_risk_snapshots(
        &self,
        params: Option<protocol::common::HistoryParams>,
    ) -> Result<protocol::common::PaginatedResponse<Vec<RiskSnapshot>>> {
        let mut path = "risk_manager/admin/risk_snapshots".to_string();

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(pagination) = params.pagination {
                if let Some(limit) = pagination.limit {
                    query_params.push(format!("limit={}", limit));
                }
                if let Some(offset) = pagination.offset {
                    query_params.push(format!("offset={}", offset));
                }
            }

            if let Some(date_range) = params.date_range {
                if let Some(start) = date_range.start_time {
                    query_params.push(format!("start_time={}", start.to_rfc3339()));
                }
                if let Some(end) = date_range.end_time {
                    query_params.push(format!("end_time={}", end.to_rfc3339()));
                }
            }

            if !query_params.is_empty() {
                path.push('?');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let snapshots: Vec<RiskSnapshot> = response.json().await?;
            Ok(protocol::common::PaginatedResponse {
                data: snapshots,
                metadata: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get admin risk snapshots failed: {}", error_text)
        }
    }

    /// Get stress test risk snapshots with specified market move percentage
    pub async fn get_stress_test_risk_snapshots(
        &self,
        percent_move: i32,
    ) -> Result<Vec<protocol::risk_manager::StressTestResult>> {
        let response = self
            .make_request(
                reqwest::Method::GET,
                &format!(
                    "risk_manager/admin/stress_test_risk_snapshots?percent_move={}",
                    percent_move
                ),
                None,
            )
            .await?;

        if response.status().is_success() {
            let stress_results: Vec<protocol::risk_manager::StressTestResult> =
                response.json().await?;
            Ok(stress_results)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get stress test risk snapshots failed: {}", error_text)
        }
    }
}

/// Settlement Gateway Client for settlement operations
pub struct SettlementGatewayClient {
    base_url: Url,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl SettlementGatewayClient {
    pub fn new(base_url: Url, user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>) -> Self {
        Self {
            base_url,
            user_token,
        }
    }

    /// Helper method to get current token
    async fn get_token(&self) -> Result<ArcStr> {
        let token = self.user_token.load();
        if let Some(stored) = &*token {
            let (token, expires_at) = &**stored;
            let now = Utc::now();
            if *expires_at > now {
                return Ok(token.clone());
            }
        }
        bail!("Token expired or not available")
    }

    /// Helper method to make authenticated HTTP requests
    async fn make_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response> {
        let url = self.base_url.join(path)?;
        debug!("{} {}", method, url);

        let token = self.get_token().await?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let mut request = client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", token.as_str()))
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        Ok(response)
    }

    /// Get settlement status
    pub async fn get_status(&self) -> Result<protocol::settlement_engine::SettlementStatus> {
        let response = self
            .make_request(reqwest::Method::GET, "settlement/status", None)
            .await?;

        if response.status().is_success() {
            let status: protocol::settlement_engine::SettlementStatus = response.json().await?;
            Ok(status)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get settlement status failed: {}", error_text)
        }
    }

    /// Get settlement history
    pub async fn get_settlement_history(
        &self,
        params: Option<protocol::common::HistoryParams>,
    ) -> Result<
        protocol::common::PaginatedResponse<Vec<protocol::settlement_engine::SettlementRecord>>,
    > {
        let mut path = "settlement/history".to_string();

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(pagination) = params.pagination {
                if let Some(limit) = pagination.limit {
                    query_params.push(format!("limit={}", limit));
                }
                if let Some(offset) = pagination.offset {
                    query_params.push(format!("offset={}", offset));
                }
            }

            if let Some(date_range) = params.date_range {
                if let Some(start) = date_range.start_time {
                    query_params.push(format!("start_time={}", start.to_rfc3339()));
                }
                if let Some(end) = date_range.end_time {
                    query_params.push(format!("end_time={}", end.to_rfc3339()));
                }
            }

            if !query_params.is_empty() {
                path.push('?');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let records: Vec<protocol::settlement_engine::SettlementRecord> =
                response.json().await?;
            Ok(protocol::common::PaginatedResponse {
                data: records,
                metadata: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get settlement history failed: {}", error_text)
        }
    }

    /// Get settlement configuration
    pub async fn get_configuration(&self) -> Result<SettlementConfig> {
        let response = self
            .make_request(reqwest::Method::GET, "settlement/config", None)
            .await?;

        if response.status().is_success() {
            let config: SettlementConfig = response.json().await?;
            Ok(config)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get settlement configuration failed: {}", error_text)
        }
    }

    /// Update settlement configuration
    pub async fn update_configuration(&self, config: SettlementConfig) -> Result<SettlementConfig> {
        let response = self
            .make_request(
                reqwest::Method::PUT,
                "settlement/config",
                Some(serde_json::to_value(config)?),
            )
            .await?;

        if response.status().is_success() {
            let updated_config: SettlementConfig = response.json().await?;
            Ok(updated_config)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Update settlement configuration failed: {}", error_text)
        }
    }
}

/// Candle Server Client for market data
pub struct CandleServerClient {
    base_url: Url,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl CandleServerClient {
    pub fn new(base_url: Url, user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>) -> Self {
        Self {
            base_url,
            user_token,
        }
    }

    /// Helper method to get current token
    async fn get_token(&self) -> Result<ArcStr> {
        let token = self.user_token.load();
        if let Some(stored) = &*token {
            let (token, expires_at) = &**stored;
            let now = Utc::now();
            if *expires_at > now {
                return Ok(token.clone());
            }
        }
        bail!("Token expired or not available")
    }

    /// Helper method to make authenticated HTTP requests
    async fn make_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response> {
        let url = self.base_url.join(path)?;
        debug!("{} {}", method, url);

        let token = self.get_token().await?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let mut request = client
            .request(method, url)
            .header("Authorization", token.as_str())
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        Ok(response)
    }

    /// Get available symbols
    pub async fn get_symbols(&self) -> Result<Vec<String>> {
        let response = self
            .make_request(reqwest::Method::GET, "candle/symbols", None)
            .await?;

        if response.status().is_success() {
            let symbols: Vec<String> = response.json().await?;
            Ok(symbols)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get symbols failed: {}", error_text)
        }
    }

    /// Get available timeframes
    pub async fn get_timeframes(&self) -> Result<Vec<String>> {
        let response = self
            .make_request(reqwest::Method::GET, "candle/timeframes", None)
            .await?;

        if response.status().is_success() {
            let timeframes: Vec<String> = response.json().await?;
            Ok(timeframes)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get timeframes failed: {}", error_text)
        }
    }

    /// Get candle data for a symbol and timeframe
    pub async fn get_candle_data(
        &self,
        symbol: &str,
        timeframe: &str,
        params: protocol::candle_server::CandleParams,
    ) -> Result<protocol::common::PaginatedResponse<Vec<Candle>>> {
        let mut query_params = Vec::new();

        if let Some(start) = params.start_time {
            query_params.push(format!("start_time={}", start.format("%Y-%m-%d")));
        }
        if let Some(end) = params.end_time {
            query_params.push(format!("end_time={}", end.format("%Y-%m-%d")));
        }
        if let Some(limit) = params.limit {
            query_params.push(format!("limit={}", limit));
        }

        let path = if query_params.is_empty() {
            format!("candle/data/{}/{}", symbol, timeframe)
        } else {
            format!(
                "candle/data/{}/{}?{}",
                symbol,
                timeframe,
                query_params.join("&")
            )
        };

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let candles: Vec<Candle> = response.json().await?;
            Ok(protocol::common::PaginatedResponse {
                data: candles,
                metadata: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get candle data failed: {}", error_text)
        }
    }

    /// Get latest candles for a symbol and timeframe
    pub async fn get_latest_candles(
        &self,
        symbol: &str,
        timeframe: &str,
        count: u32,
    ) -> Result<Vec<Candle>> {
        let path = format!("candle/latest/{}/{}?count={}", symbol, timeframe, count);
        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let candles: Vec<Candle> = response.json().await?;
            Ok(candles)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get latest candles failed: {}", error_text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_login_response() -> Result<()> {
        let res = r#"{"rid":1,"res":{"li":"market-maker-01","o":[]}}"#;
        let _res: protocol::ws::Response<Box<serde_json::value::RawValue>> =
            serde_json::from_str(res)?;
        Ok(())
    }
}
