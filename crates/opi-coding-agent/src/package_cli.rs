//! Package CLI command execution.
//!
//! Handles `opi package add/remove/list/doctor` subcommands. Runs before
//! provider construction so no API keys are needed for local package commands.
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::PackageCommand;
use crate::execution::{ExecutionFailure, PackageSource as ContributionScope};
use crate::package_activation::{self, ActivationRecord, StdinTrustConfirmer};
use crate::package_discovery::{PackageManifest, resolve_adapter_command_checked};
use crate::package_resolver::{
    InstalledPackageResolution, InstalledPackageScope, PackageDiagnostic,
    PackageDiagnosticSeverity, ResolvedInstalledPackage, git_lock_entry, local_lock_entry,
    resolve_installed_packages, resolve_local_source_path,
};
use crate::package_store::{
    PackageDeclaration, PackageLockEntry, PackageSource, PackageStore, PackageStoreError,
    PackageStoreScope,
};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitInstallFaultPoint {
    StageCacheReplacement,
    CanonicalizeLiveCache,
}

#[cfg(test)]
thread_local! {
    static NEXT_GIT_INSTALL_FAULT: std::cell::Cell<Option<GitInstallFaultPoint>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(test)]
fn fail_next_git_install_at(point: GitInstallFaultPoint) {
    NEXT_GIT_INSTALL_FAULT.with(|fault| fault.set(Some(point)));
}

#[cfg(test)]
fn check_git_install_fault(point: GitInstallFaultPoint) -> Result<(), PackageStoreError> {
    if NEXT_GIT_INSTALL_FAULT.with(|fault| {
        if fault.get() == Some(point) {
            fault.set(None);
            true
        } else {
            false
        }
    }) {
        return Err(PackageStoreError::Io(std::io::Error::other(format!(
            "injected git install failure at {point:?}"
        ))));
    }
    Ok(())
}

/// Execute a package CLI command and return an exit code.
///
/// `workspace_root` is typically `std::env::current_dir()`.
/// `user_config_dir` is the platform-specific user config directory
/// (see [`crate::config::user_config_dir`]).
pub fn handle_package_command(
    command: &PackageCommand,
    workspace_root: PathBuf,
    user_config_dir: PathBuf,
) -> i32 {
    let scope = resolve_scope(command, workspace_root.clone(), user_config_dir.clone());
    let store = PackageStore::new(scope.clone());
    match run_command(command, &store, &scope, &workspace_root, &user_config_dir) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("opi package: {e}");
            2
        }
    }
}

/// Determine the write scope from the command's `local` flag.
///
/// `list` and `doctor` use the project scope as a placeholder here; their
/// handlers read both global and project package stores.
fn resolve_scope(
    command: &PackageCommand,
    workspace_root: PathBuf,
    user_config_dir: PathBuf,
) -> PackageStoreScope {
    match command {
        PackageCommand::Add { local, .. } | PackageCommand::Remove { local, .. } if *local => {
            PackageStoreScope::Project { workspace_root }
        }
        PackageCommand::Add { .. } | PackageCommand::Remove { .. } => {
            PackageStoreScope::Global { user_config_dir }
        }
        // Enable/Disable operate on the global Package Trust store (execution
        // packages are global-only). List/Doctor read both scopes themselves.
        PackageCommand::Enable { .. }
        | PackageCommand::Disable { .. }
        | PackageCommand::List { .. }
        | PackageCommand::Doctor { .. } => PackageStoreScope::Project { workspace_root },
    }
}

fn run_command(
    command: &PackageCommand,
    store: &PackageStore,
    scope: &PackageStoreScope,
    workspace_root: &Path,
    user_config_dir: &Path,
) -> Result<(), PackageStoreError> {
    match command {
        PackageCommand::Add { source, .. } => cmd_add(store, scope, user_config_dir, source),
        PackageCommand::Remove { name_or_source, .. } => {
            cmd_remove(store, scope, user_config_dir, name_or_source)
        }
        PackageCommand::List { json } => cmd_list(workspace_root, user_config_dir, *json),
        PackageCommand::Doctor { json } => cmd_doctor(workspace_root, user_config_dir, *json),
        PackageCommand::Enable { name } => cmd_enable(user_config_dir, name),
        PackageCommand::Disable { name } => cmd_disable(user_config_dir, name),
    }
}

fn cmd_add(
    store: &PackageStore,
    scope: &PackageStoreScope,
    user_config_dir: &Path,
    source: &str,
) -> Result<(), PackageStoreError> {
    match PackageSource::parse(source)? {
        PackageSource::Local { path } => {
            install_local_package(store, scope, user_config_dir, source, path)
        }
        PackageSource::Git { url, refspec } => {
            install_git_package(store, scope, user_config_dir, source, url, refspec)
        }
    }
}

fn install_local_package(
    store: &PackageStore,
    scope: &PackageStoreScope,
    user_config_dir: &Path,
    source: &str,
    path: PathBuf,
) -> Result<(), PackageStoreError> {
    let metadata = read_package_metadata_snapshot(store, scope)?;
    let contribution_scope = contribution_scope_for(scope);
    let trust_snapshot = capture_trust_snapshot(user_config_dir, contribution_scope)?;
    let source_root = resolve_local_source_path(scope_base(scope), source, path);
    if !source_root.is_dir() {
        return Err(PackageStoreError::Package(format!(
            "package root not found: {}",
            source_root.display()
        )));
    }

    let canonical_root = source_root.canonicalize()?;
    let manifest_path = canonical_root.join("package.toml");
    if !manifest_path.is_file() {
        return Err(PackageStoreError::Package(format!(
            "package.toml not found in package root: {}",
            canonical_root.display()
        )));
    }

    let (manifest, raw_bytes) = read_manifest_and_bytes(&manifest_path)?;
    let mut lock_entry = local_lock_entry(source.to_string(), &canonical_root)
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;

    // Phase 16.5: validate executable contributions (first production caller of
    // validate_executable_contributions). Project-local packages with
    // contributions are rejected here; global packages persist the lock material
    // and an untrusted+disabled activation record.
    let adapter_ids = apply_contributions(
        &mut lock_entry,
        &manifest,
        &raw_bytes,
        &canonical_root,
        contribution_scope,
    )?;

    let previous_source = previous_lock_source(&metadata.locks, &lock_entry);
    let preserve_trust = locked_contributions_unchanged(&metadata.locks, &lock_entry);
    let declarations =
        declarations_with_package(metadata.declarations.clone(), scope, source, &lock_entry);
    let locks = locks_with_package(metadata.locks.clone(), &lock_entry);
    let activation_update = prepare_activation_update(
        user_config_dir,
        source,
        previous_source,
        preserve_trust,
        contribution_scope,
    )?;
    if let Err(error) = publish_package_metadata_and_activation(
        store,
        user_config_dir,
        source,
        previous_source,
        &manifest.name,
        &adapter_ids,
        activation_update,
        contribution_scope,
        &declarations,
        &locks,
    ) {
        let metadata_restore = metadata.restore();
        let trust_restore = restore_trust_snapshot(trust_snapshot.as_ref());
        return Err(package_update_error(
            error,
            metadata_restore,
            trust_restore,
            Ok(()),
        ));
    }
    println!(
        "Installed {} {} from {} ({})",
        manifest.name,
        display_version(manifest.version.as_deref()),
        source,
        scope_label(scope)
    );
    Ok(())
}

