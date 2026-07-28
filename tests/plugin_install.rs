//! The plugin installation fixture suite is internally consistent with the contract.
//!
//! This repository owns no runtime, so it cannot install anything — and here that is not only
//! the usual boundary. A suite that installed a plugin to test installation would be executing
//! the thing the contract exists to keep from executing.
//!
//! What it can check is that every fixture is well formed, declares an outcome its directory
//! allows, names a registered code the installation contract owns, and that every rule and every
//! limit the contract states has a fixture an implementation can fail against.
//!
//! `nostdb-cli` proves the behavior, reading this same suite from the superproject.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Every rejection the record table in section 9.1 states, paired with the fixture that
/// exercises it. Adding a row to that table without a fixture fails a test here.
const RECORD_RULES: [&str; 10] = [
    "version_absent",
    "version_unsupported",
    "installed_is_not_an_array",
    "entry_has_no_tree_digest",
    "two_entries_share_a_name",
    "entries_are_out_of_name_order",
    "digest_has_no_algorithm",
    "commit_is_a_ref",
    "scope_is_unknown",
    "approved_database_write",
];

/// The five limits section 4 states, each paired with the fixture that exceeds it and the
/// fixture that sits exactly on it.
///
/// Both halves are required. A suite with only the exceeding half would pass against a build
/// whose limit was far lower than the contract's, because a tree over 4096 entries is also over
/// 64 — and the numbers would be advisory while appearing to be checked.
const LIMITS: [(&str, &str); 5] = [
    ("there_are_too_many_entries", "exactly_the_entry_limit"),
    ("an_entry_is_too_large", "exactly_the_entry_byte_limit"),
    ("the_plugin_is_too_large", "exactly_the_total_byte_limit"),
    ("a_path_is_too_deep", "exactly_the_path_depth_limit"),
    ("a_path_is_too_long", "exactly_the_path_length_limit"),
];

