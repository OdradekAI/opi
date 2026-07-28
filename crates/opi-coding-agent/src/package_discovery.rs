//! Package progressive discovery, resource composition, and registry.
//!
//! Provides the discovery and registry system for packages that compose
//! extensions, skills, prompt fragments, and themes through validated manifests
//! or conventional directories. Package metadata (name, description, version)
//! is available without eagerly loading any contained resource content.
//!
//! # Package Format
//!
//! Each package is a directory containing a `package.toml` manifest and optional
//! resource subdirectories:
//!
//! ```text
//! my-package/
//!   package.toml
//!   extensions/
//!     my-ext/
//!       extension.toml
//!   skills/
//!     my-skill/
//!       SKILL.md
//!   fragments/
//!     my-frag/
//!       FRAGMENT.md
//!   themes/
//!     my-theme/
//!       theme.toml
//! ```
//!
//! # package.toml Format
//!
//! ```toml
//! name = "my-package"
//! description = "A collection of productivity tools."
//! version = "1.0.0"               # optional
//!
//! # Optional: explicit resource allowlists (absent = auto-discover all)
//! extensions = ["my-ext"]
//! skills = ["my-skill"]
//! fragments = ["my-frag"]
//! themes = ["my-theme"]
//!
//! # Optional: resources to exclude by name (matched across all types)
//! disabled = ["deprecated-skill"]
//! ```
//!
//! # Validated Manifests vs Conventional Directories
//!
//! When resource lists (`extensions`, `skills`, `fragments`, `themes`) are
//! present, the package uses **validated manifest** mode: only listed
//! resources are included, and all listed resources must exist (missing assets
//! produce errors).
//!
//! When resource lists are absent, the package uses **conventional directory**
//! mode: all valid resources found in the subdirectories are included.
//!
//! # Discovery Precedence
//!
//! Packages use the same precedence-based discovery as extensions and skills
//! (see [`crate::resource`]). Higher precedence values override lower ones
//! when package names collide.
//!
//! # Security
//!
//! Resource paths are validated to stay within the package directory. Path
//! traversal attempts (via symlinks or `..` components) produce security
//! diagnostic errors.
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from package discovery, manifest parsing, and resource composition.
#[derive(Debug, thiserror::Error)]
pub enum PackageDiscoveryError {
    /// The package.toml file could not be parsed as valid TOML.
    #[error("invalid package manifest at {path}: {reason}")]
    InvalidManifest { path: PathBuf, reason: String },
    /// A required field is missing or empty in the manifest.
    #[error("missing required field '{field}' in package at {path}")]
    MissingField { field: String, path: PathBuf },
    /// Two packages in the same precedence layer use the same name.
    #[error("duplicate package name '{name}' in discovery layer at {path}")]
    DuplicateName { name: String, path: PathBuf },
    /// The package name is invalid (bad characters or too long).
    #[error("invalid package name in {path}: {reason}")]
    InvalidName { path: PathBuf, reason: String },
    /// The description is invalid (too long).
    #[error("invalid description in package at {path}: {reason}")]
    InvalidDescription { path: PathBuf, reason: String },
    /// A resource listed in the include list was not found.
    #[error("missing {kind} '{name}' in package '{package_name}'")]
    MissingAsset {
        package_name: String,
        kind: String,
        name: String,
    },
    /// A resource path escapes the package directory.
    #[error(
        "security: resource path escapes package directory for {package_name}: {path} ({reason})"
    )]
    SecurityDiagnostic {
        package_name: String,
        path: PathBuf,
        reason: String,
    },
    /// An I/O error occurred during discovery or composition.
    #[error("I/O error discovering packages: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum allowed length for a package name.
const MAX_NAME_LEN: usize = 64;

/// Maximum allowed length for a package description.
const MAX_DESCRIPTION_LEN: usize = 1024;

// ---------------------------------------------------------------------------
// TOML deserialization
// ---------------------------------------------------------------------------

/// Top-level TOML structure for package files.
#[derive(Debug, Clone, Deserialize)]
struct TomlPackageFile {
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
    opi_version: Option<String>,
    extensions: Option<Vec<String>>,
    skills: Option<Vec<String>>,
    fragments: Option<Vec<String>>,
    themes: Option<Vec<String>>,
    disabled: Option<Vec<String>>,
    adapter: Option<TomlAdapterTable>,
}

