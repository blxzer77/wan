use secrecy::SecretString;
use zeroize::Zeroizing;

use crate::error::VaultError;
use crate::key::{Dek, KEY_LEN, WrapKind, WrappedKey, random_key, unwrap_with, wrap_with};

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
/// 256 bits is 52 Base32 characters (the last one carries 4 bits and 1 zero pad bit).
const ENCODED_LEN: usize = 52;
const GROUP: usize = 4;

/// The 256-bit key that can open the DEK without the master key.
pub struct RecoveryKey([u8; KEY_LEN]);

impl Drop for RecoveryKey {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.0);
    }
}

impl std::fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoveryKey(..)")
    }
}

/// The result of setting recovery up. The display string exists only here: it is returned once
/// and the vault stores nothing but the wrapped DEK.
pub struct RecoverySetup {
    display: SecretString,
    pub wrapped: WrappedKey,
}

impl RecoverySetup {
    /// Hands the recovery key to the user for saving, once. The caller must show it, ask the
    /// user to confirm they saved it, and pass that answer as `saved`. Without confirmation
    /// setup does not complete and the wrapped key is withheld.
    pub fn confirm(self, saved: bool) -> Result<(SecretString, WrappedKey), VaultError> {
        if !saved {
            return Err(VaultError::RecoveryNotConfirmed);
        }
        Ok((self.display, self.wrapped))
    }
}

impl std::fmt::Debug for RecoverySetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RecoverySetup(..)")
    }
}

/// Creates a recovery key and the wrapping of `dek` under it.
pub fn generate_recovery_key(dek: &Dek, key_version: u32) -> Result<RecoverySetup, VaultError> {
    let key = RecoveryKey(random_key());
    let wrapped = wrap_with(&key.0, WrapKind::Recovery, key_version, dek)?;
    Ok(RecoverySetup {
        display: SecretString::from(encode(&key.0)),
        wrapped,
    })
}

/// Parses what the user typed back. Case, spaces, and dashes are ignored.
pub fn parse_recovery_key(text: &str) -> Result<RecoveryKey, VaultError> {
    decode(text).map(RecoveryKey)
}

impl RecoveryKey {
    /// Opens the recovery wrapping of the DEK, for restoring on a new machine.
    pub fn unwrap_dek(&self, wrapped: &WrappedKey) -> Result<Dek, VaultError> {
        if wrapped.kind != WrapKind::Recovery {
            return Err(VaultError::Decrypt);
        }
        unwrap_with(&self.0, wrapped)
    }

    /// Re-wraps the DEK under this recovery key (after DEK rotation a new recovery key is
    /// generated instead; this is for re-binding the same key).
    pub fn wrap_dek(&self, key_version: u32, dek: &Dek) -> Result<WrappedKey, VaultError> {
        wrap_with(&self.0, WrapKind::Recovery, key_version, dek)
    }
}

fn encode(bytes: &[u8; KEY_LEN]) -> String {
    let mut chars = String::with_capacity(ENCODED_LEN);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            chars.push(char::from(ALPHABET[((buffer >> bits) & 31) as usize]));
        }
    }
    if bits > 0 {
        chars.push(char::from(ALPHABET[((buffer << (5 - bits)) & 31) as usize]));
    }

    let mut grouped = String::with_capacity(ENCODED_LEN + ENCODED_LEN / GROUP);
    for (index, ch) in chars.chars().enumerate() {
        if index > 0 && index % GROUP == 0 {
            grouped.push('-');
        }
        grouped.push(ch);
    }
    grouped
}

