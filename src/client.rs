use crate::order_gateway::*;
use crate::{protocol, types::*};
use anyhow::{anyhow, bail, Result};
use arc_swap::ArcSwapOption;
use arcstr::ArcStr;
use auth_gateway::{AuthGatewayClient, AuthGatewayConfig};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, trace};
use reqwest;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

#[derive(Clone)]
pub struct ArchitectX {
    base_url: Url,
    auth_client: Arc<AuthGatewayClient>,
    username: Option<String>,
    password: Option<String>,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl ArchitectX {
    // CR alee: default empty construction, use builder pattern
    pub fn new(base_url: Url, username: Option<&str>, password: Option<&str>) -> Self {
        // Create auth-gateway client configuration
        let auth_config = AuthGatewayConfig {
            base_url: base_url
                .join("auth/")
                .unwrap_or(base_url.clone())
                .to_string(),
            admin_secret_key: match std::env::var("DECODE_TOKEN_SECRET_KEY") {
                Ok(key) => Some(key),
                Err(_) => None,
            },
            timeout_seconds: 10,
            max_retries: 3,
            pool_max_idle_per_host: 10,
        };

        let auth_client =
            AuthGatewayClient::new(auth_config).expect("Failed to create auth-gateway client");

        Self {
            base_url,
            auth_client: Arc::new(auth_client),
            username: username.map(|s| s.to_string()),
            password: password.map(|s| s.to_string()),
            user_token: Arc::new(ArcSwapOption::const_empty()),
        }
    }

    fn username(&self) -> Result<&str> {
        self.username
            .as_deref()
            .ok_or_else(|| anyhow!("no username provided"))
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
        let token = self.get_user_token(username, password, 3600).await?;
        let token = ArcStr::from(token);
        self.user_token.store(Some(Arc::new((
            token.clone(),
            now + chrono::Duration::seconds(3300 /* one hour, less 5 minutes buffer */),
        ))));
        Ok(token)
    }

    pub async fn get_user_token(
        &self,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        expiration_seconds: i32,
    ) -> Result<String> {
        self.auth_client
            .get_user_token(
                &username.as_ref().into(),
                &password.as_ref().into(),
                expiration_seconds as u64,
            )
            .await
            .map(|token| token.expose_secret().to_string())
            .map_err(|e| anyhow!("Failed to get user token: {}", e))
    }

    pub async fn get_instrument(&self, symbol: impl AsRef<str>) -> Result<InstrumentV0> {
        let token = self.refresh_user_token(false).await?;
        let instruments = self
            .auth_client
            .get_instruments(&token.as_str().into())
            .await
            .map_err(|e| anyhow!("Failed to get instruments: {}", e))?;

        let symbol_str = symbol.as_ref();
        let auth_instrument = instruments
            .into_iter()
            .find(|i| i.symbol == symbol_str)
            .ok_or_else(|| anyhow!("Instrument not found: {}", symbol_str))?;

        Ok(InstrumentV0 {
            symbol: auth_instrument.symbol,
            tick_size: auth_instrument.tick_size,
            base_currency: auth_instrument.base_currency,
            multiplier: 1, // Default multiplier since auth-gateway doesn't provide this
            minimum_trade_quantity: auth_instrument.minimum_trade_quantity as i32,
            description: auth_instrument.description,
            product_id: auth_instrument.product_id,
            state: auth_instrument.state,
            price_scale: auth_instrument
                .price_scale
                .to_string()
                .parse::<i32>()
                .unwrap_or(1),
        })
    }

    pub async fn marketdata_client(&self) -> Result<MarketdataClient> {
        let username = self.username()?;
        let token = self.refresh_user_token(false).await?;
        MarketdataClient::connect(self.base_url.clone(), username, token).await
    }

    pub async fn order_gateway_ws(&self) -> Result<OrderGatewayWsClient> {
        let username = self.username()?;
        let token = self.refresh_user_token(false).await?;
        OrderGatewayWsClient::connect(self.base_url.clone(), username, token).await
    }

    pub fn order_gateway(&self) -> Result<OrderGatewayRestClient> {
        let username = self.username()?;
        OrderGatewayRestClient::new(self.base_url.clone(), username, self.user_token.clone())
    }

    pub async fn account_gateway_client(&self) -> Result<AccountGatewayClient> {
        let account_base_url = self.base_url.join("account/")?;
        AccountGatewayClient::connect(account_base_url, self.user_token.clone()).await
    }

    /// Get extended auth gateway client for API key management
    pub async fn auth_gateway_extended_client(&self) -> Result<AuthGatewayExtendedClient> {
        Ok(AuthGatewayExtendedClient::new(
            self.base_url.clone(),
            self.auth_client.clone(),
            self.user_token.clone(),
        ))
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

pub struct Orderbook {
    pub bids: BTreeMap<Decimal, OrderbookLevel>,
    pub asks: BTreeMap<Decimal, OrderbookLevel>,
}

pub struct OrderbookLevel {
    pub quantity: i32,
    pub order_quantities: Option<Vec<i32>>, // for LEVEL_3
}

impl From<&protocol::marketdata_publisher::L2BookUpdate> for Orderbook {
    fn from(u: &protocol::marketdata_publisher::L2BookUpdate) -> Self {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        for l in &u.bids {
            bids.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: None,
                },
            );
        }
        for l in &u.asks {
            asks.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: None,
                },
            );
        }
        Self { bids, asks }
    }
}

