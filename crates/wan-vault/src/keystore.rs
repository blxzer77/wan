//! OS key store (Keychain, Credential Manager, Secret Service) for the master key. When the
//! platform has no usable store, such as Linux without a desktop session, callers fall back to
//! the key file.

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

use crate::error::VaultError;
use crate::key::{KEY_LEN, MasterKey};

const SERVICE: &str = "wan-vault";

/// How the master key was obtained.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UnlockMethod {
    OsKeyStore,
    KeyFile,
}

/// A place that can hold one secret string for a named account. The production implementation
/// is the OS key store; tests supply an in-memory one.
pub trait KeyStore {
    /// `Ok(None)` means the store works but holds no entry yet.
    fn get(&self, account: &str) -> Result<Option<SecretString>, VaultError>;
    fn set(&self, account: &str, value: &SecretString) -> Result<(), VaultError>;
}

/// The real OS key store through `keyring`.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsKeyStore;

fn classify(error: keyring::Error) -> VaultError {
    match error {
        // No default store means there is no desktop session to talk to.
        keyring::Error::NoDefaultStore | keyring::Error::NoStorageAccess(_) => {
            VaultError::KeyStoreUnavailable(error.to_string())
        }
        other => VaultError::KeyStore(other.to_string()),
    }
}

impl KeyStore for OsKeyStore {
    fn get(&self, account: &str) -> Result<Option<SecretString>, VaultError> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(classify)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(SecretString::from(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(classify(error)),
        }
    }

    fn set(&self, account: &str, value: &SecretString) -> Result<(), VaultError> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(classify)?;
        entry.set_password(value.expose_secret()).map_err(classify)
    }
}

fn to_hex(bytes: &[u8]) -> Zeroizing<String> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    text
}

fn from_hex(text: &str) -> Result<[u8; KEY_LEN], VaultError> {
    let malformed = || VaultError::Malformed("stored master key is not valid hex");
    if text.len() != KEY_LEN * 2 {
        return Err(VaultError::Malformed(
            "stored master key has the wrong length",
        ));
    }
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    let mut bytes = [0u8; KEY_LEN];
    for (slot, [high, low]) in bytes.iter_mut().zip(pairs) {
        let high = char::from(*high).to_digit(16).ok_or_else(malformed)?;
        let low = char::from(*low).to_digit(16).ok_or_else(malformed)?;
        *slot = (high * 16 + low) as u8;
    }
    Ok(bytes)
}

/// Saves the master key in the store under `account`.
pub fn store_master_key(
    store: &impl KeyStore,
    account: &str,
    key: &MasterKey,
) -> Result<(), VaultError> {
    let hex = to_hex(key.expose());
    store.set(account, &SecretString::from(hex.as_str()))
}

/// Loads the master key. `Locked` means the store works but has no key for `account`;
/// `KeyStoreUnavailable` means there is no store, and the caller should try the key file.
pub fn load_master_key(store: &impl KeyStore, account: &str) -> Result<MasterKey, VaultError> {
    let value = store.get(account)?.ok_or(VaultError::Locked)?;
    Ok(MasterKey::from_bytes(from_hex(value.expose_secret())?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::generate_master_key;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryStore(RefCell<HashMap<String, String>>);

    impl KeyStore for MemoryStore {
        fn get(&self, account: &str) -> Result<Option<SecretString>, VaultError> {
            Ok(self
                .0
                .borrow()
                .get(account)
                .map(|value| SecretString::from(value.as_str())))
        }

        fn set(&self, account: &str, value: &SecretString) -> Result<(), VaultError> {
            self.0
                .borrow_mut()
                .insert(account.to_owned(), value.expose_secret().to_owned());
            Ok(())
        }
    }

    struct NoDesktop;

    impl KeyStore for NoDesktop {
        fn get(&self, _: &str) -> Result<Option<SecretString>, VaultError> {
            Err(VaultError::KeyStoreUnavailable("no session bus".to_owned()))
        }

        fn set(&self, _: &str, _: &SecretString) -> Result<(), VaultError> {
            Err(VaultError::KeyStoreUnavailable("no session bus".to_owned()))
        }
    }

    #[test]
    fn master_key_round_trips_through_the_store() {
        let store = MemoryStore::default();
        let key = generate_master_key();

        store_master_key(&store, "primary", &key).expect("stores");
        let loaded = load_master_key(&store, "primary").expect("loads");

        assert_eq!(loaded.expose(), key.expose());
    }

    #[test]
    fn a_working_store_without_a_key_means_locked() {
        let store = MemoryStore::default();

        assert!(matches!(
            load_master_key(&store, "primary"),
            Err(VaultError::Locked)
        ));
    }

    #[test]
    fn a_missing_desktop_session_is_reported_as_unavailable_not_locked() {
        assert!(matches!(
            load_master_key(&NoDesktop, "primary"),
            Err(VaultError::KeyStoreUnavailable(_))
        ));
        assert!(matches!(
            store_master_key(&NoDesktop, "primary", &generate_master_key()),
            Err(VaultError::KeyStoreUnavailable(_))
        ));
    }

    #[test]
    fn a_corrupted_entry_is_malformed_not_a_wrong_key() {
        let store = MemoryStore::default();
        store
            .set("primary", &SecretString::from("not-hex"))
            .expect("sets");
        store
            .set("short", &SecretString::from("abcd"))
            .expect("sets");

        for account in ["primary", "short"] {
            assert!(matches!(
                load_master_key(&store, account),
                Err(VaultError::Malformed(_))
            ));
        }
    }

    #[test]
    fn hex_encoding_is_lowercase_and_round_trips() {
        let bytes: [u8; KEY_LEN] = std::array::from_fn(|index| index as u8 * 8);

        let text = to_hex(&bytes);

        assert_eq!(text.len(), KEY_LEN * 2);
        assert!(text.starts_with("0008101820283038"));
        assert_eq!(from_hex(&text).expect("decodes"), bytes);
    }

    /// Talks to the real OS key store. It needs a desktop session, so it reports why it
    /// skipped instead of failing or silently passing on machines that have none.
    #[test]
    fn real_os_key_store_round_trips_when_a_session_is_available() {
        let account = format!("test-{}", std::process::id());
        let key = generate_master_key();

        match store_master_key(&OsKeyStore, &account, &key) {
            Ok(()) => {
                let loaded = load_master_key(&OsKeyStore, &account).expect("loads what it stored");
                assert_eq!(loaded.expose(), key.expose());
                if let Ok(entry) = keyring::Entry::new(SERVICE, &account) {
                    let _ = entry.delete_credential();
                }
            }
            Err(VaultError::KeyStoreUnavailable(reason)) => {
                eprintln!("skipped: no usable OS key store ({reason})");
            }
            Err(other) => panic!("unexpected key store failure: {other}"),
        }
    }
}
