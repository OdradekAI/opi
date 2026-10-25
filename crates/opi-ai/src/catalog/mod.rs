//! Offline bundled model catalog: authentic artifact, loader, and getters.
//!
//! The builtin bundle is the authentic Pi published model catalog, packaged
//! byte-for-byte by `scripts/pi-catalog/pack.py` from the revision pinned by
//! the local Pi tree (see `resources/catalog/builtin/manifest.json`). Loading
//! never performs network I/O, account probing, or generator runs: it parses
//! the embedded bytes, re-verifies the artifact digest, structure digest,
//! schema versions, and every entry, and fails closed with a typed
//! [`CatalogError`] on any mismatch.
//!
//! Callers hold an owned [`std::sync::Arc<Catalog>`]; installing a new
//! revision through [`CatalogHandle`] swaps the handle atomically and never
//! rebinds already-held catalogs, so a rejected candidate cannot disturb the
//! last-known revision.
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

pub mod http_source;
pub mod store;
mod wire;

pub use http_source::{CatalogDocumentError, DynamicCatalogSource, HttpCatalogSource};
pub use store::{
    ModelsStore, ModelsStoreError, RefreshGuard, StoreCommit, StoreCommitError, StoreSnapshot,
};

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock, RwLock};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Bundle format version understood by this loader (wrong or newer versions
/// are rejected with [`CatalogError::BundleSchemaVersion`]).
pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

/// Pi model-data schema version of the packaged artifact layout
/// (Pi `MODEL_DATA_SCHEMA_VERSION`).
pub const DATA_SCHEMA_VERSION: u32 = 6;

const BUILTIN_CATALOG_BYTES: &[u8] = include_bytes!("../../resources/catalog/builtin/catalog.json");
const BUILTIN_MANIFEST_BYTES: &[u8] =
    include_bytes!("../../resources/catalog/builtin/manifest.json");

// ---------------------------------------------------------------------------
// Operation identity
// ---------------------------------------------------------------------------

/// One catalog operation a model entry is qualified for.
///
/// Mirrors Pi's `type:id` entry identity: the same bare model id may appear
/// once per operation for one provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogOperation {
    /// Tool-capable chat model (`type: "chat"`).
    Chat,
    /// Image generation model (`type: "image"`).
    Image,
    /// Classifier model (`type: "classifier"`).
    Classifier,
}

impl CatalogOperation {
    /// The artifact's `type` string for this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            CatalogOperation::Chat => "chat",
            CatalogOperation::Image => "image",
            CatalogOperation::Classifier => "classifier",
        }
    }
}

impl fmt::Display for CatalogOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Typed rejection of a catalog bundle. Fail-closed: no partial catalog is
/// ever produced.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// A bundle file is not valid UTF-8 JSON or does not match the manifest
    /// shape.
    #[error("catalog bundle is not valid JSON: {detail}")]
    Json {
        /// What failed to parse.
        detail: String,
    },
    /// The manifest declares a bundle schema version this loader does not
    /// support (stale or too new).
    #[error("bundle schema version {found} is not supported (expected {supported})")]
    BundleSchemaVersion {
        /// Version this loader supports.
        supported: u32,
        /// Version declared by the manifest.
        found: u32,
    },
    /// The artifact bytes do not match the digest recorded in the manifest
    /// (tampered or truncated artifact).
    #[error("catalog digest {found} does not match manifest digest {expected}")]
    DigestMismatch {
        /// Digest recorded in the manifest.
        expected: String,
        /// Digest recomputed from the artifact bytes.
        found: String,
    },
    /// The recomputed provider/model structure does not match the structure
    /// digest recorded in the manifest (stale or edited data).
    #[error("catalog structure digest {found} does not match manifest digest {expected}")]
    StructureMismatch {
        /// Digest recorded in the manifest.
        expected: String,
        /// Digest recomputed from the artifact.
        found: String,
    },
    /// A provider recorded in the manifest is absent from the artifact.
    #[error("catalog is missing provider {provider:?}")]
    MissingProvider {
        /// Missing provider id.
        provider: String,
    },
    /// The artifact contains a provider the manifest does not record.
    #[error("catalog has unknown provider {provider:?}")]
    UnknownProvider {
        /// Unexpected provider id.
        provider: String,
    },
    /// One provider lists the same `type:id` identity twice.
    #[error("provider {provider:?} has duplicate identity {identity:?}")]
    DuplicateIdentity {
        /// Owning provider id.
        provider: String,
        /// Duplicated `type:id` identity.
        identity: String,
    },
    /// One entry violates the catalog data contract (identity, modalities,
    /// capabilities, or cost metadata).
    #[error("invalid entry {provider}/{identity}: {detail}")]
    InvalidEntry {
        /// Owning provider id.
        provider: String,
        /// `type:id` identity of the entry (best effort).
        identity: String,
        /// What is invalid.
        detail: String,
    },
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// Digests identifying how the bundled artifact was produced and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogGeneratorDigests {
    /// SHA-256 of Pi's `packages/ai/scripts/model-data.ts` (validator whose
    /// rules this loader mirrors).
    pub pi_model_data_ts_sha256: String,
    /// SHA-256 of Pi's `packages/ai/scripts/hydrate-model-catalog.ts`
    /// (hydration contract this bundle follows).
    pub pi_hydrate_model_catalog_ts_sha256: String,
    /// Path of the opi packer that staged and emitted the bundle.
    pub packer: String,
    /// SHA-256 of the packer at packaging time.
    pub packer_sha256: String,
}

