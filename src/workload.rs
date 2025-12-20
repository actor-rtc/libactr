//! Dynamic Workload implementation for callback interfaces

use crate::types::ActrId;
use crate::{ActrResult, context::ContextBridge};
use actr_framework::{Bytes, Context, MessageDispatcher, Workload};
use actr_protocol::{ActorResult, RpcEnvelope};
use async_trait::async_trait;
use std::sync::Arc;

#[uniffi::export(callback_interface)]
#[async_trait::async_trait]
pub trait WorkloadBridge: Send + Sync + 'static {
    async fn server_id(&self) -> ActrId;

    async fn on_start(&self, ctx: Arc<ContextBridge>) -> ActrResult<()>;

    async fn on_stop(&self, ctx: Arc<ContextBridge>) -> ActrResult<()>;
}

/// Dynamic workload that wraps a callback interface
#[derive(uniffi::Object)]
pub struct DynamicWorkload {
    bridge: Arc<dyn WorkloadBridge>,
}

impl DynamicWorkload {
    pub fn new(bridge: Arc<dyn WorkloadBridge>) -> Self {
        Self { bridge }
    }
}

#[async_trait]
impl Workload for DynamicWorkload {
    type Dispatcher = DynamicDispatcher;

    async fn on_start<C: Context>(&self, ctx: &C) -> ActorResult<()> {
        let ctx_bridge = ContextBridge::try_from_context(ctx)
            .map_err(actr_protocol::ProtocolError::from)?;
        self.bridge
            .on_start(ctx_bridge)
            .await
            .map_err(actr_protocol::ProtocolError::from)
    }

    async fn on_stop<C: Context>(&self, ctx: &C) -> ActorResult<()> {
        let ctx_bridge = ContextBridge::try_from_context(ctx)
            .map_err(actr_protocol::ProtocolError::from)?;
        self.bridge
            .on_stop(ctx_bridge)
            .await
            .map_err(actr_protocol::ProtocolError::from)
    }
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

        let server_id: actr_protocol::ActrId = workload.bridge.server_id().await.into();

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
