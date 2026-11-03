use std::collections::HashMap;

use chacha20poly1305::XNonce;
use chacha20poly1305::aead::{Aead, Generate, Payload};
use zeroize::Zeroizing;

use crate::aad::Aad;
use crate::error::VaultError;
use crate::key::{Dek, NONCE_LEN, cipher_for, nonce_from};

/// Version of the item layout and AAD. Bump it when either changes.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ItemType {
    Login,
    Card,
    Contact,
    ApiKey,
}

impl ItemType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Card => "card",
            Self::Contact => "contact",
            Self::ApiKey => "api_key",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "login" => Some(Self::Login),
            "card" => Some(Self::Card),
            "contact" => Some(Self::Contact),
            "api_key" => Some(Self::ApiKey),
            _ => None,
        }
    }
}

/// One encrypted field of a vault item. The fields map to columns. `item_id`, `field`,
/// `origin`, `item_type`, and `format_version` are all bound into the AAD, so editing any of
/// them in storage makes the item fail to open.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SealedItem {
    pub item_id: String,
    pub field: String,
    /// The origin an item may be filled into; empty for items without one, such as API keys.
    pub origin: String,
    pub item_type: ItemType,
    pub format_version: u32,
    pub key_version: u32,
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

fn item_aad(
    item_id: &str,
    field: &str,
    origin: &str,
    item_type: ItemType,
    format_version: u32,
) -> Aad {
    let mut aad = Aad::new("wan-vault/item");
    aad.text(item_id)
        .text(field)
        .text(origin)
        .text(item_type.label())
        .number(format_version);
    aad
}

/// Encrypts one field under the DEK with a fresh random nonce.
pub fn seal_item(
    dek: &Dek,
    key_version: u32,
    item_id: &str,
    field: &str,
    origin: &str,
    item_type: ItemType,
    plaintext: &[u8],
) -> Result<SealedItem, VaultError> {
    let nonce = XNonce::generate();
    let aad = item_aad(item_id, field, origin, item_type, FORMAT_VERSION);
    let ciphertext = cipher_for(dek.expose())
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: aad.as_slice(),
            },
        )
        .map_err(|_| VaultError::Encrypt)?;
    let mut stored = [0u8; NONCE_LEN];
    stored.copy_from_slice(&nonce);
    Ok(SealedItem {
        item_id: item_id.to_owned(),
        field: field.to_owned(),
        origin: origin.to_owned(),
        item_type,
        format_version: FORMAT_VERSION,
        key_version,
        nonce: stored,
        ciphertext,
    })
}

