//! The viewer exchange fixtures are internally consistent with the contract.
//!
//! This reads the bytes rather than trusting the generator that wrote them. A suite that checked
//! the generator's intent would agree with itself by construction, which is the failure the Stage 10
//! budget fixtures recorded: a suite written only against shapes its author invented tests the
//! author's idea of the document.
//!
//! It parses the header and the section table, which is what every implementation must do first, and
//! stops there. Decoding a payload and rendering it is `nostdb-cli`'s conformance suite and a
//! viewer's job.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"NOSTVIEW";
const HEADER_BYTES: usize = 32;
const ENTRY_BYTES: usize = 16;

/// Every rejection the contract states, paired with the fixture that reaches it.
///
/// Adding a rule to sections 3, 4, or 6 without a fixture fails the last test here.
const REJECTIONS: [&str; 17] = [
    "bad_magic",
    "unsupported_version",
    "header_checksum_does_not_match",
    "payload_checksum_does_not_match",
    "node_count_disagrees_with_the_section",
    "section_offset_outside_the_file",
    "section_length_overflows",
    "sections_overlap",
    "unknown_section_kind",
    "duplicate_section_kind",
    "a_required_section_is_absent",
    "string_index_out_of_range",
    "edge_endpoint_out_of_range",
    "source_zero_is_not_the_root",
    "evidence_is_out_of_order",
    "reserved_is_not_zero",
    "truncated_before_the_header_ends",
];

/// The section kinds version 1 defines, and which are required.
const SECTIONS: [(u16, &str, bool); 8] = [
    (1, "strings", true),
    (2, "node_ids", true),
    (3, "node_labels", true),
    (4, "node_sources", true),
    (5, "edge_endpoints", true),
    (6, "edge_relations", true),
    (7, "sources", true),
    (8, "evidence", false),
];

fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0x82f6_3b78
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn fixtures(directory: &str) -> Vec<PathBuf> {
    common::files_with_extension(directory, "bin")
}

