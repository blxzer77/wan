use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    /// No key is available, so nothing can be sealed or opened. Callers pause the step until
    /// the vault is unlocked.
    #[error("the vault is locked")]
    Locked,
    /// The ciphertext failed authentication: wrong key, tampered data, or tampered AAD.
    #[error("decryption failed")]
    Decrypt,
    #[error("encryption failed")]
    Encrypt,
    #[error("unsupported format version {0}")]
    UnsupportedFormat(u32),
    #[error("stored data is malformed: {0}")]
    Malformed(&'static str),
    #[error("the recovery key is not valid")]
    InvalidRecoveryKey,
    #[error("the recovery key was not confirmed as saved")]
    RecoveryNotConfirmed,
    #[error("two items share a nonce: {first} and {second}")]
    NonceReuse { first: String, second: String },
    #[error("key file {} must not live inside the data directory {}", .key_file.display(), .data_dir.display())]
    KeyFileInsideDataDir {
        key_file: PathBuf,
        data_dir: PathBuf,
    },
    #[error("key file {} must be readable by its owner only (mode 0400)", .0.display())]
    KeyFilePermissions(PathBuf),
    #[error("key file I/O failed: {0}")]
    KeyFileIo(#[from] std::io::Error),
    #[error("the OS key store is unavailable: {0}")]
    KeyStoreUnavailable(String),
    #[error("the OS key store failed: {0}")]
    KeyStore(String),
}