fn open(dek: &Dek, item: &SealedItem) -> Result<Zeroizing<Vec<u8>>, VaultError> {
    if item.format_version != FORMAT_VERSION {
        return Err(VaultError::UnsupportedFormat(item.format_version));
    }
    let aad = item_aad(
        &item.item_id,
        &item.field,
        &item.origin,
        item.item_type,
        item.format_version,
    );
    cipher_for(dek.expose())
        .decrypt(
            &nonce_from(&item.nonce),
            Payload {
                msg: &item.ciphertext,
                aad: aad.as_slice(),
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| VaultError::Decrypt)
}

/// Decrypts a field and lends the plaintext to `use_secret`. This is the only way plaintext
/// leaves the crate: the buffer is zeroed when the callback returns, and the callback can
/// only return what it chooses to copy out.
pub fn with_secret<R>(
    dek: &Dek,
    item: &SealedItem,
    use_secret: impl FnOnce(&[u8]) -> R,
) -> Result<R, VaultError> {
    let plain = open(dek, item)?;
    Ok(use_secret(&plain))
}

/// DEK rotation, one item at a time: decrypts under `old` and seals again under `new` with a
/// fresh nonce and the new `key_version`. Scheduling the sweep belongs to the caller.
pub fn reencrypt_item(
    old: &Dek,
    new: &Dek,
    new_key_version: u32,
    item: &SealedItem,
) -> Result<SealedItem, VaultError> {
    let plain = open(old, item)?;
    seal_item(
        new,
        new_key_version,
        &item.item_id,
        &item.field,
        &item.origin,
        item.item_type,
        &plain,
    )
}

/// Finds two items that share a nonce. Random 192-bit nonces make this astronomically
/// unlikely, so a hit means a copied row or a broken random source. Callers run it before
/// storing a batch.
pub fn find_nonce_reuse(items: &[SealedItem]) -> Result<(), VaultError> {
    let mut seen: HashMap<([u8; NONCE_LEN], u32), &SealedItem> = HashMap::new();
    for item in items {
        if let Some(first) = seen.insert((item.nonce, item.key_version), item) {
            return Err(VaultError::NonceReuse {
                first: format!("{}/{}", first.item_id, first.field),
                second: format!("{}/{}", item.item_id, item.field),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::generate_dek;

    fn sealed(dek: &Dek, plaintext: &[u8]) -> SealedItem {
        seal_item(
            dek,
            1,
            "item-1",
            "password",
            "https://example.com",
            ItemType::Login,
            plaintext,
        )
        .expect("seals")
    }

    fn opened(dek: &Dek, item: &SealedItem) -> Result<Vec<u8>, VaultError> {
        with_secret(dek, item, <[u8]>::to_vec)
    }

    #[test]
    fn seal_then_open_round_trips() {
        let dek = generate_dek();

        let item = sealed(&dek, b"hunter2");

        assert_eq!(opened(&dek, &item).expect("opens"), b"hunter2");
        assert_ne!(item.ciphertext, b"hunter2");
        assert_eq!(item.format_version, FORMAT_VERSION);
    }

    #[test]
    fn wrong_key_cannot_open() {
        let item = sealed(&generate_dek(), b"hunter2");

        assert!(matches!(
            opened(&generate_dek(), &item),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn tampering_with_every_aad_field_is_detected() {
        let dek = generate_dek();
        let item = sealed(&dek, b"hunter2");

        let mut cases: Vec<(&str, SealedItem)> = Vec::new();
        let mut other = item.clone();
        other.item_id = "item-2".to_owned();
        cases.push(("item_id", other));
        let mut other = item.clone();
        other.field = "username".to_owned();
        cases.push(("field", other));
        let mut other = item.clone();
        other.origin = "https://evil.example".to_owned();
        cases.push(("origin", other));
        let mut other = item.clone();
        other.item_type = ItemType::Card;
        cases.push(("item_type", other));
        let mut other = item.clone();
        other.nonce[0] ^= 1;
        cases.push(("nonce", other));
        let mut other = item.clone();
        other.ciphertext[0] ^= 1;
        cases.push(("ciphertext", other));

        for (name, tampered) in cases {
            assert!(
                matches!(opened(&dek, &tampered), Err(VaultError::Decrypt)),
                "changing {name} must fail to open"
            );
        }
    }

    #[test]
    fn an_unknown_format_version_is_refused_not_guessed() {
        let dek = generate_dek();
        let mut item = sealed(&dek, b"hunter2");
        item.format_version = FORMAT_VERSION + 1;

        assert!(matches!(
            opened(&dek, &item),
            Err(VaultError::UnsupportedFormat(2))
        ));
    }

    #[test]
    fn field_boundaries_cannot_be_shifted_between_aad_fields() {
        let dek = generate_dek();
        let item = seal_item(&dek, 1, "ab", "c", "", ItemType::ApiKey, b"k").expect("seals");
        let mut shifted = item.clone();
        shifted.item_id = "a".to_owned();
        shifted.field = "bc".to_owned();

        assert!(matches!(opened(&dek, &shifted), Err(VaultError::Decrypt)));
    }

    #[test]
    fn nonces_differ_between_seals_of_the_same_plaintext() {
        let dek = generate_dek();

        let first = sealed(&dek, b"same");
        let second = sealed(&dek, b"same");

        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
        assert!(find_nonce_reuse(&[first, second]).is_ok());
    }

    #[test]
    fn nonce_reuse_is_detected() {
        let dek = generate_dek();
        let first = sealed(&dek, b"one");
        let mut second = sealed(&dek, b"two");
        second.nonce = first.nonce;

        let error = find_nonce_reuse(&[first, second]).expect_err("reuse is reported");

        assert!(matches!(error, VaultError::NonceReuse { .. }));
    }

    #[test]
    fn reencrypting_moves_an_item_to_the_new_key() {
        let old = generate_dek();
        let new = generate_dek();
        let item = sealed(&old, b"hunter2");

        let moved = reencrypt_item(&old, &new, 2, &item).expect("reencrypts");

        assert_eq!(moved.key_version, 2);
        assert_eq!(moved.item_id, item.item_id);
        assert_ne!(moved.nonce, item.nonce);
        assert_eq!(opened(&new, &moved).expect("new key opens"), b"hunter2");
        assert!(opened(&old, &moved).is_err());
    }

    #[test]
    fn reencrypting_a_tampered_item_fails_instead_of_laundering_it() {
        let old = generate_dek();
        let mut item = sealed(&old, b"hunter2");
        item.origin = "https://evil.example".to_owned();

        assert!(reencrypt_item(&old, &generate_dek(), 2, &item).is_err());
    }

    #[test]
    fn item_type_labels_round_trip() {
        for item_type in [
            ItemType::Login,
            ItemType::Card,
            ItemType::Contact,
            ItemType::ApiKey,
        ] {
            assert_eq!(ItemType::from_label(item_type.label()), Some(item_type));
        }
        assert_eq!(ItemType::from_label("passport"), None);
    }
}
