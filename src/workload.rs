//! Dynamic Workload implementation for Kotlin callbacks

use crate::ActrKotlinError;
use crate::types::{ActrId, ActrType};
use actr_framework::{Bytes, Context, MessageDispatcher, Workload};
use actr_protocol::{ActorResult, ProtocolError, RpcEnvelope};
use async_trait::async_trait;
use std::sync::Arc;

/// Internal route key for RPC proxy requests
pub const RPC_PROXY_ROUTE: &str = "__internal/rpc_proxy";

/// Callback interface for Kotlin to implement workload logic
#[uniffi::export(callback_interface)]
pub trait WorkloadCallback: Send + Sync {
    /// Return the actor type for this workload
    fn actor_type(&self) -> ActrType;

    /// Handle an incoming RPC request
    fn on_request(
        &self,
        route_key: String,
        payload: Vec<u8>,
        ctx: Arc<CallContext>,
    ) -> Result<Vec<u8>, ActrKotlinError>;

    /// Called when the actor starts
    fn on_start(&self, ctx: Arc<CallContext>);

    /// Called when the actor stops
    fn on_stop(&self);
}

/// Call context exposed to Kotlin for making RPC calls
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
    pub(crate) fn new(self_id: ActrId, caller_id: Option<ActrId>, request_id: String) -> Self {
        Self {
            self_id,
            caller_id,
            request_id,
        }
    }

    pub(crate) fn from_context<C: Context>(ctx: &C) -> Self {
        Self {
            self_id: ctx.self_id().into(),
            caller_id: ctx.caller_id().map(|id| id.into()),
            request_id: ctx.request_id().to_string(),
        }
    }
}

/// Dynamic workload that wraps a Kotlin callback
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

    fn actor_type(&self) -> actr_protocol::ActrType {
        self.callback.actor_type().into()
    }
}

/// Dynamic dispatcher that routes messages to the Kotlin callback
pub struct DynamicDispatcher;

#[async_trait]
impl MessageDispatcher for DynamicDispatcher {
    type Workload = DynamicWorkload;

    async fn dispatch<C: Context>(
        workload: &Self::Workload,
        envelope: RpcEnvelope,
        ctx: &C,
    ) -> ActorResult<Bytes> {
        // Check if this is an internal RPC proxy request
        if envelope.route_key == RPC_PROXY_ROUTE {
            return handle_rpc_proxy(&envelope, ctx).await;
        }

        let call_ctx = Arc::new(CallContext::from_context(ctx));
        let payload = envelope.payload.map(|p| p.to_vec()).unwrap_or_default();

        let response = workload
            .callback
            .on_request(envelope.route_key, payload, call_ctx)
            .map_err(|e| ProtocolError::TransportError(format!("Kotlin callback error: {}", e)))?;

        Ok(Bytes::from(response))
    }
}

/// Decode a prost bytes field (field 1, wire type 2)
///
/// The format is: tag (0x0a = field 1 + wire type 2) + varint length + data
fn decode_prost_bytes_field(data: &[u8]) -> ActorResult<Vec<u8>> {
    if data.is_empty() {
        return Err(ProtocolError::DecodeError(
            "Empty prost bytes field".to_string(),
        ));
    }

    let mut offset = 0;

    // Read tag (should be 0x0a for field 1, wire type 2)
    let tag = data[offset];
    offset += 1;

    if tag != 0x0a {
        return Err(ProtocolError::DecodeError(format!(
            "Unexpected tag: expected 0x0a, got 0x{:02x}",
            tag
        )));
    }

    // Read varint length
    let (length, varint_len) = decode_varint(&data[offset..])?;
    offset += varint_len;

    // Read the actual data
    if offset + length as usize > data.len() {
        return Err(ProtocolError::DecodeError(format!(
            "Prost bytes field: data too short, need {} bytes, have {}",
            offset + length as usize,
            data.len()
        )));
    }

    Ok(data[offset..offset + length as usize].to_vec())
}

/// Decode a varint from bytes, returning (value, bytes_consumed)
fn decode_varint(data: &[u8]) -> ActorResult<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut offset = 0;

    loop {
        if offset >= data.len() {
            return Err(ProtocolError::DecodeError(
                "Varint: unexpected end of data".to_string(),
            ));
        }

        let byte = data[offset];
        offset += 1;

        result |= ((byte & 0x7f) as u64) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, offset));
        }

        shift += 7;
        if shift >= 64 {
            return Err(ProtocolError::DecodeError("Varint: too large".to_string()));
        }
    }
}

