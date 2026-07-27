use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::package_discovery::{
    OpiVersionDiagnostic, PackageResource, discover_package_root, resolve_adapter_command_checked,
};
use crate::package_store::{
    PackageDeclaration, PackageIdentity, PackageLockEntry, PackageSource, PackageStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstalledPackageScope {
    Global,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PackageDiagnostic {
    pub scope: InstalledPackageScope,
    pub source: String,
    pub severity: PackageDiagnosticSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedInstalledPackage {
    pub scope: InstalledPackageScope,
    pub declaration: PackageDeclaration,
    pub identity: PackageIdentity,
    pub lock: Option<PackageLockEntry>,
    pub package: PackageResource,
}

#[derive(Debug, Clone, Default)]
pub struct InstalledPackageResolution {
    pub packages: Vec<ResolvedInstalledPackage>,
    pub diagnostics: Vec<PackageDiagnostic>,
}

#[derive(Debug, thiserror::Error)]
pub enum PackageResolverError {
    #[error("package resolver failed: {0}")]
    Failed(String),
}

pub fn source_identity_for_resolution(
    source: &PackageSource,
    base: &Path,
) -> Result<PackageIdentity, PackageResolverError> {
    match source {
        PackageSource::Local { path } => {
            let root = if path.is_absolute() {
                path.clone()
            } else {
                base.join(path)
            };
            let canonical = root.canonicalize().map_err(|e| {
                PackageResolverError::Failed(format!("canonicalize {}: {e}", root.display()))
            })?;
            Ok(PackageIdentity {
                kind: "local".to_string(),
                value: canonical.display().to_string(),
            })
        }
        PackageSource::Git { url, .. } => Ok(PackageIdentity {
            kind: "git".to_string(),
            value: url.clone(),
        }),
    }
}

pub fn resolve_installed_packages(
    workspace_root: &Path,
    user_config_dir: &Path,
) -> Result<InstalledPackageResolution, PackageResolverError> {
    resolve_installed_packages_for_scopes(
        workspace_root,
        user_config_dir,
        &[
            InstalledPackageScope::Global,
            InstalledPackageScope::Project,
        ],
    )
}

/// Resolve only the requested package stores.
///
/// Callers that have not authorized project resources pass only
/// [`InstalledPackageScope::Global`]. Excluded stores are never opened or
/// parsed, so their errors and name precedence cannot affect global packages.
pub fn resolve_installed_packages_for_scopes(
    workspace_root: &Path,
    user_config_dir: &Path,
    scopes: &[InstalledPackageScope],
) -> Result<InstalledPackageResolution, PackageResolverError> {
    let all =
        resolve_declared_installed_packages_for_scopes(workspace_root, user_config_dir, scopes)?;
    let mut by_name: HashMap<String, ResolvedInstalledPackage> = HashMap::new();
    let mut diagnostics = all.diagnostics;
    for package in all.packages {
        let name = package.package.manifest.name.clone();
        match by_name.get(&name) {
            // An equal-or-higher precedence package already owns this name.
            Some(existing)
                if scope_precedence(existing.scope) >= scope_precedence(package.scope) =>
            {
                // A same-precedence collision is a degraded path: the later
                // package (by declaration source order) is dropped at runtime.
                // A higher-precedence override (e.g. project over global) is a
                // legitimate precedence change handled by the `_` arm below and
                // is not reported as a duplicate.
                if scope_precedence(existing.scope) == scope_precedence(package.scope) {
                    diagnostics.push(diagnostic(
                        package.scope,
                        &package.declaration.source,
                        "duplicate_name",
                        format!(
                            "duplicate manifest name '{}' in scope {:?} \
                             (already provided by '{}'); package is disabled at runtime",
                            name, package.scope, existing.declaration.source
                        ),
                    ));
                }
            }
            _ => {
                by_name.insert(name, package);
            }
        }
    }

    let mut packages: Vec<_> = by_name.into_values().collect();
    packages.sort_by(|a, b| {
        scope_precedence(a.scope)
            .cmp(&scope_precedence(b.scope))
            .then_with(|| a.package.manifest.name.cmp(&b.package.manifest.name))
    });

    Ok(InstalledPackageResolution {
        packages,
        diagnostics,
    })
}

pub fn resolve_declared_installed_packages(
    workspace_root: &Path,
    user_config_dir: &Path,
) -> Result<InstalledPackageResolution, PackageResolverError> {
    resolve_declared_installed_packages_for_scopes(
        workspace_root,
        user_config_dir,
        &[
            InstalledPackageScope::Global,
            InstalledPackageScope::Project,
        ],
    )
}

pub fn resolve_declared_installed_packages_for_scopes(
    workspace_root: &Path,
    user_config_dir: &Path,
    scopes: &[InstalledPackageScope],
) -> Result<InstalledPackageResolution, PackageResolverError> {
    let mut resolved = Vec::new();
    let mut diagnostics = Vec::new();

    if scopes.contains(&InstalledPackageScope::Global) {
        resolve_scope(
            InstalledPackageScope::Global,
            &PackageStore::global(user_config_dir.to_path_buf()),
            user_config_dir,
            0,
            &mut resolved,
            &mut diagnostics,
        )?;
    }
    if scopes.contains(&InstalledPackageScope::Project) {
        resolve_scope(
            InstalledPackageScope::Project,
            &PackageStore::project(workspace_root.to_path_buf()),
            workspace_root,
            1,
            &mut resolved,
            &mut diagnostics,
        )?;
    }

    resolved.sort_by(|a, b| {
        scope_precedence(a.scope)
            .cmp(&scope_precedence(b.scope))
            .then_with(|| a.package.manifest.name.cmp(&b.package.manifest.name))
            .then_with(|| a.declaration.source.cmp(&b.declaration.source))
    });

    Ok(InstalledPackageResolution {
        packages: resolved,
        diagnostics,
    })
}

pub fn manifest_sha256(path: &Path) -> Result<String, PackageResolverError> {
    let bytes = std::fs::read(path)
        .map_err(|e| PackageResolverError::Failed(format!("read {}: {e}", path.display())))?;
    use sha2::Digest as _;
    Ok(format!("{:x}", sha2::Sha256::digest(&bytes)))
}

pub fn local_lock_entry(
    source: String,
    package_root: &Path,
) -> Result<PackageLockEntry, PackageResolverError> {
    let canonical = package_root.canonicalize().map_err(|e| {
        PackageResolverError::Failed(format!("canonicalize {}: {e}", package_root.display()))
    })?;
    Ok(PackageLockEntry {
        identity_kind: "local".to_string(),
        identity_value: canonical.display().to_string(),
        source,
        manifest_sha256: manifest_sha256(&canonical.join("package.toml"))?,
        package_root: canonical,
        cache_path: None,
        git_commit: None,
    })
}

pub fn git_lock_entry(
    source: String,
    url: String,
    package_root: &Path,
    cache_path: &Path,
    git_commit: String,
) -> Result<PackageLockEntry, PackageResolverError> {
    let canonical_root = package_root.canonicalize().map_err(|e| {
        PackageResolverError::Failed(format!("canonicalize {}: {e}", package_root.display()))
    })?;
    let canonical_cache = cache_path.canonicalize().map_err(|e| {
        PackageResolverError::Failed(format!("canonicalize {}: {e}", cache_path.display()))
    })?;
    Ok(PackageLockEntry {
        identity_kind: "git".to_string(),
        identity_value: url,
        source,
        manifest_sha256: manifest_sha256(&canonical_root.join("package.toml"))?,
        package_root: canonical_root,
        cache_path: Some(canonical_cache),
        git_commit: Some(git_commit),
    })
}

fn resolve_scope(
    scope: InstalledPackageScope,
    store: &PackageStore,
    base_dir: &Path,
    layer_precedence: u32,
    resolved: &mut Vec<ResolvedInstalledPackage>,
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> Result<(), PackageResolverError> {
    let declarations = store.read_declarations().map_err(|e| {
        PackageResolverError::Failed(format!("read {scope:?} package declarations: {e}"))
    })?;
    let locks = store
        .read_lock()
        .map_err(|e| PackageResolverError::Failed(format!("read {scope:?} package lock: {e}")))?;

    for declaration in declarations {
        if let Some(package) = resolve_declaration(
            scope,
            base_dir,
            layer_precedence,
            &declaration,
            &locks,
            diagnostics,
        ) {
            resolved.push(package);
        }
    }

    Ok(())
}

fn resolve_declaration(
    scope: InstalledPackageScope,
    base_dir: &Path,
    layer_precedence: u32,
    declaration: &PackageDeclaration,
    locks: &[PackageLockEntry],
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> Option<ResolvedInstalledPackage> {
    let source = match PackageSource::parse(&declaration.source) {
        Ok(source) => source,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "invalid_source",
                format!("invalid package source: {e}"),
            ));
            return None;
        }
    };

    let path = match source {
        PackageSource::Local { path } => path,
        PackageSource::Git { url, .. } => {
            return resolve_git_declaration(
                scope,
                layer_precedence,
                declaration,
                &url,
                locks,
                diagnostics,
            );
        }
    };

    let source_root = resolve_local_source_path(base_dir, &declaration.source, path);
    if !source_root.is_dir() {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "source_missing",
            format!("package source does not exist: {}", source_root.display()),
        ));
        return None;
    }

    let manifest_path = source_root.join("package.toml");
    if !manifest_path.is_file() {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "manifest_missing",
            format!(
                "package source has no package.toml: {}",
                source_root.display()
            ),
        ));
        return None;
    }

    let canonical_root = match source_root.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "source_missing",
                format!("canonicalize {}: {e}", source_root.display()),
            ));
            return None;
        }
    };

    let actual_hash = match manifest_sha256(&canonical_root.join("package.toml")) {
        Ok(hash) => hash,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "manifest_hash_failed",
                e.to_string(),
            ));
            return None;
        }
    };

    let Some(lock) = find_local_lock(locks, &declaration.source, &canonical_root) else {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "lock_missing",
            "package lock entry is missing; package is disabled at runtime".to_string(),
        ));
        return None;
    };

    if lock.manifest_sha256 != actual_hash {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "lock_drifted",
            format!(
                "package manifest hash does not match the lock file (expected {}, actual {}); \
                 package is disabled at runtime",
                lock.manifest_sha256, actual_hash
            ),
        ));
        return None;
    }

    let package = match discover_package_root(&canonical_root, layer_precedence) {
        Ok(package) => package,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "discovery_failed",
                format!("{e}; package is disabled at runtime"),
            ));
            return None;
        }
    };
    validate_opi_version(scope, declaration, &package, diagnostics);
    if !validate_adapter_command(scope, declaration, &package, diagnostics) {
        return None;
    }
    if !validate_package_composition(scope, declaration, &package, diagnostics) {
        return None;
    }

    Some(ResolvedInstalledPackage {
        scope,
        declaration: declaration.clone(),
        identity: PackageIdentity {
            kind: "local".to_string(),
            value: canonical_root.display().to_string(),
        },
        lock: Some(lock.clone()),
        package,
    })
}

