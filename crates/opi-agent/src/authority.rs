//! Trusted tool authorization: immutable registration, capability identity, and
//! the mandatory fail-closed authorizer boundary.
//!
//! Tools enter the Agent loop only through an immutable [`RegisteredTool`] owned
//! by trusted assembly, never through `Tool::definition()` alone. Every tool
//! execution must cross a [`ToolAuthorizer`] bound to the effective User Policy
//! for the run; missing, failed, expired, stale, or forged authority yields zero
//! executions. This module owns the generic core mechanism; the Reference
//! Product owns the policy binding (the immutable digest-addressed
//! `EffectiveUserPolicy`, the fixed built-in capability map, and the concrete
//! authorizer implementation).
//!
//! ## Ownership boundary
//!
//! This substrate owns registration/capability identity and the fail-closed
//! per-call authorization contract. The agent loop supplies typed correlation,
//! current evidence health, stale-generation reauthorization, and the final
//! launch check. Product policy and file storage remain outside this module.
//!
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use opi_ai::message::ToolDef;
use tokio_util::sync::CancellationToken;

pub use crate::evidence::CapabilityIdentity;
use crate::evidence::{
    CallId, EvidenceGeneration, EvidenceHealth, PermissionReference, PermissionScope,
    PolicyReference, RunId, ScopedGrantReference, TurnId,
};
use crate::tool::Tool;

// ===========================================================================
// Registration identity
// ===========================================================================

/// Unique registration identifier assigned by trusted assembly. Distinct from
/// the [`RegisteredTool::provider_visible_name`] the model calls: a registration
/// id is trusted-assembly-owned provenance, not model-supplied content.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistrationId(String);

impl RegistrationId {
    /// Construct a registration id from trusted-assembly-owned provenance.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RegistrationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Origin of a registered tool, assigned by trusted assembly. An extension
/// cannot replace its origin with a model-visible field; an embedder supplies
/// its registration and authorizer together as trusted assembly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToolOrigin {
    /// A built-in Reference Product tool.
    Builtin,
    /// A tool contributed by an extension, namespaced to `extension_id`.
    Extension {
        /// The extension that owns this registration.
        extension_id: String,
    },
    /// A tool contributed by an embedder as trusted assembly.
    Embedder {
        /// The embedder that owns this registration.
        embedder_id: String,
    },
}

/// Immutable registration of one tool owned by trusted assembly.
///
/// Immutability is a usage guarantee: the [`ToolRegistry`] and the Agent hold
/// registrations behind shared (non-`&mut`) handles. A registration is built
/// once at trusted assembly and bound to the authorizer at construction; there
/// is no post-construction registration path.
pub struct RegisteredTool {
    /// Trusted-assembly-owned registration identifier.
    pub registration_id: RegistrationId,
    /// The tool name the model sees and calls.
    pub provider_visible_name: String,
    /// Registration-owned origin (built-in / extension / embedder).
    pub origin: ToolOrigin,
    /// Registration-derived capability identity.
    pub capability: CapabilityIdentity,
    /// Provider-facing definition (name, description, JSON Schema).
    pub definition: ToolDef,
    /// The tool implementation invoked only after a current `Allow`.
    pub implementation: Arc<dyn Tool>,
}

impl RegisteredTool {
    /// Build an immutable registration.
    pub fn new(
        registration_id: RegistrationId,
        provider_visible_name: String,
        origin: ToolOrigin,
        capability: CapabilityIdentity,
        definition: ToolDef,
        implementation: Arc<dyn Tool>,
    ) -> Self {
        Self {
            registration_id,
            provider_visible_name,
            origin,
            capability,
            definition,
            implementation,
        }
    }
}

// ===========================================================================
// Immutable registry
// ===========================================================================

/// Immutable registry of registered tools. Built once from trusted
/// registrations; rejects duplicate provider-visible names. Preserves insertion
/// order for deterministic provider-facing projection and resolves names to
/// registrations for the loop's per-call authorization gate.
pub struct ToolRegistry {
    tools: Vec<Arc<RegisteredTool>>,
    index: HashMap<String, usize>,
}

