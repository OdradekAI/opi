//! Shared test helpers for the Phase 17.4 trusted-authorization cutover.
//!
//! Compiled once per test binary; helpers unused by a given binary are
//! intentionally allowed dead so the `-D warnings` clippy gate stays green.
//!
//! TEST-ONLY: this module lives in the integration-test tree and is never
//! compiled into the published `opi-agent` library. The authorizers here are
//! test doubles morally equivalent to `MockProvider`/`RecordingSink`; they exist
//! so existing tool-mechanics tests survive the mandatory fail-closed
//! authorization cutover without each rebuilding a real policy, and they MUST
//! NOT appear in production code.
#![allow(dead_code)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_agent::Tool;
use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, CapabilityIdentity, RegisteredTool, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer, ToolOrigin,
};
use opi_agent::evidence::{
    EvidenceGeneration, PermissionReference, PermissionScope, PolicyReference,
};
use opi_agent::tool::{ExecutionMode, ToolError, ToolResult};
use tokio_util::sync::CancellationToken;

/// Convert raw tools into trusted registrations with a default test origin and
/// opaque `opi.workspace.read` capability identity. Tests that need a specific
/// capability or origin construct their [`RegisteredTool`] explicitly.
pub fn registrations_from(tools: Vec<Box<dyn Tool>>) -> Vec<RegisteredTool> {
    tools
        .into_iter()
        .map(|t| {
            let name = t.definition().name.clone();
            RegisteredTool::new(
                RegistrationId::new(format!("test-{name}")),
                name,
                ToolOrigin::Builtin,
                CapabilityIdentity::new("opi.workspace.read").unwrap(),
                t.definition(),
                Arc::from(t),
            )
        })
        .collect()
}

/// Build an immutable test registry from raw tools (default `Builtin` origin).
pub fn test_registry(tools: Vec<Box<dyn Tool>>) -> Arc<opi_agent::authority::ToolRegistry> {
    Arc::new(
        opi_agent::authority::ToolRegistry::from_tools(registrations_from(tools))
            .expect("distinct test tool names"),
    )
}

// ---- Test authorizers (NOT production; test infrastructure only) ----------

/// Permissive test authorizer: allows every request and echoes the request's
/// current evidence-health generation so the freshness gate passes.
#[derive(Debug, Default, Clone, Copy)]
pub struct PermissiveAuthorizer;

impl ToolAuthorizer for PermissiveAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>
    {
        Box::pin(async move {
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("test-policy").unwrap(),
                permission_ref: PermissionReference::new("test-permission").unwrap(),
                permission_scope: PermissionScope::new("test-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id.clone(),
                capability: request.capability.clone(),
                evidence_health_generation: request.evidence_health.generation(),
            })
        })
    }
}

/// A shared permissive authorizer handle for tests that just need execution to
/// proceed past the mandatory authorization gate.
pub fn permissive_authorizer() -> Arc<dyn ToolAuthorizer> {
    Arc::new(PermissiveAuthorizer)
}

/// Denying test authorizer: denies every request with a stable code + reason.
#[derive(Debug, Default, Clone, Copy)]
pub struct DenyingAuthorizer;

impl ToolAuthorizer for DenyingAuthorizer {
    fn authorize(
        &self,
        _request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>
    {
        Box::pin(async move {
            Ok(AuthorizationDecision::Deny {
                stable_code: "test_deny".to_owned(),
                redacted_reason: "denied by test authorizer".to_owned(),
            })
        })
    }
}

/// Test authorizer that always returns an `Allow` carrying a FIXED generation,
/// ignoring the request's current health. When the run's current health has a
/// different generation, the freshness gate detects the mismatch and denies —
/// the synthetic stand-in for the 17.7 scenario where evidence emission advances
/// health after an `Allow` was computed.
#[derive(Debug, Clone, Copy)]
pub struct StaleGenerationAuthorizer {
    /// The (stale) generation stamped onto every returned `Allow`.
    pub fixed_generation: EvidenceGeneration,
}

impl Default for StaleGenerationAuthorizer {
    fn default() -> Self {
        Self {
            fixed_generation: EvidenceGeneration::INITIAL,
        }
    }
}

impl ToolAuthorizer for StaleGenerationAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>
    {
        let fixed_generation = self.fixed_generation;
        Box::pin(async move {
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("test-policy").unwrap(),
                permission_ref: PermissionReference::new("test-permission").unwrap(),
                permission_scope: PermissionScope::new("test-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id.clone(),
                capability: request.capability.clone(),
                evidence_health_generation: fixed_generation,
            })
        })
    }
}

// ---- Recording tool ------------------------------------------------------

/// Tool whose execution counts how many times it runs, so fail-closed tests
/// can assert zero executions. Returns a fixed non-error result.
pub struct RecordingTool {
    name: String,
    count: Arc<AtomicUsize>,
}

impl RecordingTool {
    /// Build a recording tool named `name` sharing `count` across clones.
    pub fn new(name: impl Into<String>, count: Arc<AtomicUsize>) -> Self {
        Self {
            name: name.into(),
            count,
        }
    }

    /// How many times any clone of this tool has executed.
    pub fn count_of(count: &Arc<AtomicUsize>) -> usize {
        count.load(Ordering::SeqCst)
    }
}

impl Tool for RecordingTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: self.name.clone(),
            description: "recording test tool".to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        }
    }

    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let count = self.count.clone();
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "executed".to_owned(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: Vec::new(),
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Parallel
    }
}
