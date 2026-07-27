//! Extension resource discovery and loading.
//!
//! Provides the resource loading strategy for discovering extension manifests
//! from project, user, and explicit paths with documented precedence.
//!
//! # Precedence Model
//!
//! Extension resources are discovered from multiple layers, each with a numeric
//! precedence value. Higher precedence values override lower ones when
//! extension names collide. The standard precedence order is:
//!
//! 1. **User-level** (`~/.config/opi/extensions/` on Unix,
//!    `%APPDATA%\opi\extensions\` on Windows) — precedence 0
//! 2. **Project-level** (`.opi/extensions/` in workspace root) — precedence 1
//! 3. **Explicit** (CLI `--extension` paths or config `extensions.paths`) —
//!    precedence 2
//!
//! When two layers provide an extension with the same name, the higher
//! precedence layer wins. Within a single layer, duplicate names produce an
//! error.
//!
//! # Manifest Format
//!
//! Each extension directory must contain an `extension.toml` manifest:
//!
//! ```toml
//! [extension]
//! name = "my-extension"    # required, non-empty
//! version = "1.0.0"        # optional
//! description = "..."      # optional
//! ```
//!
//! # Path Normalization
//!
//! All paths are canonicalized (resolved to absolute form) before comparison.
//! This prevents duplicate detection bypass via relative paths or symlinks.
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::path::{Path, PathBuf};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from extension resource discovery.
#[derive(Debug, thiserror::Error)]
pub enum ResourceDiscoveryError {
    /// A manifest file could not be parsed as valid TOML.
    #[error("invalid extension manifest at {path}: {reason}")]
    InvalidManifest { path: PathBuf, reason: String },
    /// A required field is missing or empty in the manifest.
    #[error("missing required field '{field}' in manifest at {path}")]
    MissingField { field: String, path: PathBuf },
    /// Two resources in the same precedence layer use the same name.
    #[error("duplicate extension name '{name}' in discovery layer at {path}")]
    DuplicateName { name: String, path: PathBuf },
    /// An I/O error occurred during discovery.
    #[error("I/O error discovering extensions: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// Parsed extension manifest from `extension.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionManifest {
    /// Extension name. Required, non-empty, unique across all layers.
    pub name: String,
    /// Semantic version. Optional.
    pub version: Option<String>,
    /// Human-readable description. Optional.
    pub description: Option<String>,
}

/// Top-level TOML structure wrapping the `[extension]` table.
#[derive(Debug, Clone, Deserialize)]
struct TomlExtensionFile {
    extension: TomlExtensionTable,
}

/// Fields within the `[extension]` TOML table.
#[derive(Debug, Clone, Deserialize)]
struct TomlExtensionTable {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

impl ExtensionManifest {
    /// Parse a manifest from TOML content, validating required fields.
    pub fn from_toml(content: &str, path: &Path) -> Result<Self, ResourceDiscoveryError> {
        let file: TomlExtensionFile =
            toml::from_str(content).map_err(|e| ResourceDiscoveryError::InvalidManifest {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?;

        let raw = file.extension;

        let name = raw.name.filter(|n| !n.trim().is_empty()).ok_or_else(|| {
            ResourceDiscoveryError::MissingField {
                field: "name".into(),
                path: path.to_path_buf(),
            }
        })?;

        Ok(Self {
            name,
            version: raw.version,
            description: raw.description,
        })
    }
}

// ---------------------------------------------------------------------------
// Discovery types
// ---------------------------------------------------------------------------

/// A single discovery layer with root path, optional subdirectory, and
/// precedence value.
///
/// Higher precedence values override lower ones for duplicate extension names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryLayerKind {
    /// User-global configuration resources.
    User,
    /// Resources owned by the current project and therefore trust-gated.
    Project,
    /// Explicit paths supplied by the user or embedder.
    Explicit,
    /// Resources composed from an already-authorized package.
    Package,
}

/// A single discovery layer with a structural origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryLayer {
    /// Structural origin used by project-trust filtering.
    pub kind: DiscoveryLayerKind,
    /// Root directory for this discovery layer.
    pub root: PathBuf,
    /// Optional subdirectory to append to root (e.g. `.opi/extensions`).
    /// When `None`, the root is used directly.
    pub subdirectory: Option<String>,
    /// Numeric precedence. Higher values win on name collision.
    pub precedence: u32,
}

impl DiscoveryLayer {
    /// Resolve the full scan directory for this layer.
    pub fn scan_dir(&self) -> PathBuf {
        match &self.subdirectory {
            Some(sub) => self.root.join(sub),
            None => self.root.clone(),
        }
    }
}

/// Explicit resource paths from resolved configuration or embedder setup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExplicitResourcePaths {
    pub extensions: Vec<PathBuf>,
    pub packages: Vec<PathBuf>,
    pub skills: Vec<PathBuf>,
    pub fragments: Vec<PathBuf>,
    pub themes: Vec<PathBuf>,
}

/// Discovery layers for every metadata-backed resource kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceDiscoveryLayers {
    pub extensions: Vec<DiscoveryLayer>,
    pub packages: Vec<DiscoveryLayer>,
    pub skills: Vec<DiscoveryLayer>,
    pub fragments: Vec<DiscoveryLayer>,
    pub themes: Vec<DiscoveryLayer>,
}

