//! Runtime wrappers for UniFFI export

use crate::error::{ActrKotlinError, ActrKotlinResult};
use crate::init_logging;
use crate::types::{ActrConfig, ActrId, ActrType};
use crate::workload::{DynamicWorkload, WorkloadCallback};
use actr_runtime::{ActrNode, ActrRef, ActrSystem};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::{debug, error, info};

/// Callback for async discovery results
#[uniffi::export(callback_interface)]
pub trait DiscoveryCallback: Send + Sync {
    fn on_discovered(&self, actors: Vec<ActrId>);
    fn on_error(&self, error: String);
}

/// Callback for async RPC results
#[uniffi::export(callback_interface)]
pub trait RpcCallback: Send + Sync {
    fn on_success(&self, response: Vec<u8>);
    fn on_error(&self, error: String);
}

/// Wrapper for ActrSystem - the entry point for creating actors
#[derive(uniffi::Object)]
pub struct ActrSystemWrapper {
    runtime: Arc<Runtime>,
    inner: Mutex<Option<ActrSystem>>,
    config: ActrConfig,
}

#[uniffi::export]
impl ActrSystemWrapper {
    /// Create a new ActrSystem
    #[uniffi::constructor]
    pub fn create(config: ActrConfig) -> ActrKotlinResult<Arc<Self>> {
        // Initialize logging first
        init_logging();

        info!(
            "Creating ActrSystem with signaling_url={}, realm_id={}",
            config.signaling_url, config.realm_id
        );

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .map_err(|e| {
                error!("Failed to create Tokio runtime: {}", e);
                ActrKotlinError::InternalError {
                    msg: format!("Failed to create Tokio runtime: {e}"),
                }
            })?;

        let actr_config = config.to_actr_config();
        debug!(
            "ActrConfig created: visible_in_discovery={}",
            actr_config.visible_in_discovery
        );

        let system = runtime
            .block_on(async {
                info!("Connecting to signaling server...");
                ActrSystem::new(actr_config).await
            })
            .map_err(|e| {
                error!("Failed to create ActrSystem: {}", e);
                ActrKotlinError::InternalError {
                    msg: format!("Failed to create ActrSystem: {e}"),
                }
            })?;

        info!("ActrSystem created successfully");

        Ok(Arc::new(Self {
            runtime: Arc::new(runtime),
            inner: Mutex::new(Some(system)),
            config,
        }))
    }

    /// Attach a workload and create an ActrNode
    pub fn attach(
        self: Arc<Self>,
        callback: Box<dyn WorkloadCallback>,
    ) -> ActrKotlinResult<Arc<ActrNodeWrapper>> {
        let system = self
            .inner
            .lock()
            .take()
            .ok_or_else(|| ActrKotlinError::StateError {
                msg: "ActrSystem already consumed".to_string(),
            })?;

        let workload = DynamicWorkload::new(Arc::from(callback));
        let node = system.attach(workload);

        Ok(Arc::new(ActrNodeWrapper {
            runtime: self.runtime.clone(),
            inner: Mutex::new(Some(node)),
            config: self.config.clone(),
        }))
    }
}

/// Wrapper for ActrNode - a node ready to start
#[derive(uniffi::Object)]
pub struct ActrNodeWrapper {
    runtime: Arc<Runtime>,
    inner: Mutex<Option<ActrNode<DynamicWorkload>>>,
    #[allow(dead_code)]
    config: ActrConfig,
}

#[uniffi::export]
impl ActrNodeWrapper {
    /// Start the actor node and return an ActrRef
    pub fn start(self: Arc<Self>) -> ActrKotlinResult<Arc<ActrRefWrapper>> {
        let node = self
            .inner
            .lock()
            .take()
            .ok_or_else(|| ActrKotlinError::StateError {
                msg: "ActrNode already started".to_string(),
            })?;

        let runtime = self.runtime.clone();

        let actr_ref = runtime
            .block_on(async { node.start().await })
            .map_err(|e| ActrKotlinError::ConnectionError {
                msg: format!("Failed to start actor: {e}"),
            })?;

        Ok(Arc::new(ActrRefWrapper {
            runtime,
            inner: actr_ref,
        }))
    }
}

/// Wrapper for ActrRef - a reference to a running actor
#[derive(uniffi::Object)]
pub struct ActrRefWrapper {
    runtime: Arc<Runtime>,
    inner: ActrRef<DynamicWorkload>,
}

