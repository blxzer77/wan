use chacha20poly1305::aead::{Aead, Generate, Key, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use secrecy::{ExposeSecret, SecretBox};
use zeroize::Zeroizing;

use crate::aad::Aad;
use crate::error::VaultError;

pub(crate) const KEY_LEN: usize = 32;
pub(crate) const NONCE_LEN: usize = 24;

/// The key that wraps the DEK. It lives in the OS key store or in a key file and is never
/// part of a backup.
pub struct MasterKey(pub(crate) SecretBox<[u8; KEY_LEN]>);

/// The data encryption key. Items are encrypted directly under it.
pub struct Dek(pub(crate) SecretBox<[u8; KEY_LEN]>);

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MasterKey(..)")
    }
}

impl std::fmt::Debug for Dek {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Dek(..)")
    }
}

impl MasterKey {
    pub(crate) fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(SecretBox::new(Box::new(bytes)))
    }

    pub(crate) fn expose(&self) -> &[u8; KEY_LEN] {
        self.0.expose_secret()
    }
}

impl Dek {
    pub(crate) fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(SecretBox::new(Box::new(bytes)))
    }

    pub(crate) fn expose(&self) -> &[u8; KEY_LEN] {
        self.0.expose_secret()
    }
}

/// Which key produced a wrapping. It is part of the AAD, so the two stored copies of the DEK
/// cannot be swapped for each other.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WrapKind {
    Master,
    Recovery,
}

impl WrapKind {
    fn label(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Recovery => "recovery",
        }
    }
}

/// A DEK encrypted under the master key or the recovery key. The fields map to columns.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WrappedKey {
    pub kind: WrapKind,
    pub key_version: u32,
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

pub fn generate_master_key() -> MasterKey {
    MasterKey::from_bytes(random_key())
}

pub fn generate_dek() -> Dek {
    Dek::from_bytes(random_key())
}

/// Residual risk, accepted: `Key` is a `Copy` array type without a `Zeroize` impl, so the
/// short-lived stack copy made while generating or loading a key cannot be wiped. The threat
/// model already excludes an attacker who can read daemon memory (design 5.3). The copies this
/// crate owns (`SecretBox`, `Zeroizing`, and the cipher, which zeroes its key on drop under the
/// `zeroize` feature) are all wiped.
pub(crate) fn random_key() -> [u8; KEY_LEN] {
    let key = Key::<XChaCha20Poly1305>::generate();
    let mut bytes = [0u8; KEY_LEN];
    bytes.copy_from_slice(&key);
    bytes
}

pub(crate) fn cipher_for(key: &[u8; KEY_LEN]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(&Key::<XChaCha20Poly1305>::from(*key))
}

pub(crate) fn nonce_from(bytes: &[u8; NONCE_LEN]) -> XNonce {
    XNonce::from(*bytes)
}

fn wrap_aad(kind: WrapKind, key_version: u32) -> Aad {
    let mut aad = Aad::new("wan-vault/dek-wrap");
    aad.text(kind.label()).number(key_version);
    aad
}

pub(crate) fn wrap_with(
    wrapping_key: &[u8; KEY_LEN],
    kind: WrapKind,
    key_version: u32,
    dek: &Dek,
) -> Result<WrappedKey, VaultError> {
    let cipher = cipher_for(wrapping_key);
    let nonce = XNonce::generate();
    let aad = wrap_aad(kind, key_version);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: dek.expose(),
                aad: aad.as_slice(),
            },
        )
        .map_err(|_| VaultError::Encrypt)?;
    let mut stored = [0u8; NONCE_LEN];
    stored.copy_from_slice(&nonce);
    Ok(WrappedKey {
        kind,
        key_version,
        nonce: stored,
        ciphertext,
    })
}

