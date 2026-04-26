use crate::protocol::marketdata_publisher::*;
pub use crate::types::ws::ConnectionState;
pub use crate::ws_utils::ConnectionStateWatcher;
use crate::{
    protocol::{
        marketdata_publisher::{MarketdataRequest, SubscriptionLevel},
        ws::Request as WsRequest,
    },
    types::{
        trading::CandleWidth,
        ws::{InternalCommand, TokenRefreshFn, WsClientError},
    },
    ws_utils::{connection_supervisor, PendingRequests, WsSubscription},
};
use dashmap::DashMap;
use log::info;
use log::trace;
use std::sync::Arc;
use tokio::{
    sync::{
        mpsc::{self, UnboundedSender},
        oneshot, watch, Mutex,
    },
    task::JoinHandle,
};
use url::Url;
use yawc::Frame;

pub type ClientError = WsClientError;

// ---------------------------------------------------------------------------
// Subscription tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Subscription {
    Level {
        symbol: String,
        level: SubscriptionLevel,
    },
    Candles {
        symbol: String,
        width: CandleWidth,
    },
    BboCandles {
        symbol: String,
        width: CandleWidth,
    },
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct MarketdataWsClient {
    write_tx: UnboundedSender<InternalCommand>,
    pub connection_state_rx: watch::Receiver<ConnectionState>,
    pub market_data_receiver: mpsc::Receiver<MarketdataEvent>,
    subscriptions: Arc<tokio::sync::RwLock<Vec<Subscription>>>,
    pending_requests: PendingRequests,
    next_request_id: i32,
    shutdown_tx: watch::Sender<bool>,
    supervisor_handle: Arc<Mutex<JoinHandle<()>>>,
    current_connection_state: Arc<Mutex<ConnectionState>>,
}

impl MarketdataWsClient {
    /// Connect to the marketdata websocket using the standard path derivation.
    /// This joins `md/ws` to the base URL (e.g., `http://example.com` -> `ws://example.com/md/ws`).
    pub async fn connect(
        base_url: Url,
        token_refresh: TokenRefreshFn,
    ) -> Result<Self, ClientError> {
        let mut ws_base_url = base_url.clone();
        match base_url.scheme() {
            "http" => ws_base_url
                .set_scheme("ws")
                .map_err(|_| ClientError::InvalidScheme)?,
            "https" => ws_base_url
                .set_scheme("wss")
                .map_err(|_| ClientError::InvalidScheme)?,
            _ => return Err(ClientError::InvalidScheme),
        };
        let md_url = ws_base_url.join("md/ws")?;
        Self::connect_to_url(md_url, token_refresh).await
    }

    /// Connect to a marketdata websocket at a specific URL.
    /// Use this for integration tests or custom deployments where the standard
    /// path derivation doesn't apply.
    pub async fn connect_to_url(
        mut url: Url,
        token_refresh: TokenRefreshFn,
    ) -> Result<Self, ClientError> {
        match url.scheme() {
            "http" => url
                .set_scheme("ws")
                .map_err(|_| ClientError::InvalidScheme)?,
            "https" => url
                .set_scheme("wss")
                .map_err(|_| ClientError::InvalidScheme)?,
            "ws" | "wss" => {}
            _ => return Err(ClientError::InvalidScheme),
        };

        info!("connecting to {}", url);

        let (market_data_sender, market_data_receiver) = mpsc::channel(100);
        let (write_tx, write_rx) = mpsc::unbounded_channel::<InternalCommand>();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (connection_state_tx, connection_state_rx) =
            watch::channel::<ConnectionState>(ConnectionState::Disconnected);

        let subscriptions = Arc::new(tokio::sync::RwLock::new(Vec::new()));
        let pending_requests: PendingRequests = Arc::new(DashMap::new());

        let supervisor_handle = tokio::spawn(connection_supervisor(
            url.to_string(),
            token_refresh,
            write_rx,
            shutdown_rx,
            market_data_sender,
            subscriptions.clone(),
            connection_state_tx,
            pending_requests.clone(),
        ));

        Ok(Self {
            write_tx,
            connection_state_rx,
            market_data_receiver,
            subscriptions,
            pending_requests,
            next_request_id: 1,
            shutdown_tx,
            supervisor_handle: Arc::new(Mutex::new(supervisor_handle)),
            current_connection_state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
        })
    }

    /// Returns an independent [`ConnectionStateWatcher`] that can be used
    /// inside `tokio::select!` alongside mutable borrows of this client.
    pub fn state_watcher(&self) -> ConnectionStateWatcher {
        ConnectionStateWatcher::new(
            self.connection_state_rx.clone(),
            self.current_connection_state.clone(),
        )
    }

    // ---------------------------------------------------------------------------
    // Private helpers
    // ---------------------------------------------------------------------------

    async fn send_raw(&self, payload: String) -> Result<(), ClientError> {
        self.write_tx
            .send(InternalCommand::Send(Frame::text(payload)))
            .map_err(|e| ClientError::Transport(Box::new(e)))
    }

