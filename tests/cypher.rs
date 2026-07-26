//! The Cypher conformance suite is well formed, and it covers what the query subset
//! contract declares.
//!
//! This harness parses no Cypher. `nostdb-core` owns the parser and proves conformance by
//! reproducing each fixture's declared outcome; what belongs here is the check that the
//! suite itself says what the contract says.
//!
//! The coverage checks exist because a published clause with no fixture is a promise no
//! implementation can be held to. Adding a construct to the contract without a fixture
//! therefore fails this suite.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The three suites and what the contract requires of each, from
/// `docs/QUERY_SUBSET.md` section 7.
const SUITES: [(&str, &str, Option<&str>); 3] = [
    ("fixtures/cypher/supported", "accept", None),
    (
        "fixtures/cypher/unsupported",
        "reject",
        Some("CYPHER_UNSUPPORTED"),
    ),
    (
        "fixtures/cypher/semantic",
        "reject",
        Some("CYPHER_SEMANTIC_ERROR"),
    ),
];

fn queries(relative_dir: &str) -> Vec<PathBuf> {
    common::files_with_extension(relative_dir, "cypher")
}

#[test]
fn every_fixture_declares_the_outcome_its_directory_means() {
    for (directory, outcome, code) in SUITES {
        for path in queries(directory) {
            let name = path.display();
            let declared = common::expectation_for(&path);

            assert_eq!(
                declared.get("outcome").map(String::as_str),
                Some(outcome),
                "{name} is in {directory} and must declare outcome = {outcome}"
            );
            assert_eq!(
                declared.get("code").map(String::as_str),
                code,
                "{name} is in {directory} and must declare {}",
                code.map_or_else(|| "no code".to_owned(), |c| format!("code = {c}")),
            );
            assert!(
                declared
                    .get("note")
                    .is_some_and(|note| !note.trim().is_empty()),
                "{name} needs a note saying what it establishes"
            );
        }
    }
}

#[test]
fn every_fixture_holds_one_non_empty_query() {
    for (directory, _, _) in SUITES {
        for path in queries(directory) {
            let name = path.display();
            let text = common::read(
                path.strip_prefix(common::repo_root())
                    .expect("fixture is under the repository root")
                    .to_str()
                    .expect("fixture path is UTF-8"),
            );
            assert!(!text.trim().is_empty(), "{name} is empty");
            // A statement separator would make the fixture two queries, and the suite
            // declares one outcome.
            assert!(
                !text.contains(';'),
                "{name} must hold one query, with no `;`"
            );
        }
    }
}

/// Every `.expected` file pairs with a fixture.
///
/// Without this, deleting a fixture and leaving its expectation behind would look like a
/// shrinking suite rather than a mistake.
#[test]
fn no_expectation_is_orphaned() {
    for (directory, _, _) in SUITES {
        let fixtures: BTreeSet<PathBuf> = queries(directory)
            .into_iter()
            .map(|path| path.with_extension(""))
            .collect();
        for expectation in common::files_with_extension(directory, "expected") {
            let stem = expectation.with_extension("");
            assert!(
                fixtures.contains(&stem),
                "{} has no fixture",
                expectation.display()
            );
        }
    }
}

/// Text of every accepted fixture, joined, for coverage checks.
fn accepted_text() -> String {
    queries("fixtures/cypher/supported")
        .into_iter()
        .map(|path| {
            std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_declared_write_clause_has_an_accepted_fixture() {
    let accepted = accepted_text();
    for clause in [
        "CREATE",
        "MERGE",
        "SET",
        "REMOVE",
        "DETACH DELETE",
        "DELETE",
    ] {
        assert!(
            accepted.contains(clause),
            "the contract declares {clause} in section 2.2, but no accepted fixture uses it"
        );
    }
}

#[test]
fn every_declared_aggregate_has_an_accepted_fixture() {
    let accepted = accepted_text();
    for aggregate in ["count(", "sum(", "avg(", "min(", "max(", "collect("] {
        assert!(
            accepted.contains(aggregate),
            "the contract declares {aggregate}) in section 9.1, but no accepted fixture uses it"
        );
    }
}

#[test]
fn every_nostdb_procedure_and_function_has_an_accepted_fixture() {
    let accepted = accepted_text();
    for name in [
        "nostdb.links(",
        "nostdb.build_status(",
        "nostdb.evidence(",
        "nostdb.refresh_links(",
        "nostdb.source(",
        "nostdb.source_location(",
        "nostdb.source_revision(",
        "nostdb.link_alias(",
        "nostdb.is_available(",
    ] {
        assert!(
            accepted.contains(name),
            "the contract declares {name}) in section 12, but no accepted fixture uses it"
        );
    }
}

/// A capability-gated procedure belongs in the accepted suite, not the refused one.
///
/// The accepted suite requires only that a fixture parses. Declaring
/// `nostdb.refresh_links()` refused would make the fixture false as soon as an
/// implementation gains the provider capability, which is the opposite of what a
/// conformance fixture is for.
#[test]
fn the_capability_gated_procedure_is_not_declared_refused() {
    for directory in ["fixtures/cypher/unsupported", "fixtures/cypher/semantic"] {
        for path in queries(directory) {
            let text = std::fs::read_to_string(&path).expect("fixture");
            assert!(
                !text.contains("refresh_links"),
                "{} declares a capability-gated procedure permanently refused",
                path.display()
            );
        }
    }
}
