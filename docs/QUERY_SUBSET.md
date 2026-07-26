# The query subset contract

Contract key: `query_subset_version`
Current version: 1
Status: normative

The public query language is an openCypher-compatible subset. NostDB-specific behavior
uses functions, procedures, or CLI commands rather than incompatible syntax, so a query
that runs here means the same thing it would mean elsewhere.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. Why a declared subset rather than best effort

An engine that silently accepts a clause it only partly implements is worse than one that
refuses it. The caller gets an answer that looks authoritative and is wrong, and no
diagnostic ever tells them which part was approximated.

This contract therefore defines the subset as a closed list. Anything outside it is
refused with `CYPHER_UNSUPPORTED` and a source range. An implementation MUST NOT execute
an unsupported construct under a guessed alternative, and MUST NOT silently drop a clause
it cannot honor.

## 2. The subset

### 2.1 Reading

| Construct | Notes |
| --- | --- |
| `MATCH` | node and relationship patterns |
| `OPTIONAL MATCH` | unmatched rows keep `null` bindings |
| variable-length patterns | bounded only, written `*1..5`; see section 4 |
| `WHERE` | on `MATCH`, `OPTIONAL MATCH`, and `WITH` |
| `WITH` | including `DISTINCT`, `ORDER BY`, `SKIP`, `LIMIT` |
| `RETURN` | including `DISTINCT` |
| `ORDER BY` | with `ASC` and `DESC` |
| `SKIP`, `LIMIT` | non-negative integer or parameter |
| `UNWIND` | list to rows |
| `UNION`, `UNION ALL` | operands must project the same column names |
| parameters | written `$name` |

### 2.2 Writing

`CREATE`, `MERGE`, `SET`, `REMOVE`, `DELETE`, and `DETACH DELETE`, against the root
database only. A write naming a linked record is refused with
`LINKED_DATABASE_READ_ONLY`, because linked records are read-only from the root
transaction.

### 2.3 Procedures and functions

`CALL` for procedures, and the `nostdb.*` function namespace. NostDB-specific behavior
lives here rather than in new syntax, so the language stays compatible.

## 3. What is refused

Refused with `CYPHER_UNSUPPORTED`, non-exhaustively: `FOREACH`, `LOAD CSV`, `CALL {}`
subqueries, `EXISTS {}` subqueries, `MATCH` with an unbounded `*` pattern, `CREATE
INDEX` and other schema commands, `USE` and other multi-database syntax, list
comprehensions, pattern comprehensions, `CASE`, `shortestPath`, `allShortestPaths`, and
any clause not named in section 2.

The list is non-exhaustive by design. The subset is the closed list; everything not on it
is refused, so a construct nobody anticipated is refused rather than accidentally
accepted.

## 4. Variable-length patterns are bounded

A variable-length pattern MUST declare an upper bound. `*1..5` is accepted; `*`, `*1..`,
and `*..5` are refused.

An unbounded traversal over a federated graph has no cost ceiling, and the product
promises bounded query work against untrusted input. A default bound would be worse than
a refusal: the caller would receive a truncated answer that looks complete.

## 5. Result ordering

Result order is undefined unless the query contains `ORDER BY`. An implementation MAY
return rows in any order, and a caller MUST NOT depend on the order it happens to observe.

This is stated as a rule rather than left implicit so that a caller relying on incidental
order is relying on something the contract never promised.

## 6. Diagnostics

| Code | Meaning |
| --- | --- |
| `CYPHER_UNSUPPORTED` | the query uses a construct outside the subset; it did not execute |
| `CYPHER_SEMANTIC_ERROR` | the query is in the subset but meaningless: an unbound variable, a type mismatch, mismatched `UNION` columns, a negative `SKIP` |

Both carry a source range. Both mean nothing executed.

The distinction matters to a caller: `CYPHER_UNSUPPORTED` may become supported in a later
version, so retrying against a newer build is reasonable. `CYPHER_SEMANTIC_ERROR` means
the query is wrong, and retrying will not help.

## 7. Conformance

Fixtures live in [`../fixtures/cypher`](../fixtures/cypher). Each pairs with an
`.expected` file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `supported/` | `accept` | parses |
| `unsupported/` | `reject` | refused with the declared `code` |

The suite is normative for what it contains and does not claim to be exhaustive. It
currently covers reading, because reading is what the first implementation increment
accepts; write-clause fixtures arrive with write support. A construct absent from the suite
is still governed by sections 2 and 3.

As in the `.nost` suite, `outcome` and `code` are normative and any recorded position is
informative. Where a parser notices that a construct is outside the subset is an artifact
of its design, and requiring one exact position would bind every implementation to one
parser.
