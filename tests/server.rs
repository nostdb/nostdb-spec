//! The local protocol fixture suite is internally consistent with the contract.
//!
//! This repository owns no runtime, so it cannot speak the protocol. What it can check is
//! that every message fixture is well formed, declares an outcome its directory allows,
//! names the section 8 rule it exercises, and that every stated rule has a fixture an
//! implementation can fail against.
//!
//! `nostdb-server` proves the behavior, reading this same suite from the superproject.
//! Framing, endpoint permissions, the one-daemon lock, and session isolation are behavioral
//! and are not expressible as a JSON document, which section 10 says outright.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Every refusal the contract's section 8 table states, paired with the fixture that
/// exercises it. Adding a row to that table without a fixture fails the last test here.
const REFUSAL_RULES: [&str; 12] = [
    "versions_do_not_intersect",
    "version_absent_after_handshake",
    "first_message_not_hello",
    "frame_too_large",
    "body_not_an_object",
    "request_id_absent",
    "operation_absent",
    "unknown_operation",
    "database_is_a_path",
    "unknown_session",
    "second_session_on_one_connection",
    "peer_is_another_user",
];

/// The operations section 5.2 publishes. A fixture naming anything else is either a typo or
/// an operation somebody added without publishing it.
const OPERATIONS: [&str; 8] = [
    "open_session",
    "close_session",
    "query",
    "begin",
    "commit",
    "rollback",
    "status",
    "shutdown",
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

fn document(path: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(path).expect("fixture is UTF-8");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn supported_versions() -> BTreeSet<u64> {
    common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|contract| contract["key"] == "server_protocol_version")
        .and_then(|contract| contract["supported"].as_array())
        .expect("server_protocol_version is registered")
        .iter()
        .map(|value| value.as_u64().expect("a version is a number"))
        .collect()
}

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for directory in ["valid", "invalid"] {
        let relative = format!("fixtures/server/{directory}");
        assert_eq!(
            stems(&relative, "json"),
            stems(&relative, "expected"),
            "{directory} has a fixture without an expectation, or the reverse"
        );
    }
}

/// An accepted message is an object, and states a supported version unless it precedes one.
///
/// Two messages are exceptions, on purpose and for different reasons. Section 4 gives `hello`
/// no version, because a client that must already know the version in order to ask which
/// version is supported cannot negotiate at all. It gives `refused` none because there is
/// none: the two sides have just established that they share no version, and naming one would
/// be a claim about a language neither agreed to speak.
///
/// The second exception was found by this test rather than reasoned out in advance. It failed
/// on the refusal fixture, which is what made the contract say so explicitly.
#[test]
fn accepted_fixtures_state_a_supported_version_unless_they_precede_one() {
    let supported = supported_versions();

    for path in common::files_with_extension("fixtures/server/valid", "json") {
        let name = path.display();
        assert_eq!(
            expectation(&path).get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );

        let value = document(&path);
        let object = value
            .as_object()
            .unwrap_or_else(|| panic!("{name}: a message is a JSON object"));

        // Two messages precede a negotiated version and state none, for different reasons
        // section 4 gives: `hello` cannot know one yet, and `refused` establishes that there
        // is none. Everything from `welcome` onward carries it.
        match object.get("message").and_then(serde_json::Value::as_str) {
            Some("hello") => {
                assert!(
                    object.get("server_protocol_version").is_none(),
                    "{name}: hello states no version of its own, so negotiation is possible"
                );
                let offered: Vec<u64> = object
                    .get("supported_versions")
                    .and_then(serde_json::Value::as_array)
                    .unwrap_or_else(|| panic!("{name}: hello must offer supported_versions"))
                    .iter()
                    .map(|v| v.as_u64().expect("a version is a number"))
                    .collect();
                assert!(
                    offered.iter().any(|v| supported.contains(v)),
                    "{name}: an accepted handshake must offer a version the contract supports"
                );
                continue;
            }
            Some("refused") => {
                assert!(
                    object.get("server_protocol_version").is_none(),
                    "{name}: a refusal names no negotiated version, because there is none"
                );
                assert_eq!(
                    object.get("code").and_then(serde_json::Value::as_str),
                    Some("SERVER_PROTOCOL_UNSUPPORTED"),
                    "{name}: a refusal states the code the contract assigns it"
                );
                let offered = object
                    .get("supported_versions")
                    .and_then(serde_json::Value::as_array)
                    .unwrap_or_else(|| panic!("{name}: a refusal states supported_versions"));
                assert!(
                    !offered.is_empty(),
                    "{name}: a refusal with no versions listed is not actionable"
                );
                continue;
            }
            _ => {}
        }

        let version = object
            .get("server_protocol_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| {
                panic!("{name}: every message after the handshake states a version")
            });
        assert!(
            supported.contains(&version),
            "{name}: version {version} is not supported, so this belongs in invalid/"
        );

        // A request names a published operation. A response echoes a request_id instead.
        if let Some(operation) = object.get("operation").and_then(serde_json::Value::as_str) {
            assert!(
                OPERATIONS.contains(&operation),
                "{name}: {operation} is not an operation section 5.2 publishes"
            );
            assert!(
                object.get("request_id").is_some(),
                "{name}: a request carries a request_id, because responses may arrive in any order"
            );
        }

        // A query names a catalog name, never a path. The daemon is not a second route to a
        // file, so a fixture that slipped a path in would be publishing the opposite.
        if let Some(database) = object.get("database").and_then(serde_json::Value::as_str) {
            assert!(
                !database.contains('/') && !database.contains('\\') && !database.starts_with('@'),
                "{name}: database must be a bare catalog name, found {database}"
            );
        }
    }
    println!("server conformance: accepted fixtures verified");
}

