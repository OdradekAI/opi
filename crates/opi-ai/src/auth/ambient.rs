//! Explicit lazy adapter over the host process and filesystem.

use super::{AuthContext, AuthContextError, AuthProcessSnapshot};

/// Default lazy [`AuthContext`] over the real process environment and
/// filesystem.
///
/// Construction performs no access: environment variables are read and paths
/// probed only when a management operation demands them, so `Models::new` and
/// registration stay free of env/store/network work. `~` expands via `HOME`
/// (unix) or `USERPROFILE` (Windows); an unexpandable `~` answers `Ok(false)`
/// instead of touching the literal path.
#[derive(Debug, Default, Clone, Copy)]
pub struct AmbientAuthContext;

impl AmbientAuthContext {
    /// Expand a leading `~` (or `~` alone) to the home directory, when the
    /// ambient environment names one.
    fn expand_home(path: &str) -> Option<std::path::PathBuf> {
        if !path.starts_with('~') {
            return Some(std::path::PathBuf::from(path));
        }
        let home = if cfg!(windows) {
            std::env::var_os("USERPROFILE")
        } else {
            std::env::var_os("HOME")
        }?;
        if home.is_empty() {
            return None;
        }
        let rest = path.strip_prefix('~').unwrap_or(path);
        let rest = rest.strip_prefix(['/', '\\']).unwrap_or(rest);
        let mut expanded = std::path::PathBuf::from(home);
        if !rest.is_empty() {
            expanded.push(rest);
        }
        Some(expanded)
    }
}

impl AuthContext for AmbientAuthContext {
    fn home_dir(&self) -> Result<Option<std::path::PathBuf>, AuthContextError> {
        // This API's platform fallback is the source selected here; provider
        // environment precedence is resolved by the caller before this step.
        #[allow(deprecated)]
        let home = std::env::home_dir();
        Ok(home)
    }

    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => Ok(Some(value)),
            // Unset or set-but-empty: not configured, never an error.
            Ok(_) => Ok(None),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(AuthContextError::EnvNotUnicode {
                name: name.to_owned(),
            }),
        }
    }

    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        // An unexpandable `~` answers false: the referenced file cannot exist
        // for this process, and inventing a literal "~" path would be wrong.
        let Some(resolved) = Self::expand_home(path) else {
            return Ok(false);
        };
        match std::fs::metadata(&resolved) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(AuthContextError::FileProbe {
                path: path.to_owned(),
                reason: error.to_string(),
            }),
        }
    }

    fn process_snapshot(&self) -> Result<AuthProcessSnapshot, AuthContextError> {
        let cwd = std::env::current_dir().map_err(|_| AuthContextError::ProcessSnapshot {
            reason: "working directory unavailable",
        })?;
        let mut env = std::collections::BTreeMap::new();
        for (name, value) in std::env::vars_os() {
            let name = name
                .into_string()
                .map_err(|_| AuthContextError::ProcessSnapshot {
                    reason: "environment variable name is not unicode",
                })?;
            let value = value
                .into_string()
                .map_err(|_| AuthContextError::EnvNotUnicode { name: name.clone() })?;
            env.insert(name, value);
        }
        Ok(AuthProcessSnapshot { env, cwd })
    }
}
