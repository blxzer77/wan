//! Master key file. It is the unlock method when no OS key store is available, such as a
//! server or a Linux session without a desktop. Two rules are enforced here, not left to
//! documentation: the file lives outside the data directory, and only its owner can read it.

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use zeroize::Zeroizing;

use crate::error::VaultError;
use crate::key::{KEY_LEN, MasterKey};

#[cfg(unix)]
const REQUIRED_MODE: u32 = 0o400;

/// Resolves `.` and `..` lexically and, when the path exists, through symlinks, so a link
/// from outside the data directory into it cannot defeat the check.
fn resolve(path: &Path) -> std::io::Result<PathBuf> {
    if let Ok(real) = path.canonicalize() {
        return Ok(real);
    }
    let mut parent_real = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.canonicalize()?,
        _ => std::env::current_dir()?,
    };
    if let Some(Component::Normal(name)) = path.components().next_back() {
        parent_real.push(name);
    }
    Ok(parent_real)
}

fn ensure_outside(key_file: &Path, data_dir: &Path) -> Result<(), VaultError> {
    let key_real = resolve(key_file)?;
    let data_real = data_dir.canonicalize()?;
    if key_real.starts_with(&data_real) {
        return Err(VaultError::KeyFileInsideDataDir {
            key_file: key_file.to_path_buf(),
            data_dir: data_dir.to_path_buf(),
        });
    }
    Ok(())
}

/// Writes a new master key file with mode 0400. Refuses to overwrite an existing file, because
/// replacing a master key would orphan the wrapped DEK.
pub fn write_key_file(path: &Path, data_dir: &Path, key: &MasterKey) -> Result<(), VaultError> {
    ensure_outside(path, data_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(REQUIRED_MODE)
            .open(path)?;
        file.write_all(key.expose())?;
        file.sync_all()?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (path, key, OpenOptions::new(), std::io::sink().flush());
        Err(VaultError::KeyStoreUnavailable(
            "key files need Unix file permissions; use the OS key store".to_owned(),
        ))
    }
}

/// Reads the master key file after checking its location and permissions.
pub fn read_key_file(path: &Path, data_dir: &Path) -> Result<MasterKey, VaultError> {
    ensure_outside(path, data_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = path.metadata()?.permissions().mode() & 0o777;
        if mode != REQUIRED_MODE {
            return Err(VaultError::KeyFilePermissions(path.to_path_buf()));
        }
    }
    #[cfg(not(unix))]
    {
        return Err(VaultError::KeyStoreUnavailable(
            "key files need Unix file permissions; use the OS key store".to_owned(),
        ));
    }

    #[cfg(unix)]
    {
        let mut bytes = Zeroizing::new(Vec::with_capacity(KEY_LEN));
        let mut file = OpenOptions::new().read(true).open(path)?;
        file.read_to_end(&mut bytes)?;
        let key: [u8; KEY_LEN] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| VaultError::Malformed("key file has the wrong length"))?;
        Ok(MasterKey::from_bytes(key))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::key::generate_master_key;
    use std::os::unix::fs::PermissionsExt;

    struct Dirs {
        root: PathBuf,
        data: PathBuf,
        keys: PathBuf,
    }

    impl Dirs {
        fn new(label: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "wan-vault-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock is after the epoch")
                    .as_nanos()
            ));
            let data = root.join("data");
            let keys = root.join("keys");
            std::fs::create_dir_all(&data).expect("data dir");
            std::fs::create_dir_all(&keys).expect("keys dir");
            Self { root, data, keys }
        }
    }

    impl Drop for Dirs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn writes_mode_0400_and_reads_the_same_key_back() {
        let dirs = Dirs::new("roundtrip");
        let path = dirs.keys.join("master.key");
        let key = generate_master_key();

        write_key_file(&path, &dirs.data, &key).expect("writes");

        assert_eq!(
            path.metadata().expect("stat").permissions().mode() & 0o777,
            0o400
        );
        let read = read_key_file(&path, &dirs.data).expect("reads");
        assert_eq!(read.expose(), key.expose());
    }

    #[test]
    fn refuses_a_key_file_inside_the_data_directory() {
        let dirs = Dirs::new("inside");
        let path = dirs.data.join("master.key");

        let error = write_key_file(&path, &dirs.data, &generate_master_key())
            .expect_err("inside the data dir");

        assert!(matches!(error, VaultError::KeyFileInsideDataDir { .. }));
        assert!(
            !path.exists(),
            "nothing may be written before the check passes"
        );
    }

    #[test]
    fn a_symlink_into_the_data_directory_does_not_defeat_the_check() {
        let dirs = Dirs::new("symlink");
        let link = dirs.keys.join("sneaky");
        std::os::unix::fs::symlink(&dirs.data, &link).expect("symlink");

        let error = write_key_file(&link.join("master.key"), &dirs.data, &generate_master_key())
            .expect_err("resolves through the link");

        assert!(matches!(error, VaultError::KeyFileInsideDataDir { .. }));
    }

    #[test]
    fn dot_dot_segments_do_not_defeat_the_check() {
        let dirs = Dirs::new("dotdot");
        let path = dirs.keys.join("..").join("data").join("master.key");

        let error =
            write_key_file(&path, &dirs.data, &generate_master_key()).expect_err("resolves ..");

        assert!(matches!(error, VaultError::KeyFileInsideDataDir { .. }));
    }

    #[test]
    fn rejects_any_mode_other_than_0400() {
        let dirs = Dirs::new("modes");
        let path = dirs.keys.join("master.key");
        write_key_file(&path, &dirs.data, &generate_master_key()).expect("writes");

        for mode in [0o600, 0o440, 0o404, 0o644, 0o200] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("chmod");
            assert!(
                matches!(
                    read_key_file(&path, &dirs.data),
                    Err(VaultError::KeyFilePermissions(_))
                ),
                "mode {mode:o} must be rejected"
            );
        }
    }

    #[test]
    fn will_not_overwrite_an_existing_master_key() {
        let dirs = Dirs::new("overwrite");
        let path = dirs.keys.join("master.key");
        let first = generate_master_key();
        write_key_file(&path, &dirs.data, &first).expect("writes");

        let error =
            write_key_file(&path, &dirs.data, &generate_master_key()).expect_err("exists already");

        assert!(matches!(error, VaultError::KeyFileIo(_)));
        let read = read_key_file(&path, &dirs.data).expect("reads");
        assert_eq!(read.expose(), first.expose());
    }

    #[test]
    fn rejects_a_file_of_the_wrong_length() {
        let dirs = Dirs::new("length");
        let path = dirs.keys.join("master.key");
        std::fs::write(&path, b"too short").expect("writes");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).expect("chmod");

        assert!(matches!(
            read_key_file(&path, &dirs.data),
            Err(VaultError::Malformed(_))
        ));
    }
}