impl ToolRegistry {
    /// Build a registry from trusted registrations. Returns
    /// [`AuthorityError::DuplicateRegistration`] if two registrations share a
    /// provider-visible name.
    pub fn from_tools(tools: Vec<RegisteredTool>) -> Result<Self, AuthorityError> {
        let mut registered: Vec<Arc<RegisteredTool>> = Vec::with_capacity(tools.len());
        let mut index: HashMap<String, usize> = HashMap::new();
        for tool in tools {
            let name = tool.provider_visible_name.clone();
            if index.contains_key(&name) {
                return Err(AuthorityError::DuplicateRegistration { name });
            }
            index.insert(name, registered.len());
            registered.push(Arc::new(tool));
        }
        Ok(Self {
            tools: registered,
            index,
        })
    }

    /// Resolve a provider-visible name to its registration.
    pub fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.index.get(name).map(|&pos| self.tools[pos].as_ref())
    }

    /// The provider-facing definitions in insertion order. Projection is
    /// derived solely from trusted registrations, never from model content
    /// (AUT-008).
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| t.definition.clone()).collect()
    }

    /// Iterate all registrations in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &RegisteredTool> {
        self.tools.iter().map(|t| t.as_ref())
    }

    /// The number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry holds no tools.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

// ===========================================================================
// Authorization request, decision, and authorizer
// ===========================================================================

/// Opaque invocation context supplied by trusted runtime assembly.
///
/// [`Self::NoSession`] records explicit absence. [`Self::Session`] carries the
/// assembly-provided reference without interpreting its product semantics or
/// deriving it from model content or tool arguments. Evidence emission
/// separately validates a session reference as an invocation binding; a
/// malformed reference makes authorization evidence incomplete and prevents
/// tool execution when an evidence sink is present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationContext {
    /// Trusted assembly supplied no session context for this invocation.
    NoSession,
    /// Opaque trusted session context supplied by runtime assembly.
    Session(String),
}

impl InvocationContext {
    pub(crate) fn from_session_id(session_id: Option<&str>) -> Self {
        match session_id {
            Some(session_id) => Self::Session(session_id.to_owned()),
            None => Self::NoSession,
        }
    }
}

/// Core-confirmed authorization facts for one resolved, schema-validated tool
/// call.
///
/// The runtime constructs this request only after resolving an immutable
/// registration and validating the final [`Self::arguments`] against that
/// registration's schema. [`Self::invocation_context`] comes from trusted
/// assembly and is never inferred from those arguments. The run, turn, and call
/// identities correlate this exact decision with its evidence records.
///
/// [`Self::evidence_health`] is the run-local snapshot at authorization time.
/// An allow decision is executable only when it returns the same registration,
/// capability, and evidence-health generation; a stale decision is reauthorized
/// once and a persistent mismatch is denied without execution. Authorizers may
/// inspect the full arguments, but emitted outcomes must remain classified or
/// redacted.
#[derive(Debug, Clone)]
pub struct ToolAuthorizationRequest {
    /// Evidence run identity minted before the call.
    pub run_id: RunId,
    /// Evidence turn identity minted before the call.
    pub turn_id: TurnId,
    /// Evidence call identity reused by authorization and tool outcomes.
    pub call_id: CallId,
    /// Opaque trusted invocation/session context, never derived from arguments.
    pub invocation_context: InvocationContext,
    /// The resolved registered tool's trusted registration id.
    pub registration_id: RegistrationId,
    /// The registration-derived capability identity.
    pub capability: CapabilityIdentity,
    /// The final validated arguments. The exact value authorized is the value
    /// executed (AUT-002).
    pub arguments: serde_json::Value,
    /// The current versioned evidence-health snapshot (a copy per request).
    pub evidence_health: EvidenceHealth,
}

