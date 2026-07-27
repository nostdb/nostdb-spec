# The `.nost` language contract

Contract key: `nost_language_version`
Current version: 2
Status: normative

`.nost` is the optional canonical human-readable representation of a NostDB
graph. It exists for direct editing, review, and Git management. It is never
required in Embedded or Server Mode.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative.

## 1. What this document owns

This document and [`../grammar/nost.ebnf`](../grammar/nost.ebnf) define the
`.nost` language. [`../grammar/nost.pest`](../grammar/nost.pest) is a reference
encoding, and [`../fixtures/nost`](../fixtures/nost) is the conformance gate.

An implementation MAY use any parser technology. It MUST reproduce every fixture
outcome, and it MUST NOT accept input this document rejects.

Parsing, comment-preserving CST construction, error recovery, canonical
formatting, and synchronization are implemented in `nostdb-core`. This document
specifies what they must produce; it does not implement them.

### 1.1 What changed in version 2

Version 2 is not compatible with version 1. An implementation reading `@nost 1`
MUST reject it with `NOST_VERSION_UNSUPPORTED` rather than parse it
best-effort.

| Change | Version 1 | Version 2 |
| --- | --- | --- |
| Module declarations | every node and edge sat inside a `module` block | removed; a node or edge is a top-level declaration |
| Schemas | not expressible | `schema` declares typed fields, and its name is a label |
| Node type | one or more free-form labels, `node n :A :B` | one or more schema names, `node n: A, B` |
| Edge shape | `edge e id "…" :R (a -> b)` | `edge a -> b :R`, with no declaration name |
| Record identifier | mandatory `id "…"` clause holding any string | optional reserved `id` property holding a prefixed UUID |
| Field separator | newline | comma, with an optional trailing comma |
| Ownership and provenance | not expressible | `@by` and `@evidence` blocks |

Only `nost_language_version` moves. The `.nostdb` format, settings, provider,
plugin, and server contracts are versioned independently and none of them
changes because of this revision.

## 2. Encoding

A `.nost` file MUST be valid UTF-8. A byte-order mark MUST be rejected. The line
terminator is U+000A. A parser MUST accept U+000D immediately before U+000A as
trivia, and a canonical writer MUST NOT emit U+000D.

A file MUST end with exactly one U+000A.

## 3. Trivia

Trivia is whitespace and comments. It is permitted between any two tokens and
inside none.

```nost
// A line comment runs to the end of the line.
/* A block comment does not nest and ends at the first "*/". */
```

An unterminated block comment is `NOST_PARSE_ERROR`.

Trivia carries no graph meaning, but comments are not discardable: a
comment-preserving CST MUST retain every comment and its attachment, because a
formatter has to reproduce it. Attachment is defined in section 10.

## 4. Reserved words

These are never identifiers:

```text
as   bytes   datetime   edge   false   node   schema   true
```

A reserved word where an identifier is required is `NOST_PARSE_ERROR`. Reserving
them keeps a declaration head unambiguous without lookahead.

`id`, `source`, and `module` were reserved in version 1 and are not reserved
now. `id` and `source` became ordinary property and evidence keys, and a key is
an identifier. `module` went with the declaration it introduced.

A scalar type name such as `string` is not reserved either. It is only ever read
in the type position of a schema field, where an identifier is not permitted, so
reserving it would forbid a perfectly clear property named `string` for nothing.

## 5. File shape

A file is a version header, then link declarations, then schema, node, and edge
declarations:

```nost
@nost 2

@link "./packages/child"
@link "./packages/shared" as shared

schema Function {
  name: string,
  language?: string,
}

node login: Function {
  name: "login",
}
```

A link declaration after a schema, node, or edge declaration is
`NOST_PARSE_ERROR`. Fixing that order makes the canonical form reachable by
sorting alone.

Schema, node, and edge declarations MAY interleave, and a node MAY name a schema
declared later in the file. Schema resolution is semantic rather than
positional, so requiring a declaration order would reject readable files for no
benefit.

