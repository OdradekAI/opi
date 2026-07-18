use std::sync::{Arc, Mutex};

use crate::credential_store::BackendError;

/// Process-lifetime ownership of the `keyring-core` default store.
pub struct NativeKeyringGuard {
    leased: bool,
}

struct InstallState {
    store: Option<Arc<keyring_core::CredentialStore>>,
    leases: usize,
}

static INSTALL_STATE: Mutex<InstallState> = Mutex::new(InstallState {
    store: None,
    leases: 0,
});

type PlatformStoreConstructor = fn() -> Result<Arc<keyring_core::CredentialStore>, String>;

#[cfg(test)]
pub(crate) static KEYRING_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_install_state() -> std::sync::MutexGuard<'static, InstallState> {
    INSTALL_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Drop for NativeKeyringGuard {
    fn drop(&mut self) {
        if !self.leased {
            return;
        }
        let mut state = lock_install_state();
        debug_assert!(state.leases > 0, "native keyring lease underflow");
        state.leases -= 1;
        if state.leases == 0 {
            keyring_core::unset_default_store();
            state.store = None;
        }
        self.leased = false;
    }
}

/// Install the native credential store selected for the current release target.
pub fn install_native_keyring() -> Result<NativeKeyringGuard, BackendError> {
    install_native_keyring_with(native_platform_store)
}

fn install_native_keyring_with(
    constructor: PlatformStoreConstructor,
) -> Result<NativeKeyringGuard, BackendError> {
    {
        let mut state = lock_install_state();
        if state.leases > 0 {
            state.leases += 1;
            return Ok(NativeKeyringGuard { leased: true });
        }
    }
    let store = platform_store_with(constructor)?;
    install_store(store)
}

pub(crate) fn install_store(
    store: Arc<keyring_core::CredentialStore>,
) -> Result<NativeKeyringGuard, BackendError> {
    let mut state = lock_install_state();
    if state.leases == 0 {
        keyring_core::set_default_store(Arc::clone(&store));
        state.store = Some(store);
    }
    state.leases += 1;
    Ok(NativeKeyringGuard { leased: true })
}

fn classify_platform_store_error(target_os: &str, reason: String) -> BackendError {
    if target_os == "linux" && crate::credential_store::secret_service_is_unavailable(&reason) {
        BackendError::BackendUnavailable(reason)
    } else {
        BackendError::Other(reason)
    }
}

#[cfg(target_os = "windows")]
fn platform_store_with(
    constructor: PlatformStoreConstructor,
) -> Result<Arc<keyring_core::CredentialStore>, BackendError> {
    constructor().map_err(|reason| classify_platform_store_error("windows", reason))
}

#[cfg(target_os = "windows")]
fn native_platform_store() -> Result<Arc<keyring_core::CredentialStore>, String> {
    let store: Arc<keyring_core::CredentialStore> =
        windows_native_keyring_store::Store::new().map_err(|error| error.to_string())?;
    Ok(store)
}

#[cfg(target_os = "macos")]
fn platform_store_with(
    constructor: PlatformStoreConstructor,
) -> Result<Arc<keyring_core::CredentialStore>, BackendError> {
    constructor().map_err(|reason| classify_platform_store_error("macos", reason))
}

#[cfg(target_os = "macos")]
fn native_platform_store() -> Result<Arc<keyring_core::CredentialStore>, String> {
    let store: Arc<keyring_core::CredentialStore> =
        apple_native_keyring_store::keychain::Store::new().map_err(|error| error.to_string())?;
    Ok(store)
}

#[cfg(target_os = "linux")]
fn platform_store_with(
    constructor: PlatformStoreConstructor,
) -> Result<Arc<keyring_core::CredentialStore>, BackendError> {
    constructor().map_err(|reason| classify_platform_store_error("linux", reason))
}

