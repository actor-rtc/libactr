//! Runtime wrappers for UniFFI export

use crate::error::{ActrError, ActrResult};
use crate::types::{ActrId, ActrType};
use crate::workload::{DynamicWorkload, WorkloadCallback};
use actr_config::Config;
use actr_protocol::{ActrIdExt, ActrTypeExt};
use actr_runtime::{ActrNode, ActrRef, ActrSystem};
use bytes::Bytes;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Wrapper for ActrSystem - the entry point for creating actors
#[derive(uniffi::Object)]
pub struct ActrSystemWrapper {
    inner: Mutex<Option<ActrSystem>>,
    config: Config,
}

#[uniffi::export]
impl ActrSystemWrapper {
    /// Create a new ActrSystem from configuration file
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn new_from_file(config_path: String) -> ActrResult<Arc<Self>> {
        let config = actr_config::ConfigParser::from_file(&config_path).map_err(|e| {
            ActrError::ConfigError {
                msg: format!("Failed to parse config file at {}: {}", config_path, e),
            }
        })?;

        info!(
            "Creating ActrSystem with signaling_url={}, realm_id={}",
            config.signaling_url, config.realm.realm_id
        );

        info!("Connecting to signaling server...");
        let system = ActrSystem::new(config.clone()).await.map_err(|e| {
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
    config: Config,
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
        self.inner.actor_id().clone().into()
    }

    /// Discover actors of the specified type
    pub async fn discover(&self, target_type: ActrType, count: u32) -> ActrResult<Vec<ActrId>> {
        let proto_type: actr_protocol::ActrType = target_type.into();
        info!(
            "discover: looking for {} (count={count})",
            proto_type.to_string_repr(),
        );

        match self
            .inner
            .discover_route_candidates(&proto_type, count)
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
    pub async fn call(
        &self,
        target: ActrId,
        route_key: String,
        request_payload: Vec<u8>,
    ) -> ActrResult<Vec<u8>> {
        let proto_target: actr_protocol::ActrId = target.into();
        info!(
            "call_remote: target={}, route={route_key}",
            proto_target.to_string_repr()
        );

        // Send request and wait for response (target is our actor_id for logging)
        let response_bytes = self
            .inner
            .call_raw(route_key, Bytes::from(request_payload))
            .await?;

        Ok(response_bytes.to_vec())
    }
}