### 5.1 Version header

```nost
@nost 2
```

The header is mandatory and first. A version above the highest supported, or a
version this implementation no longer supports, is `NOST_VERSION_UNSUPPORTED`,
never a best-effort parse.

### 5.2 Link declarations

```nost
@link "./packages/child"
@link "./packages/child" as child
```

Both forms are valid. An alias is recommended and optional. The alias lives in
`.nost` and `.nostdb`, never in settings.

The link identity is the source string after provider-specific
canonicalization. There is no `link_id` and no target database identifier.

Two links that canonicalize to one locator are `NOST_DUPLICATE_LINK_SOURCE`. Two
links claiming one alias are `NOST_DUPLICATE_LINK_ALIAS`.

### 5.3 Schema declarations

```nost
schema Function {
  name: string,
  language?: string,
  labels?: string[],
}
```

A schema declares the typed fields a record carrying it may hold. Its name is
also the record's label, so a query matches on it with no additional syntax and
the model's requirement that every node carry at least one label is satisfied
without a special case.

Two schemas sharing a name are `NOST_DUPLICATE_SCHEMA_NAME`.

A field is required unless its key is followed by `?`. The marker is written on
the key rather than on the type because the choice is whether the property
exists: a stored null is unrepresentable, so `string?` would say something the
model cannot represent.

#### 5.3.1 Field types

| Type | Property value | Notes |
| --- | --- | --- |
| `boolean` | Boolean | — |
| `integer` | Integer | signed 64-bit |
| `double` | Float | IEEE 754 binary64, finite |
| `string` | String | — |
| `bytes` | Bytes | — |
| `datetime` | DateTime | RFC 3339 |
| `T[]` | List of `T` | scalars only; `T[][]` does not exist |

`double` rather than `float` because the value is a binary64. There is exactly
one name per model type, so a canonical writer never has to choose between
spellings.

An unknown type name is `NOST_PARSE_ERROR`, because the grammar admits only the
names above in type position.

#### 5.3.2 Edge schemas and endpoint constraints

```nost
schema CALLS (Function -> Function) {
  since?: datetime,
}
```

An endpoint constraint names the schemas the edge's source and target records
must carry, and makes the schema edge-only. A schema without one may describe a
node or an edge, and its use decides which.

#### 5.3.3 Validation is soft, and a schema is open

A record that names a schema and does not satisfy it raises
`NOST_SCHEMA_VIOLATION`, a **warning**. This matches the root PRD section 11.6,
which permits soft schema validation and reserves hard rejection for explicit
Constraints.

A record violates a schema by omitting a required field, or by giving a declared
field a value of the wrong type. It does **not** violate one by carrying a
property the schema does not declare: a schema is open. An analyzer routinely
attaches properties a hand-written schema did not anticipate, and treating those
as violations would fill a build with warnings that say nothing.

A record MAY name a schema that is never declared. The name is then an
unvalidated label, which follows from schemas being optional. A consequence is
accepted rather than solved: a misspelled schema name is indistinguishable from
an intentional bare label, and it silently becomes an unvalidated label. No
syntax can tell the two apart while schemas remain optional.

### 5.4 Node declarations

```nost
node login: Function, Public {
  name: "login",
}
```

A node names one or more schemas, separated by commas. Each name is one of the
node's labels. Labels are a set: order is not meaning, and a repeated name is
redundant rather than an error.

A node validates against the union of the schemas it names. Where two of them
declare the same key with different types, the record is `NOST_SCHEMA_CONFLICT`.
Where one marks a key optional and another does not, the key is required:
taking the stricter reading is the only rule that cannot silently weaken a
declaration its author wrote.

### 5.5 Edge declarations

```nost
edge login -> database :CALLS {}
```

An edge names exactly one schema, because a relation type is single valued, and
carries exactly two endpoints. An edge with a missing endpoint is
unrepresentable: there is no syntax for a null endpoint.

