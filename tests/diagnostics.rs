//! The diagnostic registry is internally consistent, and every registered code
//! is documented in the contract that owns it.

mod common;

use std::collections::{BTreeMap, BTreeSet};

#[test]
fn the_registry_is_well_formed() {
    let registry = common::read_json("diagnostics.json");
    assert_eq!(registry["registry_version"], 1);

    let known_contracts: BTreeSet<String> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .map(|c| c["key"].as_str().expect("key").to_string())
        .collect();

    let codes = registry["codes"].as_array().expect("codes array");
    assert!(!codes.is_empty(), "the registry declares no codes");

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for entry in codes {
        let code = entry["code"]
            .as_str()
            .expect("code is a string")
            .to_string();
        assert!(
            seen.insert(code.clone()),
            "duplicate diagnostic code {code}"
        );
        assert!(
            code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "{code} must be upper snake case, because codes are stable identifiers"
        );

        let severity = entry["severity"].as_str().expect("severity is a string");
        assert!(
            matches!(severity, "error" | "warning"),
            "{code}: unknown severity {severity}"
        );

        let contract = entry["contract"].as_str().expect("contract is a string");
        assert!(
            known_contracts.contains(contract),
            "{code}: contract {contract} is not in the version registry"
        );

        assert!(
            entry["prd_required"].is_boolean(),
            "{code}: prd_required must be a boolean"
        );

        let summary = entry["summary"].as_str().expect("summary is a string");
        assert!(!summary.trim().is_empty(), "{code}: summary is empty");
    }
}

#[test]
fn every_code_is_documented_in_its_owning_contract() {
    let owning_document: BTreeMap<String, String> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .filter_map(|c| {
            let key = c["key"].as_str()?.to_string();
            let path = c["specified_in"].as_str()?.to_string();
            Some((key, path))
        })
        .collect();

    let mut cache: BTreeMap<String, String> = BTreeMap::new();

    for entry in common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
    {
        let code = entry["code"].as_str().expect("code");
        let contract = entry["contract"].as_str().expect("contract");

        let document = owning_document.get(contract).unwrap_or_else(|| {
            panic!("{code} belongs to {contract}, which has no specified document yet")
        });
        let text = cache
            .entry(document.clone())
            .or_insert_with(|| common::read(document));

        assert!(
            text.contains(code),
            "{code} is registered but never mentioned in {document}"
        );
    }
}

#[test]
fn every_code_a_fixture_declares_is_registered() {
    let registered: BTreeSet<String> = common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|e| e["code"].as_str().expect("code").to_string())
        .collect();

    let fixture_dirs = [
        "fixtures/nost/valid",
        "fixtures/nost/invalid-syntax",
        "fixtures/nost/invalid-semantic",
        "fixtures/nostdb/header",
        "fixtures/cypher/supported",
        "fixtures/cypher/unsupported",
        "fixtures/cypher/semantic",
    ];

    let mut declared = 0usize;
    for dir in fixture_dirs {
        for path in common::files_with_extension(dir, "expected") {
            if let Some(code) = common::parse_expected(&path).get("code") {
                assert!(
                    registered.contains(code),
                    "{} declares unregistered code {code}",
                    path.display()
                );
                declared += 1;
            }
        }
    }
    assert!(declared > 0, "no fixture declares a diagnostic code");
}

/// Every code-shaped name a specified contract mentions is registered.
///
/// The reverse direction was already checked, and it missed the case that actually
/// happened: the query subset contract named `LINKED_DATABASE_READ_ONLY` while the
/// registry did not carry it, so a published contract promised a code no implementation
/// could look up. Nothing detected that, because a document may legitimately mention a
/// registered code without the registry mentioning the document.
///
/// A code-shaped name is a backticked token of upper-case letters, digits, and at least
/// one underscore. Every such token across the contracts is a diagnostic code, so the
/// pattern is exact rather than approximate. A future contract that wants an upper-snake
/// name for something else has to register it or spell it differently, which is the point.
#[test]
fn every_code_shaped_name_in_a_contract_is_registered() {
    let registered: BTreeSet<String> = common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|e| e["code"].as_str().expect("code").to_string())
        .collect();

    let documents: Vec<String> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .filter_map(|c| c["specified_in"].as_str().map(String::from))
        .collect();
    assert!(!documents.is_empty(), "no contract names a document");

    let mut mentioned = 0usize;
    for document in &documents {
        for token in code_shaped_names(&common::read(document)) {
            assert!(
                registered.contains(&token),
                "{document} mentions `{token}`, which the registry does not carry"
            );
            mentioned += 1;
        }
    }
    assert!(
        mentioned > 0,
        "no contract mentions a diagnostic code, so this check proves nothing"
    );
}

/// Backticked tokens of upper-case letters, digits, and at least one underscore.
fn code_shaped_names(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for candidate in text.split('`').skip(1).step_by(2) {
        let mut groups = candidate.split('_');
        let shaped = groups.clone().count() >= 2
            && groups.all(|group| {
                !group.is_empty()
                    && group
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            })
            && candidate.starts_with(|c: char| c.is_ascii_uppercase());
        if shaped {
            found.insert(candidate.to_owned());
        }
    }
    found
}