impl From<&protocol::marketdata_publisher::L3BookUpdate> for Orderbook {
    fn from(u: &protocol::marketdata_publisher::L3BookUpdate) -> Self {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();
        for l in &u.bids {
            bids.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: Some(l.order_quantities.clone()),
                },
            );
        }
        for l in &u.asks {
            asks.insert(
                l.price,
                OrderbookLevel {
                    quantity: l.quantity,
                    order_quantities: Some(l.order_quantities.clone()),
                },
            );
        }
        Self { bids, asks }
    }
}

pub struct MarketdataClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_request_id: i32,
    pub orderbooks: HashMap<String, Orderbook>,
}

impl MarketdataClient {
    pub async fn connect(
        base_url: Url,
        username: impl AsRef<str>,
        token: impl AsRef<str>,
    ) -> Result<Self> {
        // derive ws url
        let mut ws_base_url = base_url.clone();
        let res = match base_url.scheme() {
            "http" => ws_base_url.set_scheme("ws"),
            "https" => ws_base_url.set_scheme("wss"),
            _ => bail!("invalid url scheme"),
        };
        res.map_err(|_| anyhow!("invalid url scheme"))?;
        let md_url = ws_base_url.join("md/ws")?.to_string();

        // connect to market data publisher
        info!("connecting to {md_url}");
        let (mut ws, _) = connect_async(md_url).await?;

        // send login request
        let req = json!({
            "request_id": 1,
            "type": "login",
            "username": username.as_ref().to_string(),
            "token": token.as_ref().to_string(),
        });
        let payload = serde_json::to_string(&req)?;
        info!("sending login request: {payload}");
        ws.send(Message::Text(payload.into())).await?;

        Ok(Self {
            ws,
            next_request_id: 1,
            orderbooks: HashMap::new(),
        })
    }

    pub async fn next(
        &mut self,
    ) -> Result<Option<Arc<protocol::marketdata_publisher::MarketdataEvent>>> {
        let msg = self
            .ws
            .next()
            .await
            .ok_or_else(|| anyhow!("ws stream ended"))??;
        match msg {
            Message::Text(text) => {
                trace!("decoding marketdata message: {text}");
                match serde_json::from_str::<protocol::ws::Response<Box<serde_json::value::RawValue>>>(
                    &text,
                ) {
                    Ok(_r) => {
                        // TODO: do something
                    }
                    Err(e_as_response) => {
                        match serde_json::from_str::<
                            Arc<protocol::marketdata_publisher::MarketdataEvent>,
                        >(&text)
                        {
                            Ok(e) => {
                                self.handle_event(&e)?;
                                return Ok(Some(e));
                            }
                            Err(e_as_event) => {
                                error!("decoding marketdata message as event: {e_as_event:?}");
                                error!(
                                    "decoding marketdata message as response: {e_as_response:?}"
                                );
                                return Ok(None);
                            }
                        }
                    }
                }
            }
            Message::Ping(..) => {
                trace!("ws ping received");
            }
            Message::Binary(..) | Message::Frame(..) | Message::Pong(..) | Message::Close(..) => {}
        }
        Ok(None)
    }

