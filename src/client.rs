use crate::api_gateway::ApiGatewayRestClient;
use crate::marketdata::MarketdataWsClient;
use crate::order_gateway::*;
use anyhow::{anyhow, Result};
use arc_swap::ArcSwapOption;
use arcstr::ArcStr;
use chrono::{DateTime, Utc};
use log::warn;
use std::sync::Arc;
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
        use crate::protocol::api_gateway::{AuthenticateRequest, AuthenticationMethod};
        let client = ApiGatewayRestClient::new(self.api_gateway_base_url.clone())?;
        let res = client
            .authenticate(AuthenticateRequest {
                auth: AuthenticationMethod::UsernamePassword {
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
        let token = self.refresh_user_token(false).await?;
        OrderGatewayWsClient::connect(self.base_url.clone(), token).await
    }

    pub async fn marketdata_ws(&self) -> Result<MarketdataWsClient> {
        let token = self.refresh_user_token(false).await?;
        MarketdataWsClient::connect(self.base_url.clone(), token).await
    }
}
