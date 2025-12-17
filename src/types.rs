//! Type definitions for UniFFI export

use std::collections::HashMap;
use std::sync::Arc;

/// Actor type identifier
#[derive(Debug, Clone, uniffi::Record)]
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

impl From<&actr_protocol::ActrType> for ActrType {
    fn from(t: &actr_protocol::ActrType) -> Self {
        Self {
            manufacturer: t.manufacturer.clone(),
            name: t.name.clone(),
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

impl From<&ActrType> for actr_protocol::ActrType {
    fn from(t: &ActrType) -> Self {
        Self {
            manufacturer: t.manufacturer.clone(),
            name: t.name.clone(),
        }
    }
}

/// Actor identifier
#[derive(Debug, Clone, uniffi::Record)]
pub struct ActrId {
    pub actor_type: ActrType,
    pub serial_number: u64,
    pub realm_id: u32,
}

impl From<actr_protocol::ActrId> for ActrId {
    fn from(id: actr_protocol::ActrId) -> Self {
        Self {
            actor_type: ActrType {
                manufacturer: id.r#type.manufacturer,
                name: id.r#type.name,
            },
            serial_number: id.serial_number,
            realm_id: id.realm.realm_id,
        }
    }
}

impl From<&actr_protocol::ActrId> for ActrId {
    fn from(id: &actr_protocol::ActrId) -> Self {
        Self {
            actor_type: ActrType {
                manufacturer: id.r#type.manufacturer.clone(),
                name: id.r#type.name.clone(),
            },
            serial_number: id.serial_number,
            realm_id: id.realm.realm_id,
        }
    }
}

impl From<ActrId> for actr_protocol::ActrId {
    fn from(id: ActrId) -> Self {
        Self {
            r#type: actr_protocol::ActrType {
                manufacturer: id.actor_type.manufacturer,
                name: id.actor_type.name,
            },
            serial_number: id.serial_number,
            realm: actr_protocol::Realm {
                realm_id: id.realm_id,
            },
        }
    }
}

impl From<&ActrId> for actr_protocol::ActrId {
    fn from(id: &ActrId) -> Self {
        Self {
            r#type: actr_protocol::ActrType {
                manufacturer: id.actor_type.manufacturer.clone(),
                name: id.actor_type.name.clone(),
            },
            serial_number: id.serial_number,
            realm: actr_protocol::Realm {
                realm_id: id.realm_id,
            },
        }
    }
}

/// Configuration for actr system
#[derive(Debug, Clone, uniffi::Record)]
pub struct ActrConfig {
    pub signaling_url: String,
    pub actor_type: ActrType,
    pub realm_id: u32,
    /// WebRTC configuration (optional)
    pub webrtc: Option<WebRtcConfig>,
    /// Access control list (optional)
    pub acl: Option<AclConfig>,
}

/// WebRTC configuration
#[derive(Debug, Clone, uniffi::Record)]
pub struct WebRtcConfig {
    /// STUN server URLs (e.g., "stun:stun.l.google.com:19302")
    pub stun_urls: Vec<String>,
    /// TURN server URLs (e.g., "turn:turn.example.com:3478")
    pub turn_urls: Vec<String>,
    /// Force relay-only mode (no direct peer connections)
    pub force_relay: bool,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            stun_urls: vec![],
            turn_urls: vec![],
            force_relay: false,
        }
    }
}

/// Access Control List configuration
#[derive(Debug, Clone, uniffi::Record)]
pub struct AclConfig {
    /// ACL rules
    pub rules: Vec<AclRule>,
}

/// Single ACL rule
#[derive(Debug, Clone, uniffi::Record)]
pub struct AclRule {
    /// Permission: "ALLOW" or "DENY"
    pub permission: String,
    /// Actor types that this rule applies to (format: "manufacturer:name")
    pub types: Vec<String>,
}

/// Builder for ActrConfig
#[derive(Debug, Clone, uniffi::Object)]
pub struct ActrConfigBuilder {
    signaling_url: Option<String>,
    actor_type: Option<ActrType>,
    realm_id: u32,
    webrtc: Option<WebRtcConfig>,
    acl: Option<AclConfig>,
}