    fn handle_event(&mut self, e: &protocol::marketdata_publisher::MarketdataEvent) -> Result<()> {
        use protocol::marketdata_publisher::*;
        trace!("marketdata event: {e:?}");
        match e {
            MarketdataEvent::Heartbeat(t) => {
                debug!("heartbeat: {:?}", t.as_datetime());
            }
            MarketdataEvent::Ticker(_t) => {
                // TODO
            }
            MarketdataEvent::L1BookUpdate(u) => {
                let orderbook: Orderbook = u.into();
                self.orderbooks.insert(u.symbol.clone(), orderbook);
            }
            MarketdataEvent::L2BookUpdate(u) => {
                let orderbook: Orderbook = u.into();
                self.orderbooks.insert(u.symbol.clone(), orderbook);
            }
            MarketdataEvent::L3BookUpdate(u) => {
                let orderbook: Orderbook = u.into();
                self.orderbooks.insert(u.symbol.clone(), orderbook);
            }
        }
        Ok(())
    }

    // CR alee: also send an unsubscribe (only subscribe one level per symbol
    // at a time); maybe that's just the behavior of the publisher anyways
    pub async fn subscribe(
        &mut self,
        symbol: impl AsRef<str>,
        level: &str, // LEVEL_1, LEVEL_2, LEVEL_3
    ) -> Result<()> {
        let req_id = self.next_request_id;
        let req = json!({
            "request_id": req_id,
            "type": "subscribe",
            "symbol": symbol.as_ref().to_string(),
            "level": level,
        });
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        trace!("sending subscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self, symbol: impl AsRef<str>) -> Result<()> {
        let req_id = self.next_request_id;
        let req = json!({
            "request_id": req_id,
            "type": "unsubscribe",
            "symbol": symbol.as_ref().to_string(),
        });
        self.next_request_id += 1;
        let payload = serde_json::to_string(&req)?;
        trace!("sending unsubscribe request: {payload}");
        self.ws.send(Message::Text(payload.into())).await?;
        Ok(())
    }
}
pub struct AccountGatewayClient {
    base_url: Url,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl AccountGatewayClient {
    pub async fn connect(
        base_url: Url,
        user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
    ) -> Result<Self> {
        Ok(Self {
            base_url,
            user_token,
        })
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

        // Create a temporary client for requests
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

    /// Get account balances for a user
    pub async fn get_balances(&self, username: &str) -> Result<Vec<Balance>> {
        let path = format!("balances?username={}", username);
        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let balances: Vec<Balance> = response.json().await?;
            Ok(balances)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get balances failed: {}", error_text)
        }
    }

    /// Get account positions for a user
    pub async fn get_positions(&self, username: &str) -> Result<Vec<Position>> {
        let path = format!("positions?username={}", username);
        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let positions: Vec<Position> = response.json().await?;
            Ok(positions)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get positions failed: {}", error_text)
        }
    }

    /// Get user account status
    pub async fn get_user_status(&self, username: &str) -> Result<UserStatus> {
        let path = format!("user-status?username={}", username);
        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let status: UserStatus = response.json().await?;
            Ok(status)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get user status failed: {}", error_text)
        }
    }

