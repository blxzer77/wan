//! Write-only rule: no public function of wan-vault returns plaintext.
//!
//! This reads the crate's own source, because rustdoc JSON needs a nightly toolchain. It is a
//! regression guard, not a proof: it cannot see through type aliases or generic returns, so
//! `with_secret` (the one sanctioned exit, whose callback chooses what to copy out) is covered
//! by review, not by this test.

use std::fs;
use std::path::Path;

/// Public functions allowed to return text or bytes, with the reason. An entry that the
/// detector no longer flags is stale and fails the test.
const ALLOWED: &[(&str, &str)] = &[("label", "returns a fixed item-type label, not item data")];

/// Functions that must be found, so a broken parser cannot pass by finding nothing.
const EXPECTED: &[&str] = &[
    "seal_item",
    "with_secret",
    "wrap_dek",
    "unwrap_dek",
    "generate_recovery_key",
    "parse_recovery_key",
    "read_key_file",
    "load_master_key",
];

#[derive(Debug)]
struct PublicFn {
    name: String,
    returns: String,
}

fn strip_lifetimes(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            while chars
                .peek()
                .is_some_and(|next| next.is_alphanumeric() || *next == '_')
            {
                chars.next();
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn returns_plaintext(return_type: &str) -> bool {
    let compact: String = strip_lifetimes(return_type)
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    const SHAPES: [&str; 5] = ["Vec<u8>", "&[u8]", "[u8;", "Box<[u8]>", "&str"];
    if SHAPES.iter().any(|shape| compact.contains(shape)) {
        return true;
    }
    // Whole-token match, so `SecretString` is not mistaken for `String`.
    compact
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .any(|token| token == "String")
}

fn parse_signature(signature: &str) -> PublicFn {
    let after_fn = signature
        .split_once("fn ")
        .map_or(signature, |(_, rest)| rest);
    let name = after_fn
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .collect();
    PublicFn {
        name,
        returns: return_type(signature),
    }
}

/// Extracts the function's own return type. Only a `->` outside every `()`, `[]`, and `<>` is
/// the function's: arrows inside parameters belong to closure types, and a `;` inside `[u8; N]`
/// is part of the type, not the end of the declaration.
fn return_type(signature: &str) -> String {
    let chars: Vec<char> = signature.chars().collect();
    let mut depth = 0i32;
    let mut start = None;
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' => depth -= 1,
            // `->` contains `>`; step over it so it never lowers the depth.
            '-' if chars.get(index + 1) == Some(&'>') => {
                if depth == 0 && start.is_none() {
                    start = Some(index + 2);
                }
                index += 1;
            }
            '>' => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    let Some(start) = start else {
        return String::new();
    };

    let mut depth = 0i32;
    let mut text = String::new();
    for &ch in &chars[start..] {
        match ch {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => depth -= 1,
            '{' | ';' if depth <= 0 => break,
            _ => {}
        }
        text.push(ch);
    }
    text.split(" where ").next().unwrap_or("").trim().to_owned()
}

/// Returns the index just past the `{ ... }` block that opens at or after `start`.
///
/// It counts braces per line, so a brace inside a string or comment could skew it. Braces in
/// this crate's test modules come in balanced pairs (`{}`, `{:?}`, `{x}`), so the count holds;
/// `skips_only_the_test_module_and_scans_code_after_it` pins the behavior.
fn end_of_block(lines: &[&str], start: usize) -> usize {
    let mut depth = 0i32;
    let mut opened = false;
    for (offset, line) in lines[start..].iter().enumerate() {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return start + offset + 1;
        }
    }
    lines.len()
}

/// Finds `pub fn` items and methods of `pub trait`s. Test modules are skipped as a block, so
/// code placed after one is still scanned.
fn public_fns(source: &str) -> Vec<PublicFn> {
    let lines: Vec<&str> = source.lines().collect();
    let mut found = Vec::new();
    let mut in_pub_trait = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("#[cfg(") && line.contains("test") {
            index = end_of_block(&lines, index);
            continue;
        }
        if line.starts_with("pub trait ") {
            in_pub_trait = true;
        } else if in_pub_trait && line == "}" {
            in_pub_trait = false;
        }

        let trimmed = line.trim_start();
        let is_public = trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || (in_pub_trait && trimmed.starts_with("fn "));
        if is_public {
            let mut signature = String::new();
            while index < lines.len() {
                let part = lines[index].trim();
                signature.push_str(part);
                signature.push(' ');
                if part.ends_with('{') || part.ends_with(';') {
                    break;
                }
                index += 1;
            }
            found.push(parse_signature(&signature));
        }
        index += 1;
    }
    found
}