/// TOML structure for the `[adapter]` table.
#[derive(Debug, Clone, Deserialize)]
struct TomlAdapterTable {
    kind: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    protocol: String,
    timeout_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// Parsed package manifest from `package.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageManifest {
    /// Package name. Required, non-empty. Lowercase ASCII letters, digits,
    /// and hyphens. Maximum 64 characters.
    pub name: String,
    /// Human-readable description. Required, non-empty. Maximum 1024
    /// characters.
    pub description: String,
    /// Semantic version string. Optional.
    pub version: Option<String>,
    /// Opi version compatibility constraint. Advisory in 0.x; produces a
    /// diagnostic when incompatible. Optional.
    pub opi_version: Option<String>,
    /// Adapter configuration for process-based extensions. Optional.
    pub adapter: Option<AdapterManifest>,
    /// Explicit list of extension names to include. When `None`, all
    /// discovered extensions in the `extensions/` subdirectory are included.
    pub extensions: Option<Vec<String>>,
    /// Explicit list of skill names to include. When `None`, all discovered
    /// skills in the `skills/` subdirectory are included.
    pub skills: Option<Vec<String>>,
    /// Explicit list of fragment names to include. When `None`, all discovered
    /// fragments in the `fragments/` subdirectory are included.
    pub fragments: Option<Vec<String>>,
    /// Explicit list of theme names to include. When `None`, all discovered
    /// themes in the `themes/` subdirectory are included.
    pub themes: Option<Vec<String>>,
    /// Resource names to exclude from composition, regardless of type.
    pub disabled: Vec<String>,
}

/// Adapter manifest parsed from the `[adapter]` table in `package.toml`.
///
/// Describes a process-based extension adapter that opi communicates with
/// via the JSONL protocol.
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterManifest {
    /// Adapter kind. Currently only `"process-jsonl"` is supported.
    pub kind: String,
    /// Command to start the adapter process. May be a bare name (PATH lookup),
    /// a relative path (resolved against package root), or an absolute path.
    pub command: String,
    /// Arguments to pass to the adapter command.
    pub args: Vec<String>,
    /// Protocol identifier. Currently only `"opi-extension-jsonl-v1"` is
    /// supported.
    pub protocol: String,
    /// Request timeout in milliseconds. When `None`, a default timeout is
    /// used by the adapter host.
    pub timeout_ms: Option<u64>,
}

/// Diagnostic produced when checking `opi_version` compatibility.
#[derive(Debug, Clone, PartialEq)]
pub struct OpiVersionDiagnostic {
    /// Human-readable diagnostic message.
    pub message: String,
}