#[cfg(target_os = "linux")]
fn native_platform_store() -> Result<Arc<keyring_core::CredentialStore>, String> {
    let store: Arc<keyring_core::CredentialStore> =
        zbus_secret_service_keyring_store::Store::new().map_err(|error| error.to_string())?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    static PLATFORM_STORE_CONSTRUCTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn mock_platform_store_constructor() -> Result<Arc<keyring_core::CredentialStore>, String> {
        PLATFORM_STORE_CONSTRUCTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        keyring_core::mock::Store::new()
            .map(|store| store as Arc<keyring_core::CredentialStore>)
            .map_err(|error| error.to_string())
    }

    fn unavailable_platform_store_constructor() -> Result<Arc<keyring_core::CredentialStore>, String>
    {
        Err("connection refused".to_owned())
    }

    fn operational_platform_store_constructor() -> Result<Arc<keyring_core::CredentialStore>, String>
    {
        Err("permission denied".to_owned())
    }

    #[test]
    fn platform_initialization_errors_preserve_operational_classification() {
        use crate::credential_store::BackendError;

        for (platform, reason, unavailable) in [
            ("windows", "credential manager unavailable", false),
            ("windows", "permission denied", false),
            ("macos", "keychain unavailable", false),
            ("macos", "permission denied", false),
            ("linux", "org.freedesktop.DBus.Error.ServiceUnknown", true),
            ("linux", "connection refused", true),
            ("linux", "credential store locked", false),
            ("linux", "permission denied", false),
        ] {
            let classified = super::classify_platform_store_error(platform, reason.to_owned());
            assert_eq!(
                matches!(classified, BackendError::BackendUnavailable(_)),
                unavailable,
                "{platform}: {reason}"
            );
        }
    }

    #[test]
    fn platform_store_with_preserves_host_error_classification() {
        use crate::credential_store::BackendError;

        let unavailable = super::platform_store_with(unavailable_platform_store_constructor);
        #[cfg(target_os = "linux")]
        assert!(matches!(
            unavailable,
            Err(BackendError::BackendUnavailable(reason)) if reason == "connection refused"
        ));
        #[cfg(not(target_os = "linux"))]
        assert!(matches!(
            unavailable,
            Err(BackendError::Other(reason)) if reason == "connection refused"
        ));

        assert!(matches!(
            super::platform_store_with(operational_platform_store_constructor),
            Err(BackendError::Other(reason)) if reason == "permission denied"
        ));
    }

    #[test]
    fn native_keyring_host_selection_installs_a_default_store() {
        let _serial = super::KEYRING_TEST_LOCK.lock().expect("keyring test lock");
        keyring_core::unset_default_store();
        PLATFORM_STORE_CONSTRUCTOR_CALLS.store(0, Ordering::SeqCst);

        let first_guard = super::install_native_keyring_with(mock_platform_store_constructor)
            .expect("install mock default store");
        assert!(keyring_core::get_default_store().is_some());
        let first_store_id = keyring_core::get_default_store()
            .expect("first guard installs a default store")
            .id();

        let second_guard = super::install_native_keyring_with(mock_platform_store_constructor)
            .expect("reuse mock default store");
        assert_eq!(PLATFORM_STORE_CONSTRUCTOR_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(
            keyring_core::get_default_store()
                .expect("second guard retains a default store")
                .id(),
            first_store_id,
            "the second lease must not replace the first store"
        );

        drop(first_guard);
        assert!(keyring_core::get_default_store().is_some());
        drop(second_guard);
        assert!(keyring_core::get_default_store().is_none());
    }

    #[test]
    fn overlapping_guards_share_first_store_until_last_drop() {
        let _serial = super::KEYRING_TEST_LOCK.lock().expect("keyring test lock");
        keyring_core::unset_default_store();
        let first: Arc<keyring_core::CredentialStore> =
            keyring_core::mock::Store::new().expect("first mock store");
        let first_id = first.id();
        let second: Arc<keyring_core::CredentialStore> =
            keyring_core::mock::Store::new().expect("second mock store");

        let first_guard = super::install_store(first).expect("install first store");
        let second_guard = super::install_store(second).expect("reuse first store");
        assert_eq!(
            keyring_core::get_default_store().unwrap().id(),
            first_id,
            "the 1->2 lease transition must not replace the active store"
        );

        drop(first_guard);
        assert!(
            keyring_core::get_default_store().is_some(),
            "dropping one overlapping guard must retain the default store"
        );
        drop(second_guard);
        assert!(
            keyring_core::get_default_store().is_none(),
            "only the 1->0 lease transition may unset the default store"
        );
    }

    #[test]
    fn poisoned_install_mutex_recovers_without_losing_lease_count() {
        let _serial = super::KEYRING_TEST_LOCK.lock().expect("keyring test lock");
        keyring_core::unset_default_store();
        let first: Arc<keyring_core::CredentialStore> =
            keyring_core::mock::Store::new().expect("first mock store");
        let first_guard = super::install_store(first).expect("install first store");
        let poisoned = std::panic::catch_unwind(|| {
            let _state = super::INSTALL_STATE.lock().expect("initial state lock");
            panic!("poison native install state for test");
        });
        assert!(poisoned.is_err());

        let second: Arc<keyring_core::CredentialStore> =
            keyring_core::mock::Store::new().expect("second mock store");
        let second_guard = super::install_store(second).expect("recover active lease count");
        drop(first_guard);
        assert!(keyring_core::get_default_store().is_some());
        drop(second_guard);
        assert!(keyring_core::get_default_store().is_none());
    }
}
