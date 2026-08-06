//! Request and protocol identifiers, and version negotiation.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::WIRE_IDENTITY;

/// Error constructing a [`RequestId`] from an empty string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("request id must be a non-empty string")]
pub struct InvalidRequestId;

/// Error constructing an [`ImplementationId`] from an empty string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("implementation id must be a non-empty string")]
pub struct InvalidImplementationId;

/// Error constructing a [`ProtocolId`] from an empty string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("protocol id must be a non-empty string")]
pub struct InvalidProtocolId;

/// Host-generated opaque request id carried by every frame in one execution.
///
/// Empty ids are rejected at the type boundary: construction ([`RequestId::new`])
/// and deserialization both reject `""`, and the generated JSON Schema carries
/// `minLength: 1`. Backend frames echo the host id; this type does not validate
/// echoing by construction  --  the [`Session`](super::Session) checker detects a
/// cross-request id, and the runtime (host/backend) is responsible for actually
/// echoing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(extend("minLength" = 1))]
pub struct RequestId(String);

impl RequestId {
    /// Construct a request id, rejecting the empty string.
    pub fn new(value: String) -> Result<Self, InvalidRequestId> {
        if value.is_empty() {
            return Err(InvalidRequestId);
        }
        Ok(Self(value))
    }

    /// The id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for RequestId {
    type Error = InvalidRequestId;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Backend implementation/adapter identity reported during negotiation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(extend("minLength" = 1))]
pub struct ImplementationId(String);

impl ImplementationId {
    /// Construct an implementation id, rejecting the empty string.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidImplementationId> {
        let value = value.into();
        if value.is_empty() {
            return Err(InvalidImplementationId);
        }
        Ok(Self(value))
    }

    /// The identity as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Display for ImplementationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ImplementationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// A wire-protocol identity. The version is baked into the identity string
/// (`command-execution-jsonl-v1`); there is no separate numeric version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(extend("minLength" = 1))]
pub struct ProtocolId(String);

impl ProtocolId {
    /// Construct a protocol id, rejecting the empty string.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidProtocolId> {
        let value = value.into();
        if value.is_empty() {
            return Err(InvalidProtocolId);
        }
        Ok(Self(value))
    }

    /// The identity as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProtocolId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// `command-execution-jsonl-v1`.
pub static V1: LazyLock<ProtocolId> =
    LazyLock::new(|| ProtocolId::new(WIRE_IDENTITY).expect("v1 wire identity is non-empty"));

/// No protocol in common between the host's ordered list and the backend's set.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("no supported protocol in common (host offered {host:?}, backend supports {backend:?})")]
pub struct ProtocolIncompatible {
    /// The host's ordered list (preference order).
    pub host: Vec<ProtocolId>,
    /// The backend's supported set.
    pub backend: Vec<ProtocolId>,
}

/// Select the first protocol in `host_ordered` (host preference order) that the
/// backend also supports.
///
/// This is first-match-wins by host preference  --  **not** numeric-max. The
/// version is baked into the [`ProtocolId`] string, so a numeric ordering is not
/// expressible. The backend calls this with its own supported set and the host
/// list received on `initialize`, and places the result in
/// `ready.selected_protocol`; on no overlap it emits `failed` with
/// [`FailureCode::ProtocolIncompatible`](super::FailureCode::ProtocolIncompatible).
/// The host, on `ready`, can only validate that `selected_protocol` is in its
/// own ordered list.
pub fn select(
    host_ordered: &[ProtocolId],
    backend_supported: &BTreeSet<ProtocolId>,
) -> Result<ProtocolId, ProtocolIncompatible> {
    for id in host_ordered {
        if backend_supported.contains(id) {
            return Ok(id.clone());
        }
    }
    Err(ProtocolIncompatible {
        host: host_ordered.to_vec(),
        backend: backend_supported.iter().cloned().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_request_id_rejected() {
        assert!(RequestId::new(String::new()).is_err());
        assert!(RequestId::new("req-1".to_string()).is_ok());
        // Deserialization rejects empty.
        let err = serde_json::from_str::<RequestId>("\"\"").unwrap_err();
        assert!(err.to_string().contains("request id"));
    }

    #[test]
    fn select_prefers_host_order_not_numeric() {
        let v1 = ProtocolId::new("command-execution-jsonl-v1").unwrap();
        let v2 = ProtocolId::new("command-execution-jsonl-v2").unwrap();
        let backend = [v1.clone(), v2.clone()]
            .into_iter()
            .collect::<BTreeSet<_>>();
        // Host lists v1 first even though "v2" sorts later; first-match wins.
        assert_eq!(select(&[v1.clone(), v2.clone()], &backend), Ok(v1.clone()));
        // Host preference reversed: v2 wins.
        assert_eq!(select(&[v2.clone(), v1.clone()], &backend), Ok(v2.clone()));
    }

    #[test]
    fn select_empty_intersection_is_incompatible() {
        let v1 = ProtocolId::new("command-execution-jsonl-v1").unwrap();
        let backend = [ProtocolId::new("other").unwrap()]
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert!(select(&[v1], &backend).is_err());
    }
}
