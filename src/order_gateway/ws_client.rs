use crate::protocol::order_gateway::*;
use crate::protocol::ws::Request as WsRequest;
pub use crate::types::ws::ConnectionState;
pub use crate::ws_utils::ConnectionStateWatcher;
use crate::{
    types::ws::{InternalCommand, TokenRefreshFn, WsClientError},
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
// Subscription tracking (no replay for order gateway — stateless on reconnect)
// ---------------------------------------------------------------------------

struct NoSubscription;

impl WsSubscription for NoSubscription {
    fn to_request(&self, _request_id: &mut i32) -> Result<String, serde_json::Error> {
        Ok(String::new())
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct OrderGatewayWsClient {
    write_tx: UnboundedSender<InternalCommand>,
    pub connection_state_rx: watch::Receiver<ConnectionState>,
    pub event_receiver: mpsc::Receiver<OrderGatewayEvent>,
    pending_requests: PendingRequests,
    next_request_id: i32,
    shutdown_tx: watch::Sender<bool>,
    supervisor_handle: Arc<Mutex<JoinHandle<()>>>,
    current_connection_state: Arc<Mutex<ConnectionState>>,
}

impl OrderGatewayWsClient {
    /// Connect to the order gateway websocket using the standard path derivation.
    pub async fn connect(
        base_url: Url,
        token_refresh: TokenRefreshFn,
    ) -> Result<Self, ClientError> {
        Self::connect_inner(base_url, token_refresh, false).await
    }

    /// Connect with cancel-on-disconnect enabled.
    pub async fn connect_with_cancel_on_disconnect(
        base_url: Url,
        token_refresh: TokenRefreshFn,
    ) -> Result<Self, ClientError> {
        Self::connect_inner(base_url, token_refresh, true).await
    }

    async fn connect_inner(
        base_url: Url,
        token_refresh: TokenRefreshFn,
        cancel_on_disconnect: bool,
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
        let mut url = ws_base_url.join("orders/ws")?;
        if cancel_on_disconnect {
            url.query_pairs_mut()
                .append_pair("cancel_on_disconnect", "true");
        }
        Self::connect_to_url(url, token_refresh).await
    }

    /// Connect to an order gateway websocket at a specific URL.
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

        let (event_sender, event_receiver) = mpsc::channel(100);
        let (write_tx, write_rx) = mpsc::unbounded_channel::<InternalCommand>();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (connection_state_tx, connection_state_rx) =
            watch::channel::<ConnectionState>(ConnectionState::Disconnected);

        let pending_requests: PendingRequests = Arc::new(DashMap::new());
        let subscriptions = Arc::new(tokio::sync::RwLock::new(Vec::<NoSubscription>::new()));

        let supervisor_handle = tokio::spawn(connection_supervisor(
            url.to_string(),
            token_refresh,
            write_rx,
            shutdown_rx,
            event_sender,
            subscriptions,
            connection_state_tx,
            pending_requests.clone(),
        ));

        Ok(Self {
            write_tx,
            connection_state_rx,
            event_receiver,
            pending_requests,
            next_request_id: 1,
            shutdown_tx,
            supervisor_handle: Arc::new(Mutex::new(supervisor_handle)),
            current_connection_state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
        })
    }

    /// Returns an independent [`ConnectionStateWatcher`] for use inside `tokio::select!`.
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

    /// Build, log, and send a request, returning the assigned request ID.
    async fn send_request(&mut self, request: OrderGatewayRequest) -> Result<i32, ClientError> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let req = WsRequest {
            request_id,
            request,
        };
        let payload = serde_json::to_string(&req)?;
        trace!("sending request: {payload}");
        self.send_raw(payload).await?;
        Ok(request_id)
    }

    /// Send a request and await its response, deserializing the result into `R`.
    async fn send_request_await<R>(
        &mut self,
        request: OrderGatewayRequest,
    ) -> Result<R, ClientError>
    where
        R: serde::de::DeserializeOwned,
    {
        let (tx, rx) = oneshot::channel::<Vec<u8>>();
        let request_id = self.next_request_id;
        self.pending_requests.insert(request_id, tx);
        if let Err(e) = self.send_request(request).await {
            self.pending_requests.remove(&request_id);
            return Err(e);
        }
        let bytes = rx.await.map_err(|e| ClientError::Transport(Box::new(e)))?;
        // Parse via RawValue to avoid the Default bound on R imposed by #[serde(default)]
        let envelope: crate::protocol::ws::Response<Box<serde_json::value::RawValue>> =
            serde_json::from_slice(&bytes)?;
        let raw = envelope.response.ok_or_else(|| {
            ClientError::Transport(
                format!(
                    "empty response for request {request_id}: {:?}",
                    envelope.error
                )
                .into(),
            )
        })?;
        Ok(serde_json::from_str(raw.get())?)
    }

    // ---------------------------------------------------------------------------
    // Public API
    // ---------------------------------------------------------------------------

    pub async fn get_open_orders(&mut self) -> Result<GetOpenOrdersResponse, ClientError> {
        self.send_request_await(OrderGatewayRequest::GetOpenOrders(GetOpenOrdersRequest {}))
            .await
    }

    pub async fn place_order(
        &mut self,
        req: crate::types::PlaceOrder,
    ) -> Result<PlaceOrderResponse, ClientError> {
        self.send_request_await(OrderGatewayRequest::PlaceOrder(req.into()))
            .await
    }

    pub async fn cancel_order(
        &mut self,
        order_id: &crate::OrderId,
    ) -> Result<CancelOrderResponse, ClientError> {
        self.send_request_await(OrderGatewayRequest::CancelOrder(CancelOrderRequest {
            order_id: order_id.clone(),
        }))
        .await
    }

    pub async fn cancel_all_orders(
        &mut self,
        symbol: Option<&str>,
    ) -> Result<CancelAllOrdersResponse, ClientError> {
        self.send_request_await(OrderGatewayRequest::CancelAllOrders(
            CancelAllOrdersRequest {
                symbol: symbol.map(|s| s.to_string()),
            },
        ))
        .await
    }

    pub async fn replace_order(
        &mut self,
        req: ReplaceOrderRequest,
    ) -> Result<ReplaceOrderResponse, ClientError> {
        self.send_request_await(OrderGatewayRequest::ReplaceOrder(req))
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