pub(crate) fn unwrap_with(
    wrapping_key: &[u8; KEY_LEN],
    wrapped: &WrappedKey,
) -> Result<Dek, VaultError> {
    let cipher = cipher_for(wrapping_key);
    let aad = wrap_aad(wrapped.kind, wrapped.key_version);
    let plain = Zeroizing::new(
        cipher
            .decrypt(
                &nonce_from(&wrapped.nonce),
                Payload {
                    msg: &wrapped.ciphertext,
                    aad: aad.as_slice(),
                },
            )
            .map_err(|_| VaultError::Decrypt)?,
    );
    let bytes: [u8; KEY_LEN] = plain
        .as_slice()
        .try_into()
        .map_err(|_| VaultError::Malformed("wrapped key has the wrong length"))?;
    Ok(Dek::from_bytes(bytes))
}

/// Wraps the DEK under the master key.
pub fn wrap_dek(master: &MasterKey, key_version: u32, dek: &Dek) -> Result<WrappedKey, VaultError> {
    wrap_with(master.expose(), WrapKind::Master, key_version, dek)
}

/// Opens a master-key wrapping.
pub fn unwrap_dek(master: &MasterKey, wrapped: &WrappedKey) -> Result<Dek, VaultError> {
    if wrapped.kind != WrapKind::Master {
        return Err(VaultError::Decrypt);
    }
    unwrap_with(master.expose(), wrapped)
}

/// Master-key rotation: re-wraps the same DEK under a new master key. No item is touched.
pub fn rewrap_dek(
    old: &MasterKey,
    new: &MasterKey,
    wrapped: &WrappedKey,
) -> Result<WrappedKey, VaultError> {
    let dek = unwrap_dek(old, wrapped)?;
    wrap_dek(new, wrapped.key_version, &dek)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_then_unwrap_returns_the_same_dek() {
        let master = generate_master_key();
        let dek = generate_dek();

        let wrapped = wrap_dek(&master, 1, &dek).expect("wraps");
        let opened = unwrap_dek(&master, &wrapped).expect("unwraps");

        assert_eq!(opened.expose(), dek.expose());
    }

    #[test]
    fn wrong_master_key_cannot_unwrap() {
        let dek = generate_dek();
        let wrapped = wrap_dek(&generate_master_key(), 1, &dek).expect("wraps");

        assert!(matches!(
            unwrap_dek(&generate_master_key(), &wrapped),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn key_version_and_kind_are_authenticated() {
        let master = generate_master_key();
        let wrapped = wrap_dek(&master, 1, &generate_dek()).expect("wraps");

        let mut other_version = wrapped.clone();
        other_version.key_version = 2;
        let mut other_kind = wrapped.clone();
        other_kind.kind = WrapKind::Recovery;

        assert!(matches!(
            unwrap_dek(&master, &other_version),
            Err(VaultError::Decrypt)
        ));
        assert!(matches!(
            unwrap_dek(&master, &other_kind),
            Err(VaultError::Decrypt)
        ));
        assert!(matches!(
            unwrap_with(master.expose(), &other_kind),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let master = generate_master_key();
        let mut wrapped = wrap_dek(&master, 1, &generate_dek()).expect("wraps");
        wrapped.ciphertext[0] ^= 1;

        assert!(matches!(
            unwrap_dek(&master, &wrapped),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn rotating_the_master_key_keeps_the_dek() {
        let old = generate_master_key();
        let new = generate_master_key();
        let dek = generate_dek();
        let wrapped = wrap_dek(&old, 3, &dek).expect("wraps");

        let rewrapped = rewrap_dek(&old, &new, &wrapped).expect("rewraps");

        assert_eq!(rewrapped.key_version, 3);
        assert_eq!(
            unwrap_dek(&new, &rewrapped)
                .expect("new key opens")
                .expose(),
            dek.expose()
        );
        assert!(unwrap_dek(&old, &rewrapped).is_err());
    }

    #[test]
    fn each_wrapping_uses_a_fresh_nonce() {
        let master = generate_master_key();
        let dek = generate_dek();

        let first = wrap_dek(&master, 1, &dek).expect("wraps");
        let second = wrap_dek(&master, 1, &dek).expect("wraps");

        assert_ne!(first.nonce, second.nonce);
    }

    #[test]
    fn debug_output_never_shows_key_material() {
        let master = generate_master_key();
        let dek = generate_dek();

        assert_eq!(format!("{master:?}"), "MasterKey(..)");
        assert_eq!(format!("{dek:?}"), "Dek(..)");
    }
}
