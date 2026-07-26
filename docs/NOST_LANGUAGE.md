# The `.nost` language contract

Contract key: `nost_language_version`
Current version: 1
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
as   bytes   datetime   edge   false   id   module   node   source   true
```

A reserved word where an identifier is required is `NOST_PARSE_ERROR`. Reserving
them keeps a declaration head unambiguous without lookahead.

## 5. File shape

A file is a version header, then link declarations, then module declarations, in
that order:

```nost
@nost 1

@link "./packages/child"
@link "./packages/shared" as shared

module auth id "m_01J" source "src/auth.rs" {
  node login id "n_01J" :Function {
    name: "login"
  }
}
```

A link declaration after a module declaration is `NOST_PARSE_ERROR`. Fixing the
order makes the canonical form reachable by sorting alone.

### 5.1 Version header

```nost
@nost 1
```

The header is mandatory and first. A version above the highest supported is
`NOST_VERSION_UNSUPPORTED`, never a best-effort parse.

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

### 5.3 Module declarations

```nost
module auth id "m_01J" source "src/auth.rs" { }
```

`id` carries the opaque persisted record identifier. `source` is optional,
because a source path is a mutable locator rather than an identity, and a
user-authored or AI-proposed module need not have one.

A module holds nodes and edges. Version 1 defines no module-level properties.

### 5.4 Node declarations

```nost
node login id "n_01J" :Function :Public {
  name: "login"
}
```

A node carries one or more labels. Labels are a set: order is not meaning, and a
repeated label is redundant rather than an error.

### 5.5 Edge declarations

```nost
edge login_calls_db id "e_01J" :CALLS (login -> database) { }
```

An edge carries exactly one relation label, because a relation type is single
valued, and exactly two endpoints. An edge with a missing endpoint is
unrepresentable: there is no syntax for a null endpoint.

## 6. Endpoint reference forms

Three forms exist:

| Form | Syntax | Meaning |
| --- | --- | --- |
| local | `database` | a declaration in this file |
| aliased | `shared::authorize` | a declaration in a linked source, by that link's alias |
| locator | `"./packages/child"::handle` | a declaration in a linked source, by canonical locator |

```nost
edge a id "e_1" :CALLS (login -> database) { }
edge b id "e_2" :CALLS (login -> shared::authorize) { }
edge c id "e_3" :CALLS (login -> "./packages/child"::handle) { }
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
node example id "n_1" :Sample {
  flag: true
  count: 42
  ratio: 0.75
  scaled: -1.5e-3
  name: "login"
  payload: bytes"deadbeef"
  seen_at: datetime"2026-07-26T09:00:00Z"
  tags: ["auth", "entry"]
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

## 8. Identifiers

An identifier starts with a Unicode scalar having `XID_Start`, or `_`, and
continues with scalars having `XID_Continue`, per UAX #31. Identifiers are
case-sensitive.

Labels, relation names, property keys, declaration names, and aliases are all
identifiers, so the same rule applies everywhere.

## 9. Semantic rules the grammar cannot express

A file that parses is not necessarily valid. An implementation MUST also enforce:

| Rule | Diagnostic |
| --- | --- |
| supported language version | `NOST_VERSION_UNSUPPORTED` |
| unique link alias | `NOST_DUPLICATE_LINK_ALIAS` |
| unique canonical link source | `NOST_DUPLICATE_LINK_SOURCE` |
| unique declaration name per scope | `NOST_DUPLICATE_DECLARATION_NAME` |
| unique record identifier per file | `NOST_DUPLICATE_ID` |
| unique property key per block | `NOST_DUPLICATE_PROPERTY_KEY` |
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

1. emit the version header, then links, then modules;
2. sort links by canonical locator, ascending by Unicode scalar value;
3. sort modules by declaration name, and within a module emit nodes before edges,
   each sorted by declaration name;
4. sort labels and property keys ascending by Unicode scalar value;
5. indent with two spaces per level, and never with tabs;
6. place one declaration and one property per line;
7. emit an empty property block as `{}`;
8. emit exactly one blank line between sibling **block** declarations, meaning
   modules within a file and nodes and edges within a module body, and none before
   the first or after the last. Single-line directives form one group: link
   declarations are separated from the version header and from the first module by
   one blank line, and from each other by none;
9. terminate the file with exactly one U+000A;
10. write atomically, and reserialize the whole file rather than patching it.

Comment attachment: a comment on its own line attaches as a leading comment to
the next declaration or property in the same block, or to the enclosing block's
end if none follows. A comment after a declaration or property on the same line
attaches to it as a trailing comment. A canonical writer MUST preserve both
kinds and their attachment.

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
- never modify an imported read-only module;
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