fn resolve_git_declaration(
    scope: InstalledPackageScope,
    layer_precedence: u32,
    declaration: &PackageDeclaration,
    url: &str,
    locks: &[PackageLockEntry],
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> Option<ResolvedInstalledPackage> {
    let Some(lock) = find_git_lock(locks, &declaration.source, url) else {
        if let Some(mismatched) = find_git_identity_lock(locks, url) {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "lock_source_mismatch",
                format!(
                    "package lock source '{}' does not match declaration source '{}'",
                    mismatched.source, declaration.source
                ),
            ));
            return None;
        }
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "lock_missing",
            "package lock entry is missing; package is disabled at runtime".to_string(),
        ));
        return None;
    };

    if !lock.package_root.is_dir() {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "source_missing",
            format!(
                "package source does not exist: {}",
                lock.package_root.display()
            ),
        ));
        return None;
    }

    let manifest_path = lock.package_root.join("package.toml");
    if !manifest_path.is_file() {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "manifest_missing",
            format!(
                "package source has no package.toml: {}",
                lock.package_root.display()
            ),
        ));
        return None;
    }

    let actual_hash = match manifest_sha256(&manifest_path) {
        Ok(hash) => hash,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "manifest_hash_failed",
                e.to_string(),
            ));
            return None;
        }
    };

    if lock.manifest_sha256 != actual_hash {
        diagnostics.push(diagnostic(
            scope,
            &declaration.source,
            "lock_drifted",
            format!(
                "package manifest hash does not match the lock file (expected {}, actual {}); \
                 package is disabled at runtime",
                lock.manifest_sha256, actual_hash
            ),
        ));
        return None;
    }

    let package = match discover_package_root(&lock.package_root, layer_precedence) {
        Ok(package) => package,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "discovery_failed",
                format!("{e}; package is disabled at runtime"),
            ));
            return None;
        }
    };
    validate_opi_version(scope, declaration, &package, diagnostics);
    if !validate_adapter_command(scope, declaration, &package, diagnostics) {
        return None;
    }
    if !validate_package_composition(scope, declaration, &package, diagnostics) {
        return None;
    }

    Some(ResolvedInstalledPackage {
        scope,
        declaration: declaration.clone(),
        identity: PackageIdentity {
            kind: "git".to_string(),
            value: url.to_string(),
        },
        lock: Some(lock.clone()),
        package,
    })
}

