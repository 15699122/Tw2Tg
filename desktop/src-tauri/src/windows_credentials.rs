//! Windows-only Telegram credential provider. Never uses the default/mock store.
use xarchive_telegram::{SecretStore, SecretStoreError};

pub(crate) struct WindowsCredentialStore;

fn entry(key: &str) -> Result<keyring::Entry, SecretStoreError> {
    if key.trim().is_empty() || key.contains('\0') {
        return Err(SecretStoreError::InvalidKey);
    }
    let credential =
        keyring::windows::WinCredential::new_with_target(None, "XArchive.Telegram", key)
            .map_err(sanitize)?;
    Ok(keyring::Entry::new_with_credential(Box::new(credential)))
}

fn sanitize(error: keyring::Error) -> SecretStoreError {
    match error {
        keyring::Error::NoStorageAccess(_) => {
            SecretStoreError::AccessDenied("Windows credential access denied".into())
        }
        _ => SecretStoreError::Unavailable("Windows credential operation failed".into()),
    }
}

impl SecretStore for WindowsCredentialStore {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        match entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(sanitize(error)),
        }
    }
    fn set(&mut self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        if value.is_empty() {
            return Err(SecretStoreError::EmptyValue);
        }
        entry(key)?.set_password(value).map_err(sanitize)
    }
    fn delete(&mut self, key: &str) -> Result<(), SecretStoreError> {
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(sanitize(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_input_and_redacts_native_errors() {
        let mut store = WindowsCredentialStore;
        assert_eq!(store.get(" "), Err(SecretStoreError::InvalidKey));
        assert_eq!(store.get("bad\0key"), Err(SecretStoreError::InvalidKey));
        assert_eq!(store.set("test", ""), Err(SecretStoreError::EmptyValue));
        let error = sanitize(keyring::Error::BadEncoding(b"synthetic-secret".to_vec()));
        assert!(!format!("{error:?}").contains("synthetic-secret"));
    }

    // Opt-in because this creates a uniquely named synthetic credential in the
    // current Windows account. Never reads or overwrites production credentials.
    #[test]
    #[ignore = "requires native Windows Credential Manager access"]
    fn native_create_replace_read_delete() {
        let key = format!(
            "validation-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        );
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = WindowsCredentialStore.delete(&self.0);
            }
        }
        let _cleanup = Cleanup(key.clone());
        let mut store = WindowsCredentialStore;
        assert!(store.get(&key).unwrap().is_none());
        store.set(&key, "synthetic-one").unwrap();
        assert!(store.get(&key).unwrap().as_deref() == Some("synthetic-one"));
        store.set(&key, "synthetic-two").unwrap();
        assert!(WindowsCredentialStore.get(&key).unwrap().as_deref() == Some("synthetic-two"));
        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
        store.delete(&key).unwrap();
    }

    // Uses the test executable itself as two fresh processes, with a unique
    // synthetic key and a parent-owned cleanup guard.
    #[test]
    #[ignore = "requires native Windows Credential Manager access"]
    fn native_credential_survives_process_restart() {
        const KEY_ENV: &str = "XARCHIVE_TEST_CREDENTIAL_RESTART_KEY";
        const PHASE_ENV: &str = "XARCHIVE_TEST_CREDENTIAL_RESTART_PHASE";
        if let Some(phase) = std::env::var_os(PHASE_ENV) {
            let key = std::env::var(KEY_ENV).expect("test key must be supplied by parent");
            assert!(key.starts_with("validation-restart-"));
            let mut store = WindowsCredentialStore;
            match phase.to_str() {
                Some("replace") => {
                    assert!(store.get(&key).unwrap().as_deref() == Some("synthetic-before"));
                    store.set(&key, "synthetic-after").unwrap();
                }
                Some("read") => {
                    assert!(store.get(&key).unwrap().as_deref() == Some("synthetic-after"));
                }
                _ => panic!("invalid credential test phase"),
            }
            return;
        }

        let key = format!(
            "validation-restart-{}-{}",
            std::process::id(),
            crate::runtime::timestamp_marker()
        );
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = WindowsCredentialStore.delete(&self.0);
            }
        }
        let _cleanup = Cleanup(key.clone());
        let mut store = WindowsCredentialStore;
        assert!(store.get(&key).unwrap().is_none());
        store.set(&key, "synthetic-before").unwrap();
        for phase in ["replace", "read"] {
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "windows_credentials::tests::native_credential_survives_process_restart",
                ])
                .env(KEY_ENV, &key)
                .env(PHASE_ENV, phase)
                .output()
                .expect("credential test child must start");
            assert!(
                child.status.success(),
                "credential test child {phase} failed"
            );
            assert!(
                String::from_utf8_lossy(&child.stdout).contains("1 passed; 0 failed"),
                "credential test child {phase} did not execute exactly one passing test"
            );
        }
        store.delete(&key).unwrap();
        assert!(store.get(&key).unwrap().is_none());
    }
}