fn install_git_package(
    store: &PackageStore,
    scope: &PackageStoreScope,
    user_config_dir: &Path,
    source: &str,
    url: String,
    refspec: Option<String>,
) -> Result<(), PackageStoreError> {
    let cache_dir = store.cache_dir().join(sha256_hex(&format!("git:{url}")));
    let staging_dir = store.git_clone_to_staging(&url, refspec.as_deref(), &cache_dir)?;
    let metadata = read_package_metadata_snapshot(store, scope)?;
    let contribution_scope = contribution_scope_for(scope);
    let trust_snapshot = capture_trust_snapshot(user_config_dir, contribution_scope)?;

    let validated = (|| {
        let manifest_path = staging_dir.join("package.toml");
        if !manifest_path.is_file() {
            return Err(PackageStoreError::Package(format!(
                "package.toml not found in package root: {}",
                staging_dir.display()
            )));
        }
        let (manifest, raw_bytes) = read_manifest_and_bytes(&manifest_path)?;
        let git_commit = store.git_rev_parse_head(&staging_dir)?;
        Ok((manifest, raw_bytes, git_commit))
    })();

    let (manifest, raw_bytes, git_commit) = match validated {
        Ok(validated) => validated,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }
    };

    let mut lock_entry = match git_lock_entry(
        source.to_string(),
        url,
        &staging_dir,
        &staging_dir,
        git_commit,
    ) {
        Ok(entry) => entry,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(PackageStoreError::Package(e.to_string()));
        }
    };

    // Validate every contribution in the isolated clone before changing any
    // live package state.
    let adapter_ids = match apply_contributions(
        &mut lock_entry,
        &manifest,
        &raw_bytes,
        &staging_dir,
        contribution_scope,
    ) {
        Ok(ids) => ids,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }
    };

    let previous_source = previous_lock_source(&metadata.locks, &lock_entry);
    let preserve_trust = locked_contributions_unchanged(&metadata.locks, &lock_entry);
    let activation_update = match prepare_activation_update(
        user_config_dir,
        source,
        previous_source,
        preserve_trust,
        contribution_scope,
    ) {
        Ok(update) => update,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(error);
        }
    };

    // Package Trust is now durably disabled. Only after that fail-closed gate
    // may the live cache path expose the validated replacement bytes.
    #[cfg(test)]
    let replacement_result = check_git_install_fault(GitInstallFaultPoint::StageCacheReplacement)
        .and_then(|()| store.stage_cache_replacement(&cache_dir, &staging_dir));
    #[cfg(not(test))]
    let replacement_result = store.stage_cache_replacement(&cache_dir, &staging_dir);
    let replacement = match replacement_result {
        Ok(replacement) => replacement,
        Err(error) => {
            let staging_cleanup = match std::fs::remove_dir_all(&staging_dir) {
                Ok(()) => Ok(()),
                Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(cleanup) => Err(PackageStoreError::Io(cleanup)),
            };
            return Err(package_update_error(error, Ok(()), Ok(()), staging_cleanup));
        }
    };
    #[cfg(test)]
    let canonical_cache_result =
        check_git_install_fault(GitInstallFaultPoint::CanonicalizeLiveCache)
            .and_then(|()| cache_dir.canonicalize().map_err(PackageStoreError::Io));
    #[cfg(not(test))]
    let canonical_cache_result = cache_dir.canonicalize().map_err(PackageStoreError::Io);
    let canonical_cache = match canonical_cache_result {
        Ok(path) => path,
        Err(error) => {
            return Err(package_update_error(
                error,
                Ok(()),
                Ok(()),
                replacement.rollback(),
            ));
        }
    };
    lock_entry.package_root = canonical_cache.clone();
    lock_entry.cache_path = Some(canonical_cache);
    let declarations =
        declarations_with_package(metadata.declarations.clone(), scope, source, &lock_entry);
    let locks = locks_with_package(metadata.locks.clone(), &lock_entry);
    if let Err(e) = publish_package_metadata_and_activation(
        store,
        user_config_dir,
        source,
        previous_source,
        &manifest.name,
        &adapter_ids,
        activation_update,
        contribution_scope,
        &declarations,
        &locks,
    ) {
        let metadata_restore = metadata.restore();
        let trust_restore = restore_trust_snapshot(trust_snapshot.as_ref());
        let cache_restore = replacement.rollback();
        return Err(package_update_error(
            e,
            metadata_restore,
            trust_restore,
            cache_restore,
        ));
    }

    replacement.commit();
    println!(
        "Installed {} {} from {} ({})",
        manifest.name,
        display_version(manifest.version.as_deref()),
        source,
        scope_label(scope)
    );
    Ok(())
}