impl OpiVersionDiagnostic {
    /// Check whether the current opi version satisfies the given constraint.
    ///
    /// Returns `None` if compatible, or a diagnostic describing the
    /// incompatibility. The constraint string uses semver range syntax
    /// (e.g. `">=0.5,<0.7"`). If the constraint cannot be parsed, a
    /// diagnostic is returned describing the parse failure.
    ///
    /// In the 0.x series, this is advisory: the package loads regardless,
    /// but the diagnostic is reported through the `package doctor` command
    /// and resource metadata.
    pub fn check(constraint: &str, current_version: &str) -> Option<Self> {
        // Simple version matching for 0.x advisory diagnostics.
        // Parse the constraint as a comma-separated list of operators.
        let parts: Vec<&str> = constraint.split(',').collect();
        let current = match parse_simple_version(current_version) {
            Some(v) => v,
            None => {
                return Some(Self {
                    message: format!(
                        "cannot parse current version '{current_version}' for compatibility check"
                    ),
                });
            }
        };

        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some(version_str) = part.strip_prefix(">=") {
                let version_str = version_str.trim();
                match parse_simple_version(version_str) {
                    Some(v) if current < v => {
                        return Some(Self {
                            message: format!(
                                "incompatible opi version: {current_version} does not satisfy {constraint}"
                            ),
                        });
                    }
                    None => {
                        return Some(Self {
                            message: format!(
                                "cannot parse version constraint '{part}' in '{constraint}'"
                            ),
                        });
                    }
                    _ => {}
                }
            } else if let Some(version_str) = part.strip_prefix("<=") {
                let version_str = version_str.trim();
                match parse_simple_version(version_str) {
                    Some(v) if current > v => {
                        return Some(Self {
                            message: format!(
                                "incompatible opi version: {current_version} does not satisfy {constraint}"
                            ),
                        });
                    }
                    None => {
                        return Some(Self {
                            message: format!(
                                "cannot parse version constraint '{part}' in '{constraint}'"
                            ),
                        });
                    }
                    _ => {}
                }
            } else if let Some(version_str) = part.strip_prefix('>') {
                let version_str = version_str.trim();
                match parse_simple_version(version_str) {
                    Some(v) if current <= v => {
                        return Some(Self {
                            message: format!(
                                "incompatible opi version: {current_version} does not satisfy {constraint}"
                            ),
                        });
                    }
                    None => {
                        return Some(Self {
                            message: format!(
                                "cannot parse version constraint '{part}' in '{constraint}'"
                            ),
                        });
                    }
                    _ => {}
                }
            } else if let Some(version_str) = part.strip_prefix('<') {
                let version_str = version_str.trim();
                match parse_simple_version(version_str) {
                    Some(v) if current >= v => {
                        return Some(Self {
                            message: format!(
                                "incompatible opi version: {current_version} does not satisfy {constraint}"
                            ),
                        });
                    }
                    None => {
                        return Some(Self {
                            message: format!(
                                "cannot parse version constraint '{part}' in '{constraint}'"
                            ),
                        });
                    }
                    _ => {}
                }
            } else if let Some(version_str) = part.strip_prefix('=') {
                let version_str = version_str.trim();
                match parse_simple_version(version_str) {
                    Some(v) if current != v => {
                        return Some(Self {
                            message: format!(
                                "incompatible opi version: {current_version} does not satisfy {constraint}"
                            ),
                        });
                    }
                    None => {
                        return Some(Self {
                            message: format!(
                                "cannot parse version constraint '{part}' in '{constraint}'"
                            ),
                        });
                    }
                    _ => {}
                }
            } else {
                // Bare version or unknown prefix — try as exact match
                match parse_simple_version(part) {
                    Some(v) if current != v => {
                        return Some(Self {
                            message: format!(
                                "incompatible opi version: {current_version} does not satisfy {constraint}"
                            ),
                        });
                    }
                    None => {
                        return Some(Self {
                            message: format!(
                                "cannot parse version constraint '{part}' in '{constraint}'"
                            ),
                        });
                    }
                    _ => {}
                }
            }
        }

        None
    }
}

/// Parse a version string "X.Y" or "X.Y.Z" into a comparable tuple.
/// Two-part versions get an implicit `.0` patch.
fn parse_simple_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim();
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        2 => Some((parts[0].parse().ok()?, parts[1].parse().ok()?, 0)),
        3 => Some((
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        )),
        _ => None,
    }
}

/// Resolve the adapter command path based on its form.
///
/// - Absolute path: used as-is.
/// - Relative path (contains separators): resolved against `package_root`.
/// - Bare name (no separators): returned as-is for PATH lookup.
pub fn resolve_adapter_command(adapter: &AdapterManifest, package_root: &Path) -> PathBuf {
    let cmd = &adapter.command;
    let path = Path::new(cmd);

    if path.is_absolute() {
        path.to_path_buf()
    } else if cmd.contains('/') || cmd.contains('\\') {
        // Relative path with separators: resolve against package root
        package_root.join(cmd)
    } else {
        // Bare name: PATH lookup
        PathBuf::from(cmd)
    }
}