fn source_files() -> Vec<std::path::PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files: Vec<_> = fs::read_dir(&src)
        .expect("src directory reads")
        .map(|entry| entry.expect("directory entry reads").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

fn all_public_fns() -> Vec<PublicFn> {
    source_files()
        .iter()
        .flat_map(|path| public_fns(&fs::read_to_string(path).expect("source reads")))
        .collect()
}

#[test]
fn no_public_function_returns_plaintext() {
    let fns = all_public_fns();

    for name in EXPECTED {
        assert!(
            fns.iter().any(|f| f.name == *name),
            "the scan did not find `{name}`; the parser is broken or the function was renamed"
        );
    }

    let offenders: Vec<String> = fns
        .iter()
        .filter(|f| returns_plaintext(&f.returns))
        .filter(|f| !ALLOWED.iter().any(|(name, _)| *name == f.name))
        .map(|f| format!("{} -> {}", f.name, f.returns))
        .collect();
    assert!(
        offenders.is_empty(),
        "public functions must not return plaintext: {offenders:?}"
    );

    for (name, _) in ALLOWED {
        assert!(
            fns.iter()
                .any(|f| f.name == *name && returns_plaintext(&f.returns)),
            "`{name}` is allow-listed but no longer returns text or bytes; remove the entry"
        );
    }
}

#[test]
fn key_types_expose_no_public_field() {
    let source = source_files()
        .iter()
        .map(|path| fs::read_to_string(path).expect("source reads"))
        .collect::<Vec<_>>()
        .join("\n");

    for declaration in [
        "pub struct MasterKey(",
        "pub struct Dek(",
        "pub struct RecoveryKey(",
    ] {
        let line = source
            .lines()
            .find(|line| line.contains(declaration))
            .unwrap_or_else(|| panic!("`{declaration}` was not found"));
        assert!(
            !line.contains("(pub "),
            "`{declaration}` must keep its inner value private: {line}"
        );
    }
}

#[test]
fn detector_flags_the_shapes_it_must_catch() {
    for leaking in [
        "Vec<u8>",
        "Result<Vec<u8>, VaultError>",
        "&[u8]",
        "Zeroizing<Vec<u8>>",
        "[u8; 32]",
        "Option<[u8; 24]>",
        "Box<[u8]>",
        "String",
        "Option<String>",
        "Result<String, VaultError>",
        "&'static str",
        "&'a str",
    ] {
        assert!(returns_plaintext(leaking), "{leaking} must be flagged");
    }
    for fine in [
        "",
        "()",
        "bool",
        "SecretString",
        "Result<Dek, VaultError>",
        "Result<R, VaultError>",
        "Result<WrappedKey, VaultError>",
        "Result<(SecretString, WrappedKey), VaultError>",
        "Option<SecretString>",
        "Result<(), VaultError>",
    ] {
        assert!(!returns_plaintext(fine), "{fine} must not be flagged");
    }
}

#[test]
fn parser_reads_multi_line_signatures_and_ignores_closure_arrows() {
    let source = "\
pub fn leak(
    dek: &Dek,
    item: &SealedItem,
) -> Vec<u8> {
    todo!()
}

pub fn lend<R>(
    use_secret: impl FnOnce(&[u8]) -> R,
) -> Result<R, VaultError> {
    todo!()
}

pub trait Store {
    fn get(&self, account: &str) -> Result<Option<String>, VaultError>;
}

fn private_helper() -> Vec<u8> {
    todo!()
}

#[cfg(test)]
mod tests {
    pub fn not_scanned() -> Vec<u8> {
        todo!()
    }
}
";

    let fns = public_fns(source);

    let seen: Vec<(&str, &str)> = fns
        .iter()
        .map(|f| (f.name.as_str(), f.returns.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("leak", "Vec<u8>"),
            ("lend", "Result<R, VaultError>"),
            ("get", "Result<Option<String>, VaultError>"),
        ]
    );
    assert!(returns_plaintext(&fns[0].returns));
    assert!(!returns_plaintext(&fns[1].returns));
    assert!(returns_plaintext(&fns[2].returns));
}

