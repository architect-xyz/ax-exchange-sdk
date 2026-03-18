use crate::protocol::{common::Fill, order_gateway::*, ErrorResponse, HealthResponse};
use crate::types::trading::{Order, PlaceOrder};
use crate::OrderId;
use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use log::{debug, trace};
use reqwest;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use url::Url;

pub struct OrderGatewayRestClient {
    client: reqwest::Client,
    base_url: Url,
    token: Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
}

impl OrderGatewayRestClient {
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
            Ok(token)
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
            match serde_json::from_str::<ErrorResponse>(&res_text) {
                Ok(error_response) => Err(anyhow!(error_response.error)),
                Err(e) => Err(anyhow!("while parsing error response: {e:?}")),
            }
        }
    }

    /// Check order gateway health
    pub async fn health(&self) -> Result<HealthResponse> {
        self.request(reqwest::Method::GET, "health", None::<&str>, false)
            .await
    }

    /// Get all open orders
    pub async fn open_orders(&self) -> Result<Vec<Order>> {
        let payload = GetOpenOrdersRequest {};
        let res: GetOpenOrdersResponse = self
            .request(reqwest::Method::GET, "open-orders", Some(payload), true)
            .await?;
        let orders = res
            .orders
            .into_iter()
            .map(|o| o.try_into())
            .collect::<Result<Vec<Order>>>()?;
        Ok(orders)
    }

    pub async fn order_status(&self, order: OrderIdentifier) -> Result<OrderStatus> {
        let payload = match order {
            OrderIdentifier::OrderId(order_id) => GetOrderStatusRequest {
                order_id: Some(order_id),
                client_order_id: None,
            },
            OrderIdentifier::ClientOrderId(client_order_id) => GetOrderStatusRequest {
                order_id: None,
                client_order_id: Some(client_order_id),
            },
        };
        let res: GetOrderStatusResponse = self
            .request(reqwest::Method::GET, "order-status", Some(payload), true)
            .await?;
        Ok(res.status)
    }

    /// Place a new order
    pub async fn place_order(&self, order: PlaceOrder) -> Result<String> {
        let payload: PlaceOrderRequest = order.into();
        let res: PlaceOrderResponse = self
            .request(reqwest::Method::POST, "place-order", Some(payload), true)
            .await?;
        Ok(res.order_id)
    }

    /// Cancel an existing order
    pub async fn cancel_order(&self, order_id: &OrderId) -> Result<bool> {
        let payload = CancelOrderRequest {
            order_id: order_id.clone(),
        };
        let res: CancelOrderResponse = self
            .request(reqwest::Method::POST, "cancel-order", Some(payload), true)
            .await?;
        Ok(res.cancel_request_accepted)
    }

    /// Cancel all orders, optionally filtered by symbol
    pub async fn cancel_all_orders(&self, symbol: Option<&str>) -> Result<()> {
        let payload = CancelAllOrdersRequest {
            symbol: symbol.map(|s| s.to_string()),
        };
        let _res: CancelAllOrdersResponse = self
            .request(
                reqwest::Method::POST,
                "cancel-all-orders",
                Some(payload),
                true,
            )
            .await?;
        Ok(())
    }

    pub async fn order_fills(&self, order_id: &OrderId) -> Result<Vec<Fill>> {
        let payload = GetOrderFillsRequest {
            order_id: order_id.clone(),
        };
        let res: GetOrderFillsResponse = self
            .request(reqwest::Method::GET, "order-fills", Some(payload), true)
            .await?;
        Ok(res.fills)
    }
}
