//! The result-envelope fixture suite is internally consistent with the contract.
//!
//! This repository owns no runtime, so it checks the shape of every fixture rather than
//! producing one. The checks below are the contract's rules written as code, which is
//! what makes an `invalid/` fixture prove something: the same rule that rejects it here
//! is the rule `nostdb-core` must apply when it writes an envelope.

mod common;

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

/// The closed set of tagged value forms.
const TAGS: [&str; 5] = ["bytes", "datetime", "node", "relationship", "path"];

fn registered_codes() -> BTreeSet<String> {
    common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code string").to_owned())
        .collect()
}

fn supported_versions() -> BTreeSet<u64> {
    common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|contract| contract["key"] == "result_version")
        .and_then(|contract| contract["supported"].as_array())
        .expect("result_version is registered")
        .iter()
        .map(|value| value.as_u64().expect("a version is a number"))
        .collect()
}

/// Reports the first rule `document` breaks, or `None` when it satisfies all of them.
///
/// Every lookup is explicit rather than using `?`. An earlier draft used `?` throughout,
/// which returns `None` from this function when a member is missing — and `None` means
/// "no violation". A malformed envelope therefore read as a valid one, and the fixture
/// for a missing version passed. That is precisely the silent pass a conformance suite
/// exists to prevent, so the lookups below say what they found.
fn violation(
    document: &Value,
    codes: &BTreeSet<String>,
    versions: &BTreeSet<u64>,
) -> Option<String> {
    let Some(object) = document.as_object() else {
        return Some("an envelope is a JSON object".to_owned());
    };

    match object.get("result_version").and_then(Value::as_u64) {
        None => return Some("result_version is required and is a positive integer".to_owned()),
        Some(version) if !versions.contains(&version) => {
            return Some(format!("result_version {version} is not supported"));
        }
        Some(_) => {}
    }

    let Some(columns) = object.get("columns").and_then(Value::as_array) else {
        return Some("columns is required and is an array".to_owned());
    };
    let Some(rows) = object.get("rows").and_then(Value::as_array) else {
        return Some("rows is required and is an array".to_owned());
    };
    let Some(summary) = object.get("summary").and_then(Value::as_object) else {
        return Some("summary is required and is an object".to_owned());
    };
    let Some(warnings) = object.get("warnings").and_then(Value::as_array) else {
        return Some("warnings is required and is an array".to_owned());
    };

    for (index, row) in rows.iter().enumerate() {
        let Some(row) = row.as_array() else {
            return Some(format!("row {index} is not an array"));
        };
        if row.len() != columns.len() {
            return Some(format!(
                "row {index} has {} entries and there are {} columns",
                row.len(),
                columns.len()
            ));
        }
        for value in row {
            if let Some(problem) = value_violation(value) {
                return Some(problem);
            }
        }
    }

    match summary.get("rows").and_then(Value::as_u64) {
        None => return Some("summary.rows is required and is a count".to_owned()),
        Some(declared) if declared != rows.len() as u64 => {
            return Some(format!(
                "the summary declares {declared} rows and there are {}",
                rows.len()
            ));
        }
        Some(_) => {}
    }
    for field in ["database_generation", "linked_databases_opened"] {
        if summary.get(field).and_then(Value::as_u64).is_none() {
            return Some(format!("summary.{field} is required and is a count"));
        }
    }
    let Some(partial) = summary.get("partial").and_then(Value::as_bool) else {
        return Some("summary.partial is required and is a boolean".to_owned());
    };

    if let Some(writes) = summary.get("writes") {
        let Some(writes) = writes.as_object() else {
            return Some("summary.writes is an object".to_owned());
        };
        if writes.values().all(|value| value.as_u64() == Some(0)) {
            return Some(
                "a read omits `writes` entirely; an all-zero object blurs \"changed nothing\" \
                 with \"could not change anything\""
                    .to_owned(),
            );
        }
    }

    if partial && warnings.is_empty() {
        return Some("a partial result must say which source was unreachable".to_owned());
    }

    for warning in warnings {
        let Some(warning) = warning.as_object() else {
            return Some("a warning is an object".to_owned());
        };
        let Some(code) = warning.get("code").and_then(Value::as_str) else {
            return Some("a warning states a registered code".to_owned());
        };
        if !codes.contains(code) {
            return Some(format!("{code} is not a registered diagnostic code"));
        }
        if warning.get("message").and_then(Value::as_str).is_none() {
            return Some(format!("the {code} warning states a message"));
        }
    }

    None
}

