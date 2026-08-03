# The query result envelope contract

Contract key: `result_version`
Current version: 2
Status: normative

A query result leaves the Engine as an envelope: the columns, the rows, a summary of what
happened, and the warnings that did not stop it. Every machine-readable output format the
CLI offers is a rendering of this one shape.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

This document defines the envelope and how each value type is written in it.
[`../fixtures/result`](../fixtures/result) is the conformance gate.

It does not own the query language, which is `query_subset_version`, nor the diagnostic
codes a warning carries, which belong to the contract that owns each code.

### 1.1 What changed in version 2

Version 2 adds one value form, `{"object": {...}}`, and widens a list to hold any value
rather than scalars only. Both follow `nost_language_version` 4, under which a stored
property may be an object.

`supported` lists **2 alone**, and that is not a gap. An implementation of NostDB
*produces* this envelope and never reads one, so listing version 1 as supported would
promise a reader nothing implements. Version 1 describes what earlier builds emitted
rather than an input this one accepts.

A consumer still holding a version 1 reader meets `{"object": ...}` and MUST treat it as
an unknown tag, refusing it exactly as section 3.1 requires of any tag it does not know.
That is the reason the form is tagged rather than bare: an unknown tag is refusable, while
a bare object would have been silently misread as one of the tagged forms.

### 1.2 Data and diagnostics are separate

The envelope carries both, and they never mix. `rows` holds only what the query asked
for; `warnings` holds only what an implementation wants to say about producing it.

A caller reading rows must never have to filter commentary out of them, which is why an
implementation writing this envelope to a stream puts the envelope on standard output and
anything else on standard error.

## 2. Shape

```json
{
  "result_version": 2,
  "columns": ["name", "source"],
  "rows": [
    ["authorize", "github://example/shared/.nostdb/root.nostdb?ref=main"]
  ],
  "summary": {
    "rows": 1,
    "database_generation": 42,
    "linked_databases_opened": 2,
    "partial": true
  },
  "warnings": [
    {
      "code": "LINK_UNAVAILABLE",
      "source": "./packages/legacy",
      "message": "The declared target could not be opened."
    }
  ]
}
```

Every member is required. An empty result writes `"columns": []`, `"rows": []`, and
`"warnings": []` rather than omitting them, so a consumer never has to distinguish
"absent" from "empty".

### 2.1 `columns`

Column names in projection order, as the query named them. A name repeats only if the
query repeated it.

### 2.2 `rows`

An array of arrays. Every row has exactly as many entries as `columns`.

**Row order is meaningful only when the query contained `ORDER BY`.** Without one, an
implementation MAY produce any order, and a consumer that depends on the order it happens
to see is depending on nothing. This restates the rule in the root product contract
because a result format is exactly where somebody would assume otherwise.

### 2.3 `summary`

| Field | Type | Meaning |
| --- | --- | --- |
| `rows` | integer | how many rows `rows` holds |
| `database_generation` | integer | the generation the query read |
| `linked_databases_opened` | integer | how many linked databases were opened, excluding the root |
| `partial` | boolean | whether some declared source could not be reached |

`rows` duplicates the length of `rows`, deliberately: a consumer streaming JSONL sees the
summary without having counted, and a consumer reading JSON can check the two agree.

`partial` is `true` whenever any warning means a source was not fully traversed. Exactly
three warnings do:

| Code | Meaning |
| --- | --- |
| `LINK_UNAVAILABLE` | a declared link could not be opened |
| `LINK_CYCLE` | traversal reached a canonical source it had already opened |
| `LINK_LIMIT_EXCEEDED` | traversal reached a configured depth or database limit |

A result with `partial: true` is still a result: it holds everything that *was* reachable,
which is what the root product contract requires of an unavailable link. A consumer that
treats a partial result as a failure is discarding correct data.

These three codes are owned here rather than by a federation contract, because this is the
document that defines `partial` and `partial` is meaningless without saying which warnings
set it. The same reasoning put `LINKED_DATABASE_READ_ONLY` in the query subset contract.
Federation *behavior* is specified when link resolution lands; these are its result-shaped
edges.

A write query adds a `writes` object:

| Field | Type |
| --- | --- |
| `nodes_created`, `nodes_deleted` | integer |
| `edges_created`, `edges_deleted` | integer |
| `properties_set`, `properties_removed` | integer |
| `labels_added`, `labels_removed` | integer |

`writes` is present only when the query could write. Its absence means the query was a
read; a read MUST NOT report an all-zero `writes` object, because "changed nothing" and
"could not change anything" are different claims.

