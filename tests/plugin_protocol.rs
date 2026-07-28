//! The plugin protocol fixture suite is internally consistent with the contract.
//!
//! No fixture starts a process, and that is where this suite has to stop: one that started a
//! plugin to test starting one would be executing arbitrary code to check the rules that decide
//! whether to.
//!
//! What it can check is that every message is well formed, declares an outcome its directory
//! allows, names a registered code this contract owns, and that every refusal the contract states
//! has a fixture an implementation can fail against.
//!
//! `nostdb-cli` proves the behavior, reading this same suite from the superproject.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Every refusal the table in section 7 states, paired with a fixture that reaches it.
///
/// `PLUGIN_REQUIRED` is deliberately absent: it is raised before a process exists, so no
/// message can carry it and no fixture here could reach it. It is covered where the manager
/// decides, in the implementation's own tests.
const PROTOCOL_CODES: [&str; 5] = [
    "PLUGIN_PROTOCOL_UNSUPPORTED",
    "PLUGIN_REQUEST_INVALID",
    "PLUGIN_ACTION_UNKNOWN",
    "PLUGIN_IDENTITY_MISMATCH",
    "PLUGIN_FAILED",
];

/// The message kinds version 1 defines.
const KINDS: [&str; 3] = ["handshake", "invoke", "error"];

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
    for directory in ["message/valid", "message/invalid", "handshake"] {
        let relative = format!("fixtures/plugin-protocol/{directory}");
        assert_eq!(
            stems(&relative, "json"),
            stems(&relative, "expected"),
            "{directory} has a fixture without an expectation, or the reverse"
        );
    }
}

#[test]
fn every_accepted_message_states_a_supported_version_and_a_known_kind() {
    let supported: BTreeSet<u64> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|contract| contract["key"] == "plugin_protocol_version")
        .and_then(|contract| contract["supported"].as_array())
        .expect("plugin_protocol_version is registered")
        .iter()
        .map(|value| value.as_u64().expect("a version is a number"))
        .collect();

    let mut seen_kinds: BTreeSet<String> = BTreeSet::new();
    let mut requests = 0usize;
    let mut replies = 0usize;

    for path in common::files_with_extension("fixtures/plugin-protocol/message/valid", "json") {
        let name = path.display();
        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );

        let message = document(&path);
        let message = message
            .as_object()
            .unwrap_or_else(|| panic!("{name}: a message is a JSON object"));

        let version = message
            .get("plugin_protocol_version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| panic!("{name}: every message states its version"));
        assert!(
            supported.contains(&version),
            "{name}: version {version} is not supported, so this belongs in message/invalid/"
        );

        // A message is a request or a reply, never both and never neither. The expectation
        // declares which, and the document has to agree — a suite that only read the document
        // would pass a fixture whose expectation described a different message.
        let role = expected
            .get("role")
            .unwrap_or_else(|| panic!("{name} declares no role"));
        let kind = expected
            .get("kind")
            .unwrap_or_else(|| panic!("{name} declares no kind"));
        assert!(
            KINDS.contains(&kind.as_str()),
            "{name} declares kind {kind}, which version 1 does not define"
        );
        seen_kinds.insert(kind.clone());

        match role.as_str() {
            "request" => {
                assert_eq!(
                    message.get("request").and_then(serde_json::Value::as_str),
                    Some(kind.as_str()),
                    "{name}: the document and the expectation disagree"
                );
                assert!(!message.contains_key("reply"), "{name} is both");
                requests += 1;
            }
            "reply" => {
                assert_eq!(
                    message.get("reply").and_then(serde_json::Value::as_str),
                    Some(kind.as_str()),
                    "{name}: the document and the expectation disagree"
                );
                assert!(!message.contains_key("request"), "{name} is both");
                replies += 1;
            }
            other => panic!("{name}: role is {other}, expected request or reply"),
        }

        if kind == "error" {
            let code = expected
                .get("code")
                .unwrap_or_else(|| panic!("{name}: an error reply declares the code it carries"));
            assert_eq!(
                message.get("code").and_then(serde_json::Value::as_str),
                Some(code.as_str()),
                "{name}: the document and the expectation disagree"
            );
        }
    }

    assert!(
        requests > 0 && replies > 0,
        "the suite covers only one direction"
    );
    // Every kind the contract defines has an accepted fixture. A kind with none is a message
    // shape nothing published shows anybody how to write.
    for kind in KINDS {
        assert!(
            seen_kinds.contains(kind),
            "no accepted fixture is a {kind} message"
        );
    }
    println!("plugin protocol conformance: {requests} requests and {replies} replies verified");
}