fn cmd_remove(
    store: &PackageStore,
    scope: &PackageStoreScope,
    user_config_dir: &Path,
    name_or_source: &str,
) -> Result<(), PackageStoreError> {
    let metadata = read_package_metadata_snapshot(store, scope)?;
    let mut declarations = metadata.declarations.clone();
    let removed = if let Some(index) = declarations
        .iter()
        .position(|declaration| declaration.source == name_or_source)
    {
        Some(declarations.remove(index))
    } else {
        let matches =
            declarations_matching_manifest_name(store, scope, &declarations, name_or_source)?;
        match matches.as_slice() {
            [] => {
                eprintln!("opi package: no declaration matching '{name_or_source}'");
                None
            }
            [matched] => Some(declarations.remove(matched.index)),
            _ => {
                return Err(PackageStoreError::Package(format!(
                    "ambiguous package '{name_or_source}'; matches: {}",
                    matches
                        .iter()
                        .map(|m| {
                            format!("{} source={} name={}", scope_label(scope), m.source, m.name)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
    };

    let Some(removed) = removed else {
        return Ok(());
    };
    let contribution_scope = contribution_scope_for(scope);
    let trust_snapshot = capture_trust_snapshot(user_config_dir, contribution_scope)?;
    let update = (|| {
        store.write_declarations(&declarations)?;
        remove_locks_for_declaration(store, scope, &removed)?;
        if contribution_scope == ContributionScope::Global {
            // Best-effort: removing a package with no trust record is a no-op.
            package_activation::PackageActivationStore::global(user_config_dir.to_path_buf())
                .remove(&removed.source)
                .map_err(|error| PackageStoreError::Package(error.to_string()))?;
        }
        Ok(())
    })();

    if let Err(error) = update {
        return Err(package_update_error(
            error,
            metadata.restore(),
            restore_trust_snapshot(trust_snapshot.as_ref()),
            Ok(()),
        ));
    }
    Ok(())
}

/// Grant Package Trust and enable a contribution package (interactive).
fn cmd_enable(user_config_dir: &Path, name: &str) -> Result<(), PackageStoreError> {
    let store = package_activation::PackageActivationStore::global(user_config_dir.to_path_buf());
    let mut confirmer = StdinTrustConfirmer;
    store
        .enable(
            name,
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;
    println!("Enabled {name}.");
    Ok(())
}

/// Disable a contribution package, preserving its Package Trust record.
fn cmd_disable(user_config_dir: &Path, name: &str) -> Result<(), PackageStoreError> {
    let store = package_activation::PackageActivationStore::global(user_config_dir.to_path_buf());
    store
        .disable(name)
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;
    println!("Disabled {name}.");
    Ok(())
}

fn cmd_list(
    workspace_root: &Path,
    user_config_dir: &Path,
    json: bool,
) -> Result<(), PackageStoreError> {
    let resolution = resolve_installed_packages(workspace_root, user_config_dir)
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;
    let records = read_activation_records(user_config_dir);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if json {
        for package in &resolution.packages {
            writeln!(out, "{}", list_package_json(package, &records, &[]))
                .map_err(PackageStoreError::Io)?;
        }
        for diagnostic in &resolution.diagnostics {
            writeln!(out, "{}", list_diagnostic_json(diagnostic)).map_err(PackageStoreError::Io)?;
        }
    } else if resolution.packages.is_empty() && resolution.diagnostics.is_empty() {
        writeln!(out, "No packages installed.").map_err(PackageStoreError::Io)?;
    } else {
        writeln!(out, "scope\tname\tversion\tsource\tstatus").map_err(PackageStoreError::Io)?;
        for package in &resolution.packages {
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                installed_scope_label(package.scope),
                package.package.manifest.name,
                display_version(package.package.manifest.version.as_deref()),
                package.declaration.source,
                lifecycle_status_tag(package, &records),
            )
            .map_err(PackageStoreError::Io)?;
        }
        for diagnostic in &resolution.diagnostics {
            writeln!(
                out,
                "{}\t-\t-\t{}\t{}",
                installed_scope_label(diagnostic.scope),
                diagnostic.source,
                severity_label(&diagnostic.severity)
            )
            .map_err(PackageStoreError::Io)?;
        }
    }
    Ok(())
}

fn cmd_doctor(
    workspace_root: &Path,
    user_config_dir: &Path,
    json: bool,
) -> Result<(), PackageStoreError> {
    let resolution = resolve_installed_packages(workspace_root, user_config_dir)
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;
    let records = read_activation_records(user_config_dir);

    // Drift detection for execution packages: recompute the executable SHA-256
    // (a read; no spawn) and flag any mismatch. This does not start package code.
    let mut execution_failures: Vec<(String, Vec<String>, ExecutionFailure)> = Vec::new();
    for package in &resolution.packages {
        let drifted = executable_drifted_adapters(package);
        let contributions = package
            .lock
            .as_ref()
            .map(|lock| lock.contributions.as_slice())
            .unwrap_or(&[]);
        if contributions.is_empty() {
            continue;
        }
        let record = records
            .iter()
            .find(|record| record.source == package.declaration.source);
        let trusted = record.is_some_and(|record| record.trusted);
        let enabled = record.is_some_and(|record| record.enabled);
        if let Some(failure) = execution_lifecycle_failure(
            &package.package.manifest.name,
            trusted,
            enabled,
            !drifted.is_empty(),
        ) {
            execution_failures.push((package.package.manifest.name.clone(), drifted, failure));
        }
    }
    let has_errors = resolution
        .diagnostics
        .iter()
        .any(|d| d.severity == PackageDiagnosticSeverity::Error)
        || !execution_failures.is_empty();

    if json {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        writeln!(
            out,
            "{}",
            serde_json::Value::Array(doctor_rows(&resolution, &records))
        )
        .map_err(PackageStoreError::Io)?;
    } else if resolution.diagnostics.is_empty() && execution_failures.is_empty() {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        if resolution.packages.is_empty() {
            writeln!(out, "No packages installed.").map_err(PackageStoreError::Io)?;
        } else {
            writeln!(out, "All {} package(s) OK.", resolution.packages.len())
                .map_err(PackageStoreError::Io)?;
        }
        // Report execution-package lifecycle (trusted/enabled) in text mode too.
        write_execution_lifecycle_lines(&mut out, &resolution, &records)
            .map_err(PackageStoreError::Io)?;
    } else {
        for diagnostic in &resolution.diagnostics {
            eprintln!(
                "{}: {} ({})",
                diagnostic.source, diagnostic.code, diagnostic.message
            );
        }
        for (_, _, failure) in &execution_failures {
            eprintln!("{failure}");
            eprintln!("remediation: {}", failure.remediation());
        }
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        write_execution_lifecycle_lines(&mut out, &resolution, &records)
            .map_err(PackageStoreError::Io)?;
    }

    if has_errors {
        return Err(PackageStoreError::Package(format!(
            "{} diagnostic(s) found",
            resolution.diagnostics.len() + execution_failures.len()
        )));
    }

    Ok(())
}

fn declarations_with_package(
    mut decls: Vec<PackageDeclaration>,
    scope: &PackageStoreScope,
    source: &str,
    lock_entry: &PackageLockEntry,
) -> Vec<PackageDeclaration> {
    let mut changed = false;
    for decl in &mut decls {
        if decl.source == source {
            return decls;
        }
        if declaration_identity(scope, decl).is_some_and(|(kind, value)| {
            kind == lock_entry.identity_kind && value == lock_entry.identity_value
        }) {
            decl.source = source.to_string();
            changed = true;
            break;
        }
    }

    if !changed {
        decls.push(PackageDeclaration {
            source: source.to_string(),
            filters: Default::default(),
        });
    }

    decls
}

fn locks_with_package(
    mut locks: Vec<PackageLockEntry>,
    lock_entry: &PackageLockEntry,
) -> Vec<PackageLockEntry> {
    locks.retain(|lock| !lock_matches_entry(lock, lock_entry));
    locks.push(lock_entry.clone());
    locks
}

fn write_package_metadata(
    store: &PackageStore,
    declarations: &[PackageDeclaration],
    locks: &[PackageLockEntry],
) -> Result<(), PackageStoreError> {
    store.write_declarations(declarations)?;
    store.write_lock(locks)
}

#[derive(Debug, Default)]
struct PreparedActivationUpdate {
    had_existing: bool,
    preserved_state: Option<(bool, bool)>,
}

fn prepare_activation_update(
    user_config_dir: &Path,
    source: &str,
    previous_source: Option<&str>,
    preserve_trust: bool,
    contribution_scope: ContributionScope,
) -> Result<PreparedActivationUpdate, PackageStoreError> {
    if contribution_scope == ContributionScope::ProjectLocal {
        return Ok(PreparedActivationUpdate::default());
    }

    let activation =
        package_activation::PackageActivationStore::global(user_config_dir.to_path_buf());
    let mut records = activation.read_records()?;
    let existing_index = records.iter().position(|record| {
        record.source == source || previous_source == Some(record.source.as_str())
    });
    let preserved_state = existing_index
        .filter(|_| preserve_trust)
        .map(|index| (records[index].trusted, records[index].enabled));

    // This write is the fail-closed transaction gate. Git callers execute it
    // before swapping the live cache directory, so a crash at every later
    // boundary leaves the old or new package untrusted and disabled.
    if let Some(index) = existing_index {
        records[index].trusted = false;
        records[index].enabled = false;
        activation.write_records(&records)?;
    }

    Ok(PreparedActivationUpdate {
        had_existing: existing_index.is_some(),
        preserved_state,
    })
}

#[allow(clippy::too_many_arguments)]
fn publish_package_metadata_and_activation(
    store: &PackageStore,
    user_config_dir: &Path,
    source: &str,
    previous_source: Option<&str>,
    package_name: &str,
    adapter_ids: &[String],
    activation_update: PreparedActivationUpdate,
    contribution_scope: ContributionScope,
    declarations: &[PackageDeclaration],
    locks: &[PackageLockEntry],
) -> Result<(), PackageStoreError> {
    let activation =
        package_activation::PackageActivationStore::global(user_config_dir.to_path_buf());

    write_package_metadata(store, declarations, locks)?;
    if contribution_scope == ContributionScope::Global
        && (!adapter_ids.is_empty() || activation_update.had_existing)
    {
        activation
            .install(package_name, source, previous_source, adapter_ids)
            .map_err(|e| PackageStoreError::Package(e.to_string()))?;
        if let Some((trusted, enabled)) = activation_update.preserved_state {
            let mut records = activation.read_records()?;
            let record = records
                .iter_mut()
                .find(|record| record.source == source)
                .ok_or_else(|| {
                    PackageStoreError::Package(
                        "activation record disappeared during package update".to_string(),
                    )
                })?;
            record.trusted = trusted;
            record.enabled = enabled;
            activation.write_records(&records)?;
        }
    }
    Ok(())
}

fn locked_contributions_unchanged(
    old_locks: &[PackageLockEntry],
    new_lock: &PackageLockEntry,
) -> bool {
    if new_lock.contributions.is_empty() {
        return false;
    }
    let Some(old_lock) = old_locks
        .iter()
        .find(|lock| lock_matches_entry(lock, new_lock))
    else {
        return false;
    };
    old_lock.contributions.len() == new_lock.contributions.len()
        && old_lock
            .contributions
            .iter()
            .all(|old| new_lock.contributions.iter().any(|new| new == old))
}

fn previous_lock_source<'a>(
    old_locks: &'a [PackageLockEntry],
    new_lock: &PackageLockEntry,
) -> Option<&'a str> {
    old_locks
        .iter()
        .find(|lock| lock_matches_entry(lock, new_lock))
        .map(|lock| lock.source.as_str())
}

fn remove_locks_for_declaration(
    store: &PackageStore,
    scope: &PackageStoreScope,
    declaration: &PackageDeclaration,
) -> Result<(), PackageStoreError> {
    let identity = declaration_identity(scope, declaration);
    let mut locks = store.read_lock()?;
    let before = locks.len();
    locks.retain(|lock| {
        if lock.source == declaration.source {
            return false;
        }
        if let Some((kind, value)) = &identity {
            return !(lock.identity_kind == *kind && lock.identity_value == *value);
        }
        true
    });
    if locks.len() != before {
        store.write_lock(&locks)?;
    }
    Ok(())
}

fn declarations_matching_manifest_name(
    store: &PackageStore,
    scope: &PackageStoreScope,
    declarations: &[PackageDeclaration],
    name: &str,
) -> Result<Vec<RemoveMatch>, PackageStoreError> {
    let locks = store.read_lock()?;
    let mut matches = Vec::new();
    for (index, declaration) in declarations.iter().enumerate() {
        if let Some(manifest_name) = declaration_manifest_name(scope, declaration, &locks)?
            && manifest_name == name
        {
            matches.push(RemoveMatch {
                index,
                source: declaration.source.clone(),
                name: manifest_name,
            });
        }
    }
    Ok(matches)
}

fn declaration_manifest_name(
    scope: &PackageStoreScope,
    declaration: &PackageDeclaration,
    locks: &[PackageLockEntry],
) -> Result<Option<String>, PackageStoreError> {
    let source = match PackageSource::parse(&declaration.source) {
        Ok(source) => source,
        Err(_) => return Ok(None),
    };
    let manifest_path = match source {
        PackageSource::Local { path } => {
            let source_root =
                resolve_local_source_path(scope_base(scope), &declaration.source, path);
            let Ok(canonical_root) = source_root.canonicalize() else {
                return Ok(None);
            };
            canonical_root.join("package.toml")
        }
        PackageSource::Git { url, .. } => {
            let Some(lock) = locks.iter().find(|lock| {
                lock.identity_kind == "git"
                    && (lock.source == declaration.source || lock.identity_value == url)
            }) else {
                return Ok(None);
            };
            lock.package_root.join("package.toml")
        }
    };

    if !manifest_path.is_file() {
        return Ok(None);
    }

    Ok(Some(read_package_manifest(&manifest_path)?.name))
}

fn declaration_identity(
    scope: &PackageStoreScope,
    declaration: &PackageDeclaration,
) -> Option<(String, String)> {
    let source = PackageSource::parse(&declaration.source).ok()?;
    match source {
        PackageSource::Local { path } => {
            let source_root =
                resolve_local_source_path(scope_base(scope), &declaration.source, path);
            let canonical_root = source_root.canonicalize().ok()?;
            Some(("local".to_string(), canonical_root.display().to_string()))
        }
        PackageSource::Git { url, .. } => Some(("git".to_string(), url)),
    }
}

fn lock_matches_entry(lock: &PackageLockEntry, entry: &PackageLockEntry) -> bool {
    lock.source == entry.source
        || (lock.identity_kind == entry.identity_kind
            && lock.identity_value == entry.identity_value)
}

fn read_package_manifest(path: &Path) -> Result<PackageManifest, PackageStoreError> {
    Ok(read_manifest_and_bytes(path)?.0)
}

/// Read a package manifest and its raw bytes (the bytes are threaded into the
/// contribution validator so the manifest hash is computed over the exact
/// parsed content, with no re-read TOCTOU).
fn read_manifest_and_bytes(path: &Path) -> Result<(PackageManifest, Vec<u8>), PackageStoreError> {
    let bytes = std::fs::read(path)?;
    let manifest = PackageManifest::from_toml(&String::from_utf8_lossy(&bytes), path)
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;
    Ok((manifest, bytes))
}

/// Map a write scope to the contribution validator's install-scope enum.
fn contribution_scope_for(scope: &PackageStoreScope) -> ContributionScope {
    match scope {
        PackageStoreScope::Global { .. } => ContributionScope::Global,
        PackageStoreScope::Project { .. } => ContributionScope::ProjectLocal,
    }
}

/// Validate a package's executable contributions and attach the resulting lock
/// material to `lock_entry`. Returns the contribution adapter ids (empty for
/// non-execution packages). A project-local package with contributions fails
/// here (`ProjectLocalExecutableContribution`), rejecting the install.
fn apply_contributions(
    lock_entry: &mut PackageLockEntry,
    manifest: &PackageManifest,
    raw_bytes: &[u8],
    package_root: &Path,
    scope: ContributionScope,
) -> Result<Vec<String>, PackageStoreError> {
    if manifest.adapter_contributions.is_empty() {
        return Ok(Vec::new());
    }
    let locks = package_activation::validate_for_install(manifest, raw_bytes, package_root, scope)
        .map_err(|e| PackageStoreError::Package(e.to_string()))?;
    let adapter_ids = locks.iter().map(|l| l.adapter_id.clone()).collect();
    lock_entry.contributions = locks;
    Ok(adapter_ids)
}

fn list_package_json(
    package: &ResolvedInstalledPackage,
    records: &[ActivationRecord],
    diagnostics: &[PackageDiagnostic],
) -> serde_json::Value {
    let adapter = package.package.manifest.adapter.as_ref();
    let mut row = serde_json::json!({
        "scope": installed_scope_label(package.scope),
        "name": package.package.manifest.name.as_str(),
        "version": package.package.manifest.version.as_deref(),
        "source": package.declaration.source.as_str(),
        "status": "ok",
        "package_root": package.package.path.display().to_string(),
        "adapter_command": adapter.map(|adapter| adapter.command.as_str()),
        "adapter_resolved_command": adapter
            .and_then(|adapter| resolve_adapter_command_checked(adapter, &package.package.path).ok())
            .map(|path| path.display().to_string()),
        "diagnostics": diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
    });
    // Phase 16.5: lifecycle + lock state for execution packages.
    let contributions: &[crate::execution::LockMaterial] = package
        .lock
        .as_ref()
        .map(|l| l.contributions.as_slice())
        .unwrap_or(&[]);
    if !contributions.is_empty() {
        let record = records
            .iter()
            .find(|r| r.source == package.declaration.source);
        if let Some(obj) = row.as_object_mut() {
            obj.insert(
                "trusted".into(),
                serde_json::json!(record.map(|r| r.trusted).unwrap_or(false)),
            );
            obj.insert(
                "enabled".into(),
                serde_json::json!(record.map(|r| r.enabled).unwrap_or(false)),
            );
            obj.insert(
                "contributions".into(),
                serde_json::Value::Array(
                    contributions
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "adapter_id": c.adapter_id,
                                "target": c.target,
                                "protocol": c.protocol,
                                "package_version": c.package_version,
                                "executable_rel_path": c.executable_rel_path,
                                "executable_sha256": c.executable_sha256,
                            })
                        })
                        .collect(),
                ),
            );
        }
    }
    row
}

fn list_diagnostic_json(diagnostic: &PackageDiagnostic) -> serde_json::Value {
    serde_json::json!({
        "scope": installed_scope_label(diagnostic.scope),
        "name": serde_json::Value::Null,
        "version": serde_json::Value::Null,
        "source": diagnostic.source.as_str(),
        "status": severity_label(&diagnostic.severity),
        "package_root": serde_json::Value::Null,
        "adapter_command": serde_json::Value::Null,
        "adapter_resolved_command": serde_json::Value::Null,
        "diagnostics": [diagnostic_json(diagnostic)],
    })
}

fn doctor_rows(
    resolution: &InstalledPackageResolution,
    records: &[ActivationRecord],
) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    for package in &resolution.packages {
        let drifted = executable_drifted_adapters(package);
        let has_drift = !drifted.is_empty();
        let status = if drifted.is_empty() { "ok" } else { "drifted" };
        let mut row = serde_json::json!({
            "scope": installed_scope_label(package.scope),
            "source": package.declaration.source.as_str(),
            "name": package.package.manifest.name.as_str(),
            "status": status,
            "diagnostics": [],
        });
        let contributions: &[crate::execution::LockMaterial] = package
            .lock
            .as_ref()
            .map(|l| l.contributions.as_slice())
            .unwrap_or(&[]);
        if !contributions.is_empty() {
            let record = records
                .iter()
                .find(|r| r.source == package.declaration.source);
            if let Some(obj) = row.as_object_mut() {
                obj.insert(
                    "trusted".into(),
                    serde_json::json!(record.map(|r| r.trusted).unwrap_or(false)),
                );
                obj.insert(
                    "enabled".into(),
                    serde_json::json!(record.map(|r| r.enabled).unwrap_or(false)),
                );
                obj.insert("drifted_adapters".into(), serde_json::json!(drifted));
                let trusted = record.is_some_and(|record| record.trusted);
                let enabled = record.is_some_and(|record| record.enabled);
                if let Some(failure) = execution_lifecycle_failure(
                    &package.package.manifest.name,
                    trusted,
                    enabled,
                    has_drift,
                ) {
                    let status = if has_drift {
                        "drifted"
                    } else {
                        lifecycle_failure_status(&failure)
                    };
                    obj.insert("status".into(), status.into());
                    obj.insert("code".into(), failure.code().into());
                    obj.insert("remediation".into(), failure.remediation().into());
                    obj.insert(
                        "diagnostics".into(),
                        serde_json::json!([execution_failure_json(&failure)]),
                    );
                }
            }
        }
        rows.push(row);
    }
    for diagnostic in &resolution.diagnostics {
        rows.push(serde_json::json!({
            "scope": installed_scope_label(diagnostic.scope),
            "source": diagnostic.source.as_str(),
            "name": serde_json::Value::Null,
            "status": severity_label(&diagnostic.severity),
            "diagnostics": [diagnostic_json(diagnostic)],
        }));
    }
    rows
}

