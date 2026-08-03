# The `.nostdb` format contract

Contract key: `nostdb_format_version`
Current version: 3
Status: normative

`.nostdb` is a single opaque container holding one NostDB database. It is not
human-editable, and only `nostdb-core` writes it.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative.

## 1. What this document owns

This document defines the container: how a reader finds the payload, how it
detects corruption, how it refuses an unsupported version, and how it bounds work
against a hostile file.

[`../format/nostdb-header.json`](../format/nostdb-header.json) is the
machine-readable companion, and [`../fixtures/nostdb`](../fixtures/nostdb) is the
conformance gate.

This document does not fix the internal encoding of section payloads. A reader
locates, bounds, and checksums a section here; how node records are laid out
inside the `nodes` section is `nostdb-core`'s. That split is
deliberate: the container contract has to be stable before record encodings are
designed, so an implementation can already reject a corrupt or unsupported file.

Storage strategy is not constrained. An implementation MAY use memory mapping,
buffered reads, or an embedded page manager, provided the invariants here hold.

## 2. Design constraints

From the root PRD, the container MUST provide a magic header, an independent
format version, explicit endianness and integer widths, bounded parsing for
untrusted files, per-section checksums, a database generation, journal recovery,
atomic commit, and explicit unsupported-version diagnostics.

## 3. Layout

```text
offset 0                     header, 48 bytes
                             ...
section_table_offset          section table, section_count entries of 32 bytes
                             ...
                             section payloads, in any order
```

The header is at offset 0. The section table and the payloads MAY appear in any
order after the header, provided nothing overlaps. A writer SHOULD place the
section table immediately after the header.

## 4. Byte order and integers

All integers are unsigned, fixed width, and little-endian. There are no varints
and no signed integers in the header or section table.

Endianness is fixed rather than negotiated, so a file is byte-identical across
platforms and a fixture is portable. The magic detects a byte-swapped or
text-mangled file without a runtime endianness probe.

## 5. Magic

```text
4E 4F 53 54 44 42 1A 0A        "NOSTDB" 0x1A 0x0A
```

The 0x1A byte halts accidental text-mode display. The trailing 0x0A detects CRLF
translation during transfer: a file mangled to `... 1A 0D 0A` fails the magic
check instead of parsing as a shorter header.

A magic mismatch is `NOSTDB_CORRUPT`. A reader MUST check the magic before
reading any length.

## 6. Header

| Offset | Size | Type | Field |
| --- | --- | --- | --- |
| 0 | 8 | bytes | `magic` |
| 8 | 4 | u32 | `nostdb_format_version` |
| 12 | 4 | u32 | `header_length` |
| 16 | 8 | u64 | `database_generation` |
| 24 | 8 | u64 | `section_table_offset` |
| 32 | 4 | u32 | `section_count` |
| 36 | 4 | u32 | `flags` |
| 40 | 4 | u32 | `reserved` |
| 44 | 4 | u32 | `header_crc32c` |

Total header length is 48 bytes in every version defined so far.

- `header_length` MUST equal 48 in every version defined so far. Carrying the length
  explicitly lets a later version extend the header while an older reader still
  knows where the header ends.
- `reserved` MUST be 0. A reader MUST reject a non-zero value rather than ignore
  it, so the field stays usable later.
- `flags` is a bit set. No version defines a flag yet, and every bit MUST be 0.
- `header_crc32c` covers bytes 0 through 43 inclusive, that is the header with the
  checksum field excluded.

A failed header checksum is `NOSTDB_CORRUPT`.

## 7. Section table

Each entry is 32 bytes:

| Offset | Size | Type | Field |
| --- | --- | --- | --- |
| 0 | 4 | u32 | `section_kind` |
| 4 | 4 | u32 | `reserved` |
| 8 | 8 | u64 | `offset` |
| 16 | 8 | u64 | `length` |
| 24 | 4 | u32 | `crc32c` |
| 28 | 4 | u32 | `reserved2` |

- `reserved` and `reserved2` MUST be 0.
- `crc32c` covers exactly the `length` bytes at `offset`. A zero-length section
  has the CRC-32C of the empty input.
- A `section_kind` MUST NOT repeat within one file.

## 8. Checksums

The algorithm is CRC-32C, the Castagnoli polynomial, reflected, with reversed
polynomial `0x82F63B78`, initial value `0xFFFFFFFF`, and a final XOR of
`0xFFFFFFFF`.

CRC-32C is chosen because the requirement is corruption detection, it is cheap,
and it has wide hardware support. It is not a tamper-proofing measure. Downloaded
graph artifacts still receive an independent cryptographic digest before opening,
which is a separate provider-level requirement.

Any checksum mismatch is `NOSTDB_CORRUPT`.

## 9. Section kinds

| Kind | Name |
| --- | --- |
| 1 | `string_table` |
| 2 | `nodes` |
| 3 | `edges` |
| 4 | `properties` |
| 5 | `schemas` |
| 6 | `constraints` |
| 7 | `evidence` |
| 8 | `contributions` |
| 9 | `links` |
| 10 | `link_snapshots` |
| 11 | `analyzer_metadata` |
| 12 | `sync_metadata` |
| 13 | `indexes` |
| 14 | `build_coverage` |

Kind 0 is invalid. Kinds 15 through 32767 are reserved for future standard kinds.
Kinds 32768 and above are experimental and MUST NOT be written by a release
build.

An unknown kind in the reserved range MUST be preserved on rewrite when the file
is opened read-only and MUST NOT be silently dropped. A reader that cannot
interpret a kind still bounds and checksums it.

