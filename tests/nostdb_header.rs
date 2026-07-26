//! Reference validator for the `.nostdb` container envelope.
//!
//! This implements exactly the ordered checks in docs/NOSTDB_FORMAT.md section
//! 10 and nothing more. It locates, bounds, and checksums sections; it never
//! interprets a section payload, and it is not a storage engine. Its purpose is
//! to prove that the fixture suite, the prose contract, and
//! format/nostdb-header.json agree. `nostdb-core` implements the real reader.

mod common;

use std::collections::BTreeSet;

const CORRUPT: &str = "NOSTDB_CORRUPT";
const UNSUPPORTED: &str = "NOSTDB_FORMAT_UNSUPPORTED";
const LIMIT: &str = "NOSTDB_LIMIT_EXCEEDED";

struct Layout {
    magic: Vec<u8>,
    header_length: u64,
    entry_length: u64,
    max_section_count: u64,
    min_section_offset: u64,
    supported_versions: BTreeSet<u64>,
}

fn layout() -> Layout {
    let descriptor = common::read_json("format/nostdb-header.json");

    let magic_hex = descriptor["magic"]["hex"].as_str().expect("magic hex");
    let magic: Vec<u8> = magic_hex
        .as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).expect("hex byte"))
        .collect();
    assert_eq!(
        magic.len() as u64,
        descriptor["magic"]["length"]
            .as_u64()
            .expect("magic length"),
        "the descriptor's magic length disagrees with its hex"
    );

    let supported_versions: BTreeSet<u64> = common::read_json("versions.json")["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|c| c["key"] == "nostdb_format_version")
        .expect("nostdb_format_version is registered")["supported"]
        .as_array()
        .expect("supported array")
        .iter()
        .map(|v| v.as_u64().expect("version number"))
        .collect();

    Layout {
        magic,
        header_length: descriptor["header"]["length"]
            .as_u64()
            .expect("header length"),
        entry_length: descriptor["section_table_entry"]["length"]
            .as_u64()
            .expect("entry length"),
        max_section_count: descriptor["limits"]["max_section_count"]
            .as_u64()
            .expect("max_section_count"),
        min_section_offset: descriptor["limits"]["min_section_offset"]
            .as_u64()
            .expect("min_section_offset"),
        supported_versions,
    }
}

fn u32le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

fn u64le(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().expect("eight bytes"))
}

/// Returns `Ok(())` when the container envelope is acceptable, or the stable
/// diagnostic code a reader must raise.
fn validate(bytes: &[u8], layout: &Layout) -> Result<(), &'static str> {
    let file_length = bytes.len() as u64;

    // 1. The file is at least a header long.
    if file_length < layout.header_length {
        return Err(CORRUPT);
    }

    // 2. The magic matches, before any length is trusted.
    if &bytes[..layout.magic.len()] != layout.magic.as_slice() {
        return Err(CORRUPT);
    }

    // 3. The format version is supported.
    let version = u64::from(u32le(bytes, 8));
    if !layout.supported_versions.contains(&version) {
        return Err(UNSUPPORTED);
    }

    // 4. Version 1 fixes the header length.
    if u64::from(u32le(bytes, 12)) != layout.header_length {
        return Err(CORRUPT);
    }

    // 5. The header checksum covers the header with the checksum field excluded.
    let crc_offset = (layout.header_length - 4) as usize;
    if common::crc32c(&bytes[..crc_offset]) != u32le(bytes, crc_offset) {
        return Err(CORRUPT);
    }

    // 6. Reserved is zero and no undefined flag is set.
    if u32le(bytes, 36) != 0 || u32le(bytes, 40) != 0 {
        return Err(CORRUPT);
    }

    // 7. The section count is bounded before the table is sized.
    let section_count = u64::from(u32le(bytes, 32));
    if section_count > layout.max_section_count {
        return Err(LIMIT);
    }

    // 8. The whole section table lies inside the file, at or after the header.
    let table_offset = u64le(bytes, 24);
    let table_length = section_count
        .checked_mul(layout.entry_length)
        .ok_or(CORRUPT)?;
    let table_end = table_offset.checked_add(table_length).ok_or(CORRUPT)?;
    if table_offset < layout.header_length || table_end > file_length {
        return Err(CORRUPT);
    }

    // Reserved intervals, for the overlap check. A zero-length section occupies
    // nothing and therefore cannot overlap.
    let mut intervals: Vec<(u64, u64)> = vec![(0, layout.header_length)];
    if table_length > 0 {
        intervals.push((table_offset, table_end));
    }

    let mut kinds: BTreeSet<u32> = BTreeSet::new();
    let mut payloads: Vec<(u64, u64, u32)> = Vec::new();

    for index in 0..section_count {
        let base = (table_offset + index * layout.entry_length) as usize;

        // 9. Every section lies inside the file without wrapping.
        if u32le(bytes, base + 4) != 0 || u32le(bytes, base + 28) != 0 {
            return Err(CORRUPT);
        }
        let kind = u32le(bytes, base);
        if kind == 0 {
            return Err(CORRUPT);
        }
        let offset = u64le(bytes, base + 8);
        let length = u64le(bytes, base + 16);
        let end = offset.checked_add(length).ok_or(CORRUPT)?;
        if offset < layout.min_section_offset || end > file_length {
            return Err(CORRUPT);
        }

        // 11. A section kind never repeats.
        if !kinds.insert(kind) {
            return Err(CORRUPT);
        }

        if length > 0 {
            intervals.push((offset, end));
        }
        payloads.push((offset, length, u32le(bytes, base + 24)));
    }

    // 10. Nothing overlaps the header, the table, or another section.
    for (index, &(a_start, a_end)) in intervals.iter().enumerate() {
        for &(b_start, b_end) in &intervals[index + 1..] {
            if a_start < b_end && b_start < a_end {
                return Err(CORRUPT);
            }
        }
    }

    // 12. Every section checksum matches.
    for (offset, length, expected) in payloads {
        let start = offset as usize;
        let end = (offset + length) as usize;
        if common::crc32c(&bytes[start..end]) != expected {
            return Err(CORRUPT);
        }
    }

    Ok(())
}