/// Entry counts by operation, as packaged and re-verified at load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogCounts {
    /// Number of providers in the bundle.
    pub providers: usize,
    /// Chat model entries.
    pub chat: usize,
    /// Image model entries.
    pub image: usize,
    /// Classifier model entries.
    pub classifier: usize,
}

/// Authenticity and origin metadata of one packaged catalog revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogProvenance {
    /// Published revision id (`sha256-…`) the artifact was fetched from.
    pub revision: String,
    /// Hex SHA-256 of the artifact bytes (the revision id without prefix).
    pub artifact_sha256: String,
    /// Exact pi.dev URL the revision was published at.
    pub source_url: String,
    /// Pi package (and version) the local tree association is built on.
    pub pi_package: String,
    /// Local origin tree the revision pin was read from.
    pub local_origin: String,
    /// Pi model-data schema version of the artifact layout.
    pub data_schema_version: u32,
    /// Bundle format version of the packaged resources.
    pub bundle_schema_version: u32,
    /// SHA-256 over the normalized provider/identity/API structure.
    pub structure_sha256: String,
    /// Sorted provider ids present in the bundle.
    pub providers: Vec<String>,
    /// Entry counts by operation.
    pub counts: CatalogCounts,
    /// Generator identities (see [`CatalogGeneratorDigests`]).
    pub generator: CatalogGeneratorDigests,
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/// One input modality declared by a catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogInputModality {
    /// Text input.
    Text,
    /// Image input.
    Image,
}

/// Per-million-token cost metadata of one entry. Values may be `0.0`; only
/// finiteness is guaranteed by the catalog contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatalogCost {
    /// Input token cost.
    pub input: f64,
    /// Output token cost.
    pub output: f64,
    /// Cache-read token cost.
    pub cache_read: f64,
    /// Cache-write token cost.
    pub cache_write: f64,
}

/// One complete, operation-qualified model entry from the bundled catalog.
///
/// Core identity and capability fields are typed; `raw` retains the full
/// artifact entry (compat flags, thinking-level maps, provider-specific
/// metadata) so later consumers can project without re-fetching.
#[derive(Debug, Clone)]
pub struct CatalogModel {
    /// Owning provider id.
    pub provider: String,
    /// Bare model id (no provider prefix).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Operation this entry is qualified for.
    pub operation: CatalogOperation,
    /// Wire API id (e.g. `anthropic-messages`).
    pub api: String,
    /// Provider base URL (may be empty when the provider supplies one).
    pub base_url: String,
    /// Whether the model advertises reasoning (chat entries).
    pub reasoning: bool,
    /// Declared input modalities (always non-empty).
    pub input: Vec<CatalogInputModality>,
    /// Per-million-token cost metadata.
    pub cost: CatalogCost,
    /// Context window in tokens (chat and classifier entries).
    pub context_window: Option<u64>,
    /// Maximum output tokens (chat entries).
    pub max_tokens: Option<u64>,
    /// The complete artifact entry, unmodified.
    pub raw: Value,
}

impl CatalogModel {
    /// The entry's `type:id` catalog identity.
    pub fn identity(&self) -> String {
        format!("{}:{}", self.operation.as_str(), self.id)
    }
}

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct ProviderCatalog {
    chat: BTreeMap<String, Arc<CatalogModel>>,
    image: BTreeMap<String, Arc<CatalogModel>>,
    classifier: BTreeMap<String, Arc<CatalogModel>>,
}

impl ProviderCatalog {
    fn map(&self, operation: CatalogOperation) -> &BTreeMap<String, Arc<CatalogModel>> {
        match operation {
            CatalogOperation::Chat => &self.chat,
            CatalogOperation::Image => &self.image,
            CatalogOperation::Classifier => &self.classifier,
        }
    }
}