An edge carries no declaration name. Nothing referenced one in version 1 either,
because an endpoint names a node. Two edges that share endpoints and relation
are therefore distinguishable only by `id`. Both are kept, because the graph is
a multigraph, and a canonical writer orders them deterministically.

### 5.6 Contribution and evidence blocks

```nost
node login: Function {
  name: "login",

  @by analyzer "rust-structural" "0.1.0" unit "u_0198a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b" {
    @evidence {
      source: "./",
      path: "src/auth.rs",
      digest: "sha256:cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
      range: "12:1:340-40:2:1180",
      method: deterministic,
      confidence: extracted,
    }
  }
}
```

A contribution records who produced a record's facts and on what evidence.
Without it, a database exported to `.nost` and read back would collapse every
contribution into one user-owned contribution, which would break the root PRD
section 11.3 guarantee that an analyzer refresh preserves user edits.

Contribution blocks follow the properties in a record block and are not comma
separated, because they are blocks rather than fields.

Three owners exist:

| Owner | Syntax | Evidence |
| --- | --- | --- |
| analyzer | `analyzer "<name>" "<version>"` | required |
| AI analysis | `ai "<contract-digest>"` | required |
| user | `user` | optional; the user is the evidence |

`unit` names the source unit the contribution derives from. It is optional and
defaults to the nil source unit, which is the unit a change made outside any
analyzed source belongs to. An analyzer contribution SHOULD state one, because
the pair of owner and source unit is exactly what a refresh replaces.

An evidence block accepts these keys:

| Key | Value | Required |
| --- | --- | --- |
| `source` | string; a canonical source locator | yes |
| `digest` | string; `algorithm:hex`, lower case, at least 32 hexadecimal digits | yes |
| `method` | `deterministic`, `ai_inferred`, or `user_declared` | yes |
| `confidence` | `extracted`, `inferred(<score>)`, or `ambiguous(<score>)` | yes |
| `revision` | string; the immutable revision the source resolved to | no |
| `path` | string; the path within the source | no |
| `range` | string; `line:column:offset-line:column:offset` | no |
| `producer` | string | no when the owner is an analyzer |
| `producer_version` | string | no when the owner is an analyzer |

`producer` and `producer_version` default to the analyzer's name and version.
They are separate fields in the model because evidence may come from a producer
other than the owner, but they are the same value often enough that repeating
them on every block would be noise. When the owner is `ai` or `user` there is no
name to inherit, so both are required.

A score is a float within `0.0..=1.0`. `extracted` carries none, because a fact
read directly out of source has nothing to weigh.

A missing required key, an unknown key, a value of the wrong shape, or a score
outside the range is `NOST_INVALID_EVIDENCE`.

## 6. Endpoint reference forms

Three forms exist:

| Form | Syntax | Meaning |
| --- | --- | --- |
| local | `database` | a declaration in this file |
| aliased | `shared::authorize` | a declaration in a linked source, by that link's alias |
| locator | `"./packages/child"::handle` | a declaration in a linked source, by canonical locator |

```nost
edge login -> database :CALLS {}
edge login -> shared::authorize :CALLS {}
edge login -> "./packages/child"::handle :CALLS {}
```

The locator form is the aliasless external reference that the root PRD section
13.2 delegates to this contract. It lets an aliasless link participate in an
explicit edge without forcing an alias to be invented.

An aliased endpoint naming an undeclared alias is `NOST_UNKNOWN_LINK_ALIAS`. A
locator endpoint whose locator matches no link declaration is also
`NOST_UNKNOWN_LINK_ALIAS`, because both name a link that does not exist.

An endpoint that resolves to no declaration in an available source is
`NOST_UNRESOLVED_ENDPOINT`, a warning. The Engine then creates a Placeholder
Node. It MUST NOT store an Edge with a null endpoint, and it SHOULD preserve the
Placeholder identifier when the reference later resolves.

Aliases and locators reference linked sources, which are read-only from the root
transaction.

## 7. Property values