fn expectation(path: &Path) -> BTreeMap<String, String> {
    common::parse_expected(&path.with_extension("expected"))
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// One entry from the section table.
struct Section {
    kind: u16,
    offset: usize,
    length: usize,
    checksum: u32,
}

/// Reads the header and section table the way section 3 states, refusing rather than assuming.
fn read(bytes: &[u8]) -> Result<(u32, u32, u32, Vec<Section>), String> {
    if bytes.len() < HEADER_BYTES {
        return Err("shorter than its own header".to_owned());
    }
    if &bytes[0..8] != MAGIC {
        return Err("the magic is not NOSTVIEW".to_owned());
    }
    let version = u16_at(bytes, 8);
    if version != 1 {
        return Err(format!("view_exchange_version {version}"));
    }
    if crc32c(&bytes[0..24]) != u32_at(bytes, 24) {
        return Err("the header checksum does not match".to_owned());
    }
    if u32_at(bytes, 28) != 0 {
        return Err("the reserved field is not zero".to_owned());
    }

    let count = usize::from(u16_at(bytes, 10));
    if count > 16 {
        return Err(format!("{count} sections, over the bound of 16"));
    }
    let table_end = HEADER_BYTES + count * ENTRY_BYTES;
    if bytes.len() < table_end {
        return Err("the section table runs past the end".to_owned());
    }

    let mut sections = Vec::with_capacity(count);
    let mut kinds: BTreeSet<u16> = BTreeSet::new();
    for index in 0..count {
        let at = HEADER_BYTES + index * ENTRY_BYTES;
        let kind = u16_at(bytes, at);
        if u16_at(bytes, at + 2) != 0 {
            return Err("a section table entry's reserved field is not zero".to_owned());
        }
        if !SECTIONS.iter().any(|(known, _, _)| *known == kind) {
            return Err(format!("section kind {kind} is not one version 1 defines"));
        }
        if !kinds.insert(kind) {
            return Err(format!("section kind {kind} appears twice"));
        }
        let offset = u32_at(bytes, at + 4) as usize;
        let length = u32_at(bytes, at + 8) as usize;
        let end = offset
            .checked_add(length)
            .ok_or("a section length overflows")?;
        if offset < table_end || end > bytes.len() {
            return Err(format!("section {kind} lies outside the file"));
        }
        sections.push(Section {
            kind,
            offset,
            length,
            checksum: u32_at(bytes, at + 12),
        });
    }

    // Overlap, checked after every entry is in range so the message names the right problem.
    let mut ordered: Vec<&Section> = sections.iter().collect();
    ordered.sort_by_key(|section| section.offset);
    for pair in ordered.windows(2) {
        if pair[0].offset + pair[0].length > pair[1].offset {
            return Err(format!(
                "sections {} and {} overlap",
                pair[0].kind, pair[1].kind
            ));
        }
    }

    for section in &sections {
        let payload = &bytes[section.offset..section.offset + section.length];
        if crc32c(payload) != section.checksum {
            return Err(format!(
                "section {} does not match its checksum",
                section.kind
            ));
        }
    }

    for (kind, name, required) in SECTIONS {
        if required && !kinds.contains(&kind) {
            return Err(format!("the required section {name} is absent"));
        }
    }

    Ok((
        u32_at(bytes, 12),
        u32_at(bytes, 16),
        u32_at(bytes, 20),
        sections,
    ))
}

/// The width one entry of a section occupies, when the contract fixes one.
fn stride(kind: u16) -> Option<usize> {
    match kind {
        2..=4 | 6 => Some(4),
        5 => Some(8),
        7 => Some(12),
        _ => None,
    }
}

/// The count a section's length must agree with.
fn expected_count(kind: u16, nodes: u32, edges: u32, sources: u32) -> Option<u32> {
    match kind {
        2..=4 => Some(nodes),
        5 | 6 => Some(edges),
        7 => Some(sources),
        _ => None,
    }
}

#[test]
fn every_fixture_pairs_with_an_expectation() {
    for directory in ["container/valid", "container/invalid"] {
        let relative = format!("fixtures/view-exchange/{directory}");
        let bins: BTreeSet<String> = fixtures(&relative)
            .iter()
            .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
            .collect();
        let expected: BTreeSet<String> = common::files_with_extension(&relative, "expected")
            .iter()
            .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
            .collect();
        assert_eq!(
            bins, expected,
            "{directory} has a fixture without an expectation, or the reverse"
        );
    }
}

#[test]
fn every_accepted_container_reads_and_states_its_declared_counts() {
    let mut verified = 0usize;
    for path in fixtures("fixtures/view-exchange/container/valid") {
        let name = path.display();
        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("accept"),
            "{name} must declare outcome = accept"
        );

        let bytes = std::fs::read(&path).expect("fixture is readable");
        let (nodes, edges, sources, sections) = read(&bytes)
            .unwrap_or_else(|reason| panic!("{name} is accepted by the contract: {reason}"));

        for (key, found) in [("nodes", nodes), ("edges", edges), ("sources", sources)] {
            let declared: u32 = expected
                .get(key)
                .unwrap_or_else(|| panic!("{name} declares no {key}"))
                .parse()
                .unwrap_or_else(|error| panic!("{name}: {key} is not a number: {error}"));
            assert_eq!(declared, found, "{name}: the header states {found} {key}");
        }

        // Section 3.1: a length that disagrees with the header's counts is a disagreement, and a
        // reader trusting either one would draw a graph nobody sent.
        for section in &sections {
            if let (Some(stride), Some(count)) = (
                stride(section.kind),
                expected_count(section.kind, nodes, edges, sources),
            ) {
                assert_eq!(
                    section.length,
                    stride * count as usize,
                    "{name}: section {} is {} bytes for {count} entries",
                    section.kind,
                    section.length
                );
            }
        }

        // Evidence is optional, so its presence is declared rather than inferred.
        let declared_evidence: usize = expected
            .get("evidence")
            .unwrap_or_else(|| panic!("{name} declares no evidence count"))
            .parse()
            .expect("a number");
        let carried = sections.iter().find(|section| section.kind == 8);
        match (declared_evidence, carried) {
            (0, None) => {}
            (count, Some(section)) => {
                let payload = &bytes[section.offset..section.offset + section.length];
                assert_eq!(
                    u32_at(payload, 0) as usize,
                    count,
                    "{name}: the evidence section states a different count"
                );
            }
            (count, None) => panic!("{name} declares {count} evidence entries and carries none"),
        }
        verified += 1;
    }
    assert!(verified > 0, "no accepted containers were found");
    println!("view exchange conformance: {verified} accepted containers verified");
}