/// Read the global Package Trust records (best-effort; empty on absence/error).
fn read_activation_records(user_config_dir: &Path) -> Vec<ActivationRecord> {
    package_activation::PackageActivationStore::global(user_config_dir.to_path_buf())
        .read_records()
        .unwrap_or_default()
}

/// A human-readable lifecycle status tag for the text table.
fn lifecycle_status_tag(
    package: &ResolvedInstalledPackage,
    records: &[ActivationRecord],
) -> String {
    let contributions: &[crate::execution::LockMaterial] = package
        .lock
        .as_ref()
        .map(|l| l.contributions.as_slice())
        .unwrap_or(&[]);
    if contributions.is_empty() {
        return "ok".to_string();
    }
    let drifted = executable_drifted_adapters(package);
    if !drifted.is_empty() {
        return format!("drifted[{}]", drifted.join(","));
    }
    let record = records
        .iter()
        .find(|r| r.source == package.declaration.source);
    let trusted = record.map(|r| r.trusted).unwrap_or(false);
    let enabled = record.map(|r| r.enabled).unwrap_or(false);
    match (trusted, enabled) {
        (true, true) => "ok (trusted, enabled)".to_string(),
        (true, false) => "ok (trusted, disabled)".to_string(),
        (false, _) => "ok (untrusted, disabled)".to_string(),
    }
}

