# The graph change set contract

Contract key: `change_set_version`
Current version: 1
Status: normative

A change set is a batch of proposed graph changes that a producer hands to the Engine. An
analyzer builds one from source; an AI Skill proposes one; a person may write one by hand
and apply it with `nostdb apply`.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

This document defines the on-disk document: its shape, its operations, and what makes one
invalid before any database is consulted. [`../fixtures/change_set`](../fixtures/change_set)
is the conformance gate.

It does not own the graph model, the identifier form, or the diagnostic codes a rejection
carries. Those belong to `nostdb_format_version`, to the `.nost` language contract, and to
the contract that owns each code.

### 1.1 A change set is a proposal, not a transaction

Nothing here is a permission. A set that satisfies every rule in this document may still be
refused by the Engine — for a stale baseline, a missing endpoint, an owner that may not
touch a record, a Constraint it would break. This document defines only what can be decided
by reading the document itself.

The distinction matters because a producer must not be able to widen its own authority by
writing a well-formed file. The set says what it wants; the Engine decides what happens.

### 1.2 One set, one owner

Every contribution in a set belongs to the owner the set declares. A set cannot contain one
operation attributed to an analyzer and another to a user.

The reason is the ownership rule the whole model rests on: a refresh replaces only its own
producer's contributions. If one document could carry two owners, applying it would mean two
replacement scopes in one transaction, and a failure partway through would leave a state
neither producer asked for.

## 2. Document shape

```json
{
  "change_set_version": 1,
  "base_generation": 7,
  "owner": { "kind": "analyzer", "name": "rust", "version": "1" },
  "source_snapshot": "tree:sha256:0f1e2d...",
  "operations": [
    {
      "operation": "upsert_node",
      "labels": ["Function"],
      "properties": { "name": "login" },
      "source_unit": "u_0198a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b",
      "evidence": []
    }
  ]
}
```

Every member is required. `operations` MUST NOT be empty: proposing nothing is a mistake in
whatever produced the document, and accepting it would turn that mistake into silence.

### 2.1 `base_generation`

The database generation the set was computed against.

An implementation MUST refuse a set whose `base_generation` does not match the database. A
set resolves identifiers against a graph it read; applying it to a graph that has moved
means resolving against something the producer never saw, which is how work nobody reviewed
gets overwritten.

### 2.2 `owner`

| `kind` | Further members | Meaning |
| --- | --- | --- |
| `analyzer` | `name`, `version` | a deterministic analyzer |
| `ai_analysis` | `contract_digest` | AI analysis, identified by the contract it ran under |
| `user` | none | a person |

An analyzer's version is part of its identity. Upgrading an analyzer MUST NOT silently adopt
the previous version's facts as the new version's own, and carrying the version in the owner
is what prevents it.

An analyzer-owned or AI-owned operation MUST carry evidence. A user-owned one need not,
because the user is the evidence.

### 2.3 `source_snapshot`

The immutable source the changes were derived from, as a non-empty string. A provider
supplies a commit for a remote source; a local working tree has no such identity and states
a content-derived revision instead, which MUST be distinguishable from a commit.

## 3. Operations

Applied in order. Each carries an `operation` member naming its kind.

### 3.1 `upsert_node`

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | no | the record to update, or absent to have one assigned |
| `labels` | array of string | yes | at least one |
| `properties` | object | no | a key MUST NOT repeat |
| `source_unit` | string | yes | what the contribution derives from |
| `evidence` | array | yes | provenance, possibly empty for a user |

`id` absent means "assign one", which is what a producer discovering a new symbol does.
`id` present names an existing record or asserts a chosen identifier.

Two operations in one set MUST NOT upsert the same `id`. A set that did would be two
statements about one record with no stated order of precedence, and picking one is a guess.

### 3.2 `upsert_edge`

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | no | as for a node |
| `source` | endpoint | yes | never absent |
| `target` | endpoint | yes | never absent |
| `relation` | string | yes | exactly one |
| `properties` | object | no | a key MUST NOT repeat |
| `source_unit` | string | yes | |
| `evidence` | array | yes | |

An endpoint is `{"local": "n_..."}` or `{"source": "./child", "local": "n_..."}` for a record
in a linked database. An Edge always has two non-null endpoints, so neither member may be
absent, null, or empty.

An implementation MUST refuse an operation whose endpoint names a linked source. A write
affects only the root database.

### 3.3 `remove_contribution`

| Member | Type | Required |
| --- | --- | --- |
| `source_unit` | string | yes |

Removes the set owner's contribution for one source unit, from every record that carries
one. It cannot name another owner: the owner is the set's, which is the narrowest thing a
producer is permitted to withdraw.

A record left with no contributions is removed, and every Edge that loses an endpoint is
removed with it.

### 3.4 `resolve_placeholder`

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `placeholder` | string | yes | the record being resolved |
| `outcome` | object | yes | `{"preserved": true}` or `{"replacement": "n_..."}` |
| `source_unit` | string | yes | |
| `evidence` | array | yes | |

Preserving the identifier is the preferred outcome, because every Edge already pointing at
the Placeholder stays valid. A replacement is recorded as an explicit identity replacement
rather than two identities being merged silently.

### 3.5 `upsert_link` and `remove_link`

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `source` | string | yes | the canonical locator, which is the link's identity |
| `alias` | string | no | `upsert_link` only |

An alias lives in the graph and never in settings. Two operations MUST NOT declare the same
locator, and MUST NOT claim the same alias for two locators. One set MUST NOT both declare
and remove one locator.

## 4. Rejected documents

An implementation MUST reject, rather than repair, each of the following with
`CHANGE_SET_INVALID`, except an unsupported version, which is `CHANGE_SET_VERSION_UNSUPPORTED`:

| Condition | Why |
| --- | --- |
| `change_set_version` absent, or not a supported version | the version is what makes every other rule interpretable |
| the document is not a JSON object, or a required member is absent | there is nothing to read |
| `operations` is empty | proposing nothing is a mistake in the producer, not a no-op |
| an operation names no `operation` kind, or an unknown one | guessing which was meant is how a typo becomes a graph change |
| a node draft has no label | a record with no label cannot be found by any query that would maintain it |
| two operations upsert one `id` | two statements about one record with no stated precedence |
| a property key repeats within one operation | the same reason |
| an edge is missing an endpoint | an Edge always has two |
| two link operations conflict, as section 3.5 describes | the locator is the link's identity, so two claims on it are two links |
| an analyzer-owned or AI-owned operation carries no evidence | a fact with nothing behind it is indistinguishable from one somebody made up |

Rejection reports every problem found rather than the first, so a producer can fix a batch
in one pass.

## 5. Versions

`change_set_version` MUST be present and MUST be a positive integer. A version this build
does not read is refused with `CHANGE_SET_VERSION_UNSUPPORTED`, in both directions.

Unlike settings, a change set is not preserved and rewritten, so there is nothing to be
gained by reading the part of one that is understood. A writer that cannot state what an
operation means MUST NOT apply it.

## 6. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/change_set`](../fixtures/change_set). Each fixture pairs with an `.expected`
file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `valid/` | `accept` | the document is read and its operations are understood |
| `invalid/` | `reject` | the document is refused |

A `valid` fixture is not a promise that applying it succeeds. Section 1.1 is the whole
reason: these fixtures test what can be decided by reading the document, and everything a
database would decide is out of their reach.
