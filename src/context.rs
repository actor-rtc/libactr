use actr_framework::{Bytes, Context, DataStream, Dest, MediaSample};
use actr_protocol::{ActorResult, ActrId, RpcRequest};
use actr_runtime::context::RuntimeContext;
use futures_util::future::BoxFuture;
use std::sync::Arc;

/// Context provided to the workload
#[derive(uniffi::Object, Clone)]
pub struct ContextBridge {
    pub(crate) inner: RuntimeContext,
}

impl ContextBridge {
    pub fn new(inner: RuntimeContext) -> Arc<Self> {
        Arc::new(Self { inner })
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl ContextBridge {
    /// Call a remote actor via RPC (simplified for FFI)
    pub async fn call_raw(
        &self,
        target: crate::types::ActrId,
        route_key: String,
        payload: Vec<u8>,
    ) -> crate::error::ActrResult<Vec<u8>> {
        let target_id: ActrId = target.into();
        let resp = self
            .inner
            .call_raw(&target_id, &route_key, Bytes::from(payload))
            .await?;
        Ok(resp.to_vec())
    }

    pub async fn tell_raw(
        &self,
        target: crate::types::ActrId,
        _route_key: String,
        _payload: Vec<u8>,
    ) -> crate::error::ActrResult<()> {
        let _target_id: ActrId = target.into();

        Ok(())
    }

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

    async fn call_raw(
        &self,
        target: &ActrId,
        route_key: &str,
        payload: Bytes,
    ) -> ActorResult<Bytes> {
        self.inner.call_raw(target, route_key, payload).await
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