/// The members every record entry states. `subdirectory` is the one that may be absent.
const ENTRY_MEMBERS: [&str; 9] = [
    "name",
    "repository",
    "commit",
    "manifest_digest",
    "tree_digest",
    "scope",
    "manifest_version",
    "plugin_version",
    "approved_permissions",
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

fn is_a_digest(text: &str) -> bool {
    match text.strip_prefix("sha256:") {
        Some(hex) => {
            hex.len() == 64
                && hex
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        }
        None => false,
    }
}

fn is_a_commit(text: &str) -> bool {
    text.len() == 40
        && text
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for directory in [
        "record/valid",
        "record/invalid",
        "range",
        "range-invalid",
        "tree",
    ] {
        let relative = format!("fixtures/plugin-install/{directory}");
        assert_eq!(
            stems(&relative, "json"),
            stems(&relative, "expected"),
            "{directory} has a fixture without an expectation, or the reverse"
        );
    }
}

#[test]
fn accepted_records_obey_every_rule_the_contract_states() {
    let supported: BTreeSet<u64> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|contract| contract["key"] == "plugin_install_version")
        .and_then(|contract| contract["supported"].as_array())
        .expect("plugin_install_version is registered")
        .iter()
        .map(|value| value.as_u64().expect("a version is a number"))
        .collect();

    let mut checked = 0usize;
    for path in common::files_with_extension("fixtures/plugin-install/record/valid", "json") {
        let name = path.display();
        assert_eq!(
            expectation(&path).get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );

        let record = document(&path);
        let record = record
            .as_object()
            .unwrap_or_else(|| panic!("{name}: a record is a JSON object"));

        let version = record
            .get("plugin_install_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("{name}: plugin_install_version must be a positive integer"));
        assert!(
            supported.contains(&version),
            "{name}: version {version} is not supported, so this belongs in record/invalid/"
        );

        let installed = record
            .get("installed")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{name}: installed must be an array"));

        let mut previous: Option<&str> = None;
        for entry in installed {
            let entry = entry
                .as_object()
                .unwrap_or_else(|| panic!("{name}: an entry is an object"));

            for member in ENTRY_MEMBERS {
                assert!(
                    entry.contains_key(member),
                    "{name}: an entry has no {member}, so this belongs in record/invalid/"
                );
            }

            let plugin = entry["name"].as_str().expect("name is a string");
            if let Some(previous) = previous {
                assert!(
                    previous < plugin,
                    "{name}: {plugin} follows {previous}, which is not ascending order"
                );
            }
            previous = Some(plugin);

            for digest in ["manifest_digest", "tree_digest"] {
                let value = entry[digest].as_str().expect("a digest is a string");
                assert!(
                    is_a_digest(value),
                    "{name}: {digest} is {value}, which is not sha256 and 64 lower-case hex"
                );
            }

            let commit = entry["commit"].as_str().expect("commit is a string");
            assert!(
                is_a_commit(commit),
                "{name}: commit is {commit}, which is not 40 lower-case hex characters"
            );

            let scope = entry["scope"].as_str().expect("scope is a string");
            assert!(
                matches!(scope, "project" | "global"),
                "{name}: scope is {scope}"
            );
            // A record's file decides its scope, so a fixture asserting the scope it carries is
            // asserting where the file would live. The expectation states it when it is not the
            // project default, which keeps that readable rather than implied.
            if let Some(declared) = expectation(&path).get("scope") {
                assert_eq!(
                    declared, scope,
                    "{name}: the expectation and the entry disagree"
                );
            }

            let repository = entry["repository"]
                .as_str()
                .expect("repository is a string");
            assert!(
                repository.starts_with("https://github.com/")
                    && !repository.contains('?')
                    && !repository.contains('#'),
                "{name}: repository is {repository}, which carries a ref or a fragment"
            );

            let permissions = entry["approved_permissions"]
                .as_object()
                .unwrap_or_else(|| panic!("{name}: approved_permissions is an object"));
            assert_eq!(
                permissions
                    .get("database_write")
                    .and_then(serde_json::Value::as_bool),
                Some(false),
                "{name}: an approved record may never grant database_write"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no accepted record declares an installation");
    println!("plugin install conformance: {checked} accepted installations verified");
}

#[test]
fn rejected_records_declare_a_registered_record_code() {
    let registered = registered_codes();
    for path in common::files_with_extension("fixtures/plugin-install/record/invalid", "json") {
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
            code.starts_with("PLUGIN_RECORD_"),
            "{name} declares {code}, which is not a record refusal"
        );

        // A rejected record must still be readable JSON. A fixture that is not parseable would
        // prove an implementation refuses malformed JSON, which every JSON reader already does,
        // rather than proving a rule this contract states.
        document(&path);
    }
    println!(
        "plugin install conformance: {} rejected records verified",
        stems("fixtures/plugin-install/record/invalid", "json").len()
    );
}

#[test]
fn both_record_codes_are_reachable() {
    let declared: BTreeSet<String> =
        common::files_with_extension("fixtures/plugin-install/record/invalid", "json")
            .iter()
            .filter_map(|path| expectation(path).get("code").cloned())
            .collect();

    for code in ["PLUGIN_RECORD_INVALID", "PLUGIN_RECORD_VERSION_UNSUPPORTED"] {
        assert!(declared.contains(code), "no fixture declares {code}");
    }
}

#[test]
fn every_record_rule_in_the_contract_has_a_fixture() {
    let present = stems("fixtures/plugin-install/record/invalid", "json");
    let missing: Vec<&str> = RECORD_RULES
        .iter()
        .copied()
        .filter(|rule| !present.contains(*rule))
        .collect();
    assert!(
        missing.is_empty(),
        "these record rules have no fixture: {missing:?}"
    );

    let known: BTreeSet<&str> = RECORD_RULES.iter().copied().collect();
    let extra: Vec<&String> = present
        .iter()
        .filter(|stem| !known.contains(stem.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "these fixtures exercise a rule the contract does not state: {extra:?}"
    );
}

#[test]
fn every_range_fixture_declares_a_range_and_an_engine() {
    let mut admitted = 0usize;
    let mut excluded = 0usize;
    for path in common::files_with_extension("fixtures/plugin-install/range", "json") {
        let name = path.display();
        let fixture = document(&path);
        let range = fixture["range"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: range is a string"));
        assert!(
            !range.is_empty(),
            "{name}: an empty range belongs in range-invalid/"
        );
        fixture["engine"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: engine is a string"));

        match expectation(&path).get("outcome").map(String::as_str) {
            Some("admit") => admitted += 1,
            Some("exclude") => excluded += 1,
            other => panic!("{name}: outcome is {other:?}, expected admit or exclude"),
        }
    }
    // Both outcomes, because a suite of only one of them proves half a comparator.
    assert!(admitted > 0 && excluded > 0, "the range suite is one-sided");
    println!(
        "plugin install conformance: {admitted} admitted and {excluded} excluded ranges verified"
    );
}

#[test]
fn an_unparseable_range_is_a_manifest_refusal() {
    let registered = registered_codes();
    for path in common::files_with_extension("fixtures/plugin-install/range-invalid", "json") {
        let name = path.display();
        let fixture = document(&path);
        assert!(
            fixture
                .get("range")
                .is_some_and(serde_json::Value::is_string),
            "{name}: range is a string"
        );
        assert!(
            fixture.get("engine").is_none(),
            "{name}: a range that does not parse is refused without an engine to compare against"
        );

        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("reject"),
            "{name} must declare outcome = reject"
        );
        // A malformed range is a malformed manifest member, not an incompatibility. The
        // distinction is the whole reason two codes exist: one says fix the manifest, the other
        // says this build is not the one the plugin is for.
        let code = expected
            .get("code")
            .unwrap_or_else(|| panic!("{name} must declare a code"));
        assert_eq!(
            code, "PLUGIN_MANIFEST_INVALID",
            "{name}: a range that does not parse is a manifest refusal"
        );
        assert!(
            registered.contains(code),
            "{name} declares unregistered code {code}"
        );
    }
    println!(
        "plugin install conformance: {} unparseable ranges verified",
        stems("fixtures/plugin-install/range-invalid", "json").len()
    );
}

/// The number of entries a tree fixture places inside the plugin.
///
/// This is the fixture's own arithmetic, not an implementation's: it expands `repeat` and
/// applies the subdirectory narrowing that section 3.1 states, so an accepted fixture's declared
/// `accepted_entries` is checked rather than trusted.
fn accepted_entry_count(fixture: &serde_json::Value) -> usize {
    let prefix = fixture["subdirectory"]
        .as_str()
        .map(|sub| format!("{}/", sub.trim_end_matches('/')));
    fixture["entries"]
        .as_array()
        .expect("entries array")
        .iter()
        .filter(|entry| {
            let path = entry["path"].as_str().expect("path is a string");
            match &prefix {
                Some(prefix) => path.starts_with(prefix.as_str()),
                None => true,
            }
        })
        .map(|entry| {
            entry["repeat"]
                .as_u64()
                .map_or(1, |count| usize::try_from(count).expect("a count fits"))
        })
        .sum()
}

#[test]
fn every_tree_fixture_declares_a_decidable_outcome() {
    let registered = registered_codes();
    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for path in common::files_with_extension("fixtures/plugin-install/tree", "json") {
        let name = path.display();
        let fixture = document(&path);
        let entries = fixture["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("{name}: entries is an array"));
        assert!(
            !entries.is_empty(),
            "{name}: a tree with no entries decides nothing"
        );

        for entry in entries {
            entry["path"]
                .as_str()
                .unwrap_or_else(|| panic!("{name}: a path is a string"));
            assert!(
                entry["bytes"].is_u64(),
                "{name}: an entry states a byte count, because every rule is decidable from an enumeration"
            );
        }

        let expected = expectation(&path);
        match expected.get("outcome").map(String::as_str) {
            Some("accept") => {
                let declared: usize = expected
                    .get("accepted_entries")
                    .unwrap_or_else(|| panic!("{name}: an accepted tree declares accepted_entries"))
                    .parse()
                    .unwrap_or_else(|error| {
                        panic!("{name}: accepted_entries is not a number: {error}")
                    });
                assert_eq!(
                    declared,
                    accepted_entry_count(&fixture),
                    "{name}: the declared count and the fixture's own entries disagree"
                );
                accepted += 1;
            }
            Some("reject") => {
                let code = expected
                    .get("code")
                    .unwrap_or_else(|| panic!("{name} must declare a code"));
                assert!(
                    matches!(
                        code.as_str(),
                        "PLUGIN_SOURCE_INVALID" | "PLUGIN_LIMIT_EXCEEDED"
                    ),
                    "{name} declares {code}, which is not a tree refusal"
                );
                assert!(
                    registered.contains(code),
                    "{name} declares unregistered code {code}"
                );
                rejected += 1;
            }
            other => panic!("{name}: outcome is {other:?}, expected accept or reject"),
        }
    }
    assert!(accepted > 0 && rejected > 0, "the tree suite is one-sided");
    println!(
        "plugin install conformance: {accepted} accepted and {rejected} rejected trees verified"
    );
}

#[test]
fn every_limit_has_a_fixture_over_it_and_a_fixture_exactly_on_it() {
    let present = stems("fixtures/plugin-install/tree", "json");
    for (over, exactly) in LIMITS {
        assert!(
            present.contains(over),
            "no fixture exceeds the limit: {over}"
        );
        assert!(
            present.contains(exactly),
            "no fixture sits exactly on the limit: {exactly}"
        );
        assert_eq!(
            expectation(
                &common::repo_root().join(format!("fixtures/plugin-install/tree/{over}.json"))
            )
            .get("outcome")
            .map(String::as_str),
            Some("reject"),
            "{over} must be rejected"
        );
        assert_eq!(
            expectation(
                &common::repo_root().join(format!("fixtures/plugin-install/tree/{exactly}.json"))
            )
            .get("outcome")
            .map(String::as_str),
            Some("accept"),
            "{exactly} must be accepted"
        );
    }
    println!(
        "plugin install conformance: {} limits pinned from both sides",
        LIMITS.len()
    );
}

/// Both tree codes are reachable, and the source refusals cover more than one condition.
///
/// A single fixture declaring `PLUGIN_SOURCE_INVALID` would make the code reachable while
/// leaving most of section 3 unexercised, and the table there states six separate rules.
#[test]
fn both_tree_codes_are_reachable() {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in common::files_with_extension("fixtures/plugin-install/tree", "json") {
        if let Some(code) = expectation(&path).get("code") {
            *counts.entry(code.clone()).or_default() += 1;
        }
    }
    for code in ["PLUGIN_SOURCE_INVALID", "PLUGIN_LIMIT_EXCEEDED"] {
        let found = counts.get(code).copied().unwrap_or_default();
        assert!(
            found > 1,
            "{code} has {found} fixtures, which does not cover its rules"
        );
    }
}