/// Write one lifecycle status line per execution package (text mode).
fn write_execution_lifecycle_lines(
    out: &mut impl Write,
    resolution: &InstalledPackageResolution,
    records: &[ActivationRecord],
) -> std::io::Result<()> {
    for package in &resolution.packages {
        let contributions: &[crate::execution::LockMaterial] = package
            .lock
            .as_ref()
            .map(|l| l.contributions.as_slice())
            .unwrap_or(&[]);
        if contributions.is_empty() {
            continue;
        }
        writeln!(
            out,
            "{}: {}",
            package.package.manifest.name,
            lifecycle_status_tag(package, records)
        )?;
    }
    Ok(())
}

/// Adapter ids whose locked executable SHA-256 no longer matches the file
/// (executable deleted or altered). A read; never spawns package code.
pub(crate) fn executable_drifted_adapters(package: &ResolvedInstalledPackage) -> Vec<String> {
    use sha2::Digest as _;
    let mut drifted = Vec::new();
    let Some(lock) = &package.lock else {
        return drifted;
    };
    for c in &lock.contributions {
        let exe = package.package.path.join(&c.executable_rel_path);
        let matches = match read_regular_file_without_blocking(&exe) {
            Ok(bytes) => format!("{:x}", sha2::Sha256::digest(&bytes)) == c.executable_sha256,
            Err(()) => false,
        };
        if !matches {
            drifted.push(c.adapter_id.clone());
        }
    }
    drifted
}