impl PackageManifest {
    /// Parse a manifest from TOML content, validating required fields.
    pub fn from_toml(content: &str, path: &Path) -> Result<Self, PackageDiscoveryError> {
        let file: TomlPackageFile =
            toml::from_str(content).map_err(|e| PackageDiscoveryError::InvalidManifest {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?;

        let name = file.name.filter(|n| !n.trim().is_empty()).ok_or_else(|| {
            PackageDiscoveryError::MissingField {
                field: "name".into(),
                path: path.to_path_buf(),
            }
        })?;

        validate_package_name(&name, path)?;

        let description = file
            .description
            .filter(|d| !d.trim().is_empty())
            .ok_or_else(|| PackageDiscoveryError::MissingField {
                field: "description".into(),
                path: path.to_path_buf(),
            })?;

        validate_description(&description, path)?;

        let adapter = file
            .adapter
            .map(|a| validate_and_build_adapter(a, path))
            .transpose()?;

        Ok(Self {
            name,
            description,
            version: file.version,
            opi_version: file.opi_version,
            adapter,
            extensions: file.extensions,
            skills: file.skills,
            fragments: file.fragments,
            themes: file.themes,
            disabled: file.disabled.unwrap_or_default(),
        })
    }
}

/// Validate adapter fields and build an [`AdapterManifest`].
fn validate_and_build_adapter(
    table: TomlAdapterTable,
    path: &Path,
) -> Result<AdapterManifest, PackageDiscoveryError> {
    if table.kind != "process-jsonl" {
        return Err(PackageDiscoveryError::InvalidManifest {
            path: path.to_path_buf(),
            reason: format!("unsupported adapter kind '{}'", table.kind),
        });
    }

    if table.command.trim().is_empty() {
        return Err(PackageDiscoveryError::MissingField {
            field: "adapter.command".into(),
            path: path.to_path_buf(),
        });
    }

    if table.protocol != "opi-extension-jsonl-v1" {
        return Err(PackageDiscoveryError::InvalidManifest {
            path: path.to_path_buf(),
            reason: format!("unsupported adapter protocol '{}'", table.protocol),
        });
    }

    Ok(AdapterManifest {
        kind: table.kind,
        command: table.command,
        args: table.args,
        protocol: table.protocol,
        timeout_ms: table.timeout_ms,
    })
}

// ---------------------------------------------------------------------------
// Resource kinds
// ---------------------------------------------------------------------------

/// The kind of a resource within a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// Extension resource.
    Extension,
    /// Skill resource.
    Skill,
    /// Prompt fragment resource.
    Fragment,
    /// Theme resource.
    Theme,
}

impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Extension => write!(f, "extension"),
            Self::Skill => write!(f, "skill"),
            Self::Fragment => write!(f, "fragment"),
            Self::Theme => write!(f, "theme"),
        }
    }
}

/// Resource type metadata: subdirectory name and marker file.
struct ResourceTypeSpec {
    kind: ResourceKind,
    subdir: &'static str,
    marker: &'static str,
}

const RESOURCE_TYPES: &[ResourceTypeSpec] = &[
    ResourceTypeSpec {
        kind: ResourceKind::Extension,
        subdir: "extensions",
        marker: "extension.toml",
    },
    ResourceTypeSpec {
        kind: ResourceKind::Skill,
        subdir: "skills",
        marker: "SKILL.md",
    },
    ResourceTypeSpec {
        kind: ResourceKind::Fragment,
        subdir: "fragments",
        marker: "FRAGMENT.md",
    },
    ResourceTypeSpec {
        kind: ResourceKind::Theme,
        subdir: "themes",
        marker: "theme.toml",
    },
];

// ---------------------------------------------------------------------------
// Composed resource
// ---------------------------------------------------------------------------

/// A discovered resource within a package.
///
/// Contains the resource kind, directory name, and path. Does not hold parsed
/// resource manifests — those are loaded on demand by the respective discovery
/// modules when needed.
#[derive(Debug, Clone)]
pub struct ComposedResource {
    /// The kind of resource.
    pub kind: ResourceKind,
    /// Resource directory name (used as identifier).
    pub name: String,
    /// Absolute path to the resource directory.
    pub path: PathBuf,
}