### 2.4 `warnings`

| Field | Type | Required |
| --- | --- | --- |
| `code` | string; a registered diagnostic code | yes |
| `message` | string | yes |
| `source` | string; a canonical locator | no |
| `range` | object with `line`, `column`, `offset` for `start` and `end` | no |

A warning never changes what the rows mean. Anything that would has to be an error, and
an error produces no envelope at all.

## 3. Value encoding

A stored value has one JSON form, and so does each thing only a query can produce.

| Value | JSON | Note |
| --- | --- | --- |
| null | `null` | missing or non-applicable; a stored property is never null |
| boolean | `true`, `false` | — |
| integer | number | signed 64-bit |
| double | number | finite; a non-finite value is unrepresentable |
| string | string | — |
| bytes | `{"bytes": "<lower-case hex>"}` | tagged, because JSON has no byte string |
| datetime | `{"datetime": "<RFC 3339>"}` | tagged, so it stays distinguishable from a string |
| list | array | holds any value, including lists and objects |
| object | `{"object": {...}}` | tagged, because an entry named `path` must not read as a path |
| node | `{"node": "n_<uuid>"}` | the record identifier |
| relationship | `{"relationship": "e_<uuid>"}` | the record identifier |
| path | `{"path": {"nodes": [...], "relationships": [...]}}` | alternating, in order |

### 3.1 Why the tagged forms

A byte string, a timestamp, a node, and a relationship would all otherwise be strings,
and a consumer could not tell them apart from text that happens to look like one. Tagging
costs four characters and removes a whole class of misreading.

A tagged object has exactly one member. An implementation MUST NOT add a second, because
a consumer distinguishes the forms by that member's name.

An **object property value is tagged for that reason and not for symmetry.** Three of the
tag names — `bytes`, `datetime`, and `node` — are reserved words in `.nost`, so no property
key can ever be one of them. The other three are not: `relationship`, `path`, and `object`
are ordinary identifiers, and a stored object may carry any of them as a key.

Emitted bare, `{"path": "src/main.rs"}` would be a path to every consumer following the
table above, and nothing in the payload could distinguish the two readings. The tag is what
keeps a user's choice of key from changing the type a consumer infers.

That only three of the six collide is not a reason to emit the object bare and reject those
three. A property key is the user's to choose, and a rule forbidding `path` as a key so the
envelope could stay untagged would push a format's problem onto the data.

Inside `{"object": {...}}` the member values are encoded by the same table, recursively,
so a nested byte string is still tagged and a nested object is tagged again.

### 3.2 An integer and a double are both JSON numbers

JSON has one number type, so `1` and `1.0` are the same token to some readers. This is
accepted rather than solved: the query language already treats them as equal, and tagging
every number would make the common case unreadable for the sake of a distinction the
language itself does not draw.

An implementation writing a double whose value is integral SHOULD write it with a decimal
point, so a reader that preserves the distinction sees it.

## 4. The other formats

JSON is the canonical rendering. Three others are defined, and each carries strictly less
than the envelope, which is why JSON is the one a program should ask for.

### 4.1 JSONL

One JSON value per line, terminated by U+000A:

1. a header line: `{"result_version": 1, "columns": [...]}`;
2. one line per row, each a JSON array;
3. a trailer line: `{"summary": {...}, "warnings": [...]}`.

The header comes first so a consumer can bind column names before the first row, and the
trailer last because a summary is not known until the rows are. A consumer MAY stop after
any line; it then has fewer rows, never wrong ones.

### 4.2 CSV

[RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) with a header row of column names.
A value is rendered as its JSON form would be, except that a string is written bare
rather than quoted as JSON, and null is written as an empty field.

CSV carries no summary and no warnings. An implementation MUST write the warnings
somewhere a caller can still see them, which for a command-line tool is standard error.

### 4.3 Table

For a person. No stability is promised: column widths, padding, and truncation MAY change
between versions, and a script MUST NOT parse it. Every other format is stable.

## 5. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/result`](../fixtures/result). Each fixture pairs a `.json` envelope with an
`.expected` file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `valid/` | `accept` | the envelope satisfies every rule above |
| `invalid/` | `reject` | the envelope breaks the stated rule and must be refused |

A fixture in `invalid/` names the rule it breaks in its `note`. These exist so an
implementation writing envelopes can be checked against them, not only one reading them:
a writer that can emit a rejected shape has a defect the reader would otherwise absorb.