#[test]
fn the_descriptor_layout_is_contiguous_and_complete() {
    let descriptor = common::read_json("format/nostdb-header.json");
    for section in ["header", "section_table_entry"] {
        let declared = descriptor[section]["length"].as_u64().expect("length");
        let mut cursor = 0u64;
        for field in descriptor[section]["fields"].as_array().expect("fields") {
            let offset = field["offset"].as_u64().expect("offset");
            let size = field["size"].as_u64().expect("size");
            let name = field["name"].as_str().expect("name");
            assert_eq!(
                offset, cursor,
                "{section} field {name} starts at {offset} but {cursor} was expected, \
                 so the layout has a hole or an overlap"
            );
            cursor += size;
        }
        assert_eq!(
            cursor, declared,
            "{section} fields total {cursor} bytes but the declared length is {declared}"
        );
    }
}

#[test]
fn the_crc_known_answers_hold() {
    // Standard CRC-32C check value, so a broken implementation cannot silently
    // agree with fixtures generated by the same broken implementation.
    assert_eq!(common::crc32c(b"123456789"), 0xE306_9283);
    assert_eq!(common::crc32c(b""), 0x0000_0000);
}

#[test]
fn every_header_fixture_pairs_with_an_expectation() {
    let fixtures: BTreeSet<String> = common::files_with_extension("fixtures/nostdb/header", "hex")
        .iter()
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    let expectations: BTreeSet<String> =
        common::files_with_extension("fixtures/nostdb/header", "expected")
            .iter()
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
    assert_eq!(fixtures, expectations);
}

#[test]
fn every_header_fixture_reproduces_its_declared_outcome() {
    let layout = layout();
    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for path in common::files_with_extension("fixtures/nostdb/header", "hex") {
        let expectation = common::expectation_for(&path);
        let name = path.display();
        let bytes = common::decode_hex_fixture(&path);
        let result = validate(&bytes, &layout);

        match expectation.get("outcome").map(String::as_str) {
            Some("accept") => {
                assert!(
                    !expectation.contains_key("code"),
                    "{name} declares accept, so it must declare no code"
                );
                assert_eq!(result, Ok(()), "{name} must be accepted");
                accepted += 1;
            }
            Some("reject") => {
                let expected = expectation
                    .get("code")
                    .unwrap_or_else(|| panic!("{name} must declare a code"));
                match result {
                    Ok(()) => panic!("{name} must be rejected but was accepted"),
                    Err(actual) => assert_eq!(
                        actual,
                        expected.as_str(),
                        "{name} must be rejected with {expected}, not {actual}"
                    ),
                }
                rejected += 1;
            }
            other => panic!("{name} has unusable outcome {other:?}"),
        }
    }

    assert!(
        accepted >= 2,
        "the suite needs more than one accepted container"
    );
    assert!(rejected >= 10, "the suite needs broad rejection coverage");
}

#[test]
fn every_rejection_class_in_the_contract_has_a_fixture() {
    let mut codes: BTreeSet<String> = BTreeSet::new();
    for path in common::files_with_extension("fixtures/nostdb/header", "expected") {
        if let Some(code) = common::parse_expected(&path).get("code") {
            codes.insert(code.clone());
        }
    }
    for required in [CORRUPT, UNSUPPORTED, LIMIT] {
        assert!(
            codes.contains(required),
            "no header fixture exercises {required}"
        );
    }
}

#[test]
fn mutating_any_header_byte_is_detected() {
    // The header checksum has to catch a single-byte change anywhere it covers.
    let layout = layout();
    let original = common::decode_hex_fixture(
        &common::repo_root().join("fixtures/nostdb/header/valid_no_sections.hex"),
    );
    assert_eq!(validate(&original, &layout), Ok(()));

    let covered = (layout.header_length - 4) as usize;
    for index in 0..covered {
        let mut mutated = original.clone();
        mutated[index] ^= 0x01;
        assert!(
            validate(&mutated, &layout).is_err(),
            "flipping a bit at offset {index} went undetected"
        );
    }
}