pub fn resolve_local_source_path(
    base_dir: &Path,
    raw_source: &str,
    parsed_path: PathBuf,
) -> PathBuf {
    let path = if cfg!(windows) {
        parsed_path
    } else {
        PathBuf::from(raw_source.replace('\\', "/"))
    };

    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn find_local_lock<'a>(
    locks: &'a [PackageLockEntry],
    source: &str,
    canonical_root: &Path,
) -> Option<&'a PackageLockEntry> {
    let identity_value = canonical_root.display().to_string();
    locks.iter().find(|lock| {
        lock.identity_kind == "local"
            && (lock.source == source
                || lock.identity_value == identity_value
                || lock.package_root == canonical_root)
    })
}

fn validate_adapter_command(
    scope: InstalledPackageScope,
    declaration: &PackageDeclaration,
    package: &PackageResource,
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> bool {
    let Some(adapter) = package.manifest.adapter.as_ref() else {
        return true;
    };
    match resolve_adapter_command_checked(adapter, &package.path) {
        Ok(_) => true,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "adapter_command_invalid",
                e.to_string(),
            ));
            false
        }
    }
}

fn validate_opi_version(
    scope: InstalledPackageScope,
    declaration: &PackageDeclaration,
    package: &PackageResource,
    diagnostics: &mut Vec<PackageDiagnostic>,
) {
    let Some(constraint) = package.manifest.opi_version.as_deref() else {
        return;
    };
    if let Some(diagnostic) = OpiVersionDiagnostic::check(constraint, env!("CARGO_PKG_VERSION")) {
        diagnostics.push(diagnostic_with_severity(
            scope,
            &declaration.source,
            PackageDiagnosticSeverity::Warning,
            "opi_version_incompatible",
            diagnostic.message,
        ));
    }
}

