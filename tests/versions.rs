//! The contract version registry is internally consistent, and every contract
//! marked `specified` names a document that exists.

mod common;

use std::collections::BTreeSet;

#[test]
fn the_registry_is_well_formed() {
    let registry = common::read_json("versions.json");
    assert_eq!(registry["registry_version"], 1);

    let contracts = registry["contracts"].as_array().expect("contracts array");
    assert!(!contracts.is_empty(), "the registry declares no contracts");

    let mut seen: BTreeSet<String> = BTreeSet::new();
    for contract in contracts {
        let key = contract["key"]
            .as_str()
            .expect("key is a string")
            .to_string();
        assert!(seen.insert(key.clone()), "duplicate contract key {key}");
        assert!(
            key.ends_with("_version"),
            "contract key {key} must end with _version"
        );

        let current = contract["current"].as_u64().expect("current is a number");
        assert!(current >= 1, "{key}: current must be at least 1");

        let supported: Vec<u64> = contract["supported"]
            .as_array()
            .expect("supported array")
            .iter()
            .map(|v| v.as_u64().expect("supported entry is a number"))
            .collect();
        assert!(!supported.is_empty(), "{key}: supported must not be empty");
        assert!(
            supported.contains(&current),
            "{key}: current {current} is not in supported {supported:?}"
        );

        let summary = contract["summary"].as_str().expect("summary is a string");
        assert!(!summary.trim().is_empty(), "{key}: summary is empty");

        let status = contract["status"].as_str().expect("status is a string");
        match status {
            "specified" => {
                let path = contract["specified_in"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{key}: a specified contract needs specified_in"));
                assert!(
                    common::repo_root().join(path).is_file(),
                    "{key}: specified_in names a missing file {path}"
                );
            }
            "deferred" => assert!(
                contract["specified_in"].is_null(),
                "{key}: a deferred contract must have specified_in null"
            ),
            other => panic!("{key}: unknown status {other}"),
        }
    }
}

#[test]
fn the_specified_contracts_are_exactly_those_that_have_been_authored() {
    let registry = common::read_json("versions.json");
    let specified: BTreeSet<String> = registry["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .filter(|c| c["status"] == "specified")
        .map(|c| c["key"].as_str().expect("key").to_string())
        .collect();

    // This is a deliberate tripwire. A new specified contract is a significant event, so
    // adding one must fail this test until the expectation is updated on purpose.
    let expected: BTreeSet<String> = [
        "nost_language_version",
        "nostdb_format_version",
        "query_subset_version",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    assert_eq!(
        specified, expected,
        "specifying another contract means updating this expectation and the Stage record"
    );
}

#[test]
fn the_human_table_agrees_with_the_machine_registry() {
    let registry = common::read_json("versions.json");
    let document = common::read("VERSIONS.md");

    for contract in registry["contracts"].as_array().expect("contracts array") {
        let key = contract["key"].as_str().expect("key");
        let status = contract["status"].as_str().expect("status");
        let current = contract["current"].as_u64().expect("current");

        let row = document
            .lines()
            .find(|line| line.starts_with("| `") && line.contains(&format!("`{key}`")))
            .unwrap_or_else(|| panic!("VERSIONS.md has no table row for {key}"));

        assert!(
            row.contains(status),
            "VERSIONS.md row for {key} does not state status {status}: {row}"
        );
        assert!(
            row.contains(&format!("| {current} |")),
            "VERSIONS.md row for {key} does not state current version {current}: {row}"
        );
    }
}
