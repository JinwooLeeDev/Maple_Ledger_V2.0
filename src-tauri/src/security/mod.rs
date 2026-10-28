use std::collections::HashMap;

use keyring_core::{api::CredentialStoreApi, Entry, Error};
use serde::Serialize;
use zeroize::Zeroizing;

const TARGET: &str = "com.mapleledger.app/nexon-open-api";
// Windows generic credentials permit a 2560-byte blob; passwords use UTF-16.
const MAX_UTF16_UNITS: usize = 1280;

#[derive(Debug, Serialize, PartialEq)]
pub struct CredentialError {
    pub code: &'static str,
    pub message: &'static str,
}

impl CredentialError {
    pub fn unavailable() -> Self {
        Self {
            code: "credential_store_unavailable",
            message: "Windows 보안 저장소에 접근하지 못했습니다. 잠시 후 다시 시도해 주세요.",
        }
    }

    fn invalid() -> Self {
        Self {
            code: "invalid_key_input",
            message:
                "API 키를 입력해 주세요. 공백·제어 문자 없이 1,280자 이내로 입력할 수 있습니다.",
        }
    }
}

// Never format upstream errors: some variants can contain secret bytes.
fn storage_error(error: Error) -> CredentialError {
    match error {
        Error::BadEncoding(_) | Error::BadDataFormat(_, _) => CredentialError {
            code: "credential_data_unreadable",
            message: "저장된 키를 읽을 수 없습니다. 삭제 후 다시 등록해 주세요.",
        },
        _ => CredentialError::unavailable(),
    }
}

pub struct ApiKeyStore {
    target: String,
}

impl Default for ApiKeyStore {
    fn default() -> Self {
        Self {
            target: TARGET.into(),
        }
    }
}

impl ApiKeyStore {
    fn entry(&self) -> Result<Entry, CredentialError> {
        let store = windows_native_keyring_store::Store::new().map_err(storage_error)?;
        let modifiers = HashMap::from([
            ("target", self.target.as_str()),
            // Persist on this PC across app and Windows sign-in sessions; no roaming.
            ("persistence", "Local"),
        ]);
        store
            .build("com.mapleledger.app", "nexon-open-api", Some(&modifiers))
            .map_err(storage_error)
    }

    // Rust-only retrieval. No IPC command exposes this return value.
    pub(crate) fn read(&self) -> Result<Option<Zeroizing<String>>, CredentialError> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(storage_error(error)),
        }
    }

    pub fn is_registered(&self) -> Result<bool, CredentialError> {
        self.read().map(|value| value.is_some())
    }

    pub fn save(&self, key: &str) -> Result<(), CredentialError> {
        let key = validate_key(key)?;
        self.entry()?.set_password(key).map_err(storage_error)
    }

    pub fn delete(&self) -> Result<(), CredentialError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(storage_error(error)),
        }
    }
}

fn validate_key(key: &str) -> Result<&str, CredentialError> {
    let key = key.trim();
    if key.is_empty()
        || key.chars().any(|c| c.is_whitespace() || c.is_control())
        || key.encode_utf16().count() > MAX_UTF16_UNITS
    {
        return Err(CredentialError::invalid());
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::time::{SystemTime, UNIX_EPOCH};

    const FAKE_KEY: &str = "stage2-synthetic-key-not-valid-for-nexon";

    #[test]
    fn input_validation_does_not_claim_api_validity() {
        assert!(validate_key(" arbitrary-not-validated-key ").is_ok());
        for key in ["", "   ", "a b", "a\nb", "a\0b"] {
            assert!(validate_key(key).is_err());
        }
        assert!(validate_key(&"a".repeat(1280)).is_ok());
        assert!(validate_key(&"a".repeat(1281)).is_err());
        assert!(validate_key(&"😀".repeat(641)).is_err());
    }

    #[test]
    fn upstream_secret_bytes_are_not_in_public_error() {
        let error = storage_error(Error::BadEncoding(FAKE_KEY.as_bytes().to_vec()));
        let public = format!("{error:?}");
        assert!(!public.contains(FAKE_KEY));
        assert_eq!(error.code, "credential_data_unreadable");
        let error = storage_error(Error::Invalid("secret".into(), FAKE_KEY.into()));
        assert!(!format!("{error:?}").contains(FAKE_KEY));
    }

    struct Cleanup(ApiKeyStore);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete();
        }
    }

    #[test]
    fn credential_manager_roundtrip_and_process_restart() {
        // The child process reads the isolated test entry, never the app entry.
        if let Ok(target) = std::env::var("MAPLE_LEDGER_TEST_CREDENTIAL_TARGET") {
            assert!(target.starts_with("com.mapleledger.app/test/"));
            let store = ApiKeyStore { target };
            assert!(store.is_registered().unwrap());
            assert!(store
                .read()
                .unwrap()
                .is_some_and(|key| key.as_str() == FAKE_KEY));
            return;
        }

        let target = format!(
            "com.mapleledger.app/test/{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let guard = Cleanup(ApiKeyStore {
            target: target.clone(),
        });
        let store = &guard.0;
        assert!(!store.is_registered().unwrap());
        store.delete().unwrap(); // Missing-entry deletion is idempotent.
        store.save(FAKE_KEY).unwrap();
        assert!(store.is_registered().unwrap());
        assert!(store
            .read()
            .unwrap()
            .is_some_and(|key| key.as_str() == FAKE_KEY));
        assert!(store.save("   ").is_err());
        assert!(store
            .read()
            .unwrap()
            .is_some_and(|key| key.as_str() == FAKE_KEY));
        store.save("replacement-synthetic-key").unwrap();
        assert!(store
            .read()
            .unwrap()
            .is_some_and(|key| key.as_str() == "replacement-synthetic-key"));
        store.save(FAKE_KEY).unwrap();

        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "security::tests::credential_manager_roundtrip_and_process_restart",
            ])
            .env("MAPLE_LEDGER_TEST_CREDENTIAL_TARGET", target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(child.success(), "Separate-process credential read failed");
        store.delete().unwrap();
        assert!(!store.is_registered().unwrap());
        store.delete().unwrap();
    }
}
