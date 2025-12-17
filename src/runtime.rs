//! Runtime wrappers for UniFFI export

use crate::error::{ActrError, ActrResult};
use crate::init_logging;
use crate::types::{ActrConfig, ActrId, ActrType};
use crate::workload::{DynamicWorkload, WorkloadCallback};
use actr_runtime::{ActrNode, ActrRef, ActrSystem};
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Wrapper for ActrSystem - the entry point for creating actors
#[derive(uniffi::Object)]
pub struct ActrSystemWrapper {
    inner: Mutex<Option<ActrSystem>>,
    config: ActrConfig,
}

#[uniffi::export]
impl ActrSystemWrapper {
    /// Create a new ActrSystem
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn create(config: ActrConfig) -> ActrResult<Arc<Self>> {
        // Initialize logging first
        init_logging();

        info!(
            "Creating ActrSystem with signaling_url={}, realm_id={}",
            config.signaling_url, config.realm_id
        );

        let actr_config = config.to_actr_config()?;
        debug!(
            "ActrConfig created: visible_in_discovery={}",
            actr_config.visible_in_discovery
        );

        info!("Connecting to signaling server...");
        let system = ActrSystem::new(actr_config).await.map_err(|e| {
            error!("Failed to create ActrSystem: {}", e);
            ActrError::InternalError {
                msg: format!("Failed to create ActrSystem: {e}"),
            }
        })?;

        info!("ActrSystem created successfully");

        Ok(Arc::new(Self {
            inner: Mutex::new(Some(system)),
            config,
        }))
    }

    /// Attach a workload and create an ActrNode
    pub fn attach(
        self: Arc<Self>,
        callback: Box<dyn WorkloadCallback>,
    ) -> ActrResult<Arc<ActrNodeWrapper>> {
        let system = self
            .inner
            .lock()
            .take()
            .ok_or_else(|| ActrError::StateError {
                msg: "ActrSystem already consumed".to_string(),
            })?;

        let workload = DynamicWorkload::new(Arc::from(callback));
        let node = system.attach(workload);

        Ok(Arc::new(ActrNodeWrapper {
            inner: Mutex::new(Some(node)),
            config: self.config.clone(),
        }))
    }
}

/// Wrapper for ActrNode - a node ready to start
#[derive(uniffi::Object)]
pub struct ActrNodeWrapper {
    inner: Mutex<Option<ActrNode<DynamicWorkload>>>,
    #[allow(dead_code)]
    config: ActrConfig,
}

#[uniffi::export(async_runtime = "tokio")]
impl ActrNodeWrapper {
    /// Start the actor node and return an ActrRef
    pub async fn start(self: Arc<Self>) -> ActrResult<Arc<ActrRefWrapper>> {
        let node = self
            .inner
            .lock()
            .take()
            .ok_or_else(|| ActrError::StateError {
                msg: "ActrNode already started".to_string(),
            })?;

        let actr_ref = node.start().await.map_err(|e| ActrError::ConnectionError {
            msg: format!("Failed to start actor: {e}"),
        })?;

        Ok(Arc::new(ActrRefWrapper { inner: actr_ref }))
    }
}

/// Wrapper for ActrRef - a reference to a running actor
#[derive(uniffi::Object)]
pub struct ActrRefWrapper {
    inner: ActrRef<DynamicWorkload>,
}

#[uniffi::export(async_runtime = "tokio")]
impl ActrRefWrapper {
    /// Get the actor's ID
    pub fn actor_id(&self) -> ActrId {
        self.inner.actor_id().into()
    }

    /// Discover actors of the specified type
    pub async fn discover(&self, target_type: ActrType, count: u32) -> ActrResult<Vec<ActrId>> {
        let target_proto: actr_protocol::ActrType = target_type.clone().into();

        info!(
            "discover: looking for {}/{} (count={})",
            target_type.manufacturer, target_type.name, count
        );

        match self
            .inner
            .discover_route_candidates(&target_proto, count)
            .await
        {
            Ok(ids) => {
                info!("discover: found {} candidates", ids.len());
                for id in &ids {
                    debug!(
                        "  - {}/{} #{}",
                        id.r#type.manufacturer, id.r#type.name, id.serial_number
                    );
                }
                Ok(ids.into_iter().map(|id| id.into()).collect())
            }
            Err(e) => {
                error!("discover failed: {}", e);
                Err(ActrError::RpcError {
                    msg: format!("Discovery failed: {e}"),
                })
            }
        }
    }

    /// Trigger shutdown
    pub fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Wait for shutdown to complete
    pub async fn wait_for_shutdown(&self) {
        self.inner.wait_for_shutdown().await;
    }

    /// Check if the actor is shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.inner.is_shutting_down()
    }

    /// Call a remote actor via RPC proxy
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
    pub async fn call_remote(
        &self,
        target: ActrId,
        route_key: String,
        payload: Vec<u8>,
    ) -> ActrResult<Vec<u8>> {
        info!(
            "call_remote: target={}/{} #{}, route={}",
            target.actor_type.manufacturer, target.actor_type.name, target.serial_number, route_key
        );

        // Build the proxy request payload
        let proxy_payload = build_rpc_proxy_payload(&target, &route_key, &payload);

        // Create a proxy request that will be handled by DynamicDispatcher
        let request = RpcProxyRequest {
            payload: proxy_payload,
        };

        match self.inner.call(request).await {
            Ok(response) => {
                info!("call_remote: got response, len={}", response.payload.len());
                Ok(response.payload)
            }
            Err(e) => {
                error!("call_remote failed: {}", e);
                Err(ActrError::RpcError {
                    msg: format!("Remote call failed: {e}"),
                })
            }
        }
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