/// End to end, from a full signature to the verdict. Testing the detector and the parser
/// separately let `-> [u8; 32]` through: the `;` inside the array type cut the return type
/// down to `[u8`, which no rule matched.
#[test]
fn full_signatures_are_judged_end_to_end() {
    let leaking = [
        "pub fn a(dek: &Dek) -> [u8; 32] { ",
        "pub fn b(dek: &Dek) -> Option<[u8; 24]> { ",
        "pub fn c(dek: &Dek) -> Result<[u8; 32], VaultError> { ",
        "pub fn d(cb: impl Fn(&[u8]) -> bool) -> Vec<u8> { ",
        "pub fn e() -> Box<dyn Fn() -> Vec<u8>> { ",
        "pub fn f(dek: &Dek) -> &'static [u8] { ",
        "pub fn g<T: Clone>(t: T) -> String where T: Send { ",
        "pub fn h(x: [u8; 4]) -> Vec<u8>; ",
    ];
    for signature in leaking {
        let parsed = parse_signature(signature);
        assert!(
            returns_plaintext(&parsed.returns),
            "{signature:?} parsed as {:?} and was not flagged",
            parsed.returns
        );
    }

    // A closure arrow in the parameters must not be mistaken for the function's own return.
    let fine = [
        "pub fn p(cb: impl Fn() -> Vec<u8>) { ",
        "pub fn q(cb: impl FnOnce(&[u8]) -> String) -> bool { ",
        "pub fn r(x: [u8; 32]) -> Result<(), VaultError> { ",
        "pub fn s(cb: impl FnOnce(&[u8]) -> R) -> Result<R, VaultError> { ",
        "pub fn t() -> SecretString { ",
        // Not plaintext: the detector names plaintext carriers, it does not judge closures.
        "pub fn u() -> Box<dyn Fn() -> u8> + Send { ",
    ];
    for signature in fine {
        let parsed = parse_signature(signature);
        assert!(
            !returns_plaintext(&parsed.returns),
            "{signature:?} parsed as {:?} and was wrongly flagged",
            parsed.returns
        );
    }
}

/// Regression: the scanner used to stop at the first `#[cfg(test)]`, so a function appended
/// after the test module was never checked.
#[test]
fn skips_only_the_test_module_and_scans_code_after_it() {
    let source = "\
pub fn before() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_debug_output() {
        assert_eq!(format!(\"{:?}\", Dek), \"Dek(..)\");
        let nested = { 1 };
    }

    pub fn hidden() -> Vec<u8> {
        todo!()
    }
}

pub fn after() -> Vec<u8> {
    todo!()
}

#[cfg(all(test, unix))]
mod unix_tests {
    pub fn also_hidden() -> String {
        todo!()
    }
}

pub fn last() -> bool {
    true
}
";

    let names: Vec<String> = public_fns(source).into_iter().map(|f| f.name).collect();

    assert_eq!(names, ["before", "after", "last"]);
}
