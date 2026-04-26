use crate::marketdata::MarketdataWsClient;
use crate::order_gateway::*;
use crate::types::ws::TokenRefreshFn;
use crate::{api_gateway::ApiGatewayRestClient, environment::Environment};
use anyhow::{anyhow, Result};
use arc_swap::ArcSwapOption;
use arcstr::ArcStr;
use chrono::{DateTime, Utc};
use futures::FutureExt;
use log::warn;
use std::sync::Arc;
use url::Url;

#[derive(Clone)]
pub struct ArchitectX {
    base_url: Url,
    api_gateway_base_url: Url,
    order_gateway_base_url: Url,
    api_key: Option<String>,
    api_secret: Option<String>,
    user_token: Arc<ArcSwapOption<(ArcStr, DateTime<Utc>)>>,
}

impl ArchitectX {
    pub fn new(
        environment: Environment,
        api_key: Option<impl AsRef<str>>,
        api_secret: Option<impl AsRef<str>>,
    ) -> Result<Self> {
        let base_url = environment.base_url();
        Ok(Self {
            base_url: base_url.clone(),
            api_gateway_base_url: base_url.join("api/")?,
            order_gateway_base_url: base_url.join("orders/")?,
            api_key: api_key.map(|k| k.as_ref().to_string()),
            api_secret: api_secret.map(|s| s.as_ref().to_string()),
            user_token: Arc::new(ArcSwapOption::const_empty()),
        })
    }

    pub fn set_api_gateway_base_url(&mut self, base_url: Url) {
        self.api_gateway_base_url = base_url;
    }

    pub fn set_order_gateway_base_url(&mut self, base_url: Url) {
        self.order_gateway_base_url = base_url;
    }

    /// Authenticate with api key and secret.
    ///
    /// This method currently exchanges the api key and secret for a
    /// user token directly.
    pub async fn authenticate(
        &self,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Result<ArcStr> {
        use crate::protocol::api_gateway::{AuthenticateRequest, AuthenticationMethod};
        let auth = AuthenticationMethod::ApiKeySecret {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        };
        let client = ApiGatewayRestClient::new(self.api_gateway_base_url.clone())?;
        let res = client
            .authenticate(AuthenticateRequest {
                auth,
                expiration_seconds: 3600,
            })
            .await?;
        let token: ArcStr = res.token.expose_secret().to_string().into();
        let expires = Utc::now() + chrono::Duration::seconds(3300);
        self.user_token
            .store(Some(Arc::new((token.clone(), expires))));
        Ok(token)
    }

    pub async fn login(
        &self,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
        totp: Option<impl AsRef<str>>,
    ) -> Result<ArcStr> {
        use crate::protocol::api_gateway::{AuthenticateRequest, AuthenticationMethod};
        let auth = AuthenticationMethod::UsernamePassword {
            username: username.as_ref().to_string(),
            password: password.as_ref().to_string(),
            totp: totp.map(|t| t.as_ref().to_string()),
        };
        let client = ApiGatewayRestClient::new(self.api_gateway_base_url.clone())?;
        let res = client
            .authenticate(AuthenticateRequest {
                auth,
                expiration_seconds: 3600,
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

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("no api_key provided"))?;
        let api_secret = self
            .api_secret
            .as_ref()
            .ok_or_else(|| anyhow!("no secret provided"))?;
        self.authenticate(api_key, api_secret).await
    }

    pub fn api_gateway(&self) -> Result<ApiGatewayRestClient> {
        let mut client = ApiGatewayRestClient::new(self.api_gateway_base_url.clone())?;
        let auth = self.user_token.load();
        if let Some(token) = &*auth {
            let (token, expires_at) = &**token;
            if *expires_at > Utc::now() {
                client.set_token(token.as_str().to_string(), *expires_at);
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
                client.set_token(token.as_str().to_string(), *expires_at);
            } else {
                warn!("while creating order gateway client: token expired");
            }
        }
        Ok(client)
    }

    pub async fn order_gateway_ws(&self) -> Result<OrderGatewayWsClient> {
        let this = self.clone();
        let refresh: TokenRefreshFn = Arc::new(move || {
            let this = this.clone();
            async move { this.refresh_user_token(false).await }.boxed()
        });
        OrderGatewayWsClient::connect(self.base_url.clone(), refresh)
            .await
            .map_err(anyhow::Error::from)
    }

    pub async fn order_gateway_ws_with_cancel_on_disconnect(&self) -> Result<OrderGatewayWsClient> {
        let this = self.clone();
        let refresh: TokenRefreshFn = Arc::new(move || {
            let this = this.clone();
            async move { this.refresh_user_token(false).await }.boxed()
        });
        OrderGatewayWsClient::connect_with_cancel_on_disconnect(self.base_url.clone(), refresh)
            .await
            .map_err(anyhow::Error::from)
    }

    pub async fn marketdata_ws(&self) -> Result<MarketdataWsClient> {
        let this = self.clone();
        let refresh: TokenRefreshFn = Arc::new(move || {
            let this = this.clone();
            async move { this.refresh_user_token(false).await }.boxed()
        });
        MarketdataWsClient::connect(self.base_url.clone(), refresh)
            .await
            .map_err(anyhow::Error::from)
    }
}