/// An immutable, fully validated catalog revision.
///
/// Built only through [`Catalog::builtin`] or [`Catalog::from_bytes`]; every
/// construction re-verifies digest, schema, provider set, structure digest,
/// and each entry, so a `Catalog` value always reflects authentic packaged
/// data.
#[derive(Debug)]
pub struct Catalog {
    provenance: CatalogProvenance,
    providers: BTreeMap<String, ProviderCatalog>,
}

impl Catalog {
    /// The builtin offline bundle: parses and validates the embedded
    /// artifact once per process. Performs no network I/O.
    pub fn builtin() -> Result<&'static Arc<Catalog>, &'static CatalogError> {
        static BUILTIN: OnceLock<Result<Arc<Catalog>, CatalogError>> = OnceLock::new();
        match BUILTIN
            .get_or_init(|| Catalog::from_bytes(BUILTIN_CATALOG_BYTES, BUILTIN_MANIFEST_BYTES))
        {
            Ok(catalog) => Ok(catalog),
            Err(error) => Err(error),
        }
    }

    /// The embedded builtin bundle bytes: `(catalog, manifest)`.
    ///
    /// Exposed so callers (and tests) can revalidate, copy, or derive
    /// mutated candidates from the authentic bytes without new I/O.
    pub fn builtin_bundle_bytes() -> (&'static [u8], &'static [u8]) {
        (BUILTIN_CATALOG_BYTES, BUILTIN_MANIFEST_BYTES)
    }

    /// Validate and build a catalog from raw bundle bytes.
    ///
    /// Checks, in order: manifest shape, bundle schema version, artifact
    /// digest, provider set, per-entry contract, duplicate `type:id`
    /// identity, and the structure digest. Any failure returns a typed
    /// [`CatalogError`] and no catalog.
    pub fn from_bytes(
        catalog_bytes: &[u8],
        manifest_bytes: &[u8],
    ) -> Result<Arc<Catalog>, CatalogError> {
        let manifest: Manifest =
            serde_json::from_slice(manifest_bytes).map_err(|error| CatalogError::Json {
                detail: format!("manifest: {error}"),
            })?;
        if manifest.bundle_schema_version != BUNDLE_SCHEMA_VERSION {
            return Err(CatalogError::BundleSchemaVersion {
                supported: BUNDLE_SCHEMA_VERSION,
                found: manifest.bundle_schema_version,
            });
        }

        let digest = sha256_hex(catalog_bytes);
        if digest != manifest.artifact.sha256 {
            return Err(CatalogError::DigestMismatch {
                expected: manifest.artifact.sha256,
                found: digest,
            });
        }

        let entries: BTreeMap<String, Vec<Value>> =
            serde_json::from_slice(catalog_bytes).map_err(|error| CatalogError::Json {
                detail: format!("catalog: {error}"),
            })?;
        Catalog::build(manifest, entries)
    }