fn diagnostic_json(diagnostic: &PackageDiagnostic) -> serde_json::Value {
    serde_json::json!({
        "severity": severity_label(&diagnostic.severity),
        "code": diagnostic.code.as_str(),
        "message": diagnostic.message.as_str(),
    })
}

fn scope_base(scope: &PackageStoreScope) -> &Path {
    match scope {
        PackageStoreScope::Project { workspace_root } => workspace_root,
        PackageStoreScope::Global { user_config_dir } => user_config_dir,
    }
}

fn scope_label(scope: &PackageStoreScope) -> &'static str {
    match scope {
        PackageStoreScope::Project { .. } => "project",
        PackageStoreScope::Global { .. } => "global",
    }
}

fn installed_scope_label(scope: InstalledPackageScope) -> &'static str {
    match scope {
        InstalledPackageScope::Project => "project",
        InstalledPackageScope::Global => "global",
    }
}

fn severity_label(severity: &PackageDiagnosticSeverity) -> &'static str {
    match severity {
        PackageDiagnosticSeverity::Info => "info",
        PackageDiagnosticSeverity::Warning => "warning",
        PackageDiagnosticSeverity::Error => "error",
    }
}

fn display_version(version: Option<&str>) -> &str {
    version.unwrap_or("-")
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest as _;
    format!("{:x}", sha2::Sha256::digest(input.as_bytes()))
}

struct RemoveMatch {
    index: usize,
    source: String,
    name: String,
}

struct PackageMetadataSnapshot {
    declarations: Vec<PackageDeclaration>,
    locks: Vec<PackageLockEntry>,
    declarations_file: PackageFileSnapshot,
    lock_file: PackageFileSnapshot,
}

pub(crate) fn execution_lifecycle_failure(
    name: &str,
    trusted: bool,
    enabled: bool,
    drifted: bool,
) -> Option<ExecutionFailure> {
    if drifted || !trusted {
        Some(ExecutionFailure::PackageUntrusted {
            name: name.to_string(),
        })
    } else if !enabled {
        Some(ExecutionFailure::ContributionDisabled {
            name: name.to_string(),
        })
    } else {
        None
    }
}

fn lifecycle_failure_status(failure: &ExecutionFailure) -> &'static str {
    match failure {
        ExecutionFailure::PackageUntrusted { .. } => "untrusted",
        ExecutionFailure::ContributionDisabled { .. } => "disabled",
        _ => "error",
    }
}

fn execution_failure_json(failure: &ExecutionFailure) -> serde_json::Value {
    serde_json::json!({
        "severity": "error",
        "code": failure.code(),
        "message": failure.to_string(),
        "remediation": failure.remediation(),
    })
}

fn read_regular_file_without_blocking(path: &Path) -> Result<Vec<u8>, ()> {
    use std::io::Read as _;

    let canonical = path.canonicalize().map_err(|_| ())?;
    if !std::fs::metadata(&canonical)
        .map_err(|_| ())?
        .file_type()
        .is_file()
    {
        return Err(());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let mut file = options.open(canonical).map_err(|_| ())?;
    if !file.metadata().map_err(|_| ())?.file_type().is_file() {
        return Err(());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|_| ())?;
    Ok(bytes)
}

struct PackageFileSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

impl PackageFileSnapshot {
    fn capture(path: PathBuf) -> Result<Self, PackageStoreError> {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(PackageStoreError::Io(error)),
        };
        Ok(Self { path, bytes })
    }

    fn restore(&self) -> Result<(), PackageStoreError> {
        match &self.bytes {
            Some(bytes) => {
                if let Some(parent) = self.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&self.path, bytes)?;
                Ok(())
            }
            None => match std::fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(PackageStoreError::Io(error)),
            },
        }
    }
}