#[test]
fn every_rejected_container_declares_the_registered_code() {
    let registered: BTreeSet<String> = common::read_json("diagnostics.json")["codes"]
        .as_array()
        .expect("codes array")
        .iter()
        .map(|entry| entry["code"].as_str().expect("code").to_owned())
        .collect();

    for path in fixtures("fixtures/view-exchange/container/invalid") {
        let name = path.display();
        let expected = expectation(&path);
        assert_eq!(
            expected.get("outcome").map(String::as_str),
            Some("reject"),
            "{name} must declare outcome = reject"
        );
        let code = expected
            .get("code")
            .unwrap_or_else(|| panic!("{name} declares no code"));
        assert!(
            registered.contains(code),
            "{name} declares unregistered code {code}"
        );
        // Section 6.1: a file that cannot be read is never reported as a capacity problem. The two
        // send a user to different places.
        assert_eq!(
            code, "VIEW_EXCHANGE_INVALID",
            "{name}: a container that cannot be read is not a capacity problem"
        );
    }
    println!(
        "view exchange conformance: {} rejected containers verified",
        fixtures("fixtures/view-exchange/container/invalid").len()
    );
}

/// The rejections this harness can decide from the header and table alone are refused here.
///
/// The rest — a string index out of range, an endpoint past the node count, evidence out of order —
/// are payload rules, and refusing them needs a decoder. `nostdb-cli` has one and its suite covers
/// them; this asserts the split rather than leaving the reader to wonder which half ran.
#[test]
fn the_container_level_rejections_are_refused_by_reading_the_header() {
    let container_level = [
        "bad_magic",
        "unsupported_version",
        "header_checksum_does_not_match",
        "payload_checksum_does_not_match",
        "section_offset_outside_the_file",
        "section_length_overflows",
        "sections_overlap",
        "unknown_section_kind",
        "duplicate_section_kind",
        "a_required_section_is_absent",
        "reserved_is_not_zero",
        "truncated_before_the_header_ends",
    ];
    let mut refused = 0usize;
    for path in fixtures("fixtures/view-exchange/container/invalid") {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("a name");
        if !container_level.contains(&stem) {
            continue;
        }
        let bytes = std::fs::read(&path).expect("fixture is readable");
        assert!(
            read(&bytes).is_err(),
            "{stem} is refused by the contract and read here"
        );
        refused += 1;
    }
    assert_eq!(
        refused,
        container_level.len(),
        "a container-level rejection has no fixture"
    );
    println!("view exchange conformance: {refused} container-level rejections verified");
}

#[test]
fn every_rejection_the_contract_states_has_a_fixture() {
    let present: BTreeSet<String> = fixtures("fixtures/view-exchange/container/invalid")
        .iter()
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
        .collect();
    let missing: Vec<&str> = REJECTIONS
        .iter()
        .copied()
        .filter(|rule| !present.contains(*rule))
        .collect();
    assert!(
        missing.is_empty(),
        "these rules have no fixture: {missing:?}"
    );

    let known: BTreeSet<&str> = REJECTIONS.iter().copied().collect();
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
fn the_crc_reproduces_the_standard_check_value() {
    // The same check value the container contract uses, so this harness and that one are provably
    // computing the same function rather than two functions that happen to agree on the fixtures.
    assert_eq!(crc32c(b"123456789"), 0xE306_9283);
}