```nost
node example: Sample {
  flag: true,
  count: 42,
  ratio: 0.75,
  scaled: -1.5e-3,
  name: "login",
  payload: bytes"deadbeef",
  seen_at: datetime"2026-07-26T09:00:00Z",
  tags: ["auth", "entry"],
}
```

| Type | Syntax | Rules |
| --- | --- | --- |
| Boolean | `true`, `false` | — |
| Integer | `42`, `-7` | signed 64-bit; overflow is `NOST_INTEGER_OUT_OF_RANGE` |
| Float | `0.75`, `-1.5e-3` | MUST be finite; otherwise `NOST_NON_FINITE_NUMBER` |
| String | `"text"` | escapes below |
| Bytes | `bytes"deadbeef"` | hexadecimal, even digit count |
| DateTime | `datetime"2026-07-26T09:00:00Z"` | RFC 3339; otherwise `NOST_INVALID_DATETIME` |
| List | `["a", "b"]` | scalars only, no nesting, no trailing comma |

Properties are separated by commas. A trailing comma after the last property is
accepted; a canonical writer does not emit one. A list literal admits no
trailing comma, which keeps a one-element list unambiguous.

String escapes are `\"`, `\\`, `\n`, `\r`, `\t`, and `\u{H...}`. A raw U+000A or
U+000D inside a string literal is `NOST_PARSE_ERROR`.

`bytes` and `datetime` are tagged literals so a byte string and a timestamp stay
distinguishable from an ordinary string without inspecting the value.

### 7.1 There is no null

The grammar has no null literal, so a stored null is unrepresentable. This
matches the root PRD: in a query, `null` means missing or non-applicable, and
assigning `null` removes a property. In `.nost`, a property that should not
exist is simply absent.

`a: null` is `NOST_PARSE_ERROR`, not a property set to nothing.

### 7.2 The confidence property key

A property named `confidence_score` carries a confidence value and MUST be a float
within `0.0..=1.0`. Any other value is `NOST_NON_FINITE_NUMBER`.

The key is named here rather than left to convention, because the range rule is
unenforceable without knowing which property it governs. An ordinary float property
is not range-restricted.

### 7.3 Duplicate keys

A property block that sets one key twice is `NOST_DUPLICATE_PROPERTY_KEY`. An
implementation MUST NOT silently keep the last value.

### 7.4 Reserved property keys

Two keys are interpreted by the Engine rather than stored as ordinary
properties:

| Key | Type | Meaning |
| --- | --- | --- |
| `id` | `string` | the opaque record identifier, in the form of section 8.1 |
| `labels` | `string[]` | additional labels beyond the schema names |

Both are optional. Omitting `id` lets the Engine mint one, which is what the
root PRD section 11.2 means by an identifier a user *may* declare. Omitting
`labels` leaves the schema names as the record's only labels.

They are reserved keys rather than dedicated syntax so that one rule covers
everything inside a record block.

A schema MAY declare `labels?: string[]`, which documents that its records carry
extra labels and type-checks the value like any other declared field. Declaring
it is not a precondition for using it. Requiring the declaration was considered
and dropped: schemas are optional and open, so a record naming no schema could
then never carry a label, which would be an asymmetry with nothing behind it.

An `id` value that is not a kind prefix followed by a canonical UUID is
`NOST_INVALID_ID`. Two declarations claiming one identifier are
`NOST_DUPLICATE_ID`.

## 8. Identifiers

An identifier starts with a Unicode scalar having `XID_Start`, or `_`, and
continues with scalars having `XID_Continue`, per UAX #31. Identifiers are
case-sensitive.

Schema names, relation names, property keys, evidence keys, declaration names,
and aliases are all identifiers, so the same rule applies everywhere.

### 8.1 The record identifier form

A record identifier is a two-character kind prefix followed by a UUID in
canonical lower-case text:

```text
n_0198a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b
```

| Prefix | Kind |
| --- | --- |
| `n_` | node |
| `e_` | edge |
| `u_` | source unit |