fn capture_trust_snapshot(
    user_config_dir: &Path,
    scope: ContributionScope,
) -> Result<Option<PackageFileSnapshot>, PackageStoreError> {
    if scope == ContributionScope::ProjectLocal {
        return Ok(None);
    }
    PackageFileSnapshot::capture(
        package_activation::PackageActivationStore::global(user_config_dir.to_path_buf())
            .store()
            .trust_path(),
    )
    .map(Some)
}

fn restore_trust_snapshot(snapshot: Option<&PackageFileSnapshot>) -> Result<(), PackageStoreError> {
    match snapshot {
        Some(snapshot) => snapshot.restore(),
        None => Ok(()),
    }
}

impl PackageMetadataSnapshot {
    fn restore(&self) -> Result<(), PackageStoreError> {
        let declarations = self.declarations_file.restore();
        let lock = self.lock_file.restore();
        match (declarations, lock) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(declarations), Err(lock)) => Err(PackageStoreError::Package(format!(
                "declarations rollback failed: {declarations}; lock rollback failed: {lock}"
            ))),
        }
    }
}

fn read_package_metadata_snapshot(
    store: &PackageStore,
    scope: &PackageStoreScope,
) -> Result<PackageMetadataSnapshot, PackageStoreError> {
    let declarations_path = scope.config_path();
    let lock_path = scope.lock_path();
    Ok(PackageMetadataSnapshot {
        declarations: store.read_declarations()?,
        locks: store.read_lock()?,
        declarations_file: PackageFileSnapshot::capture(declarations_path)?,
        lock_file: PackageFileSnapshot::capture(lock_path)?,
    })
}