    fn build(
        manifest: Manifest,
        entries: BTreeMap<String, Vec<Value>>,
    ) -> Result<Arc<Catalog>, CatalogError> {
        let expected: std::collections::BTreeSet<&str> =
            manifest.providers.iter().map(String::as_str).collect();
        for provider in &manifest.providers {
            if !entries.contains_key(provider) {
                return Err(CatalogError::MissingProvider {
                    provider: provider.clone(),
                });
            }
        }
        for provider in entries.keys() {
            if !expected.contains(provider.as_str()) {
                return Err(CatalogError::UnknownProvider {
                    provider: provider.clone(),
                });
            }
        }

        let mut providers = BTreeMap::new();
        let mut structure: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut counts = CatalogCounts {
            providers: manifest.providers.len(),
            chat: 0,
            image: 0,
            classifier: 0,
        };

        for provider_id in &manifest.providers {
            let raw_entries = &entries[provider_id];
            if raw_entries.is_empty() {
                return Err(CatalogError::InvalidEntry {
                    provider: provider_id.clone(),
                    identity: "<provider>".to_string(),
                    detail: "provider has an empty model list".to_string(),
                });
            }
            let mut provider_catalog = ProviderCatalog::default();
            let mut provider_structure: BTreeMap<String, String> = BTreeMap::new();
            for raw in raw_entries {
                let model = CatalogModel::parse(provider_id, raw)?;
                let identity = model.identity();
                if provider_structure.contains_key(&identity) {
                    return Err(CatalogError::DuplicateIdentity {
                        provider: provider_id.clone(),
                        identity,
                    });
                }
                provider_structure.insert(identity, model.api.clone());
                let target = match model.operation {
                    CatalogOperation::Chat => {
                        counts.chat += 1;
                        &mut provider_catalog.chat
                    }
                    CatalogOperation::Image => {
                        counts.image += 1;
                        &mut provider_catalog.image
                    }
                    CatalogOperation::Classifier => {
                        counts.classifier += 1;
                        &mut provider_catalog.classifier
                    }
                };
                target.insert(model.id.clone(), Arc::new(model));
            }
            structure.insert(provider_id.clone(), provider_structure);
            providers.insert(provider_id.clone(), provider_catalog);
        }

        let found_structure = sha256_hex(
            serde_json::to_string(&structure)
                .map_err(|error| CatalogError::Json {
                    detail: error.to_string(),
                })?
                .as_bytes(),
        );
        if found_structure != manifest.structure_sha256 {
            return Err(CatalogError::StructureMismatch {
                expected: manifest.structure_sha256,
                found: found_structure,
            });
        }

        let provenance = CatalogProvenance {
            revision: manifest.artifact.revision,
            artifact_sha256: manifest.artifact.sha256,
            source_url: manifest.artifact.url,
            pi_package: manifest.source.pi_package,
            local_origin: manifest.source.local_origin,
            data_schema_version: manifest.data_schema_version,
            bundle_schema_version: manifest.bundle_schema_version,
            structure_sha256: manifest.structure_sha256,
            providers: manifest.providers,
            counts,
            generator: CatalogGeneratorDigests {
                pi_model_data_ts_sha256: manifest.generator.pi_model_data_ts_sha256,
                pi_hydrate_model_catalog_ts_sha256: manifest
                    .generator
                    .pi_hydrate_model_catalog_ts_sha256,
                packer: manifest.generator.packer,
                packer_sha256: manifest.generator.packer_sha256,
            },
        };
        Ok(Arc::new(Catalog {
            provenance,
            providers,
        }))
    }

    /// Authenticity and origin metadata of this revision.
    pub fn provenance(&self) -> &CatalogProvenance {
        &self.provenance
    }

    /// Sorted provider ids present in this catalog.
    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.providers.keys().map(String::as_str)
    }

    /// All entries of one provider qualified for an operation.
    pub fn models(
        &self,
        provider: &str,
        operation: CatalogOperation,
    ) -> Option<impl Iterator<Item = &CatalogModel>> {
        self.providers
            .get(provider)
            .map(|catalog| catalog.map(operation).values().map(Arc::as_ref))
    }

    /// One entry by provider, operation, and bare model id.
    pub fn model(
        &self,
        provider: &str,
        operation: CatalogOperation,
        id: &str,
    ) -> Option<&CatalogModel> {
        self.providers
            .get(provider)
            .and_then(|catalog| catalog.map(operation).get(id))
            .map(Arc::as_ref)
    }
}

// ---------------------------------------------------------------------------
// CatalogHandle — atomic last-known publication
// ---------------------------------------------------------------------------

/// Owned holder of the active catalog revision.
///
/// [`CatalogHandle::try_install`] validates a candidate completely before an
/// atomic swap; a rejected candidate leaves the last-known revision readable.
/// Held [`Arc<Catalog>`] values are never rebound by a later install.
#[derive(Debug)]
pub struct CatalogHandle {
    current: RwLock<Arc<Catalog>>,
}

impl CatalogHandle {
    /// Create a handle over an already validated catalog.
    pub fn new(initial: Arc<Catalog>) -> Self {
        Self {
            current: RwLock::new(initial),
        }
    }

    /// The active (last-known) revision.
    pub fn current(&self) -> Arc<Catalog> {
        self.current.read().expect("catalog handle lock").clone()
    }

