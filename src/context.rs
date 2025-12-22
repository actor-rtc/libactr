use actr_framework::{Bytes, Context, DataStream, Dest};
use actr_protocol::{ActrId, PayloadType, RpcRequest};
use actr_runtime::{ActorResult, MediaSample, context::RuntimeContext};
use futures_util::future::BoxFuture;
use std::any::TypeId;
use std::sync::Arc;

use crate::{ActrError, ActrResult};

/// Context provided to the workload
#[derive(uniffi::Object, Clone)]
pub struct ContextBridge {
    pub(crate) inner: RuntimeContext,
}

impl ContextBridge {
    pub fn new(inner: RuntimeContext) -> Arc<Self> {
        Arc::new(Self { inner })
    }

    /// Try to create a ContextBridge from a generic Context implementation.
    ///
    /// This performs a runtime type check and fails if the context is not a
    /// `RuntimeContext`.
    pub fn try_from_context<C: Context + 'static>(ctx: &C) -> ActrResult<Arc<Self>> {
        if TypeId::of::<C>() != TypeId::of::<RuntimeContext>() {
            return Err(ActrError::InternalError {
                msg: format!(
                    "Context type mismatch: expected RuntimeContext, got {}",
                    std::any::type_name::<C>()
                ),
            });
        }

        let runtime_ctx = unsafe { &*(ctx as *const C as *const RuntimeContext) };
        Ok(Arc::new(Self {
            inner: runtime_ctx.clone(),
        }))
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl ContextBridge {
    /// Call a remote actor via RPC (simplified for FFI)
    ///
    /// # Arguments
    /// - `target`: Target actor ID
    /// - `route_key`: RPC route key (e.g., "echo.EchoService.Echo")
    /// - `payload_type`: Payload transmission type (RpcReliable, RpcSignal, etc.)
    /// - `payload`: Request payload bytes (protobuf encoded)
    /// - `timeout_ms`: Timeout in milliseconds
    ///
    /// # Returns
    /// Response payload bytes (protobuf encoded)
    pub async fn call_raw(
        &self,
        target: crate::types::ActrId,
        route_key: String,
        payload_type: crate::types::PayloadType,
        payload: Vec<u8>,
        timeout_ms: i64,
    ) -> crate::error::ActrResult<Vec<u8>> {
        let target_id: ActrId = target.into();
        let proto_payload_type: PayloadType = payload_type.into();
        let resp = self
            .inner
            .call_raw(
                &Dest::Actor(target_id),
                route_key,
                proto_payload_type,
                Bytes::from(payload),
                timeout_ms,
            )
            .await?;
        Ok(resp.to_vec())
    }

    /// Send a one-way message to an actor (fire-and-forget)
    ///
    /// # Arguments
    /// - `target`: Target actor ID
    /// - `route_key`: RPC route key (e.g., "echo.EchoService.Echo")
    /// - `payload_type`: Payload transmission type (RpcReliable, RpcSignal, etc.)
    /// - `payload`: Message payload bytes (protobuf encoded)
    pub async fn tell_raw(
        &self,
        target: crate::types::ActrId,
        route_key: String,
        payload_type: crate::types::PayloadType,
        payload: Vec<u8>,
    ) -> crate::error::ActrResult<()> {
        let target_id: ActrId = target.into();
        let proto_payload_type: PayloadType = payload_type.into();
        self.inner
            .tell_raw(
                &Dest::Actor(target_id),
                route_key,
                proto_payload_type,
                Bytes::from(payload),
            )
            .await?;
        Ok(())
    }

    /// Send a DataStream to a remote actor (Fast Path)
    ///
    /// # Arguments
    /// - `target`: Target actor ID
    /// - `chunk`: DataStream containing stream_id, sequence, payload, etc.
    pub async fn send_data_stream_raw(
        &self,
        target: crate::types::ActrId,
        chunk: crate::types::DataStream,
    ) -> crate::error::ActrResult<()> {
        let target_id: ActrId = target.into();
        let chunk: DataStream = chunk.into();
        self.inner
            .send_data_stream(&Dest::Actor(target_id), chunk)
            .await?;
        Ok(())
    }

    /// Discover an actor of the specified type
    ///
    /// # Arguments
    /// - `target_type`: Actor type to discover (manufacturer + name)
    ///
    /// # Returns
    /// The ActrId of a discovered actor
    pub async fn discover(
        &self,
        target_type: crate::types::ActrType,
    ) -> crate::error::ActrResult<crate::types::ActrId> {
        let proto_type: actr_protocol::ActrType = target_type.into();
        let id = self.inner.discover_route_candidate(&proto_type).await?;
        Ok(id.into())
    }
}

#[async_trait::async_trait]
impl Context for ContextBridge {
    fn self_id(&self) -> &ActrId {
        self.inner.self_id()
    }

    fn caller_id(&self) -> Option<&ActrId> {
        self.inner.caller_id()
    }

    fn request_id(&self) -> &str {
        self.inner.request_id()
    }

    async fn call<R: RpcRequest>(&self, target: &Dest, request: R) -> ActorResult<R::Response> {
        self.inner.call(target, request).await
    }

    // TODO: this func need fix
    async fn call_raw(
        &self,
        target: &ActrId,
        route_key: &str,
        payload: Bytes,
    ) -> ActorResult<Bytes> {
        self.inner
            .call_raw(
                &Dest::Actor(target.clone()),
                route_key.to_string(),
                PayloadType::RpcReliable,
                payload,
                30000,
            )
            .await
    }

    async fn tell<R: RpcRequest>(&self, target: &Dest, message: R) -> ActorResult<()> {
        self.inner.tell(target, message).await
    }

    async fn discover_route_candidate(
        &self,
        target_type: &actr_protocol::ActrType,
    ) -> ActorResult<ActrId> {
        self.inner.discover_route_candidate(target_type).await
    }

    async fn register_stream<F>(&self, stream_id: String, callback: F) -> ActorResult<()>
    where
        F: Fn(DataStream, ActrId) -> BoxFuture<'static, ActorResult<()>> + Send + Sync + 'static,
    {
        self.inner.register_stream(stream_id, callback).await
    }

    async fn unregister_stream(&self, stream_id: &str) -> ActorResult<()> {
        self.inner.unregister_stream(stream_id).await
    }

    async fn send_data_stream(&self, target: &Dest, chunk: DataStream) -> ActorResult<()> {
        self.inner.send_data_stream(target, chunk).await
    }

    async fn register_media_track<F>(&self, track_id: String, callback: F) -> ActorResult<()>
    where
        F: Fn(MediaSample, ActrId) -> BoxFuture<'static, ActorResult<()>> + Send + Sync + 'static,
    {
        self.inner.register_media_track(track_id, callback).await
    }

    async fn unregister_media_track(&self, track_id: &str) -> ActorResult<()> {
        self.inner.unregister_media_track(track_id).await
    }

    async fn send_media_sample(
        &self,
        target: &Dest,
        track_id: &str,
        sample: MediaSample,
    ) -> ActorResult<()> {
        self.inner.send_media_sample(target, track_id, sample).await
    }
}