/// The closed authorization decision returned by a [`ToolAuthorizer`].
#[derive(Debug, Clone)]
pub enum AuthorizationDecision {
    /// Authorization granted against the effective User Policy at the carried
    /// evidence-health generation. The runtime verifies the carried
    /// `registration_id`, `capability`, and `evidence_health_generation` still
    /// match the current call before executing; a mismatch is stale and is not
    /// executed.
    Allow {
        /// Opaque product-owned effective-policy reference (e.g. digest).
        policy_ref: PolicyReference,
        /// Opaque product-owned permission reference.
        permission_ref: PermissionReference,
        /// Opaque product-owned permission scope.
        permission_scope: PermissionScope,
        /// Separately versioned scoped grant used by this decision, if any.
        scoped_grant_ref: Option<ScopedGrantReference>,
        /// The registration the decision covers.
        registration_id: RegistrationId,
        /// The capability the decision covers.
        capability: CapabilityIdentity,
        /// The evidence-health generation the decision was computed against.
        evidence_health_generation: EvidenceGeneration,
    },
    /// Authorization denied. `stable_code` and `redacted_reason` are the
    /// controlled, secret-free outcome surfaced to the model and to evidence.
    Deny {
        /// Stable snake_case denial code.
        stable_code: String,
        /// Redacted, non-secret denial reason.
        redacted_reason: String,
    },
}

/// An authorizer malfunction (not a denial). Missing authorizer, authorizer
/// error, and authorizer unavailability all yield zero executions (AUT-005); a
/// denial is a normal [`AuthorizationDecision::Deny`], not an error.
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    /// The authorizer could not reach a decision (internal failure).
    #[error("authorization failed: {0}")]
    Failed(String),
    /// The authorizer is unavailable for this run.
    #[error("authorization unavailable: {0}")]
    Unavailable(String),
}

/// Trusted tool authorizer, bound to the effective User Policy for the run by
/// trusted assembly. Implementations are product-owned (the Reference Product
/// authorizer closes over its `EffectiveUserPolicy` and reuses the existing
/// `command.execute` permission broker). The authorizer is not replaceable by
/// next-turn state or model-visible content.
///
/// `authorize` receives the loop's [`CancellationToken`] so an interactive
/// permission broker or any blocking decision is cancellable; cancellation is
/// treated as fail-closed (zero execution), mirroring [`Tool::execute`].
pub trait ToolAuthorizer: Send + Sync {
    /// Decide authorization for one resolved, schema-validated tool call.
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        cancel: CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<AuthorizationDecision, AuthorizationError>> + Send>>;
}