fn value_violation(value: &Value) -> Option<String> {
    match value {
        Value::Object(object) => {
            if object.len() != 1 {
                return Some(format!(
                    "a tagged value carries exactly one member, found {}",
                    object.len()
                ));
            }
            let (tag, inner) = object.iter().next()?;
            if !TAGS.contains(&tag.as_str()) {
                return Some(format!("`{tag}` is not one of the tagged forms {TAGS:?}"));
            }
            if tag == "path" {
                let Some(path) = inner.as_object() else {
                    return Some("a path carries an object".to_owned());
                };
                let nodes = path.get("nodes").and_then(Value::as_array).map(Vec::len);
                let relationships = path
                    .get("relationships")
                    .and_then(Value::as_array)
                    .map(Vec::len);
                let (Some(nodes), Some(relationships)) = (nodes, relationships) else {
                    return Some("a path carries nodes and relationships".to_owned());
                };
                if nodes != relationships + 1 {
                    return Some(format!(
                        "a path alternates, so {nodes} nodes need {} relationships, found \
                         {relationships}",
                        nodes.saturating_sub(1)
                    ));
                }
            }
            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| item.is_array().then(|| "a list holds scalars".to_owned())),
        _ => None,
    }
}

fn documents(directory: &str) -> Vec<std::path::PathBuf> {
    common::files_with_extension(directory, "json")
}

fn expectation(path: &Path) -> std::collections::BTreeMap<String, String> {
    common::parse_expected(&path.with_extension("expected"))
}

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for directory in ["valid", "invalid"] {
        let relative = format!("fixtures/result/{directory}");
        let fixtures: BTreeSet<String> = documents(&relative)
            .iter()
            .filter_map(|path| Some(path.file_stem()?.to_str()?.to_owned()))
            .collect();
        let expectations: BTreeSet<String> = common::files_with_extension(&relative, "expected")
            .iter()
            .filter_map(|path| Some(path.file_stem()?.to_str()?.to_owned()))
            .collect();
        assert_eq!(fixtures, expectations, "{directory} is unpaired");
    }
}

#[test]
fn accepted_envelopes_satisfy_every_rule() {
    let codes = registered_codes();
    let versions = supported_versions();
    let paths = documents("fixtures/result/valid");
    for path in &paths {
        let name = path.display();
        assert_eq!(
            expectation(path).get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );
        let document: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            violation(&document, &codes, &versions),
            None,
            "{name} must satisfy every rule"
        );
    }
    println!(
        "result conformance: {} accepted envelopes verified",
        paths.len()
    );
}

#[test]
fn rejected_envelopes_break_a_stated_rule() {
    let codes = registered_codes();
    let versions = supported_versions();
    let paths = documents("fixtures/result/invalid");
    for path in &paths {
        let name = path.display();
        let declared = expectation(path);
        assert_eq!(
            declared.get("outcome").map(String::as_str),
            Some("reject"),
            "{name} must declare outcome = reject"
        );
        assert!(
            declared.get("note").is_some_and(|note| !note.is_empty()),
            "{name} must name the rule it breaks"
        );

        let document: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            violation(&document, &codes, &versions).is_some(),
            "{name} declares reject but satisfies every rule this suite checks"
        );
    }
    println!(
        "result conformance: {} rejected envelopes verified",
        paths.len()
    );
}

#[test]
fn every_tagged_form_the_contract_lists_has_an_accepted_fixture() {
    // A form no fixture exercises is prose an implementation can skip.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for path in documents("fixtures/result/valid") {
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for row in document["rows"].as_array().into_iter().flatten() {
            for value in row.as_array().into_iter().flatten() {
                if let Some(object) = value.as_object()
                    && let Some(tag) = object.keys().next()
                {
                    seen.insert(tag.clone());
                }
            }
        }
    }
    let missing: Vec<&str> = TAGS
        .iter()
        .copied()
        .filter(|tag| !seen.contains(*tag))
        .collect();
    assert!(
        missing.is_empty(),
        "these tagged forms have no fixture: {missing:?}"
    );
}
