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