/// Discovery layers produced by composing package-contained resources.
#[derive(Debug, Clone, Default)]
pub struct PackageComposedResourceLayers {
    pub extensions: Vec<crate::resource::DiscoveryLayer>,
    pub skills: Vec<crate::resource::DiscoveryLayer>,
    pub fragments: Vec<crate::resource::DiscoveryLayer>,
    pub themes: Vec<crate::resource::DiscoveryLayer>,
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Package resource
// ---------------------------------------------------------------------------

/// A discovered package resource with its manifest, filesystem path, and layer
/// precedence.
///
/// The manifest metadata is available immediately. Resource composition is
/// performed on demand via [`compose`](PackageResource::compose).
#[derive(Debug, Clone)]
pub struct PackageResource {
    /// The parsed package manifest (metadata only).
    pub manifest: PackageManifest,
    /// Absolute path to the package directory.
    pub path: PathBuf,
    /// Path to the `package.toml` file for reference.
    pub package_toml_path: PathBuf,
    /// Precedence value of the discovery layer that produced this resource.
    pub layer_precedence: u32,
}

impl PackageResource {
    /// Compose all resources from this package, applying filtering and security
    /// checks.
    ///
    /// Scans the `extensions/`, `skills/`, `fragments/`, and `themes/`
    /// subdirectories. When include lists are present in the manifest, only
    /// listed resources are included and all must be found. Resources in the
    /// `disabled` list are excluded. Resource paths are validated to stay
    /// within the package directory.
    pub fn compose(&self) -> Result<Vec<ComposedResource>, PackageDiscoveryError> {
        let mut resources = Vec::new();
        let disabled: HashSet<&str> = self.manifest.disabled.iter().map(|s| s.as_str()).collect();

        let include_lists: [(ResourceKind, &Option<Vec<String>>); 4] = [
            (ResourceKind::Extension, &self.manifest.extensions),
            (ResourceKind::Skill, &self.manifest.skills),
            (ResourceKind::Fragment, &self.manifest.fragments),
            (ResourceKind::Theme, &self.manifest.themes),
        ];

        for spec in RESOURCE_TYPES {
            let include_list = include_lists
                .iter()
                .find(|(k, _)| *k == spec.kind)
                .map(|(_, l)| *l)
                .unwrap_or(&None);

            self.compose_type(spec, include_list, &disabled, &mut resources)?;
        }

        Ok(resources)
    }

