//! Shared helpers for the conformance suite.
//!
//! These helpers read fixtures and descriptors. They interpret no graph data.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository root, resolved from the manifest directory rather than the working
/// directory, so the suite behaves the same under `cargo test` and in CI.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

pub fn read_json(relative: &str) -> serde_json::Value {
    let text = read(relative);
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{relative} is not valid JSON: {e}"))
}

/// Files with the given extension in a directory, sorted for deterministic order.
pub fn files_with_extension(relative_dir: &str, extension: &str) -> Vec<PathBuf> {
    let dir = repo_root().join(relative_dir);
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some(extension))
        .collect();
    out.sort();
    assert!(
        !out.is_empty(),
        "no *.{extension} fixtures in {relative_dir}"
    );
    out
}

/// Parses an `.expected` file of `key = value` lines. `#` starts a comment.
pub fn parse_expected(path: &Path) -> BTreeMap<String, String> {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut map = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or_else(|| {
            panic!(
                "{}:{} is not a `key = value` line: {raw}",
                path.display(),
                index + 1
            )
        });
        let previous = map.insert(key.trim().to_string(), value.trim().to_string());
        assert!(
            previous.is_none(),
            "{} repeats key {}",
            path.display(),
            key.trim()
        );
    }
    map
}

/// The `.expected` file that pairs with a fixture.
pub fn expectation_for(fixture: &Path) -> BTreeMap<String, String> {
    let expected = fixture.with_extension("expected");
    assert!(
        expected.exists(),
        "fixture {} has no .expected file",
        fixture.display()
    );
    parse_expected(&expected)
}

/// Decodes the commented hexadecimal fixture format described in
/// docs/NOSTDB_FORMAT.md section 15.
pub fn decode_hex_fixture(path: &Path) -> Vec<u8> {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut digits = String::new();
    for line in text.lines() {
        let payload = line.split('#').next().unwrap_or("");
        for ch in payload.chars() {
            if ch.is_ascii_whitespace() {
                continue;
            }
            assert!(
                ch.is_ascii_hexdigit(),
                "{} contains a non-hexadecimal character {ch:?}",
                path.display()
            );
            digits.push(ch);
        }
    }
    assert!(
        digits.len() % 2 == 0,
        "{} has an odd number of hexadecimal digits",
        path.display()
    );
    digits
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let s = std::str::from_utf8(pair).expect("ascii");
            u8::from_str_radix(s, 16).expect("hexadecimal byte")
        })
        .collect()
}

/// CRC-32C, the Castagnoli polynomial, reflected, as specified in
/// docs/NOSTDB_FORMAT.md section 8.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