    /// Validate and atomically install a candidate revision.
    ///
    /// On any [`CatalogError`] the active revision is unchanged.
    pub fn try_install(
        &self,
        catalog_bytes: &[u8],
        manifest_bytes: &[u8],
    ) -> Result<(), CatalogError> {
        let candidate = Catalog::from_bytes(catalog_bytes, manifest_bytes)?;
        let mut guard = self.current.write().expect("catalog handle lock");
        *guard = candidate;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Parsing internals
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Manifest {
    bundle_schema_version: u32,
    artifact: ManifestArtifact,
    source: ManifestSource,
    data_schema_version: u32,
    structure_sha256: String,
    providers: Vec<String>,
    generator: ManifestGenerator,
}

#[derive(Debug, Deserialize)]
struct ManifestArtifact {
    revision: String,
    sha256: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct ManifestSource {
    pi_package: String,
    local_origin: String,
}

#[derive(Debug, Deserialize)]
struct ManifestGenerator {
    pi_model_data_ts_sha256: String,
    pi_hydrate_model_catalog_ts_sha256: String,
    packer: String,
    packer_sha256: String,
}

impl CatalogModel {
    fn parse(provider_id: &str, raw: &Value) -> Result<CatalogModel, CatalogError> {
        let invalid = |identity: String, detail: String| CatalogError::InvalidEntry {
            provider: provider_id.to_string(),
            identity,
            detail,
        };
        let object = raw
            .as_object()
            .ok_or_else(|| invalid("<unparsed>".into(), "entry must be a JSON object".into()))?;

        let type_str = string_field(object, "type")
            .ok_or_else(|| invalid("<unparsed>".into(), "missing type".into()))?;
        let operation = match type_str.as_str() {
            "chat" => CatalogOperation::Chat,
            "image" => CatalogOperation::Image,
            "classifier" => CatalogOperation::Classifier,
            other => {
                return Err(invalid(
                    format!("{other}:<unparsed>"),
                    format!("type must be chat, image, or classifier, got {other:?}"),
                ));
            }
        };
        let id = string_field(object, "id")
            .ok_or_else(|| invalid(format!("{type_str}:<unparsed>"), "missing id".into()))?;
        let identity = format!("{type_str}:{id}");

        let api = string_field(object, "api")
            .ok_or_else(|| invalid(identity.clone(), "missing api".into()))?;
        let name = string_field(object, "name")
            .filter(|name| !name.is_empty())
            .ok_or_else(|| invalid(identity.clone(), "missing non-empty name".into()))?;
        let base_url = string_field(object, "baseUrl")
            .ok_or_else(|| invalid(identity.clone(), "missing baseUrl string".into()))?;
        if string_field(object, "provider") != Some(provider_id.to_string()) {
            return Err(invalid(
                identity,
                "provider field does not match its key".into(),
            ));
        }

        let input_values = object
            .get("input")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(identity.clone(), "missing input modality list".into()))?;
        if input_values.is_empty() {
            return Err(invalid(
                identity.clone(),
                "input modality list is empty".into(),
            ));
        }
        let mut input = Vec::with_capacity(input_values.len());
        for value in input_values {
            input.push(match value.as_str() {
                Some("text") => CatalogInputModality::Text,
                Some("image") => CatalogInputModality::Image,
                _ => return Err(invalid(identity.clone(), "invalid input modality".into())),
            });
        }

        match operation {
            CatalogOperation::Image => {
                let has_image_output = object
                    .get("output")
                    .and_then(Value::as_array)
                    .is_some_and(|list| list.iter().any(|value| value.as_str() == Some("image")));
                if !has_image_output {
                    return Err(invalid(
                        identity.clone(),
                        "image entry needs image output".into(),
                    ));
                }
            }
            CatalogOperation::Chat | CatalogOperation::Classifier => {
                if object.contains_key("output") {
                    return Err(invalid(
                        identity.clone(),
                        "non-image entry must not declare output modalities".into(),
                    ));
                }
            }
        }

        let cost_object = object
            .get("cost")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid(identity.clone(), "missing cost metadata".into()))?;
        let cost_field = |field: &str| -> Result<f64, CatalogError> {
            cost_object
                .get(field)
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite())
                .ok_or_else(|| invalid(identity.clone(), format!("invalid cost.{field}")))
        };
        let cost = CatalogCost {
            input: cost_field("input")?,
            output: cost_field("output")?,
            cache_read: cost_field("cacheRead")?,
            cache_write: cost_field("cacheWrite")?,
        };

        let positive_u64 = |field: &str| -> Result<u64, CatalogError> {
            object
                .get(field)
                .and_then(Value::as_u64)
                .filter(|value| *value > 0)
                .ok_or_else(|| invalid(identity.clone(), format!("invalid {field}")))
        };

        let (reasoning, context_window, max_tokens) = match operation {
            CatalogOperation::Chat => {
                let reasoning = object
                    .get("reasoning")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| invalid(identity.clone(), "missing reasoning boolean".into()))?;
                (
                    reasoning,
                    Some(positive_u64("contextWindow")?),
                    Some(positive_u64("maxTokens")?),
                )
            }
            CatalogOperation::Classifier => (false, Some(positive_u64("contextWindow")?), None),
            CatalogOperation::Image => (false, None, None),
        };

        Ok(CatalogModel {
            provider: provider_id.to_string(),
            id,
            name,
            operation,
            api,
            base_url,
            reasoning,
            input,
            cost,
            context_window,
            max_tokens,
            raw: raw.clone(),
        })
    }
}

fn string_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