    /// Compose resources of a single type.
    fn compose_type(
        &self,
        spec: &ResourceTypeSpec,
        include_list: &Option<Vec<String>>,
        disabled: &HashSet<&str>,
        resources: &mut Vec<ComposedResource>,
    ) -> Result<(), PackageDiscoveryError> {
        let type_dir = self.path.join(spec.subdir);

        if !type_dir.is_dir() {
            // If include list exists, all entries must be found
            if let Some(includes) = include_list {
                for name in includes {
                    if !disabled.contains(name.as_str()) {
                        return Err(PackageDiscoveryError::MissingAsset {
                            package_name: self.manifest.name.clone(),
                            kind: spec.kind.to_string(),
                            name: name.clone(),
                        });
                    }
                }
            }
            return Ok(());
        }

        let canonical_package = self.path.canonicalize()?;

        if let Some(includes) = include_list {
            // Validated manifest mode: only include listed resources
            for name in includes {
                if disabled.contains(name.as_str()) {
                    continue;
                }

                let resource_dir = type_dir.join(name);
                if !resource_dir.is_dir() {
                    return Err(PackageDiscoveryError::MissingAsset {
                        package_name: self.manifest.name.clone(),
                        kind: spec.kind.to_string(),
                        name: name.clone(),
                    });
                }

                let marker = resource_dir.join(spec.marker);
                if !marker.exists() {
                    return Err(PackageDiscoveryError::MissingAsset {
                        package_name: self.manifest.name.clone(),
                        kind: spec.kind.to_string(),
                        name: name.clone(),
                    });
                }

                // Security check
                let canonical_resource = resource_dir.canonicalize()?;
                if !canonical_resource.starts_with(&canonical_package) {
                    return Err(PackageDiscoveryError::SecurityDiagnostic {
                        package_name: self.manifest.name.clone(),
                        path: canonical_resource,
                        reason: format!(
                            "resource path escapes package directory for {} '{}'",
                            spec.kind, name
                        ),
                    });
                }

                resources.push(ComposedResource {
                    kind: spec.kind,
                    name: name.clone(),
                    path: resource_dir,
                });
            }
        } else {
            // Conventional directory mode: auto-discover all valid resources
            let entries = std::fs::read_dir(&type_dir)?;
            for entry in entries {
                let entry = entry?;
                let resource_dir = entry.path();

                if !resource_dir.is_dir() {
                    continue;
                }

                let resource_name = match resource_dir.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };

                // Skip disabled resources
                if disabled.contains(resource_name.as_str()) {
                    continue;
                }

                // Check marker file exists
                let marker = resource_dir.join(spec.marker);
                if !marker.exists() {
                    continue;
                }

                // Security check
                let canonical_resource = resource_dir.canonicalize()?;
                if !canonical_resource.starts_with(&canonical_package) {
                    return Err(PackageDiscoveryError::SecurityDiagnostic {
                        package_name: self.manifest.name.clone(),
                        path: canonical_resource,
                        reason: format!(
                            "resource path escapes package directory for {} '{}'",
                            spec.kind, resource_name
                        ),
                    });
                }

                resources.push(ComposedResource {
                    kind: spec.kind,
                    name: resource_name,
                    path: resource_dir,
                });
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Discover packages across multiple layers with precedence-based
/// deduplication.
///
/// Each layer's scan directory is enumerated for subdirectories containing
/// `package.toml` files. When multiple layers produce packages with the same
/// name, the one with the highest `precedence` value is kept. Duplicate names
/// within the same precedence layer are reported as an error.
///
/// Returns the deduplicated list of discovered package resources, sorted by
/// name. Missing scan directories are silently skipped.
pub fn discover_packages(
    layers: &[crate::resource::DiscoveryLayer],
) -> Result<Vec<PackageResource>, PackageDiscoveryError> {
    let mut seen: std::collections::HashMap<String, PackageResource> =
        std::collections::HashMap::new();

    for layer in layers {
        let scan_dir = layer.scan_dir();
        if !scan_dir.is_dir() {
            continue;
        }

        if scan_dir.join("package.toml").exists() {
            discover_package_dir(&scan_dir, layer, &mut seen)?;
            continue;
        }

        let entries = match std::fs::read_dir(&scan_dir) {
            Ok(entries) => entries,
            Err(e) => return Err(PackageDiscoveryError::Io(e)),
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let pkg_toml = path.join("package.toml");
            if !pkg_toml.exists() {
                continue;
            }

            discover_package_dir(&path, layer, &mut seen)?;
        }
    }

    let mut resources: Vec<PackageResource> = seen.into_values().collect();
    resources.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(resources)
}

pub fn resolve_adapter_command_checked(
    adapter: &AdapterManifest,
    package_root: &Path,
) -> Result<PathBuf, PackageDiscoveryError> {
    let cmd = &adapter.command;
    let path = Path::new(cmd);

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    if path
        .components()
        .any(|component| matches!(component, Component::Prefix(_)))
    {
        return Err(PackageDiscoveryError::SecurityDiagnostic {
            package_name: "adapter".to_string(),
            path: path.to_path_buf(),
            reason: "adapter command escapes package root".to_string(),
        });
    }

    if !(cmd.contains('/') || cmd.contains('\\')) {
        return Ok(PathBuf::from(cmd));
    }

    let root = normalize_path(package_root);
    let resolved = normalize_path(&package_root.join(path));
    if !resolved.starts_with(&root) {
        return Err(PackageDiscoveryError::SecurityDiagnostic {
            package_name: "adapter".to_string(),
            path: resolved,
            reason: "adapter command escapes package root".to_string(),
        });
    }

    Ok(resolved)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

pub fn discover_package_root(
    path: &Path,
    layer_precedence: u32,
) -> Result<PackageResource, PackageDiscoveryError> {
    let layer = crate::resource::DiscoveryLayer {
        kind: crate::resource::DiscoveryLayerKind::Package,
        root: path.to_path_buf(),
        subdirectory: None,
        precedence: layer_precedence,
    };
    let mut seen = std::collections::HashMap::new();
    discover_package_dir(path, &layer, &mut seen)?;
    seen.into_values()
        .next()
        .ok_or_else(|| PackageDiscoveryError::InvalidManifest {
            path: path.join("package.toml"),
            reason: "package.toml did not produce a package resource".to_string(),
        })
}

fn discover_package_dir(
    path: &Path,
    layer: &crate::resource::DiscoveryLayer,
    seen: &mut std::collections::HashMap<String, PackageResource>,
) -> Result<(), PackageDiscoveryError> {
    let pkg_toml = path.join("package.toml");
    let content = std::fs::read_to_string(&pkg_toml)?;
    let manifest = PackageManifest::from_toml(&content, &pkg_toml)?;

    let canonical = path.canonicalize()?;

    match seen.get(&manifest.name) {
        Some(existing) if layer.precedence == existing.layer_precedence => {
            return Err(PackageDiscoveryError::DuplicateName {
                name: manifest.name,
                path: canonical,
            });
        }
        Some(existing) if layer.precedence < existing.layer_precedence => return Ok(()),
        Some(_) | None => {
            seen.insert(
                manifest.name.clone(),
                PackageResource {
                    manifest,
                    path: canonical,
                    package_toml_path: pkg_toml,
                    layer_precedence: layer.precedence,
                },
            );
        }
    }

    Ok(())
}

/// Compose package resources into direct discovery layers grouped by kind.
///
/// Composition diagnostics are collected instead of panicking so production
/// harness construction can surface them in metadata.
pub fn package_composed_resource_layers(
    packages: &[PackageResource],
) -> PackageComposedResourceLayers {
    let mut result = PackageComposedResourceLayers::default();
    let mut ordered: Vec<&PackageResource> = packages.iter().collect();
    ordered.sort_by(|a, b| {
        a.layer_precedence
            .cmp(&b.layer_precedence)
            .then_with(|| a.manifest.name.cmp(&b.manifest.name))
    });

    for package in ordered {
        let mut resources = match package.compose() {
            Ok(resources) => resources,
            Err(e) => {
                result
                    .diagnostics
                    .push(format!("package '{}': {e}", package.manifest.name));
                continue;
            }
        };
        resources.sort_by(|a, b| {
            resource_kind_order(a.kind)
                .cmp(&resource_kind_order(b.kind))
                .then_with(|| a.name.cmp(&b.name))
        });
        for resource in resources {
            let layer = crate::resource::DiscoveryLayer {
                kind: crate::resource::DiscoveryLayerKind::Package,
                root: resource.path,
                subdirectory: None,
                precedence: package.layer_precedence,
            };
            match resource.kind {
                ResourceKind::Extension => result.extensions.push(layer),
                ResourceKind::Skill => result.skills.push(layer),
                ResourceKind::Fragment => result.fragments.push(layer),
                ResourceKind::Theme => result.themes.push(layer),
            }
        }
    }

    result
}

fn resource_kind_order(kind: ResourceKind) -> u8 {
    match kind {
        ResourceKind::Extension => 0,
        ResourceKind::Skill => 1,
        ResourceKind::Fragment => 2,
        ResourceKind::Theme => 3,
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// A registry of discovered packages supporting progressive disclosure and
/// resource composition.
pub struct PackageRegistry {
    packages: Vec<PackageResource>,
}

impl PackageRegistry {
    /// Build a registry from discovered package resources.
    pub fn from_resources(packages: Vec<PackageResource>) -> Self {
        Self { packages }
    }

    /// Return sorted list of all package names.
    pub fn names(&self) -> Vec<&str> {
        self.packages
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect()
    }

    /// Look up a package by name, returning its resource (metadata only).
    pub fn get(&self, name: &str) -> Option<&PackageResource> {
        self.packages.iter().find(|p| p.manifest.name == name)
    }

    /// Format all package metadata as a string suitable for inclusion in a
    /// system prompt or command listing.
    pub fn format_for_prompt(&self) -> String {
        if self.packages.is_empty() {
            return String::new();
        }

        let parts: Vec<String> = self
            .packages
            .iter()
            .map(|p| {
                let version = p
                    .manifest
                    .version
                    .as_deref()
                    .map(|v| format!(" v{v}"))
                    .unwrap_or_default();
                format!(
                    "- {}: {}{}",
                    p.manifest.name, p.manifest.description, version
                )
            })
            .collect();
        parts.join("\n")
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validate that a package name contains only allowed characters and is within
/// length bounds.
fn validate_package_name(name: &str, path: &Path) -> Result<(), PackageDiscoveryError> {
    if name.len() > MAX_NAME_LEN {
        return Err(PackageDiscoveryError::InvalidName {
            path: path.to_path_buf(),
            reason: format!(
                "name exceeds maximum length of {MAX_NAME_LEN} characters ({} found)",
                name.len()
            ),
        });
    }

    for ch in name.chars() {
        let valid = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-';
        if !valid {
            return Err(PackageDiscoveryError::InvalidName {
                path: path.to_path_buf(),
                reason: format!(
                    "name contains invalid character '{ch}': \
                     only lowercase a-z, 0-9, and hyphens are allowed"
                ),
            });
        }
    }

    Ok(())
}

/// Validate that a description is within length bounds.
fn validate_description(desc: &str, path: &Path) -> Result<(), PackageDiscoveryError> {
    if desc.len() > MAX_DESCRIPTION_LEN {
        return Err(PackageDiscoveryError::InvalidDescription {
            path: path.to_path_buf(),
            reason: format!(
                "description exceeds maximum length of {MAX_DESCRIPTION_LEN} characters \
                 ({} found)",
                desc.len()
            ),
        });
    }
    Ok(())
}
