//! Vault cryptography: pure functions, values in and values out.
//!
//! The crate owns no database, defines no persistence trait, and exports no function that
//! returns plaintext. `wand` stores the plain structs below and wires the callers (ADR 0002).
//!
//! Key hierarchy: a master key (MK) wraps the data encryption key (DEK), and the recovery key
//! wraps a second copy of the same DEK. Items are encrypted directly under the DEK with
//! XChaCha20-Poly1305 and a random 24-byte nonce.

mod aad;
mod audit;
mod error;
mod item;
mod key;
mod keyfile;
mod keystore;
mod recovery;

pub use audit::{AuditRecord, Trigger};
pub use error::VaultError;
pub use item::{
    FORMAT_VERSION, ItemType, SealedItem, find_nonce_reuse, reencrypt_item, seal_item, with_secret,
};
pub use key::{
    Dek, MasterKey, WrapKind, WrappedKey, generate_dek, generate_master_key, rewrap_dek,
    unwrap_dek, wrap_dek,
};
pub use keyfile::{read_key_file, write_key_file};
pub use keystore::{KeyStore, OsKeyStore, UnlockMethod, load_master_key, store_master_key};
pub use recovery::{RecoveryKey, RecoverySetup, generate_recovery_key, parse_recovery_key};
