//! Deferred response values; validation uses caller-supplied identity and time.

/// A deferred-response handle returned by a capable backend (pi
/// `DeferredHandle`).
///
/// Identity is source-owned (`provider`, `model_id`, `api`, `id`); optional
/// expiry and poll hints are provider advice, never an SDK polling
/// schedule. `data` carries provider conversion data needed to reconstruct
/// the final assistant message. The handle never carries a credential.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeferredHandle {
    /// Provider id that produced the deferred response.
    pub provider: String,
    /// Bare model id the deferred generation ran on.
    pub model_id: String,
    /// Wire/api identity the deferred route serves.
    pub api: String,
    /// Provider token, such as a response id or batch id plus row id.
    pub id: String,
    /// Optional absolute expiry in epoch milliseconds (pi field name
    /// `expiresAt`, without a unit suffix).
    #[serde(rename = "expiresAt", default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,
    /// Optional provider poll advice in milliseconds; caller-owned only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_after_ms: Option<u64>,
    /// Provider conversion data for the final message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl DeferredHandle {
    /// Validate this handle's route identity against the resolved
    /// (`provider`, `model_id`, `api`) route before any authentication.
    pub fn validate_identity(
        &self,
        provider: &str,
        model_id: &str,
        api: &str,
    ) -> Result<(), DeferredError> {
        if self.provider != provider || self.model_id != model_id || self.api != api {
            return Err(DeferredError::RouteMismatch(Box::new(
                RouteMismatchDetail {
                    handle_provider: self.provider.clone(),
                    handle_model_id: self.model_id.clone(),
                    handle_api: self.api.clone(),
                    route_provider: provider.to_owned(),
                    route_model_id: model_id.to_owned(),
                    route_api: api.to_owned(),
                },
            )));
        }
        Ok(())
    }

    /// Validate expiry against a caller-injected monotonic reading.
    ///
    /// The SDK never reads a clock itself: the caller (agent loop, consumer
    /// fixture) owns time so tests stay deterministic and the SDK gains no
    /// hidden polling or timing behavior.
    pub fn validate_expiry(&self, now_ms: u64) -> Result<(), DeferredError> {
        if let Some(expires_at_ms) = self.expires_at_ms
            && now_ms >= expires_at_ms
        {
            return Err(DeferredError::Expired {
                handle_id: self.id.clone(),
                expires_at_ms,
                now_ms,
            });
        }
        Ok(())
    }
}

/// Identity triple of a handle and the route it failed to match (boxed in
/// [`DeferredError::RouteMismatch`] to keep the error type small).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error(
    "handle {handle_provider}/{handle_model_id}/{handle_api} does not match route {route_provider}/{route_model_id}/{route_api}"
)]
pub struct RouteMismatchDetail {
    /// Handle's provider id.
    pub handle_provider: String,
    /// Handle's bare model id.
    pub handle_model_id: String,
    /// Handle's wire/api identity.
    pub handle_api: String,
    /// Resolved route's provider id.
    pub route_provider: String,
    /// Resolved route's bare model id.
    pub route_model_id: String,
    /// Resolved route's wire/api identity.
    pub route_api: String,
}

/// Typed deferred-operation failures surfaced by the SDK surface.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DeferredError {
    /// The resolved route does not match the handle's identity.
    #[error("deferred handle route mismatch: {0}")]
    RouteMismatch(#[from] Box<RouteMismatchDetail>),
    /// The handle expired according to the caller-injected clock reading.
    #[error("deferred handle '{handle_id}' expired at {expires_at_ms} (now {now_ms})")]
    Expired {
        handle_id: String,
        expires_at_ms: u64,
        now_ms: u64,
    },
    /// No deferred backend serves the handle's provider (pi: "does not
    /// support deferred responses").
    #[error("provider '{provider}' does not support deferred responses")]
    Unsupported {
        /// Provider id the fetch/cancel targeted.
        provider: String,
    },
}