/// Handle RPC proxy requests from Kotlin
///
/// The envelope payload is prost-encoded with field 1 containing our proxy payload.
/// Our proxy payload format (simple binary):
/// - 8 bytes: target ActrId serialNumber (big-endian u64)
/// - 4 bytes: target realm_id (big-endian u32)
/// - 2 bytes: manufacturer length (big-endian u16)
/// - N bytes: manufacturer string
/// - 2 bytes: name length (big-endian u16)
/// - N bytes: name string
/// - 2 bytes: route_key length (big-endian u16)
/// - N bytes: route_key string
/// - remaining: actual RPC payload
async fn handle_rpc_proxy<C: Context>(envelope: &RpcEnvelope, ctx: &C) -> ActorResult<Bytes> {
    use tracing::{debug, info};

    let envelope_payload = envelope
        .payload
        .as_ref()
        .ok_or_else(|| ProtocolError::DecodeError("RPC proxy: missing payload".to_string()))?;

    // The payload is prost-encoded: field 1 (tag 0x0a) + length + actual data
    // We need to decode it first
    let proxy_payload = decode_prost_bytes_field(envelope_payload)?;

    // Parse the proxy request
    let payload = &proxy_payload;
    let mut offset = 0;

    // Read serial_number (8 bytes, big-endian u64)
    if payload.len() < offset + 8 {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for serial_number".to_string(),
        ));
    }
    let serial_number = u64::from_be_bytes(payload[offset..offset + 8].try_into().unwrap());
    offset += 8;

    // Read realm_id (4 bytes, big-endian u32)
    if payload.len() < offset + 4 {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for realm_id".to_string(),
        ));
    }
    let realm_id = u32::from_be_bytes(payload[offset..offset + 4].try_into().unwrap());
    offset += 4;

    // Read manufacturer
    if payload.len() < offset + 2 {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for manufacturer length".to_string(),
        ));
    }
    let mfr_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
    offset += 2;
    if payload.len() < offset + mfr_len {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for manufacturer".to_string(),
        ));
    }
    let manufacturer = String::from_utf8_lossy(&payload[offset..offset + mfr_len]).to_string();
    offset += mfr_len;

    // Read name
    if payload.len() < offset + 2 {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for name length".to_string(),
        ));
    }
    let name_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
    offset += 2;
    if payload.len() < offset + name_len {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for name".to_string(),
        ));
    }
    let name = String::from_utf8_lossy(&payload[offset..offset + name_len]).to_string();
    offset += name_len;

    // Read route_key
    if payload.len() < offset + 2 {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for route_key length".to_string(),
        ));
    }
    let route_len = u16::from_be_bytes(payload[offset..offset + 2].try_into().unwrap()) as usize;
    offset += 2;
    if payload.len() < offset + route_len {
        return Err(ProtocolError::DecodeError(
            "RPC proxy: payload too short for route_key".to_string(),
        ));
    }
    let route_key = String::from_utf8_lossy(&payload[offset..offset + route_len]).to_string();
    offset += route_len;

    // Remaining is the actual RPC payload
    let rpc_payload = Bytes::copy_from_slice(&payload[offset..]);

    // Construct target ActrId
    let target_id = actr_protocol::ActrId {
        realm: actr_protocol::Realm { realm_id },
        serial_number,
        r#type: actr_protocol::ActrType {
            manufacturer: manufacturer.clone(),
            name: name.clone(),
        },
    };

    info!(
        "RPC proxy: calling remote target={}/{} #{}, route_key={}, payload_len={}",
        manufacturer,
        name,
        serial_number,
        route_key,
        rpc_payload.len()
    );

    // Use Context.call_raw() to make the remote call
    let response = ctx.call_raw(&target_id, &route_key, rpc_payload).await?;

    debug!(
        "RPC proxy: got response from remote, response_len={}",
        response.len()
    );

    // Encode response as prost field 1 (bytes) for RpcProxyResponse::decode
    let encoded_response = encode_prost_bytes_field(&response);

    Ok(Bytes::from(encoded_response))
}

/// Encode data as a prost bytes field (field 1, wire type 2)
///
/// The format is: tag (0x0a = field 1 + wire type 2) + varint length + data
fn encode_prost_bytes_field(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(1 + 10 + data.len()); // tag + max varint + data

    // Tag: field 1, wire type 2 (length-delimited) = 0x0a
    result.push(0x0a);

    // Length as varint
    encode_varint(data.len() as u64, &mut result);

    // Data
    result.extend_from_slice(data);

    result
}

/// Encode a u64 as varint
fn encode_varint(mut value: u64, buf: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            buf.push(byte);
            return;
        } else {
            buf.push(byte | 0x80);
        }
    }
}