    /// Get open interest data
    pub async fn get_open_interest(&self) -> Result<Vec<OpenInterest>> {
        let response = self
            .make_request(reqwest::Method::GET, "open-interest", None)
            .await?;

        if response.status().is_success() {
            let data: Vec<OpenInterest> = response.json().await?;
            Ok(data)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get open interest failed: {}", error_text)
        }
    }

    /// Get user's fill history
    pub async fn get_fills(
        &self,
        username: &str,
        params: Option<HistoryParams>,
    ) -> Result<ApiResponse<Vec<Fill>>> {
        let mut path = format!("fills?username={}", username);

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

            if let Some(filters) = params.filters {
                for (key, value) in filters {
                    query_params.push(format!("{}={}", key, value));
                }
            }

            if !query_params.is_empty() {
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let fills: Vec<Fill> = response.json().await?;
            Ok(ApiResponse {
                data: fills,
                metadata: None, // TODO: Extract from response headers if available
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get fills failed: {}", error_text)
        }
    }

    /// Get recent fills for a specific symbol
    pub async fn get_last_fills(
        &self,
        username: &str,
        symbol: &str,
        count: u32,
    ) -> Result<Vec<Fill>> {
        let path = format!(
            "last-fills?username={}&symbol={}&count={}",
            username, symbol, count
        );
        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let fills: Vec<Fill> = response.json().await?;
            Ok(fills)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get last fills failed: {}", error_text)
        }
    }

    /// Get funding payment history
    pub async fn get_funding_history(
        &self,
        username: &str,
        params: Option<HistoryParams>,
    ) -> Result<ApiResponse<Vec<FundingHistory>>> {
        let mut path = format!("funding-history?username={}", username);

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
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let records: Vec<FundingHistory> = response.json().await?;
            Ok(ApiResponse {
                data: records,
                metadata: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get funding history failed: {}", error_text)
        }
    }

    /// Get deposit history
    pub async fn get_deposit_history(
        &self,
        username: &str,
        params: Option<HistoryParams>,
    ) -> Result<ApiResponse<Vec<DepositRecord>>> {
        let mut path = format!("deposit-history?username={}", username);

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
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let records: Vec<DepositRecord> = response.json().await?;
            Ok(ApiResponse {
                data: records,
                metadata: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get deposit history failed: {}", error_text)
        }
    }

    /// Get withdrawal history
    pub async fn get_withdrawal_history(
        &self,
        username: &str,
        params: Option<HistoryParams>,
    ) -> Result<ApiResponse<Vec<WithdrawalRecord>>> {
        let mut path = format!("withdrawal-history?username={}", username);

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
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let records: Vec<WithdrawalRecord> = response.json().await?;
            Ok(ApiResponse {
                data: records,
                metadata: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get withdrawal history failed: {}", error_text)
        }
    }

    /// Submit deposit request
    pub async fn deposit(&self, request: DepositRequest) -> Result<()> {
        let response = self
            .make_request(
                reqwest::Method::POST,
                "deposit",
                Some(serde_json::to_value(request)?),
            )
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Deposit failed: {}", error_text)
        }
    }

    /// Submit withdrawal request
    pub async fn withdraw(&self, request: WithdrawRequest) -> Result<()> {
        let response = self
            .make_request(
                reqwest::Method::POST,
                "withdraw",
                Some(serde_json::to_value(request)?),
            )
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Withdraw failed: {}", error_text)
        }
    }

    /// Liquidate account
    pub async fn liquidate(&self, request: LiquidateRequest) -> Result<LiquidateResponse> {
        let response = self
            .make_request(
                reqwest::Method::POST,
                "liquidate",
                Some(serde_json::to_value(request)?),
            )
            .await?;

        if response.status().is_success() {
            let liquidate_response: LiquidateResponse = response.json().await?;
            Ok(liquidate_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Liquidate failed: {}", error_text)
        }
    }

    /// Get trading volume statistics
    pub async fn get_trading_volume(
        &self,
        username: &str,
        params: Option<DateRangeParams>,
    ) -> Result<Decimal> {
        let mut path = format!("trading-volume?username={}", username);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(start) = params.start_time {
                query_params.push(format!("start_time={}", start.to_rfc3339()));
            }
            if let Some(end) = params.end_time {
                query_params.push(format!("end_time={}", end.to_rfc3339()));
            }

            if !query_params.is_empty() {
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let volume: Decimal = response.json().await?;
            Ok(volume)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get trading volume failed: {}", error_text)
        }
    }

    /// Get deposit statistics
    pub async fn get_deposit_stats(
        &self,
        username: &str,
        params: Option<DateRangeParams>,
    ) -> Result<DepositStats> {
        let mut path = format!("deposit-stats?username={}", username);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(start) = params.start_time {
                query_params.push(format!("start_time={}", start.to_rfc3339()));
            }
            if let Some(end) = params.end_time {
                query_params.push(format!("end_time={}", end.to_rfc3339()));
            }

            if !query_params.is_empty() {
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let response_text = response.text().await?;
            match serde_json::from_str::<DepositStats>(&response_text) {
                Ok(stats) => Ok(stats),
                Err(e) => {
                    bail!(
                        "Failed to deserialize deposit stats from response '{}': {}",
                        response_text,
                        e
                    )
                }
            }
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get deposit stats failed: {}", error_text)
        }
    }

    /// Get withdrawal statistics
    pub async fn get_withdrawal_stats(
        &self,
        username: &str,
        params: Option<DateRangeParams>,
    ) -> Result<WithdrawalStats> {
        let mut path = format!("withdrawal-stats?username={}", username);

        if let Some(params) = params {
            let mut query_params = Vec::new();

            if let Some(start) = params.start_time {
                query_params.push(format!("start_time={}", start.format("%Y-%m-%d")));
            }
            if let Some(end) = params.end_time {
                query_params.push(format!("end_time={}", end.format("%Y-%m-%d")));
            }

            if !query_params.is_empty() {
                path.push('&');
                path.push_str(&query_params.join("&"));
            }
        }

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let response_text = response.text().await?;
            match serde_json::from_str::<WithdrawalStats>(&response_text) {
                Ok(stats) => Ok(stats),
                Err(e) => {
                    bail!(
                        "Failed to deserialize withdrawal stats from response '{}': {}",
                        response_text,
                        e
                    )
                }
            }
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get withdrawal stats failed: {}", error_text)
        }
    }

    /// Get admin statistics (requires admin privileges)
    pub async fn get_admin_stats(&self, params: DateRangeParams) -> Result<AdminResponse> {
        let mut query_params = Vec::new();

        if let Some(start) = params.start_time {
            query_params.push(format!("start_time={}", start.format("%Y-%m-%d")));
        }
        if let Some(end) = params.end_time {
            query_params.push(format!("end_time={}", end.format("%Y-%m-%d")));
        }

        let path = if query_params.is_empty() {
            "admin-stats".to_string()
        } else {
            format!("admin-stats?{}", query_params.join("&"))
        };

        let response = self.make_request(reqwest::Method::GET, &path, None).await?;

        if response.status().is_success() {
            let stats: AdminResponse = response.json().await?;
            Ok(stats)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get admin stats failed: {}", error_text)
        }
    }
}

/// Extended Auth Gateway Client for API key management and additional auth operations
pub struct AuthGatewayExtendedClient {
    base_url: Url,
    base_client: Arc<AuthGatewayClient>,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl AuthGatewayExtendedClient {
    pub fn new(
        base_url: Url,
        base_client: Arc<AuthGatewayClient>,
        user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
    ) -> Self {
        Self {
            base_url,
            base_client,
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

    /// Create a new user account
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<CreateUserResponse> {
        let response = self
            .make_request(
                reqwest::Method::POST,
                "auth/create_user",
                Some(serde_json::to_value(request)?),
            )
            .await?;

        if response.status().is_success() {
            let user_response: CreateUserResponse = response.json().await?;
            Ok(user_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Create user failed: {}", error_text)
        }
    }

    /// Create a new API key
    pub async fn create_api_key(&self, request: CreateApiKeyRequest) -> Result<ApiKeyResponse> {
        let response = self
            .make_request(
                reqwest::Method::POST,
                "auth/create_api_key",
                Some(serde_json::to_value(request)?),
            )
            .await?;

        if response.status().is_success() {
            let api_key_response: ApiKeyResponse = response.json().await?;
            Ok(api_key_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Create API key failed: {}", error_text)
        }
    }

    /// Get all API keys for the current user
    pub async fn get_api_keys(&self, username: &str) -> Result<Vec<String>> {
        let request_body = serde_json::json!({
            "username": username
        });

        let response = self
            .make_request(
                reqwest::Method::POST,
                "auth/get_api_keys",
                Some(request_body),
            )
            .await?;

        if response.status().is_success() {
            let api_keys_response: GetApiKeysResponse = response.json().await?;
            Ok(api_keys_response.api_keys)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get API keys failed: {}", error_text)
        }
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, api_key: &str) -> Result<RevokeApiKeyResponse> {
        let request_body = serde_json::json!({
            "api_key": api_key
        });

        let response = self
            .make_request(
                reqwest::Method::POST,
                "auth/revoke_api_key",
                Some(request_body),
            )
            .await?;

        if response.status().is_success() {
            let revoke_response: RevokeApiKeyResponse = response.json().await?;
            Ok(revoke_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Revoke API key failed: {}", error_text)
        }
    }

    /// Refresh the current token
    pub async fn refresh_token(&self) -> Result<String> {
        // Delegate to the base client's token refresh functionality
        let token = self.user_token.load();
        if let Some(stored) = &*token {
            let (token, _) = &**stored;
            Ok(token.to_string())
        } else {
            bail!("No token available to refresh")
        }
    }

    /// Validate a token
    pub async fn validate_token(&self, token: &str) -> Result<TokenValidation> {
        let request_body = serde_json::json!({
            "token": token
        });

        let response = self
            .make_request(
                reqwest::Method::POST,
                "auth/validate_token",
                Some(request_body),
            )
            .await?;

        if response.status().is_success() {
            let validation: TokenValidation = response.json().await?;
            Ok(validation)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Validate token failed: {}", error_text)
        }
    }

    /// Get user token (delegates to base client)
    pub async fn get_user_token(
        &self,
        username: &str,
        password: &str,
        expiration_seconds: u64,
    ) -> Result<String> {
        self.base_client
            .get_user_token(&username.into(), &password.into(), expiration_seconds)
            .await
            .map(|token| token.expose_secret().to_string())
            .map_err(|e| anyhow!("Failed to get user token: {}", e))
    }

    /// Get instruments (delegates to base client)
    pub async fn get_instruments(&self, token: &str) -> Result<Vec<auth_gateway::Instrument>> {
        self.base_client
            .get_instruments(&token.into())
            .await
            .map_err(|e| anyhow!("Failed to get instruments: {}", e))
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
        params: Option<HistoryParams>,
    ) -> Result<ApiResponse<Vec<RiskSnapshot>>> {
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
            Ok(ApiResponse {
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
    ) -> Result<Vec<StressTestResult>> {
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
            let stress_results: Vec<StressTestResult> = response.json().await?;
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
    pub async fn get_status(&self) -> Result<SettlementStatus> {
        let response = self
            .make_request(reqwest::Method::GET, "settlement/status", None)
            .await?;

        if response.status().is_success() {
            let status: SettlementStatus = response.json().await?;
            Ok(status)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Get settlement status failed: {}", error_text)
        }
    }

    /// Get settlement history
    pub async fn get_settlement_history(
        &self,
        params: Option<HistoryParams>,
    ) -> Result<ApiResponse<Vec<SettlementRecord>>> {
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
            let records: Vec<SettlementRecord> = response.json().await?;
            Ok(ApiResponse {
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
        params: CandleParams,
    ) -> Result<ApiResponse<Vec<Candle>>> {
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
            Ok(ApiResponse {
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