fn package_update_error(
    error: PackageStoreError,
    metadata_restore: Result<(), PackageStoreError>,
    trust_restore: Result<(), PackageStoreError>,
    cache_restore: Result<(), PackageStoreError>,
) -> PackageStoreError {
    if metadata_restore.is_ok() && trust_restore.is_ok() && cache_restore.is_ok() {
        return error;
    }

    let mut details = vec![format!("{error}")];
    if let Err(e) = metadata_restore {
        details.push(format!("metadata rollback failed: {e}"));
    }
    if let Err(e) = trust_restore {
        details.push(format!("trust rollback failed: {e}"));
    }
    if let Err(e) = cache_restore {
        details.push(format!("cache rollback failed: {e}"));
    }
    PackageStoreError::Package(format!(
        "package metadata update failed after cache replacement: {}",
        details.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_resolver::local_lock_entry;
    use std::collections::BTreeMap;

    fn file_state(path: &Path) -> (bool, Option<Vec<u8>>) {
        (path.exists(), std::fs::read(path).ok())
    }

    fn seed_local_package(store: &PackageStore, base: &Path, source: &str) {
        let root = base.join(source.trim_start_matches("./"));
        std::fs::create_dir_all(&root).expect("package root");
        std::fs::write(
            root.join("package.toml"),
            "name = \"adapter\"\ndescription = \"adapter\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        store
            .write_declarations(&[PackageDeclaration {
                source: source.into(),
                filters: Default::default(),
            }])
            .expect("declaration");
        store
            .write_lock(&[local_lock_entry(source.into(), &root).expect("lock")])
            .expect("write lock");
    }

    struct GitExecutionRepo {
        _tmp: tempfile::TempDir,
        bare_url: String,
        first_commit: String,
        second_commit: String,
    }

    fn git_in(cwd: &Path, args: &[&str]) -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("git command")
    }

    fn assert_git_ok(output: std::process::Output, action: &str) -> std::process::Output {
        assert!(
            output.status.success(),
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn write_git_execution_package(root: &Path, executable: &[u8]) {
        use sha2::{Digest as _, Sha256};

        std::fs::create_dir_all(root.join("bin")).expect("bin directory");
        let command = root.join("bin/adapter");
        std::fs::write(&command, executable).expect("adapter executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755))
                .expect("executable permissions");
        }
        let sha = format!("{:x}", Sha256::digest(executable));
        let target = package_activation::host_target_triple();
        std::fs::write(
            root.join("package.toml"),
            format!(
                "version = \"0.8.0\"\n\
                 opi_version = \">=0.7,<0.8\"\n\
                 name = \"git-execution\"\n\
                 description = \"git execution fixture\"\n\
                 [[contributions.adapters]]\n\
                 capability = \"command.execute\"\n\
                 id = \"git-execution\"\n\
                 transport = \"process-jsonl\"\n\
                 command = \"bin/adapter\"\n\
                 args = [\"backend\", \"--stdio\"]\n\
                 protocol = \"command-execution-jsonl-v1\"\n\
                 target = \"{target}\"\n\
                 sha256 = \"{sha}\"\n\
                 handshake_timeout_ms = 5000\n\
                 adapter_config = {{}}\n"
            ),
        )
        .expect("package manifest");
    }

    fn git_execution_repo_with_changed_executable() -> GitExecutionRepo {
        let tmp = tempfile::tempdir().expect("git fixture");
        let bare_dir = tmp.path().join("bare.git");
        let work_dir = tmp.path().join("work");
        std::fs::create_dir_all(&work_dir).expect("work directory");
        assert_git_ok(
            std::process::Command::new("git")
                .args(["init", "--bare"])
                .arg(&bare_dir)
                .output()
                .expect("git init --bare"),
            "git init --bare",
        );
        assert_git_ok(git_in(&work_dir, &["init"]), "git init");
        assert_git_ok(
            git_in(&work_dir, &["config", "core.autocrlf", "false"]),
            "disable autocrlf",
        );
        write_git_execution_package(&work_dir, b"adapter-v1");
        assert_git_ok(git_in(&work_dir, &["add", "."]), "git add first");
        assert_git_ok(
            git_in(&work_dir, &["commit", "-m", "first"]),
            "git commit first",
        );
        let first = assert_git_ok(git_in(&work_dir, &["rev-parse", "HEAD"]), "first sha");
        let first_commit = String::from_utf8_lossy(&first.stdout).trim().to_string();

        let bare_url = format!(
            "file:///{}",
            bare_dir.display().to_string().replace('\\', "/")
        );
        assert_git_ok(
            git_in(&work_dir, &["remote", "add", "origin", &bare_url]),
            "git remote add",
        );
        assert_git_ok(
            git_in(&work_dir, &["push", "origin", "HEAD:refs/heads/main"]),
            "git push first",
        );
        write_git_execution_package(&work_dir, b"adapter-v2");
        assert_git_ok(git_in(&work_dir, &["add", "."]), "git add second");
        assert_git_ok(
            git_in(&work_dir, &["commit", "-m", "second"]),
            "git commit second",
        );
        let second = assert_git_ok(git_in(&work_dir, &["rev-parse", "HEAD"]), "second sha");
        let second_commit = String::from_utf8_lossy(&second.stdout).trim().to_string();
        assert_git_ok(
            git_in(&work_dir, &["push", "origin", "HEAD:refs/heads/main"]),
            "git push second",
        );

        GitExecutionRepo {
            _tmp: tmp,
            bare_url,
            first_commit,
            second_commit,
        }
    }

    fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(path).expect("read cache tree") {
                let entry = entry.expect("cache entry");
                let path = entry.path();
                let file_type = entry.file_type().expect("cache entry type");
                if file_type.is_dir() {
                    visit(root, &path, files);
                } else if file_type.is_file() {
                    files.insert(
                        path.strip_prefix(root)
                            .expect("relative cache path")
                            .to_path_buf(),
                        std::fs::read(path).expect("cache file bytes"),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    fn directory_entries(path: &Path) -> Vec<PathBuf> {
        let mut entries = std::fs::read_dir(path)
            .expect("cache parent")
            .map(|entry| entry.expect("cache parent entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    fn assert_git_update_fault_rolls_back(point: GitInstallFaultPoint) {
        let user = tempfile::tempdir().expect("user config");
        let repo = git_execution_repo_with_changed_executable();
        let first_source = format!("git:{}@{}", repo.bare_url, repo.first_commit);
        let second_source = format!("git:{}@{}", repo.bare_url, repo.second_commit);
        let scope = PackageStoreScope::Global {
            user_config_dir: user.path().to_path_buf(),
        };
        let store = PackageStore::new(scope.clone());

        cmd_add(&store, &scope, user.path(), &first_source).expect("first install");
        let activation =
            package_activation::PackageActivationStore::global(user.path().to_path_buf());
        let mut records = activation.read_records().expect("initial trust record");
        records[0].trusted = true;
        records[0].enabled = true;
        activation.write_records(&records).expect("trusted record");

        let lock_path = scope.lock_path();
        let declarations_path = scope.config_path();
        let old_lock_bytes = std::fs::read(&lock_path).expect("old lock bytes");
        let old_declaration_bytes =
            std::fs::read(&declarations_path).expect("old declaration bytes");
        let old_lock = store.read_lock().expect("old lock").remove(0);
        let old_cache = old_lock.package_root.clone();
        let old_cache_bytes = snapshot_tree(&old_cache);
        let old_cache_entries = directory_entries(old_cache.parent().expect("cache parent"));

        fail_next_git_install_at(point);
        let error = cmd_add(&store, &scope, user.path(), &second_source)
            .expect_err("injected git update fault");
        assert!(error.to_string().contains("injected git install failure"));

        assert_eq!(std::fs::read(&lock_path).unwrap(), old_lock_bytes);
        assert_eq!(
            std::fs::read(&declarations_path).unwrap(),
            old_declaration_bytes
        );
        assert_eq!(store.read_lock().unwrap(), [old_lock]);
        assert_eq!(snapshot_tree(&old_cache), old_cache_bytes);
        assert_eq!(
            directory_entries(old_cache.parent().expect("cache parent")),
            old_cache_entries,
            "failed update must not leave a staged or backup cache published"
        );

        let records = activation.read_records().expect("trust after fault");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].source, first_source);
        assert!(
            !records[0].trusted,
            "committed invalidation must be durable"
        );
        assert!(!records[0].enabled, "committed disablement must be durable");
    }

    #[test]
    fn global_remove_rolls_back_exact_files_when_trust_write_fails() {
        let user = tempfile::tempdir().expect("user config");
        let scope = PackageStoreScope::Global {
            user_config_dir: user.path().to_path_buf(),
        };
        let store = PackageStore::new(scope.clone());
        seed_local_package(&store, user.path(), "./adapter");
        let activation =
            package_activation::PackageActivationStore::global(user.path().to_path_buf());
        activation
            .write_records(&[ActivationRecord {
                name: "adapter".into(),
                source: "./adapter".into(),
                trusted: true,
                enabled: true,
            }])
            .expect("trust record");

        let paths = [scope.config_path(), scope.lock_path(), store.trust_path()];
        let before = paths
            .iter()
            .map(|path| file_state(path))
            .collect::<Vec<_>>();
        package_activation::fail_next_record_write_for_test();

        let error = cmd_remove(&store, &scope, user.path(), "./adapter")
            .expect_err("injected trust write failure must fail removal");
        assert!(error.to_string().contains("injected one-shot"));
        let after = paths
            .iter()
            .map(|path| file_state(path))
            .collect::<Vec<_>>();
        assert_eq!(after, before, "all package files must be restored exactly");

        cmd_remove(&store, &scope, user.path(), "./adapter")
            .expect("one-shot failure must allow a successful retry");
        assert!(store.read_declarations().expect("declarations").is_empty());
        assert!(store.read_lock().expect("locks").is_empty());
        assert!(activation.read_records().expect("records").is_empty());
    }

    #[test]
    fn project_remove_does_not_mutate_global_trust() {
        let workspace = tempfile::tempdir().expect("workspace");
        let user = tempfile::tempdir().expect("user config");
        let scope = PackageStoreScope::Project {
            workspace_root: workspace.path().to_path_buf(),
        };
        let store = PackageStore::new(scope.clone());
        seed_local_package(&store, workspace.path(), "./adapter");
        let activation =
            package_activation::PackageActivationStore::global(user.path().to_path_buf());
        activation
            .write_records(&[ActivationRecord {
                name: "global-adapter".into(),
                source: "./adapter".into(),
                trusted: true,
                enabled: true,
            }])
            .expect("global trust record");
        let trust_path = activation.store().trust_path();
        let trust_before = file_state(&trust_path);

        cmd_remove(&store, &scope, user.path(), "./adapter").expect("project remove");

        assert_eq!(file_state(&trust_path), trust_before);
        assert!(store.read_declarations().expect("declarations").is_empty());
        assert!(store.read_lock().expect("locks").is_empty());
    }

    #[test]
    fn git_update_stage_failure_preserves_old_state_and_durable_invalidation() {
        assert_git_update_fault_rolls_back(GitInstallFaultPoint::StageCacheReplacement);
    }

    #[test]
    fn git_update_canonicalize_failure_restores_old_state_and_durable_invalidation() {
        assert_git_update_fault_rolls_back(GitInstallFaultPoint::CanonicalizeLiveCache);
    }
}
