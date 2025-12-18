//! Dynamic Workload implementation for callback interfaces

use crate::ActrError;
use actr_framework::{Bytes, Context, MessageDispatcher, Workload};
use actr_protocol::{ActorResult, ProtocolError, RpcEnvelope};
use actr_runtime::{ActrId, ActrType};
use async_trait::async_trait;
use std::sync::Arc;

/// Callback interface for implementing workload logic
#[uniffi::export(callback_interface)]
#[async_trait]
pub trait WorkloadCallback: Send + Sync {
    /// Return the actor type for this workload
    fn actor_type(&self) -> ActrType;

    /// Handle an incoming RPC request
    async fn on_request(
        &self,
        route_key: String,
        payload: Vec<u8>,
        ctx: Arc<CallContext>,
    ) -> Result<Vec<u8>, ActrError>;

    /// Called when the actor starts
    fn on_start(&self, ctx: Arc<CallContext>);

    /// Called when the actor stops
    fn on_stop(&self);
}

/// Call context exposed for making RPC calls
#[derive(uniffi::Object)]
pub struct CallContext {
    self_id: ActrId,
    caller_id: Option<ActrId>,
    request_id: String,
}

#[uniffi::export]
impl CallContext {
    pub fn self_id(&self) -> ActrId {
        self.self_id.clone()
    }

    pub fn caller_id(&self) -> Option<ActrId> {
        self.caller_id.clone()
    }

    pub fn request_id(&self) -> String {
        self.request_id.clone()
    }
}

impl CallContext {
    pub(crate) fn from_context<C: Context>(ctx: &C) -> Self {
        Self {
            self_id: ctx.self_id().clone(),
            caller_id: ctx.caller_id().map(|id| id.clone()),
            request_id: ctx.request_id().to_string(),
        }
    }
}

/// Dynamic workload that wraps a callback interface
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
        let call_ctx = Arc::new(CallContext::from_context(ctx));
        let payload = envelope.payload.map(|p| p.to_vec()).unwrap_or_default();

        let response = workload
            .callback
            .on_request(envelope.route_key, payload, call_ctx)
            .await
            .map_err(|e| ProtocolError::TransportError(format!("Callback error: {}", e)))?;

        Ok(Bytes::from(response))
    }
}
