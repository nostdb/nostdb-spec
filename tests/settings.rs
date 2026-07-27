//! The settings fixture suite is internally consistent with the contract.
//!
//! This repository owns no runtime, so it cannot read a settings document the way an
//! implementation does. What it can check is that every fixture is well formed, declares
//! an outcome its directory allows, names a registered code, and that every rejection
//! rule the contract states has a fixture an implementation can fail against.
//!
//! `nostdb-core` proves the behavior, reading this same suite from the superproject.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

/// Every rejection the contract's section 6 table states, paired with the fixture that
/// exercises it. Adding a row to that table without a fixture fails the last test here.
const REJECTION_RULES: [&str; 8] = [
    "missing_version",
    "not_an_object",
    "absolute_database_path",
    "link_with_alias",
    "link_without_source",
    "duplicate_link_source",
    "negative_token_budget",
    "wrong_field_type",
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

fn expectation(path: &Path) -> std::collections::BTreeMap<String, String> {
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

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for directory in ["valid", "invalid"] {
        let relative = format!("fixtures/settings/{directory}");
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
        .find(|contract| contract["key"] == "settings_version")
        .and_then(|contract| contract["supported"].as_array())
        .expect("settings_version is registered")
        .iter()
        .map(|value| value.as_u64().expect("a version is a number"))
        .collect();

    for path in common::files_with_extension("fixtures/settings/valid", "json") {
        let name = path.display();
        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );

        let text = std::fs::read_to_string(&path).expect("fixture is UTF-8");
        let document: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|error| panic!("{name}: {error}"));
        let object = document
            .as_object()
            .unwrap_or_else(|| panic!("{name}: a settings document is a JSON object"));
        let version = object
            .get("settings_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("{name}: settings_version must be a positive integer"));
        assert!(
            supported.contains(&version),
            "{name}: version {version} is not supported, so this belongs in invalid/"
        );
    }
    println!("settings conformance: accepted fixtures verified");
}

#[test]
fn rejected_fixtures_declare_a_registered_settings_code() {
    let registered = registered_codes();
    for path in common::files_with_extension("fixtures/settings/invalid", "json") {
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
            code.starts_with("SETTINGS_"),
            "{name} declares {code}, which the settings contract does not own"
        );

        // A rejected document must still be readable JSON. A fixture that is not even
        // parseable would prove an implementation refuses malformed JSON, which every
        // JSON reader already does, rather than proving a settings rule.
        let text = std::fs::read_to_string(&path).expect("fixture is UTF-8");
        serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|error| panic!("{name} must be valid JSON: {error}"));
    }
    println!("settings conformance: rejected fixtures verified");
}

#[test]
fn merge_fixtures_carry_a_global_a_project_and_a_result() {
    let mut count = 0_usize;
    for path in common::files_with_extension("fixtures/settings/merge", "expected") {
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .expect("fixture name")
            .to_owned();
        let expected = common::parse_expected(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("merge"),
            "{stem} must declare outcome = merge"
        );

        let base = path.parent().expect("fixture directory");
        for suffix in ["global.json", "project.json", "expected.json"] {
            let member = base.join(format!("{stem}.{suffix}"));
            assert!(
                member.is_file(),
                "{stem} is missing its {suffix}: {}",
                member.display()
            );
            let text = std::fs::read_to_string(&member).expect("fixture is UTF-8");
            let document: serde_json::Value = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("{}: {error}", member.display()));
            assert!(
                document.get("settings_version").is_some(),
                "{}: every merge member states its version",
                member.display()
            );
        }
        count += 1;
    }
    assert!(count > 0, "the merge suite is empty");
    println!("settings conformance: {count} merge fixtures verified");
}

#[test]
fn every_rejection_rule_in_the_contract_has_a_fixture() {
    // The contract lists what an implementation must reject rather than repair. A rule
    // with no fixture is prose an implementation can ignore.
    let present = stems("fixtures/settings/invalid", "json");
    let missing: Vec<&str> = REJECTION_RULES
        .iter()
        .copied()
        .filter(|rule| !present.contains(*rule))
        .collect();
    assert!(
        missing.is_empty(),
        "these rejection rules have no fixture: {missing:?}"
    );

    // And every settings diagnostic the registry carries is reachable, except the one a
    // single document cannot express: an orphan entry is a disagreement between a
    // settings file and a database, so it needs a pair this suite does not define.
    let declared: BTreeSet<String> =
        common::files_with_extension("fixtures/settings/invalid", "json")
            .iter()
            .filter_map(|path| expectation(path).get("code").cloned())
            .collect();
    let owned: BTreeSet<String> = common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .filter(|entry| entry["contract"] == "settings_version")
        .map(|entry| entry["code"].as_str().expect("code string").to_owned())
        .filter(|code| code != "ORPHAN_LINK_SETTINGS")
        .collect();
    let uncovered: Vec<&String> = owned.difference(&declared).collect();
    assert!(
        uncovered.is_empty(),
        "these settings diagnostics have no fixture: {uncovered:?}"
    );
}