fn find_git_lock<'a>(
    locks: &'a [PackageLockEntry],
    source: &str,
    url: &str,
) -> Option<&'a PackageLockEntry> {
    locks.iter().find(|lock| {
        lock.identity_kind == "git" && lock.source == source && lock.identity_value == url
    })
}

fn find_git_identity_lock<'a>(
    locks: &'a [PackageLockEntry],
    url: &str,
) -> Option<&'a PackageLockEntry> {
    locks
        .iter()
        .find(|lock| lock.identity_kind == "git" && lock.identity_value == url)
}

fn validate_package_composition(
    scope: InstalledPackageScope,
    declaration: &PackageDeclaration,
    package: &PackageResource,
    diagnostics: &mut Vec<PackageDiagnostic>,
) -> bool {
    match package.compose() {
        Ok(_) => true,
        Err(e) => {
            diagnostics.push(diagnostic(
                scope,
                &declaration.source,
                "composition_failed",
                e.to_string(),
            ));
            false
        }
    }
}

fn diagnostic(
    scope: InstalledPackageScope,
    source: &str,
    code: &str,
    message: String,
) -> PackageDiagnostic {
    diagnostic_with_severity(
        scope,
        source,
        PackageDiagnosticSeverity::Error,
        code,
        message,
    )
}

fn diagnostic_with_severity(
    scope: InstalledPackageScope,
    source: &str,
    severity: PackageDiagnosticSeverity,
    code: &str,
    message: String,
) -> PackageDiagnostic {
    PackageDiagnostic {
        scope,
        source: source.to_string(),
        severity,
        code: code.to_string(),
        message,
    }
}

fn scope_precedence(scope: InstalledPackageScope) -> u8 {
    match scope {
        InstalledPackageScope::Global => 0,
        InstalledPackageScope::Project => 1,
    }
}