const USER_LAYER_PRECEDENCE: u32 = 0;
const PROJECT_LAYER_PRECEDENCE: u32 = 1;
const EXPLICIT_LAYER_PRECEDENCE: u32 = 2;

/// Build the standard user/project/explicit discovery layers for a workspace.
///
/// Missing directories are handled by the per-kind discovery functions. Relative
/// explicit paths are resolved against `workspace_root`.
pub fn standard_discovery_layers(
    workspace_root: &Path,
    user_config_dir: Option<&Path>,
    explicit: ExplicitResourcePaths,
) -> ResourceDiscoveryLayers {
    ResourceDiscoveryLayers {
        extensions: standard_layers_for_kind(
            workspace_root,
            user_config_dir,
            "extensions",
            ".opi/extensions",
            &explicit.extensions,
        ),
        packages: standard_layers_for_kind(
            workspace_root,
            user_config_dir,
            "packages",
            ".opi/packages",
            &explicit.packages,
        ),
        skills: standard_layers_for_kind(
            workspace_root,
            user_config_dir,
            "skills",
            ".opi/skills",
            &explicit.skills,
        ),
        fragments: standard_layers_for_kind(
            workspace_root,
            user_config_dir,
            "fragments",
            ".opi/fragments",
            &explicit.fragments,
        ),
        themes: standard_layers_for_kind(
            workspace_root,
            user_config_dir,
            "themes",
            ".opi/themes",
            &explicit.themes,
        ),
    }
}

fn standard_layers_for_kind(
    workspace_root: &Path,
    user_config_dir: Option<&Path>,
    user_subdir: &str,
    project_subdir: &str,
    explicit_paths: &[PathBuf],
) -> Vec<DiscoveryLayer> {
    let mut layers = Vec::new();
    if let Some(user_config_dir) = user_config_dir {
        layers.push(DiscoveryLayer {
            kind: DiscoveryLayerKind::User,
            root: user_config_dir.to_path_buf(),
            subdirectory: Some(user_subdir.to_owned()),
            precedence: USER_LAYER_PRECEDENCE,
        });
    }
    layers.push(DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: workspace_root.to_path_buf(),
        subdirectory: Some(project_subdir.to_owned()),
        precedence: PROJECT_LAYER_PRECEDENCE,
    });
    layers.extend(explicit_paths.iter().map(|path| DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: resolve_explicit_path(workspace_root, path),
        subdirectory: None,
        precedence: EXPLICIT_LAYER_PRECEDENCE,
    }));
    layers
}

fn resolve_explicit_path(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

/// A discovered extension resource with its manifest, filesystem path, and
/// layer precedence.
#[derive(Debug, Clone)]
pub struct ExtensionResource {
    /// The parsed extension manifest.
    pub manifest: ExtensionManifest,
    /// Absolute path to the extension directory.
    pub path: PathBuf,
    /// Precedence value of the discovery layer that produced this resource.
    pub layer_precedence: u32,
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Discover extension resources across multiple layers with precedence-based
/// deduplication.
///
/// Layers are processed in order. For each layer, the scan directory is
/// enumerated for subdirectories containing `extension.toml` files. When
/// multiple layers produce extensions with the same name, the one with the
/// highest `precedence` value is kept. Duplicate names within the same
/// precedence layer are reported as an error.
///
/// Returns the deduplicated list of discovered resources, or the first error
/// encountered during manifest parsing.
pub fn discover_extension_resources(
    layers: &[DiscoveryLayer],
) -> Result<Vec<ExtensionResource>, ResourceDiscoveryError> {
    let mut seen: std::collections::HashMap<String, ExtensionResource> =
        std::collections::HashMap::new();

    for layer in layers {
        let scan_dir = layer.scan_dir();
        if !scan_dir.is_dir() {
            continue;
        }

        if scan_dir.join("extension.toml").exists() {
            discover_extension_dir(&scan_dir, layer, &mut seen)?;
            continue;
        }

        let entries = match std::fs::read_dir(&scan_dir) {
            Ok(entries) => entries,
            Err(e) => return Err(ResourceDiscoveryError::Io(e)),
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Only process directories.
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("extension.toml");
            if !manifest_path.exists() {
                continue;
            }

            discover_extension_dir(&path, layer, &mut seen)?;
        }
    }

    // Return resources sorted by name for deterministic ordering.
    let mut resources: Vec<ExtensionResource> = seen.into_values().collect();
    resources.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(resources)
}

fn discover_extension_dir(
    path: &Path,
    layer: &DiscoveryLayer,
    seen: &mut std::collections::HashMap<String, ExtensionResource>,
) -> Result<(), ResourceDiscoveryError> {
    let manifest_path = path.join("extension.toml");
    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest = ExtensionManifest::from_toml(&content, &manifest_path)?;

    let canonical = path.canonicalize()?;

    match seen.get(&manifest.name) {
        Some(existing) if layer.precedence == existing.layer_precedence => {
            return Err(ResourceDiscoveryError::DuplicateName {
                name: manifest.name,
                path: canonical,
            });
        }
        Some(existing) if layer.precedence < existing.layer_precedence => return Ok(()),
        Some(_) | None => {
            seen.insert(
                manifest.name.clone(),
                ExtensionResource {
                    manifest,
                    path: canonical,
                    layer_precedence: layer.precedence,
                },
            );
        }
    }

    Ok(())
}
