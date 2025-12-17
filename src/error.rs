//! Error types for actr-kotlin

/// Error type for actr-kotlin operations
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ActrKotlinError {
    #[error("Configuration error: {msg}")]
    ConfigError { msg: String },

    #[error("Connection error: {msg}")]
    ConnectionError { msg: String },

    #[error("RPC error: {msg}")]
    RpcError { msg: String },

    #[error("State error: {msg}")]
    StateError { msg: String },

    #[error("Internal error: {msg}")]
    InternalError { msg: String },

    #[error("Timeout error: {msg}")]
    TimeoutError { msg: String },

    #[error("Workload error: {msg}")]
    WorkloadError { msg: String },
}

pub type ActrKotlinResult<T> = Result<T, ActrKotlinError>;

impl From<actr_protocol::ProtocolError> for ActrKotlinError {
    fn from(e: actr_protocol::ProtocolError) -> Self {
        ActrKotlinError::RpcError {
            msg: e.to_string(),
        }
    }
}