The prefix is part of the form so a node identifier is not silently accepted
where an edge identifier is required.

A minted identifier is a UUID version 7. An implementation MUST accept any
well-formed UUID in a stated identifier, including one it did not mint, because
a `.nost` file may carry identifiers produced by an older or a different
implementation. It MUST NOT depend on the version nibble of a stated value.

The form is specified here rather than left to each implementation because a
`.nost` file states identifiers and every implementation reads them. An
unspecified form is exactly the divergence this repository exists to prevent.

## 9. Semantic rules the grammar cannot express

A file that parses is not necessarily valid. An implementation MUST also enforce:

| Rule | Diagnostic |
| --- | --- |
| supported language version | `NOST_VERSION_UNSUPPORTED` |
| unique link alias | `NOST_DUPLICATE_LINK_ALIAS` |
| unique canonical link source | `NOST_DUPLICATE_LINK_SOURCE` |
| unique declaration name per scope | `NOST_DUPLICATE_DECLARATION_NAME` |
| unique schema name | `NOST_DUPLICATE_SCHEMA_NAME` |
| unique record identifier per file | `NOST_DUPLICATE_ID` |
| record identifier is a prefixed canonical UUID | `NOST_INVALID_ID` |
| unique property key per block | `NOST_DUPLICATE_PROPERTY_KEY` |
| two schemas on one record agree on a shared field type | `NOST_SCHEMA_CONFLICT` |
| a record satisfies the schemas it names | `NOST_SCHEMA_VIOLATION`, a warning |
| a contribution and its evidence are well formed | `NOST_INVALID_EVIDENCE` |
| declared alias or locator exists | `NOST_UNKNOWN_LINK_ALIAS` |
| integer fits in 64 bits | `NOST_INTEGER_OUT_OF_RANGE` |
| float is finite, and a `confidence_score` property is within 0.0 to 1.0 | `NOST_NON_FINITE_NUMBER` |
| datetime is RFC 3339 | `NOST_INVALID_DATETIME` |
| endpoint resolves, else Placeholder | `NOST_UNRESOLVED_ENDPOINT` |

Every code is registered in [`../diagnostics.json`](../diagnostics.json). Every
diagnostic MUST carry a source range.

Fixtures under `fixtures/nost/invalid-semantic` parse successfully and declare
the code an implementation must raise. This repository checks that the declared
code is registered; `nostdb-core` proves the diagnostic itself.

## 10. Canonical form

The canonical form exists so that a second format pass is byte-identical and a
Git diff reflects graph change rather than layout churn.

A canonical writer MUST:

1. emit the version header, then links, then schemas, then nodes, then edges;
2. sort links by canonical locator, ascending by Unicode scalar value;
3. sort schemas and nodes by name, and sort edges by source endpoint, then
   target endpoint, then relation name, then identifier;
4. sort schema fields, property keys, and label values ascending by Unicode
   scalar value, and sort a record's contribution blocks by owner and then
   source unit;
5. indent with two spaces per level, and never with tabs;
6. place one declaration, one field, and one property per line;
7. separate fields and properties with a comma, and emit no trailing comma;
8. emit an empty field, record, or contribution block as `{}`;
9. emit exactly one blank line between sibling **block** declarations, meaning
   schema, node, and edge declarations within a file, and none before the first
   or after the last. Single-line directives form one group: link declarations
   are separated from the version header and from the first block declaration by
   one blank line, and from each other by none;
10. terminate the file with exactly one U+000A;
11. write atomically, and reserialize the whole file rather than patching it.

Comment attachment: a comment on its own line attaches as a leading comment to
the next declaration, field, or property in the same block, or to the enclosing
block's end if none follows. A comment after a declaration, field, or property
on the same line attaches to it as a trailing comment. A canonical writer MUST
preserve both kinds and their attachment.

Sorting is by Unicode scalar value, not locale collation, so the canonical form
does not depend on environment.