#[test]
fn every_rejected_message_declares_a_registered_protocol_code() {
    let registered = registered_codes();
    for path in common::files_with_extension("fixtures/plugin-protocol/message/invalid", "json") {
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
            PROTOCOL_CODES.contains(&code.as_str()),
            "{name} declares {code}, which this contract does not own"
        );

        // Section 7.1: which side has the defect decides the code. A malformed reply is the
        // plugin breaking the protocol; an invalid request is its complaint about what it was
        // sent. A version disagreement is neither side's defect and is raised by whichever
        // received it.
        let role = expected
            .get("role")
            .unwrap_or_else(|| panic!("{name} declares no role"));
        match (role.as_str(), code.as_str()) {
            (_, "PLUGIN_PROTOCOL_UNSUPPORTED") => {}
            ("request", "PLUGIN_REQUEST_INVALID") | ("reply", "PLUGIN_FAILED") => {}
            (role, code) => {
                panic!("{name} is a {role} declaring {code}, which section 7.1 does not permit")
            }
        }

        // A rejected message must still be readable JSON. A fixture that is not parseable would
        // prove an implementation refuses malformed JSON, which every JSON reader already does.
        document(&path);
    }
    println!(
        "plugin protocol conformance: {} rejected messages verified",
        stems("fixtures/plugin-protocol/message/invalid", "json").len()
    );
}

#[test]
fn every_handshake_fixture_pairs_an_approval_with_a_reply() {
    let registered = registered_codes();
    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for path in common::files_with_extension("fixtures/plugin-protocol/handshake", "json") {
        let name = path.display();
        let fixture = document(&path);

        // The approval is the authority, so a fixture that did not state one would be testing a
        // handshake against nothing.
        let approved = fixture["approved"]
            .as_object()
            .unwrap_or_else(|| panic!("{name}: an approved record"));
        approved
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("{name}: the approval names the plugin"));
        approved
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| panic!("{name}: the approval declares its actions"));
        assert!(
            fixture["handshake"].is_object(),
            "{name}: a handshake reply"
        );

        match expectation(&path).get("outcome").map(String::as_str) {
            Some("accept") => accepted += 1,
            Some("reject") => {
                let code = expectation(&path)
                    .get("code")
                    .cloned()
                    .unwrap_or_else(|| panic!("{name} must declare a code"));
                assert!(
                    registered.contains(&code),
                    "{name} declares unregistered code {code}"
                );
                rejected += 1;
            }
            other => panic!("{name}: outcome is {other:?}, expected accept or reject"),
        }
    }
    assert!(
        accepted > 0 && rejected > 0,
        "the handshake suite is one-sided"
    );
    println!(
        "plugin protocol conformance: {accepted} accepted and {rejected} rejected handshakes verified"
    );
}

/// Every code this contract owns is reachable from a fixture, or is excluded on purpose.
///
/// Registering a code no fixture declares is how a code becomes documentation. The exclusion is
/// stated in `PROTOCOL_CODES` rather than left as a gap, so the list says what it does not cover.
#[test]
fn every_protocol_code_is_reachable_or_deliberately_excluded() {
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for directory in ["message/valid", "message/invalid", "handshake"] {
        let relative = format!("fixtures/plugin-protocol/{directory}");
        for path in common::files_with_extension(&relative, "json") {
            if let Some(code) = expectation(&path).get("code") {
                declared.insert(code.clone());
            }
        }
    }
    for code in PROTOCOL_CODES {
        assert!(
            declared.contains(code),
            "no plugin protocol fixture declares {code}"
        );
    }

    // And the registry's own view: every code assigned to this contract is either in the list
    // above or is the one the contract says no message can carry.
    let owned: BTreeSet<String> = common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .filter(|entry| entry["contract"] == "plugin_protocol_version")
        .map(|entry| entry["code"].as_str().expect("code").to_owned())
        .collect();
    let unaccounted: Vec<&String> = owned
        .iter()
        .filter(|code| {
            !PROTOCOL_CODES.contains(&code.as_str()) && code.as_str() != "PLUGIN_REQUIRED"
        })
        .collect();
    assert!(
        unaccounted.is_empty(),
        "these codes are assigned to this contract and no fixture reaches them: {unaccounted:?}"
    );
}