#[uniffi::export]
impl ActrRefWrapper {
    /// Get the actor's ID
    pub fn actor_id(&self) -> ActrId {
        self.inner.actor_id().into()
    }

    /// Discover actors of the specified type (blocking)
    pub fn discover_sync(
        &self,
        target_type: ActrType,
        count: u32,
    ) -> ActrKotlinResult<Vec<ActrId>> {
        let target_proto: actr_protocol::ActrType = target_type.clone().into();

        info!(
            "discover_sync: looking for {}/{} (count={})",
            target_type.manufacturer, target_type.name, count
        );

        self.runtime.block_on(async {
            match self
                .inner
                .discover_route_candidates(&target_proto, count)
                .await
            {
                Ok(ids) => {
                    info!("discover_sync: found {} candidates", ids.len());
                    for id in &ids {
                        debug!(
                            "  - {}/{} #{}",
                            id.r#type.manufacturer, id.r#type.name, id.serial_number
                        );
                    }
                    Ok(ids.into_iter().map(|id| id.into()).collect())
                }
                Err(e) => {
                    error!("discover_sync failed: {}", e);
                    Err(ActrKotlinError::RpcError {
                        msg: format!("Discovery failed: {e}"),
                    })
                }
            }
        })
    }

    /// Discover actors of the specified type (async)
    pub fn discover(
        &self,
        target_type: ActrType,
        count: u32,
        callback: Box<dyn DiscoveryCallback>,
    ) {
        let inner = self.inner.clone();
        let target_proto: actr_protocol::ActrType = target_type.into();

        self.runtime.spawn(async move {
            match inner.discover_route_candidates(&target_proto, count).await {
                Ok(ids) => {
                    let result: Vec<ActrId> = ids.into_iter().map(|id| id.into()).collect();
                    callback.on_discovered(result);
                }
                Err(e) => {
                    callback.on_error(format!("Discovery failed: {e}"));
                }
            }
        });
    }

    /// Trigger shutdown
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Wait for shutdown to complete (blocking)
    pub fn wait_for_shutdown(&self) {
        self.runtime.block_on(async {
            self.inner.wait_for_shutdown().await;
        });
    }

    /// Check if the actor is shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.inner.is_shutting_down()
    }

    /// Call a remote actor via RPC proxy (blocking)
    ///
    /// This sends a request through the local workload's RPC proxy mechanism,
    /// which forwards the call to the remote actor via WebRTC.
    ///
    /// # Arguments
    /// - `target`: Target actor ID
    /// - `route_key`: RPC route key (e.g., "echo.EchoService/Echo")
    /// - `payload`: Request payload bytes (protobuf encoded)
    ///
    /// # Returns
    /// Response payload bytes (protobuf encoded)
    pub fn call_remote_sync(
        &self,
        target: ActrId,
        route_key: String,
        payload: Vec<u8>,
    ) -> ActrKotlinResult<Vec<u8>> {
        info!(
            "call_remote_sync: target={}/{} #{}, route={}",
            target.actor_type.manufacturer, target.actor_type.name, target.serial_number, route_key
        );

        // Build the proxy request payload
        let proxy_payload = build_rpc_proxy_payload(&target, &route_key, &payload);

        self.runtime.block_on(async {
            // Create a proxy request that will be handled by DynamicDispatcher
            let request = RpcProxyRequest {
                payload: proxy_payload,
            };

            match self.inner.call(request).await {
                Ok(response) => {
                    info!(
                        "call_remote_sync: got response, len={}",
                        response.payload.len()
                    );
                    Ok(response.payload)
                }
                Err(e) => {
                    error!("call_remote_sync failed: {}", e);
                    Err(ActrKotlinError::RpcError {
                        msg: format!("Remote call failed: {e}"),
                    })
                }
            }
        })
    }
}