    /// Build, log, and send a single WS request, incrementing the request id.
    async fn send_request<'a>(
        &mut self,
        request: MarketdataRequest<'a>,
    ) -> Result<(), ClientError> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let (tx, rx) = oneshot::channel::<Vec<u8>>();
        self.pending_requests.insert(request_id, tx);
        let req = WsRequest {
            request_id,
            request,
        };
        let payload = serde_json::to_string(&req)?;
        trace!("sending request: {payload}");
        if let Err(e) = self.send_raw(payload).await {
            self.pending_requests.remove(&request_id);
            return Err(e);
        }
        // Await and check the server's ack/error response.
        let bytes = rx.await.map_err(|e| ClientError::Transport(Box::new(e)))?;
        let envelope: crate::protocol::ws::Response<Box<serde_json::value::RawValue>> =
            serde_json::from_slice(&bytes)?;
        if let Some(err) = envelope.error {
            return Err(ClientError::ServerError {
                code: err.code,
                message: err.message.unwrap_or_default(),
            });
        }
        Ok(())
    }

    /// Add `sub` to the tracked subscription list if not already present.
    async fn add_subscription(&self, sub: Subscription) {
        let mut subs = self.subscriptions.write().await;
        if !subs.contains(&sub) {
            subs.push(sub);
        }
    }

    /// Remove subscriptions matching `predicate` from the tracked list.
    async fn remove_subscription(&self, predicate: impl Fn(&Subscription) -> bool) {
        self.subscriptions.write().await.retain(|s| !predicate(s));
    }

    // ---------------------------------------------------------------------------
    // Public subscribe / unsubscribe API
    // ---------------------------------------------------------------------------

    pub async fn subscribe(
        &mut self,
        symbol: impl AsRef<str>,
        level: SubscriptionLevel,
    ) -> Result<(), ClientError> {
        let symbol = symbol.as_ref().to_string();
        let sub = Subscription::Level {
            symbol: symbol.clone(),
            level,
        };
        self.add_subscription(sub.clone()).await;
        let result = self
            .send_request(MarketdataRequest::Subscribe {
                symbol: &symbol,
                level,
            })
            .await;
        if result.is_err() {
            self.remove_subscription(|s| s == &sub).await;
        }
        result
    }

    pub async fn unsubscribe(&mut self, symbol: impl AsRef<str>) -> Result<(), ClientError> {
        let symbol = symbol.as_ref().to_string();
        self.remove_subscription(
            |s| matches!(s, Subscription::Level { symbol: s, .. } if s == &symbol),
        )
        .await;
        self.send_request(MarketdataRequest::Unsubscribe { symbol: &symbol })
            .await
    }

    pub async fn subscribe_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol = symbol.as_ref().to_string();
        let sub = Subscription::Candles {
            symbol: symbol.clone(),
            width,
        };
        self.add_subscription(sub.clone()).await;
        let result = self
            .send_request(MarketdataRequest::SubscribeCandles {
                symbol: &symbol,
                width,
            })
            .await;
        if result.is_err() {
            self.remove_subscription(|s| s == &sub).await;
        }
        result
    }

    pub async fn unsubscribe_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol = symbol.as_ref().to_string();
        self.remove_subscription(
            |s| matches!(s, Subscription::Candles { symbol: s, width: w } if s == &symbol && *w == width),
        )
        .await;
        self.send_request(MarketdataRequest::UnsubscribeCandles {
            symbol: &symbol,
            width,
        })
        .await
    }

    pub async fn subscribe_bbo_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol = symbol.as_ref().to_string();
        let sub = Subscription::BboCandles {
            symbol: symbol.clone(),
            width,
        };
        self.add_subscription(sub.clone()).await;
        let result = self
            .send_request(MarketdataRequest::SubscribeBboCandles {
                symbol: &symbol,
                width,
            })
            .await;
        if result.is_err() {
            self.remove_subscription(|s| s == &sub).await;
        }
        result
    }

    pub async fn unsubscribe_bbo_candles(
        &mut self,
        symbol: impl AsRef<str>,
        width: CandleWidth,
    ) -> Result<(), ClientError> {
        let symbol = symbol.as_ref().to_string();
        self.remove_subscription(
            |s| matches!(s, Subscription::BboCandles { symbol: s, width: w } if s == &symbol && *w == width),
        )
        .await;
        self.send_request(MarketdataRequest::UnsubscribeBboCandles {
            symbol: &symbol,
            width,
        })
        .await
    }

    // ---------------------------------------------------------------------------
    // Connection lifecycle
    // ---------------------------------------------------------------------------

    pub async fn shutdown(&self, reason: &'static str) -> Result<(), ClientError> {
        info!("Shutdown requested: {reason}");
        let _ = self.shutdown_tx.send(true);
        let _ = self.write_tx.send(InternalCommand::Close);
        self.supervisor_handle.lock().await.abort();
        Ok(())
    }

    /// Waits until the connection reaches [`ConnectionState::Connected`].
    pub async fn wait_for_connection(&self) {
        let mut watcher = self.state_watcher();
        if *watcher.rx.borrow() == ConnectionState::Connected {
            return;
        }
        loop {
            match watcher.run_till_event().await {
                ConnectionState::Connected => return,
                ConnectionState::Exited => return,
                _ => continue,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WsSubscription impl
// ---------------------------------------------------------------------------

impl WsSubscription for Subscription {
    fn to_request(&self, request_id: &mut i32) -> Result<String, serde_json::Error> {
        let req = match self {
            Subscription::Level { symbol, level } => WsRequest {
                request_id: *request_id,
                request: MarketdataRequest::Subscribe {
                    symbol: symbol.as_str(),
                    level: *level,
                },
            },
            Subscription::Candles { symbol, width } => WsRequest {
                request_id: *request_id,
                request: MarketdataRequest::SubscribeCandles {
                    symbol: symbol.as_str(),
                    width: *width,
                },
            },
            Subscription::BboCandles { symbol, width } => WsRequest {
                request_id: *request_id,
                request: MarketdataRequest::SubscribeBboCandles {
                    symbol: symbol.as_str(),
                    width: *width,
                },
            },
        };
        *request_id += 1;
        serde_json::to_string(&req)
    }
}
