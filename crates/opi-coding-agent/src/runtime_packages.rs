//! Runtime startup for installed package declarations.

use std::path::Path;

use opi_agent::Diagnostic;
use opi_agent::extension::ExtensionRegistry;

use crate::adapter_extension::start_adapters_from_packages;
use crate::diagnostic_bridge::{diagnostic_from_package, diagnostic_from_package_resolution_error};
use crate::package_discovery::PackageResource;
use crate::package_resolver::{InstalledPackageScope, resolve_installed_packages};
use crate::project_trust::TrustDecision;

/// Installed packages and adapter registry prepared before harness startup.
pub struct RuntimePackageStartup {
    pub extension_registry: ExtensionRegistry,
    pub installed_packages: Vec<PackageResource>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Resolve installed package declarations and start package adapters.
///
/// Trust-permissive: equivalent to
/// [`start_installed_package_runtime_with_trust`] with [`TrustDecision::Trusted`]
/// (every project loads). The interactive trust-gated path (task 15.7) and the
/// headless policy (task 15.8.1) use the trust-aware entry.
pub async fn start_installed_package_runtime(
    workspace_root: &Path,
    user_config_dir: &Path,
) -> RuntimePackageStartup {
    start_installed_package_runtime_with_trust(
        workspace_root,
        user_config_dir,
        TrustDecision::Trusted,
    )
    .await
}

/// Resolve installed package declarations and start package adapters, gating
/// project-scope declarations on `trust_decision` (task 15.7, T6).
///
/// When `trust_decision` is [`TrustDecision::Untrusted`], project-scope
/// (`.opi/packages.toml`) declarations are filtered out before
/// [`start_adapters_from_packages`] -> [`crate::adapter_host::AdapterHost::start`]
/// (the only child-spawn site), so an untrusted project's adapter native
/// children do not start. User-global declarations (`scope == Global`) are
/// always loaded. The gate is at declaration-load, not at process spawn,
/// matching the project-trust design.
pub async fn start_installed_package_runtime_with_trust(
    workspace_root: &Path,
    user_config_dir: &Path,
    trust_decision: TrustDecision,
) -> RuntimePackageStartup {
    let registry = ExtensionRegistry::new();
    let mut diagnostics = Vec::new();
    let resolution = match resolve_installed_packages(workspace_root, user_config_dir) {
        Ok(resolution) => resolution,
        Err(e) => {
            diagnostics.push(diagnostic_from_package_resolution_error(e));
            return RuntimePackageStartup {
                extension_registry: registry,
                installed_packages: Vec::new(),
                diagnostics,
            };
        }
    };

    diagnostics.extend(resolution.diagnostics.iter().map(diagnostic_from_package));
    let project_trusted = !matches!(trust_decision, TrustDecision::Untrusted);
    let installed_packages = resolution
        .packages
        .into_iter()
        .filter(|pkg| project_trusted || matches!(pkg.scope, InstalledPackageScope::Global))
        .map(|package| package.package)
        .collect::<Vec<_>>();
    let (extension_registry, adapter_diagnostics) =
        start_adapters_from_packages(&installed_packages, workspace_root, registry).await;
    diagnostics.extend(adapter_diagnostics);

    RuntimePackageStartup {
        extension_registry,
        installed_packages,
        diagnostics,
    }
}
