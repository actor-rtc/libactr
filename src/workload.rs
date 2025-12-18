//! Dynamic Workload implementation for callback interfaces

use crate::ActrError;
use crate::types::ActrId;
use actr_framework::{Bytes, Context, MessageDispatcher, Workload};
use actr_protocol::{ActorResult, RpcEnvelope};
use async_trait::async_trait;
use std::sync::Arc;

/// Callback interface for implementing workload logic
#[uniffi::export(callback_interface)]
#[async_trait]
pub trait WorkloadCallback: Send + Sync {
    async fn server_id(&self) -> ActrId;

    /// Handle an incoming RPC request
    async fn on_request(&self, route_key: String, payload: Vec<u8>) -> Result<Vec<u8>, ActrError>;
}

/// Dynamic workload that wraps a callback interface
#[derive(uniffi::Object)]
pub struct DynamicWorkload {
    callback: Arc<dyn WorkloadCallback>,
}

impl DynamicWorkload {
    pub fn new(callback: Arc<dyn WorkloadCallback>) -> Self {
        Self { callback }
    }

    pub fn callback(&self) -> &Arc<dyn WorkloadCallback> {
        &self.callback
    }
}

impl Workload for DynamicWorkload {
    type Dispatcher = DynamicDispatcher;
}

/// Dynamic dispatcher that routes messages to the callback interface
pub struct DynamicDispatcher;

#[async_trait]
impl MessageDispatcher for DynamicDispatcher {
    type Workload = DynamicWorkload;

    async fn dispatch<C: Context>(
        workload: &Self::Workload,
        envelope: RpcEnvelope,
        ctx: &C,
    ) -> ActorResult<Bytes> {
        let payload = envelope.payload.map(|p| p.to_vec()).unwrap_or_default();

        let server_id: actr_protocol::ActrId = workload.callback.server_id().await.into();

        let response = ctx
            .call_raw(
                &server_id,
                envelope.route_key.as_str(),
                Bytes::from(payload),
            )
            .await?;

        Ok(Bytes::from(response))
    }
}