/// Registry-construction failure. (Authorizer decision failures use
/// [`AuthorizationError`].)
#[derive(Debug, thiserror::Error)]
pub enum AuthorityError {
    /// Two registrations share a provider-visible name.
    #[error("duplicate tool registration for provider-visible name '{name}'")]
    DuplicateRegistration {
        /// The colliding provider-visible name.
        name: String,
    },
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ExecutionMode, ToolError, ToolResult};
    use opi_ai::message::{OutputContent, ToolDef};

    /// Minimal tool whose definition is configurable; execute is never reached
    /// in these registry tests.
    struct StubTool {
        def: ToolDef,
    }

    impl Tool for StubTool {
        fn definition(&self) -> ToolDef {
            self.def.clone()
        }
        fn execute(
            &self,
            _call_id: &str,
            _arguments: serde_json::Value,
            _signal: CancellationToken,
            _on_update: Option<crate::tool::UpdateCallback>,
        ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
            Box::pin(async {
                Ok(ToolResult {
                    content: vec![OutputContent::Text {
                        text: "stub".to_owned(),
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

    fn stub_tool(name: &str) -> Arc<dyn Tool> {
        Arc::new(StubTool {
            def: ToolDef {
                name: name.to_owned(),
                description: "stub".to_owned(),
                input_schema: serde_json::json!({ "type": "object" }),
            },
        })
    }

    fn registered(name: &str, capability: CapabilityIdentity) -> RegisteredTool {
        RegisteredTool::new(
            RegistrationId::new(format!("reg-{name}")),
            name.to_owned(),
            ToolOrigin::Builtin,
            capability,
            ToolDef {
                name: name.to_owned(),
                description: name.to_owned(),
                input_schema: serde_json::json!({ "type": "object" }),
            },
            stub_tool(name),
        )
    }

    #[test]
    fn registry_rejects_duplicate_provider_visible_names() {
        let cap = CapabilityIdentity::new("acme.documents.read").unwrap();
        let result = ToolRegistry::from_tools(vec![
            registered("read", cap.clone()),
            registered("read", cap),
        ]);
        assert!(matches!(
            result,
            Err(AuthorityError::DuplicateRegistration { name }) if name == "read"
        ));
    }

    #[test]
    fn registry_resolves_names_and_preserves_insertion_order() {
        let registry = ToolRegistry::from_tools(vec![
            registered(
                "read",
                CapabilityIdentity::new("acme.documents.read").unwrap(),
            ),
            registered(
                "shell",
                CapabilityIdentity::new("acme.process.run").unwrap(),
            ),
            registered(
                "write",
                CapabilityIdentity::new("acme.documents.write").unwrap(),
            ),
        ])
        .expect("distinct names");

        // Resolution by provider-visible name.
        assert!(registry.get("read").is_some());
        assert!(registry.get("missing").is_none());
        assert_eq!(
            registry.get("shell").unwrap().capability,
            CapabilityIdentity::new("acme.process.run").unwrap()
        );

        // Definitions preserve insertion order (not alphabetical, not map order).
        let defs = registry.definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["read", "shell", "write"]);
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn capability_identities_are_stable_and_distinct() {
        let read = CapabilityIdentity::new("acme.documents.read").unwrap();
        let write = CapabilityIdentity::new("acme.documents.write").unwrap();
        let execute = CapabilityIdentity::new("acme.process.run").unwrap();
        let extension = CapabilityIdentity::new("acme.extension:ext1:custom").unwrap();
        assert_eq!(read.as_str(), "acme.documents.read");
        assert_eq!(write.as_str(), "acme.documents.write");
        assert_eq!(execute.as_str(), "acme.process.run");
        assert_eq!(extension.as_str(), "acme.extension:ext1:custom");
        // Identities are pairwise distinct.
        let ids = [
            read.as_str(),
            write.as_str(),
            execute.as_str(),
            extension.as_str(),
        ];
        let unique: std::collections::BTreeSet<&str> = ids.into_iter().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn authorization_decision_carries_freshness_fields() {
        // Allow carries the generation it was computed against; Deny carries a
        // stable code + redacted reason (no secrets, no raw args).
        let allow = AuthorizationDecision::Allow {
            policy_ref: PolicyReference::new("digest").unwrap(),
            permission_ref: PermissionReference::new("perm").unwrap(),
            permission_scope: PermissionScope::new("scope").unwrap(),
            scoped_grant_ref: Some(ScopedGrantReference::new("grant").unwrap()),
            registration_id: RegistrationId::new("reg-read"),
            capability: CapabilityIdentity::new("acme.documents.read").unwrap(),
            evidence_health_generation: EvidenceGeneration::INITIAL,
        };
        match allow {
            AuthorizationDecision::Allow {
                evidence_health_generation,
                ..
            } => {
                assert_eq!(evidence_health_generation, EvidenceGeneration::INITIAL);
            }
            AuthorizationDecision::Deny { .. } => panic!("expected Allow"),
        }
        let deny = AuthorizationDecision::Deny {
            stable_code: "expired_scope".to_owned(),
            redacted_reason: "permission scope expired".to_owned(),
        };
        match deny {
            AuthorizationDecision::Deny {
                stable_code,
                redacted_reason,
            } => {
                assert_eq!(stable_code, "expired_scope");
                assert!(redacted_reason.contains("expired"));
            }
            AuthorizationDecision::Allow { .. } => panic!("expected Deny"),
        }
    }
}
