//! Conformance of the `.nost` fixture suite against the reference encoding.
//!
//! `outcome` and `code` are normative. `reference_line` and `reference_column`
//! are informative: they pin the reference encoding's behavior so an
//! unintended grammar change is visible, and they bind no other implementation.
//! See docs/NOST_LANGUAGE.md section 11.1.

mod common;

use pest::Parser as _;
use pest::error::LineColLocation;
use std::collections::BTreeSet;
use std::fs;

#[derive(pest_derive::Parser)]
#[grammar = "../grammar/nost.pest"]
struct NostReference;

fn position(error: &pest::error::Error<Rule>) -> (usize, usize) {
    match error.line_col {
        LineColLocation::Pos((line, column)) => (line, column),
        LineColLocation::Span((line, column), _) => (line, column),
    }
}

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for dir in ["valid", "invalid-syntax", "invalid-semantic"] {
        let relative = format!("fixtures/nost/{dir}");
        let fixtures: BTreeSet<String> = common::files_with_extension(&relative, "nost")
            .iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        let expectations: BTreeSet<String> = common::files_with_extension(&relative, "expected")
            .iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            fixtures, expectations,
            "{dir} has a fixture without an expectation, or the reverse"
        );
    }
}

#[test]
fn accepted_fixtures_parse() {
    for path in common::files_with_extension("fixtures/nost/valid", "nost") {
        let expectation = common::expectation_for(&path);
        assert_eq!(
            expectation.get("outcome").map(String::as_str),
            Some("accept"),
            "{} must declare outcome = accept",
            path.display()
        );

        let source = fs::read_to_string(&path).expect("fixture is UTF-8");
        if let Err(error) = NostReference::parse(Rule::nost_file, &source) {
            panic!("{} must parse but did not:\n{error}", path.display());
        }
    }
}

#[test]
fn accepted_fixtures_obey_the_encoding_rules() {
    // docs/NOST_LANGUAGE.md section 2: UTF-8, no byte-order mark, U+000A line
    // terminator, and exactly one trailing U+000A.
    for path in common::files_with_extension("fixtures/nost/valid", "nost") {
        let bytes = fs::read(&path).expect("readable");
        let name = path.display();
        assert!(
            !bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
            "{name} must not begin with a byte-order mark"
        );
        assert!(!bytes.contains(&b'\r'), "{name} must not contain U+000D");
        assert_eq!(
            bytes.last(),
            Some(&b'\n'),
            "{name} must end with exactly one U+000A"
        );
        assert_ne!(
            bytes.get(bytes.len().wrapping_sub(2)),
            Some(&b'\n'),
            "{name} must end with exactly one U+000A, not more"
        );
    }
}

#[test]
fn syntactically_invalid_fixtures_are_rejected() {
    let registered = registered_codes();

    for path in common::files_with_extension("fixtures/nost/invalid-syntax", "nost") {
        let expectation = common::expectation_for(&path);
        let name = path.display();

        assert_eq!(
            expectation.get("outcome").map(String::as_str),
            Some("reject"),
            "{name} must declare outcome = reject"
        );
        let code = expectation
            .get("code")
            .unwrap_or_else(|| panic!("{name} must declare a code"));
        assert_eq!(
            code, "NOST_PARSE_ERROR",
            "{name} must declare NOST_PARSE_ERROR"
        );
        assert!(
            registered.contains(code),
            "{name} declares unregistered code {code}"
        );

        let source = fs::read_to_string(&path).expect("fixture is UTF-8");
        let error = match NostReference::parse(Rule::nost_file, &source) {
            Ok(_) => panic!("{name} must be rejected but parsed"),
            Err(error) => error,
        };

        // Informative: pins the reference encoding only.
        let (line, column) = position(&error);
        let expect_line: usize = expectation
            .get("reference_line")
            .unwrap_or_else(|| panic!("{name} must record reference_line"))
            .parse()
            .expect("reference_line is a number");
        let expect_column: usize = expectation
            .get("reference_column")
            .unwrap_or_else(|| panic!("{name} must record reference_column"))
            .parse()
            .expect("reference_column is a number");
        assert_eq!(
            (line, column),
            (expect_line, expect_column),
            "{name}: the reference encoding now reports {line}:{column} instead of \
             the recorded {expect_line}:{expect_column}. Re-record it if the grammar \
             change was intended."
        );
    }
}

#[test]
fn semantically_invalid_fixtures_parse_and_declare_a_registered_code() {
    let registered = registered_codes();

    for path in common::files_with_extension("fixtures/nost/invalid-semantic", "nost") {
        let expectation = common::expectation_for(&path);
        let name = path.display();

        assert_eq!(
            expectation.get("outcome").map(String::as_str),
            Some("accept_then_diagnose"),
            "{name} must declare outcome = accept_then_diagnose"
        );
        let code = expectation
            .get("code")
            .unwrap_or_else(|| panic!("{name} must declare a code"));
        assert!(
            registered.contains(code),
            "{name} declares unregistered code {code}"
        );
        assert_ne!(
            code, "NOST_PARSE_ERROR",
            "{name} is a semantic fixture, so its code must not be NOST_PARSE_ERROR"
        );

        let source = fs::read_to_string(&path).expect("fixture is UTF-8");
        if let Err(error) = NostReference::parse(Rule::nost_file, &source) {
            panic!("{name} is semantically invalid but must still parse:\n{error}");
        }
    }
}

#[test]
fn the_semantic_rule_table_is_covered_by_fixtures() {
    // Every diagnostic the language contract attributes to a semantic rule needs a
    // fixture, otherwise nostdb-core has nothing to prove itself against.
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for path in common::files_with_extension("fixtures/nost/invalid-semantic", "nost") {
        if let Some(code) = common::expectation_for(&path).get("code") {
            declared.insert(code.clone());
        }
    }

    let registry = common::read_json("diagnostics.json");
    let expected: BTreeSet<String> = registry["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .filter(|entry| entry["contract"] == "nost_language_version")
        .map(|entry| entry["code"].as_str().expect("code string").to_string())
        .filter(|code| code != "NOST_PARSE_ERROR")
        .collect();

    let uncovered: Vec<&String> = expected.difference(&declared).collect();
    assert!(
        uncovered.is_empty(),
        "these .nost semantic diagnostics have no fixture: {uncovered:?}"
    );
}

fn registered_codes() -> BTreeSet<String> {
    common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code string").to_string())
        .collect()
}