/// A refused message names the rule it breaks, and carries a code only where the contract
/// assigns one.
///
/// Section 8 assigns a code to the version refusal alone. The rest are malformed rather than
/// unauthorized, and a code is a contract with a caller: a peer that cannot frame a message
/// is not yet a caller. Requiring a code everywhere would have meant inventing codes to
/// satisfy a test.
#[test]
fn rejected_fixtures_name_a_stated_rule() {
    let registered: BTreeSet<String> = common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code string").to_owned())
        .collect();
    let known: BTreeSet<&str> = REFUSAL_RULES.iter().copied().collect();

    for path in common::files_with_extension("fixtures/server/invalid", "json") {
        let name = path.display();
        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("reject"),
            "{name} must declare outcome = reject"
        );

        let rule = expected
            .get("rule")
            .unwrap_or_else(|| panic!("{name} must declare the rule it exercises"));
        assert!(
            known.contains(rule.as_str()),
            "{name} names {rule}, which section 8 does not state"
        );

        if let Some(code) = expected.get("code") {
            assert!(
                registered.contains(code),
                "{name} declares unregistered code {code}"
            );
            assert!(
                code.starts_with("SERVER_"),
                "{name} declares {code}, which the protocol contract does not own"
            );
            assert_eq!(
                rule, "versions_do_not_intersect",
                "{name}: section 8 assigns a code to the version refusal alone"
            );
        }

        // A refused message must still be readable JSON, or the fixture proves only that a
        // JSON reader rejects malformed JSON.
        document(&path);
    }
    println!("server conformance: rejected fixtures verified");
}

#[test]
fn both_server_codes_are_reachable() {
    let from_fixtures: BTreeSet<String> = ["valid", "invalid"]
        .iter()
        .flat_map(|dir| common::files_with_extension(&format!("fixtures/server/{dir}"), "json"))
        .filter_map(|path| expectation(&path).get("code").cloned())
        .collect();

    assert!(
        from_fixtures.contains("SERVER_PROTOCOL_UNSUPPORTED"),
        "no protocol fixture declares SERVER_PROTOCOL_UNSUPPORTED"
    );

    // SERVER_ALREADY_RUNNING is deliberately absent from the fixtures. It is a lifecycle
    // outcome rather than a message rule: it is reported when a start request finds a healthy
    // daemon, which is a running process rather than a document. The contract states it in
    // section 2.1 and nostdb-server proves it with a lifecycle test.
    let contract = common::read("docs/SERVER_PROTOCOL.md");
    assert!(
        contract.contains("SERVER_ALREADY_RUNNING"),
        "SERVER_ALREADY_RUNNING must be stated by the contract that owns it"
    );
}

#[test]
fn every_refusal_rule_in_the_contract_has_a_fixture() {
    let present: BTreeSet<String> = common::files_with_extension("fixtures/server/invalid", "json")
        .iter()
        .filter_map(|path| expectation(path).get("rule").cloned())
        .collect();

    let missing: Vec<&str> = REFUSAL_RULES
        .iter()
        .copied()
        .filter(|rule| !present.contains(*rule))
        .collect();
    assert!(
        missing.is_empty(),
        "these refusal rules have no fixture: {missing:?}"
    );
    println!(
        "server conformance: {} refusal rules verified",
        REFUSAL_RULES.len()
    );
}