fn decode(text: &str) -> Result<[u8; KEY_LEN], VaultError> {
    let cleaned = Zeroizing::new(
        text.chars()
            .filter(|ch| !ch.is_whitespace() && *ch != '-')
            .map(|ch| ch.to_ascii_uppercase())
            .collect::<String>(),
    );
    if cleaned.len() != ENCODED_LEN {
        return Err(VaultError::InvalidRecoveryKey);
    }

    let mut bytes = [0u8; KEY_LEN];
    let mut buffer: u32 = 0;
    let mut bits = 0;
    let mut index = 0;
    for ch in cleaned.bytes() {
        let value = ALPHABET
            .iter()
            .position(|candidate| *candidate == ch)
            .ok_or(VaultError::InvalidRecoveryKey)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            if index < KEY_LEN {
                bytes[index] = ((buffer >> bits) & 0xFF) as u8;
                index += 1;
            }
        }
    }
    // The 52nd character holds 4 data bits; the remaining pad bit must be zero.
    if buffer & ((1 << bits) - 1) != 0 {
        return Err(VaultError::InvalidRecoveryKey);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::generate_dek;
    use secrecy::ExposeSecret;

    fn displayed(setup: RecoverySetup) -> (String, WrappedKey) {
        let (display, wrapped) = setup.confirm(true).expect("confirmed");
        (display.expose_secret().to_owned(), wrapped)
    }

    #[test]
    fn recovery_key_is_shown_in_groups_and_opens_the_dek() {
        let dek = generate_dek();

        let (text, wrapped) = displayed(generate_recovery_key(&dek, 1).expect("generates"));

        assert_eq!(text.len(), ENCODED_LEN + ENCODED_LEN / GROUP - 1);
        assert!(text.split('-').all(|group| group.len() <= GROUP));
        let key = parse_recovery_key(&text).expect("parses what was shown");
        assert_eq!(
            key.unwrap_dek(&wrapped).expect("opens").expose(),
            dek.expose()
        );
    }

    #[test]
    fn typing_back_ignores_case_spaces_and_dashes() {
        let dek = generate_dek();
        let (text, wrapped) = displayed(generate_recovery_key(&dek, 1).expect("generates"));
        let sloppy = text.to_lowercase().replace('-', " ");

        let key = parse_recovery_key(&sloppy).expect("parses");

        assert!(key.unwrap_dek(&wrapped).is_ok());
    }

    #[test]
    fn setup_withholds_everything_until_the_user_confirms() {
        let setup = generate_recovery_key(&generate_dek(), 1).expect("generates");

        assert!(matches!(
            setup.confirm(false),
            Err(VaultError::RecoveryNotConfirmed)
        ));
    }

    #[test]
    fn a_different_recovery_key_cannot_open_the_dek() {
        let dek = generate_dek();
        let (_, wrapped) = displayed(generate_recovery_key(&dek, 1).expect("generates"));
        let (other_text, _) = displayed(generate_recovery_key(&dek, 1).expect("generates"));
        let other = parse_recovery_key(&other_text).expect("parses");

        assert!(matches!(
            other.unwrap_dek(&wrapped),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn a_master_wrapping_is_not_accepted_as_a_recovery_wrapping() {
        use crate::key::{generate_master_key, wrap_dek};
        let dek = generate_dek();
        let (text, _) = displayed(generate_recovery_key(&dek, 1).expect("generates"));
        let key = parse_recovery_key(&text).expect("parses");
        let master_wrapped = wrap_dek(&generate_master_key(), 1, &dek).expect("wraps");

        assert!(matches!(
            key.unwrap_dek(&master_wrapped),
            Err(VaultError::Decrypt)
        ));
    }

    #[test]
    fn malformed_input_is_rejected() {
        for bad in [
            "",
            "ABCD",
            &"A".repeat(51),
            &"A".repeat(53),
            &format!("{}1", "A".repeat(51)),
            &format!("{}B", "A".repeat(51)),
        ] {
            assert!(
                matches!(parse_recovery_key(bad), Err(VaultError::InvalidRecoveryKey)),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn encode_and_decode_are_inverse_for_known_vectors() {
        let zeros = [0u8; KEY_LEN];
        let ones = [0xFFu8; KEY_LEN];

        assert_eq!(decode(&encode(&zeros)).expect("decodes"), zeros);
        assert_eq!(decode(&encode(&ones)).expect("decodes"), ones);
        assert!(encode(&zeros).starts_with("AAAA-AAAA"));
    }

    #[test]
    fn debug_output_never_shows_key_material() {
        let setup = generate_recovery_key(&generate_dek(), 1).expect("generates");
        assert_eq!(format!("{setup:?}"), "RecoverySetup(..)");
        let (text, _) = displayed(setup);
        let key = parse_recovery_key(&text).expect("parses");
        assert_eq!(format!("{key:?}"), "RecoveryKey(..)");
    }
}