`links` holds link declarations, which are semantic. `link_snapshots` holds the
last resolved immutable snapshot metadata for each link, which is operational.
Keeping them apart is what lets a link identity stay the canonical locator while
its resolved commit changes.

## 10. Bounded parsing

A reader MUST treat the file as untrusted and MUST validate before allocating
from any length it read:

1. the file is at least 48 bytes;
2. the magic matches;
3. `nostdb_format_version` is supported;
4. `header_length` is exactly 48;
5. the header checksum matches;
6. `reserved` is 0 and every `flags` bit is 0;
7. `section_count` is at most 4096, else `NOSTDB_LIMIT_EXCEEDED`;
8. `section_table_offset` is at least 48, and the whole table lies inside the
   file;
9. every section lies inside the file, with `offset` at least 48 and
   `offset + length` not overflowing and not past the end;
10. no section overlaps the header, the section table, or another section;
11. no `section_kind` repeats;
12. each section checksum matches.

Checks 1 through 6 come before any length-driven allocation. Failing check 7 is
`NOSTDB_LIMIT_EXCEEDED`; failing 8 through 12 is `NOSTDB_CORRUPT`.

The section-count limit exists so a corrupt `section_count` cannot make a reader
allocate a large table before it has validated anything.

## 11. Generation and atomic commit

`database_generation` is a monotonically increasing u64. It starts at 1 for a
newly created database, and every successful commit increases it by at least 1.

A commit MUST be atomic: after a crash, a reader MUST observe either the previous
generation or the new one in full, never a mixture. A failed mutation MUST leave
the last valid generation readable.

Generation plus content digest, never wall-clock time, is what synchronization
compares. Two files with the same generation and different digests indicate
divergence, not a newer file.

## 12. Journal and recovery

A writer MUST use a journal or an equivalent atomic-replacement strategy such
that:

- an interrupted commit is either replayed to completion or discarded whole;
- recovery is idempotent, so replaying twice equals replaying once;
- a journal record carries its own checksum, and a torn record is discarded
  rather than replayed;
- recovery never advances `database_generation` past the last complete commit.

The journal lives beside the database, under the project's `.nostdb/journal`
directory, not inside the container.

## 13. Version handling and migration

### 13.1 What changed in version 3

A property value may be an **object**, and a list element is a value rather than a
scalar. The `properties` section therefore carries a map tag it did not have, and
a list may hold lists and objects.

Version 2 is **not** supported, and it briefly was. It was kept readable on the
reasoning that a `.nostdb` holds user-owned contributions no analyzer can rebuild
from source, so refusing it would destroy data to avoid one decode branch.

That reasoning describes a **released** product. NostDB is not released: no
database exists whose loss would be a user's rather than a developer's, so the
compatibility was paid for — a version branch in the schema reader, and a version
field retained on the container to feed it — and bought nothing. Both are gone, and
a version 2 container is `NOSTDB_FORMAT_UNSUPPORTED`: a database to rebuild.

What the bump does **not** change is the refusal itself. An unsupported version is
still reported with the version in the diagnostic rather than decoded on a guessed
layout, which is the migration *detection* the root PRD section 12 requires.
Rebuilding costs time and, for supported source, no external tokens.

`nostdb_format_version` moves alone. The `.nost` language moved to version 4 for
the same underlying change, and there it does **not** keep its predecessor
readable, because a `.nost` file is editable text that is usually regenerated. The
asymmetry is recorded in [`../VERSIONS.md`](../VERSIONS.md).

### 13.2 What changed in version 2

A contribution's owner was one of three tagged shapes — a name and a version, a
bare contract digest, or the user — and is one interned name. There is no reader
for the earlier tags, so version 1 is **not** supported.

Refusing it at the header is deliberate. A version 1 database read by a version 2
reader would decode until it reached an owner byte and then report an unknown
tag, which is what a corrupt file reports. `NOSTDB_FORMAT_UNSUPPORTED` says what
is true: a database to rebuild, not a database to fear. Structural analysis of
supported source spends no external tokens, so rebuilding costs time and no
money.

`nostdb_format_version` moves alone. The `.nost` language moved for its own
reason in the same revision, and the two remain independently versioned.

### 13.3 The comparison

A reader compares `nostdb_format_version` against the versions it supports:

| Condition | Behavior |
| --- | --- |
| supported | open |
| below the minimum supported | `NOSTDB_FORMAT_UNSUPPORTED`, with the version in the diagnostic |
| above the maximum supported | `NOSTDB_FORMAT_UNSUPPORTED`, never a best-effort parse |

A reader MUST NOT guess a layout from a version it does not know, even when the
header checksum passes. `header_length` alone is not permission to interpret
unknown fields.

Migration, when offered, MUST write a new file and promote it atomically, leaving
the original readable until promotion succeeds.

## 14. What the container MUST NOT contain

- plaintext credentials, tokens, private keys, or PEM content;
- executable plugin code;
- any requirement that a daemon be running;
- a file path used as the permanent identity of an Entity or Schema.

A readable `.nostdb` MUST open in Embedded Mode with no daemon.

## 15. Conformance

Fixtures live in [`../fixtures/nostdb/header`](../fixtures/nostdb/header) as
commented hexadecimal so that a container fixture stays reviewable in a diff. Each
`.hex` file pairs with an `.expected` file declaring `accept` or the diagnostic
code a reader must raise.

Hexadecimal fixture syntax: whitespace is insignificant, `#` starts a comment
that runs to the end of the line, and the remaining characters MUST form an even
number of hexadecimal digits.