/// Build the RPC proxy payload format
///
/// Format:
/// - 8 bytes: serial_number (u64 big-endian)
/// - 4 bytes: realm_id (u32 big-endian)
/// - 2 bytes: manufacturer length (u16 big-endian)
/// - N bytes: manufacturer string
/// - 2 bytes: name length (u16 big-endian)
/// - N bytes: name string
/// - 2 bytes: route_key length (u16 big-endian)
/// - N bytes: route_key string
/// - remaining: actual payload
fn build_rpc_proxy_payload(target: &ActrId, route_key: &str, payload: &[u8]) -> Vec<u8> {
    let manufacturer = target.actor_type.manufacturer.as_bytes();
    let name = target.actor_type.name.as_bytes();
    let route = route_key.as_bytes();

    let total_len =
        8 + 4 + 2 + manufacturer.len() + 2 + name.len() + 2 + route.len() + payload.len();
    let mut buf = Vec::with_capacity(total_len);

    buf.extend_from_slice(&target.serial_number.to_be_bytes());
    buf.extend_from_slice(&target.realm_id.to_be_bytes());
    buf.extend_from_slice(&(manufacturer.len() as u16).to_be_bytes());
    buf.extend_from_slice(manufacturer);
    buf.extend_from_slice(&(name.len() as u16).to_be_bytes());
    buf.extend_from_slice(name);
    buf.extend_from_slice(&(route.len() as u16).to_be_bytes());
    buf.extend_from_slice(route);
    buf.extend_from_slice(payload);

    buf
}

/// RPC Proxy Request - wraps a raw payload for forwarding to remote actor
struct RpcProxyRequest {
    payload: Vec<u8>,
}

/// RPC Proxy Response - wraps raw response bytes from remote actor
struct RpcProxyResponse {
    payload: Vec<u8>,
}

impl actr_protocol::RpcRequest for RpcProxyRequest {
    type Response = RpcProxyResponse;

    fn route_key() -> &'static str {
        crate::workload::RPC_PROXY_ROUTE
    }
}

impl actr_protocol::prost::Message for RpcProxyRequest {
    fn encode_raw(&self, buf: &mut impl bytes::BufMut)
    where
        Self: Sized,
    {
        // Encode as field 1, wire type 2 (length-delimited)
        actr_protocol::prost::encoding::encode_key(
            1,
            actr_protocol::prost::encoding::WireType::LengthDelimited,
            buf,
        );
        actr_protocol::prost::encoding::encode_varint(self.payload.len() as u64, buf);
        buf.put_slice(&self.payload);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: actr_protocol::prost::encoding::WireType,
        buf: &mut impl bytes::Buf,
        ctx: actr_protocol::prost::encoding::DecodeContext,
    ) -> Result<(), actr_protocol::prost::DecodeError>
    where
        Self: Sized,
    {
        if tag == 1 {
            actr_protocol::prost::encoding::bytes::merge(wire_type, &mut self.payload, buf, ctx)
        } else {
            actr_protocol::prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        1 + actr_protocol::prost::encoding::encoded_len_varint(self.payload.len() as u64)
            + self.payload.len()
    }

    fn clear(&mut self) {
        self.payload.clear();
    }
}

impl Default for RpcProxyRequest {
    fn default() -> Self {
        Self {
            payload: Vec::new(),
        }
    }
}

impl actr_protocol::prost::Message for RpcProxyResponse {
    fn encode_raw(&self, buf: &mut impl bytes::BufMut)
    where
        Self: Sized,
    {
        // Encode as field 1, wire type 2 (length-delimited)
        // tag = (1 << 3) | 2 = 0x0a
        actr_protocol::prost::encoding::encode_key(
            1,
            actr_protocol::prost::encoding::WireType::LengthDelimited,
            buf,
        );
        actr_protocol::prost::encoding::encode_varint(self.payload.len() as u64, buf);
        buf.put_slice(&self.payload);
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: actr_protocol::prost::encoding::WireType,
        buf: &mut impl bytes::Buf,
        ctx: actr_protocol::prost::encoding::DecodeContext,
    ) -> Result<(), actr_protocol::prost::DecodeError>
    where
        Self: Sized,
    {
        // Only handle field 1 (our payload field)
        if tag == 1 {
            actr_protocol::prost::encoding::bytes::merge(wire_type, &mut self.payload, buf, ctx)
        } else {
            actr_protocol::prost::encoding::skip_field(wire_type, tag, buf, ctx)
        }
    }

    fn encoded_len(&self) -> usize {
        // tag (1 byte) + length varint + payload
        1 + actr_protocol::prost::encoding::encoded_len_varint(self.payload.len() as u64)
            + self.payload.len()
    }

    fn clear(&mut self) {
        self.payload.clear();
    }
}

impl Default for RpcProxyResponse {
    fn default() -> Self {
        Self {
            payload: Vec::new(),
        }
    }
}
