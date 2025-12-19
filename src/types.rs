//! UniFFI-exported types for cross-language bindings
//!
//! These types mirror the types from actr-protocol but with UniFFI derives.
//! They provide automatic conversion to/from the original types.

use actr_protocol;

/// Security realm identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct Realm {
    pub realm_id: u32,
}

impl From<actr_protocol::Realm> for Realm {
    fn from(r: actr_protocol::Realm) -> Self {
        Self {
            realm_id: r.realm_id,
        }
    }
}

impl From<Realm> for actr_protocol::Realm {
    fn from(r: Realm) -> Self {
        Self {
            realm_id: r.realm_id,
        }
    }
}

/// Actor type (manufacturer + name)
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ActrType {
    pub manufacturer: String,
    pub name: String,
}

impl From<actr_protocol::ActrType> for ActrType {
    fn from(t: actr_protocol::ActrType) -> Self {
        Self {
            manufacturer: t.manufacturer,
            name: t.name,
        }
    }
}

impl From<ActrType> for actr_protocol::ActrType {
    fn from(t: ActrType) -> Self {
        Self {
            manufacturer: t.manufacturer,
            name: t.name,
        }
    }
}

/// Actor identifier (realm + serial_number + type)
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ActrId {
    pub realm: Realm,
    pub serial_number: u64,
    pub r#type: ActrType,
}

impl From<actr_protocol::ActrId> for ActrId {
    fn from(id: actr_protocol::ActrId) -> Self {
        Self {
            realm: id.realm.into(),
            serial_number: id.serial_number,
            r#type: id.r#type.into(),
        }
    }
}

impl From<ActrId> for actr_protocol::ActrId {
    fn from(id: ActrId) -> Self {
        Self {
            realm: id.realm.into(),
            serial_number: id.serial_number,
            r#type: id.r#type.into(),
        }
    }
}

/// Metadata entry for DataStream
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
}

impl From<actr_protocol::MetadataEntry> for MetadataEntry {
    fn from(m: actr_protocol::MetadataEntry) -> Self {
        Self {
            key: m.key,
            value: m.value,
        }
    }
}

impl From<MetadataEntry> for actr_protocol::MetadataEntry {
    fn from(m: MetadataEntry) -> Self {
        Self {
            key: m.key,
            value: m.value,
        }
    }
}

/// DataStream for fast-path data transmission
///
/// Used for streaming application data (non-media):
/// - File transfer chunks
/// - Game state updates
/// - Custom protocol streams
#[derive(Debug, Clone, uniffi::Record)]
pub struct DataStream {
    /// Stream identifier (globally unique)
    pub stream_id: String,
    /// Sequence number for ordering
    pub sequence: u64,
    /// Payload data
    pub payload: Vec<u8>,
    /// Optional metadata
    pub metadata: Vec<MetadataEntry>,
    /// Optional timestamp in milliseconds
    pub timestamp_ms: Option<i64>,
}

impl From<actr_protocol::DataStream> for DataStream {
    fn from(ds: actr_protocol::DataStream) -> Self {
        Self {
            stream_id: ds.stream_id,
            sequence: ds.sequence,
            payload: ds.payload.to_vec(),
            metadata: ds.metadata.into_iter().map(|m| m.into()).collect(),
            timestamp_ms: ds.timestamp_ms,
        }
    }
}

impl From<DataStream> for actr_protocol::DataStream {
    fn from(ds: DataStream) -> Self {
        Self {
            stream_id: ds.stream_id,
            sequence: ds.sequence,
            payload: ds.payload.into(),
            metadata: ds.metadata.into_iter().map(|m| m.into()).collect(),
            timestamp_ms: ds.timestamp_ms,
        }
    }
}