## 11. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/nost`](../fixtures/nost). Each fixture pairs with an `.expected`
file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `valid/` | `accept` | parses |
| `invalid-syntax/` | `reject` | fails with the declared `code` |
| `invalid-semantic/` | `accept_then_diagnose` | parses, and raises the declared `code` |

### 11.1 Normative and informative keys

`outcome` and `code` are normative. Every implementation MUST reproduce them.

`reference_line` and `reference_column` are **informative**. They record where the
reference encoding in `grammar/nost.pest` reports the failure, and they exist to
catch an unintended change in that encoding. An implementation MUST NOT be judged
against them.

The reason is that the position at which a parser detects a syntax error is an
artifact of its technology. A PEG reports the furthest position it reached while
backtracking, a table-driven parser reports the offending token, and a parser with
error recovery may report several positions. All three are correct. Requiring one
exact column would bind every implementation to one parser design, which is
precisely what this repository must avoid.

What every implementation MUST do is reject the input and attach a source range
to the diagnostic. Where inside the construct that range begins is an
implementation's own quality decision.

`note` is informative prose explaining what the fixture pins down.

### 11.2 Adding fixtures

Add a fixture with every language change. A fixture that no implementation can
fail is documentation rather than a conformance test, so an accepted fixture
SHOULD exercise a construct no other accepted fixture already covers.

## 12. Synchronization with the database

`.nost` is a representation of a database, so the two can disagree. This section
defines when each is authoritative. It is placed after conformance to avoid
renumbering earlier sections that other documents reference.

Synchronization compares a **baseline**, never wall-clock time:

```text
database_generation    the generation the .nost file was produced from
database_digest        digest of the database at that generation
nost_content_digest    digest of the .nost text as produced
```

A timestamp comparison would be wrong here, because two machines and two clocks can
disagree while both files are legitimate. A generation advances only on a successful
commit, and a digest changes only when bytes change.

### 12.1 The state machine

| Database since baseline | `.nost` since baseline | Result |
| --- | --- | --- |
| unchanged | unchanged | no-op |
| unchanged | changed | validate the `.nost` file, then atomically update the database |
| changed | unchanged | `NOST_SOURCE_STALE`; the file must be regenerated explicitly |
| changed | changed | `SYNC_CONFLICT`; **modify neither representation** |

`SYNC_CONFLICT` is not a merge failure to retry. Both sides hold work derived from
one baseline, and choosing either would discard the other silently, so the Engine
stops and reports. Resolving it is a human decision.

`NOST_SOURCE_STALE` is not corruption. The database is authoritative and readable;
the file simply no longer describes it. Regeneration is explicit rather than
automatic, because a stale file may still hold edits its author has not applied.

### 12.2 Requirements

An implementation MUST:

- validate syntax, references, and semantic rules before mutating anything;
- adopt a changed `.nost` file as one atomic transaction, so a failure leaves the
  previous database generation readable;
- detect a source edit that lands during synchronization, by re-checking the content
  digest before commit;
- preserve comments through the canonical reserialization synchronization performs;
- preserve every contribution and its evidence, and never merge two owners into
  one;
- never modify a record reached through a link, which is read-only;
- report created, updated, deleted, and unresolved deltas.

An implementation MUST NOT:

- resolve a conflict by preferring the newer file, the larger file, or either
  representation by default;
- regenerate a stale `.nost` file as a side effect of an unrelated command.

### 12.3 Determinism this depends on

Adopting a `.nost` file and re-exporting it MUST produce the same bytes, and
committing identical graph content MUST produce an identical database digest.
Without both, a baseline comparison would report a change where none exists and
synchronization would never reach the no-op state.

Minting an identifier is the one step that is not reproducible, because a minted
identifier is a UUID version 7 and carries a timestamp and random bits. That does
not weaken either requirement above. Minting happens only for a record with no
stated `id`, the minted value is written into the `.nost` file on the next
export, and every later round trip carries it. Both requirements are therefore
about content that already has identifiers, which is every file a canonical
writer has produced.