#[uniffi::export]
impl ActrConfigBuilder {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            signaling_url: None,
            actor_type: None,
            realm_id: 0,
            webrtc: None,
            acl: None,
        }
    }

    pub fn signaling_url(&self, url: String) -> Arc<Self> {
        Arc::new(Self {
            signaling_url: Some(url),
            actor_type: self.actor_type.clone(),
            realm_id: self.realm_id,
            webrtc: self.webrtc.clone(),
            acl: self.acl.clone(),
        })
    }

    pub fn actor_type(&self, manufacturer: String, name: String) -> Arc<Self> {
        Arc::new(Self {
            signaling_url: self.signaling_url.clone(),
            actor_type: Some(ActrType { manufacturer, name }),
            realm_id: self.realm_id,
            webrtc: self.webrtc.clone(),
            acl: self.acl.clone(),
        })
    }

    pub fn realm_id(&self, realm_id: u32) -> Arc<Self> {
        Arc::new(Self {
            signaling_url: self.signaling_url.clone(),
            actor_type: self.actor_type.clone(),
            realm_id,
            webrtc: self.webrtc.clone(),
            acl: self.acl.clone(),
        })
    }

    pub fn webrtc(&self, config: WebRtcConfig) -> Arc<Self> {
        Arc::new(Self {
            signaling_url: self.signaling_url.clone(),
            actor_type: self.actor_type.clone(),
            realm_id: self.realm_id,
            webrtc: Some(config),
            acl: self.acl.clone(),
        })
    }

    pub fn acl(&self, config: AclConfig) -> Arc<Self> {
        Arc::new(Self {
            signaling_url: self.signaling_url.clone(),
            actor_type: self.actor_type.clone(),
            realm_id: self.realm_id,
            webrtc: self.webrtc.clone(),
            acl: Some(config),
        })
    }

    pub fn build(&self) -> Result<ActrConfig, crate::ActrKotlinError> {
        let signaling_url =
            self.signaling_url
                .clone()
                .ok_or_else(|| crate::ActrKotlinError::ConfigError {
                    msg: "signaling_url is required".to_string(),
                })?;

        let actor_type = self.actor_type.clone().unwrap_or_else(|| ActrType {
            manufacturer: "acme".to_string(),
            name: "kotlin.actor".to_string(),
        });

        Ok(ActrConfig {
            signaling_url,
            actor_type,
            realm_id: self.realm_id,
            webrtc: self.webrtc.clone(),
            acl: self.acl.clone(),
        })
    }
}

impl Default for ActrConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ActrConfig {
    /// Convert to actr_config::Config for runtime use
    pub(crate) fn to_actr_config(&self) -> actr_config::Config {
        use actr_config::*;
        use url::Url;

        // Convert WebRTC config
        let webrtc = self
            .webrtc
            .as_ref()
            .map_or_else(WebRtcConfig::default, |wc| {
                let mut ice_servers = Vec::new();

                // Add STUN servers
                if !wc.stun_urls.is_empty() {
                    ice_servers.push(IceServer {
                        urls: wc.stun_urls.clone(),
                        username: None,
                        credential: None,
                    });
                }

                // Add TURN servers
                if !wc.turn_urls.is_empty() {
                    ice_servers.push(IceServer {
                        urls: wc.turn_urls.clone(),
                        username: None,
                        credential: None,
                    });
                }

                let ice_transport_policy = if wc.force_relay {
                    IceTransportPolicy::Relay
                } else {
                    IceTransportPolicy::All
                };

                WebRtcConfig {
                    ice_servers,
                    ice_transport_policy,
                }
            });

        // Convert ACL config
        let acl = self.acl.as_ref().map(|ac| {
            use actr_protocol::AclRule;
            use actr_protocol::acl_rule::{Permission, Principal};

            let rules = ac
                .rules
                .iter()
                .map(|rule| {
                    let permission = match rule.permission.to_uppercase().as_str() {
                        "ALLOW" => Permission::Allow as i32,
                        "DENY" => Permission::Deny as i32,
                        _ => Permission::Deny as i32, // Default to deny for invalid values
                    };

                    let principals = rule
                        .types
                        .iter()
                        .map(|type_str| {
                            // Parse "manufacturer:name" format
                            let parts: Vec<&str> = type_str.splitn(2, ':').collect();
                            let (manufacturer, name) = if parts.len() == 2 {
                                (parts[0].to_string(), parts[1].to_string())
                            } else {
                                ("acme".to_string(), type_str.clone())
                            };

                            Principal {
                                realm: None,
                                actr_type: Some(actr_protocol::ActrType { manufacturer, name }),
                            }
                        })
                        .collect();

                    AclRule {
                        principals,
                        permission,
                    }
                })
                .collect();

            actr_protocol::Acl { rules }
        });

        Config {
            package: PackageInfo {
                name: self.actor_type.name.clone(),
                actr_type: actr_protocol::ActrType {
                    manufacturer: self.actor_type.manufacturer.clone(),
                    name: self.actor_type.name.clone(),
                },
                description: None,
                authors: vec![],
                license: None,
            },
            exports: vec![],
            dependencies: vec![],
            signaling_url: Url::parse(&self.signaling_url)
                .unwrap_or_else(|_| Url::parse("ws://localhost:8081/signaling/ws").unwrap()),
            realm: actr_protocol::Realm {
                realm_id: self.realm_id,
            },
            visible_in_discovery: true,
            acl,
            mailbox_path: None,
            tags: vec![],
            scripts: HashMap::new(),
            webrtc,
            observability: ObservabilityConfig {
                filter_level: "info".to_string(),
                tracing_enabled: false,
                tracing_endpoint: "http://localhost:4317".to_string(),
                tracing_service_name: self.actor_type.name.clone(),
            },
        }
    }
}
