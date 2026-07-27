//! The catalog fixture suite is internally consistent with the contract.
//!
//! This repository owns no runtime, so it cannot read a catalog the way the daemon does.
//! What it can check is that every fixture is well formed, declares an outcome its directory
//! allows, names a registered code, and that every rejection rule the contract states has a
//! fixture an implementation can fail against.
//!
//! `nostdb-server` proves the behavior, reading this same suite from the superproject.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Every rejection the contract's section 6 table states, paired with the fixture that
/// exercises it. Adding a row to that table without a fixture fails the last test here.
const REJECTION_RULES: [&str; 11] = [
    "missing_version",
    "unsupported_version",
    "not_an_object",
    "databases_absent",
    "databases_not_an_object",
    "name_with_path_separator",
    "name_with_sigil",
    "entry_not_an_object",
    "path_absent",
    "relative_path",
    "empty_path",
];

fn stems(directory: &str, extension: &str) -> BTreeSet<String> {
    common::files_with_extension(directory, extension)
        .iter()
        .map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix(&format!(".{extension}")))
                .expect("fixture name")
                .to_owned()
        })
        .collect()
}

fn expectation(path: &Path) -> BTreeMap<String, String> {
    common::parse_expected(&path.with_extension("expected"))
}

fn registered_codes() -> BTreeSet<String> {
    common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code string").to_owned())
        .collect()
}

fn document(path: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(path).expect("fixture is UTF-8");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for directory in ["valid", "invalid"] {
        let relative = format!("fixtures/catalog/{directory}");
        assert_eq!(
            stems(&relative, "json"),
            stems(&relative, "expected"),
            "{directory} has a fixture without an expectation, or the reverse"
        );
    }
}

#[test]
fn accepted_fixtures_are_objects_carrying_a_supported_version() {
    let supported: BTreeSet<u64> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|contract| contract["key"] == "catalog_version")
        .and_then(|contract| contract["supported"].as_array())
        .expect("catalog_version is registered")
        .iter()
        .map(|value| value.as_u64().expect("a version is a number"))
        .collect();

    for path in common::files_with_extension("fixtures/catalog/valid", "json") {
        let name = path.display();
        assert_eq!(
            expectation(&path).get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );

        let object = document(&path);
        let object = object
            .as_object()
            .unwrap_or_else(|| panic!("{name}: a catalog is a JSON object"));

        let version = object
            .get("catalog_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("{name}: catalog_version must be a positive integer"));
        assert!(
            supported.contains(&version),
            "{name}: version {version} is not supported, so this belongs in invalid/"
        );

        let databases = object
            .get("databases")
            .and_then(serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("{name}: databases must be an object"));

        for (catalog_name, entry) in databases {
            assert!(
                is_a_valid_name(catalog_name),
                "{name}: {catalog_name} is not a valid name, so this belongs in invalid/"
            );
            let path_value = entry
                .as_object()
                .and_then(|e| e.get("path"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("{name}: {catalog_name} has no string path"));
            assert!(
                path_value.starts_with('/'),
                "{name}: {catalog_name} must state an absolute path, found {path_value}"
            );
        }
    }
    println!("catalog conformance: accepted fixtures verified");
}

#[test]
fn rejected_fixtures_declare_a_registered_catalog_code() {
    let registered = registered_codes();
    for path in common::files_with_extension("fixtures/catalog/invalid", "json") {
        let name = path.display();
        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("reject"),
            "{name} must declare outcome = reject"
        );

        let code = expected
            .get("code")
            .unwrap_or_else(|| panic!("{name} must declare a code"));
        assert!(
            registered.contains(code),
            "{name} declares unregistered code {code}"
        );
        assert!(
            code.starts_with("CATALOG_"),
            "{name} declares {code}, which the catalog contract does not own"
        );

        // A rejected document must still be readable JSON. A fixture that is not even
        // parseable would prove an implementation refuses malformed JSON, which every JSON
        // reader already does, rather than proving a catalog rule.
        document(&path);
    }
    println!("catalog conformance: rejected fixtures verified");
}

/// The two codes the contract owns are each reachable from a fixture.
///
/// Registering a code no fixture declares is how a code becomes documentation. The version
/// refusal is the one most easily left out, because every other rule reports the same code.
#[test]
fn both_catalog_codes_are_reachable() {
    let declared: BTreeSet<String> =
        common::files_with_extension("fixtures/catalog/invalid", "json")
            .iter()
            .filter_map(|path| expectation(path).get("code").cloned())
            .collect();

    for code in ["CATALOG_INVALID", "CATALOG_VERSION_UNSUPPORTED"] {
        assert!(
            declared.contains(code),
            "no catalog fixture declares {code}"
        );
    }
}

#[test]
fn every_rejection_rule_in_the_contract_has_a_fixture() {
    let present = stems("fixtures/catalog/invalid", "json");
    let missing: Vec<&str> = REJECTION_RULES
        .iter()
        .copied()
        .filter(|rule| !present.contains(*rule))
        .collect();
    assert!(
        missing.is_empty(),
        "these rejection rules have no fixture: {missing:?}"
    );

    // And no fixture exercises a rule the contract does not state, which would be a test
    // asserting behavior nothing published requires.
    let known: BTreeSet<&str> = REJECTION_RULES.iter().copied().collect();
    let extra: Vec<&String> = present
        .iter()
        .filter(|stem| !known.contains(stem.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "these fixtures exercise no stated rejection rule: {extra:?}"
    );
    println!(
        "catalog conformance: {} rejection rules verified",
        REJECTION_RULES.len()
    );
}

/// The name form in section 3.2: `[A-Za-z0-9][A-Za-z0-9_-]*`.
fn is_a_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
