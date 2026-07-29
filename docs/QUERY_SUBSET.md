# The query subset contract

Contract key: `query_subset_version`
Current version: 1
Status: normative

Version 1 is complete. The first published revision declared reading, writing, and `CALL`
in scope and said that write fixtures would arrive with write support; sections 8 through 12
are that completion, not a second version. Aggregation and inline property maps are named by
the root product contract as part of the same MVP subset, so they belong here rather than to
a version 2.

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
| `UNION`, `UNION ALL` | operands must project the same column names, and every operand must be read-only |
| parameters | written `$name` |
| inline property maps | written `{key: expression}` in a node or relationship pattern; see section 8 |
| aggregation | `count`, `sum`, `avg`, `min`, `max`, `collect`; see section 9 |

### 2.2 Writing

`CREATE`, `MERGE`, `SET`, `REMOVE`, `DELETE`, and `DETACH DELETE`, against the root
database only. A write naming a linked record is refused with
`LINKED_DATABASE_READ_ONLY`, because linked records are read-only from the root
transaction. Section 10 gives the exact semantics, and section 11 covers transactions.

### 2.3 Procedures and functions

`CALL` for procedures, and the `nostdb.*` function namespace. NostDB-specific behavior
lives here rather than in new syntax, so the language stays compatible. Section 12 is the
registry.

## 3. What is refused

Refused with `CYPHER_UNSUPPORTED`, non-exhaustively: `FOREACH`, `LOAD CSV`, `CALL {}`
subqueries, `EXISTS {}` subqueries, `MATCH` with an unbounded `*` pattern, `CREATE
INDEX` and other schema commands, `USE` and other multi-database syntax, list
comprehensions, pattern comprehensions, `CASE`, `shortestPath`, `allShortestPaths`,
`ON CREATE` and `ON MATCH` on `MERGE`, whole-record assignment written `SET n = {...}` or
`SET n += {...}`, `DISTINCT` inside an aggregate, a variable-length pattern in a write
clause, and any clause not named in section 2.

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
| `CYPHER_UNKNOWN_LABEL` | a warning: the query matched on a label no record carries; it executed |
| `CYPHER_SEMANTIC_ERROR` | the query is in the subset but meaningless: an unbound variable, a type mismatch, mismatched `UNION` columns, a negative `SKIP` |
| `LINKED_DATABASE_READ_ONLY` | a write named a record belonging to a linked source; nothing was modified |

All three carry a source range. All three mean nothing executed.

The distinction matters to a caller: `CYPHER_UNSUPPORTED` may become supported in a later
version, so retrying against a newer build is reasonable. `CYPHER_SEMANTIC_ERROR` means
the query is wrong, and retrying will not help. `LINKED_DATABASE_READ_ONLY` means the query
is well-formed but aimed at the wrong database: the same write succeeds against that
database opened as a root.

## 7. Conformance

Fixtures live in [`../fixtures/cypher`](../fixtures/cypher). Each pairs with an
`.expected` file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `supported/` | `accept` | parses |
| `unsupported/` | `reject` | refused with `CYPHER_UNSUPPORTED`, and no query is produced |
| `semantic/` | `reject` | refused with `CYPHER_SEMANTIC_ERROR` against any graph, and nothing is modified |

Every `semantic/` fixture is refused against any graph, including an empty one. That is what
lets the suite carry no graph, and it is a weaker requirement than refusing while parsing on
purpose: where an implementation notices that a query is meaningless depends on whether it
does scope analysis before execution, and the contract does not require one design over the
other. What it does require is the code, and that nothing was modified.

A semantic error that needs particular data to detect, such as an unbound variable or a
negative `LIMIT`, is governed by section 6 and is not fixtured, because a fixture carrying
no graph could not state its outcome honestly.

The suite is normative for what it contains and does not claim to be exhaustive. A
construct absent from the suite is still governed by sections 2 and 3.

As in the `.nost` suite, `outcome` and `code` are normative and any recorded position is
informative. Where a parser notices that a construct is outside the subset is an artifact
of its design, and requiring one exact position would bind every implementation to one
parser.

## 8. Inline property maps

A node or relationship pattern MAY carry an inline map, written `{key: expression}`. Each
key is an identifier under the `.nost` identifier rule, and each value is an expression in
the subset.

The map means different things in a reading and a writing clause, and that difference is
deliberate rather than an inconsistency:

| Clause | Meaning |
| --- | --- |
| `MATCH`, `OPTIONAL MATCH` | a filter: the record matches only when every named property equals the given value |
| `CREATE` | the properties the new record is given |
| `MERGE` | both: the properties matched on, and the properties given when nothing matched |

An expression in a map MUST NOT evaluate to `null`. A stored `null` is unrepresentable, so
`{name: null}` in `CREATE` has nothing to store, and in `MATCH` it would compare against a
value no stored property can hold. It is `CYPHER_SEMANTIC_ERROR`.

Maps exist in the subset because `MERGE` is meaningless without them: `MERGE (n:Function)`
matches any Function at all, so an implementation offering `MERGE` without maps would offer
a clause nobody can use correctly.

## 9. Aggregation

### 9.1 The functions

| Function | Over no values | Notes |
| --- | --- | --- |
| `count(expression)` | `0` | counts rows where the expression is not `null` |
| `count(*)` | `0` | counts rows, including rows where every value is `null` |
| `sum(expression)` | `0` | integer when every value is an integer and the total fits in a signed 64-bit value, otherwise a float |
| `avg(expression)` | `null` | always a float when it has a value |
| `min(expression)`, `max(expression)` | `null` | ordered by the total order in section 9.4 |
| `collect(expression)` | `[]` | a list, in row order, skipping `null` |

Every aggregate except `count(*)` ignores a `null` input rather than treating it as a
value. `avg` over no values is `null` and not `0`, because a mean of nothing is not zero
and reporting zero would be a wrong number rather than an absent one.

### 9.2 Grouping

A projection containing at least one aggregate is an aggregating projection. Its grouping
key is the tuple of values of its non-aggregate items, compared by the total order in
section 9.4. One row is produced per distinct grouping key.

A projection whose every item is an aggregate has an empty grouping key and produces
exactly one row, even over zero input rows. `RETURN count(*)` over an empty graph therefore
returns one row holding `0`, not zero rows.

A projection with at least one non-aggregate item produces one row per group and therefore
zero rows over zero input rows.

### 9.3 Where an aggregate may appear

An aggregate MAY appear in a `RETURN` or `WITH` projection. Anywhere else it is
`CYPHER_SEMANTIC_ERROR`:

- in a `WHERE` attached to `MATCH` or `OPTIONAL MATCH`, because the predicate runs before
  any grouping exists;
- inside another aggregate;
- in a pattern, a map, an `UNWIND` list, a `SKIP`, or a `LIMIT`.

An aggregate MAY be part of a larger expression, as in `count(n) + 1`.

Filtering on an aggregate uses `WITH`:

```cypher
MATCH (n)-[:CALLS]->(m) WITH n, count(m) AS calls WHERE calls > 3 RETURN n.name, calls
```

After an aggregating projection the incoming bindings no longer exist, so the scope its
`WHERE` and `ORDER BY` see is its own column names alone. `ORDER BY count(n)` is therefore
written `RETURN ..., count(n) AS total ORDER BY total`.

`DISTINCT` inside an aggregate, written `count(DISTINCT n)`, is refused with
`CYPHER_UNSUPPORTED`.

### 9.4 The total order

`ORDER BY`, `min`, `max`, grouping, and `DISTINCT` all need to compare values that Cypher
leaves loosely ordered across types. This contract fixes one total order, so an ordered
query over a mixed column is reproducible:

```text
null < boolean < number < string < list < node < relationship < path
```

Within a kind: booleans order `false` before `true`; integers and floats order together by
numeric value; strings order by Unicode scalar value; lists order element by element.

This order exists to make results reproducible, not to give cross-type comparison a
meaning. A relational comparison such as `1 < "a"` still yields `null`, per section 9.5.

### 9.5 Null in a predicate

Only `true` passes a predicate. A comparison involving `null` yields `null`, and a `null`
predicate does not pass, so a row whose property is missing is dropped rather than kept.
Arithmetic overflow and division by zero yield `null` rather than failing the query.

## 10. Write clauses

Every write applies to the root database. A write clause and a reading clause may appear in
any order within one query part, and a later clause sees what an earlier clause wrote.

A query part containing at least one write clause MAY omit `RETURN`; it then produces zero
columns and zero rows. A read-only part MUST end with `RETURN`.

### 10.1 `CREATE`

`CREATE` creates every node and relationship in its patterns that is not already bound. A
variable already bound is reused, not recreated; naming an already-bound variable in a new
node pattern is `CYPHER_SEMANTIC_ERROR`.

NostDB's model constrains what openCypher permits, and the constraint is a refusal rather
than a silent repair:

| Rule | Outcome when broken |
| --- | --- |
| a created node carries at least one label | `CYPHER_SEMANTIC_ERROR` |
| a created relationship names exactly one relation type | `CYPHER_SEMANTIC_ERROR` |
| a created relationship is directed, written `->` or `<-` | `CYPHER_SEMANTIC_ERROR` |
| a write pattern is not variable-length, and is not a named path | `CYPHER_UNSUPPORTED` |

The label rule is NostDB's, not openCypher's: a Node without a label cannot be stored, so
accepting `CREATE (n)` would mean either inventing a label or storing an invalid record.

The label rule is also the one rule here that a pattern alone does not settle, because
`CREATE (a)-[:R]->(b)` over two already-bound variables is legitimate and carries no labels.
An implementation may therefore report it later than the other three. Nothing is modified
either way.

### 10.2 `MERGE`

`MERGE` takes one pattern. If the pattern matches, every match is kept and nothing is
created. If it does not match, the pattern is created exactly once. The same model rules as
`CREATE` apply.

`ON CREATE` and `ON MATCH` are refused with `CYPHER_UNSUPPORTED`.

### 10.3 `SET` and `REMOVE`

| Form | Effect |
| --- | --- |
| `SET n.key = expression` | sets a property |
| `SET n.key = null` | removes the property, because a stored `null` is unrepresentable |
| `SET n:Label` | adds a label |
| `REMOVE n.key` | removes a property |
| `REMOVE n:Label` | removes a label |

Removing a node's last label is `CYPHER_SEMANTIC_ERROR`, for the reason in section 10.1.

A `SET` or `REMOVE` naming a `null` does nothing, which is what keeps an unmatched
`OPTIONAL MATCH` row usable in a write. So does a `DELETE` of `null`.

A write that reports no change MUST leave the database exactly as it was, contributions and
all. `REMOVE` of a property that is absent, and `SET` of a label the node already carries,
are the cases that arise: neither may record a contribution, a timestamp, or any other
trace. Section 11 lets a transaction decide from its change count whether to advance the
generation, and that decision is only correct if a count of zero means the file is
untouched.

Assigning a property the value it already holds does count as a change, as it does in
openCypher. The rule above is one-directional on purpose: reporting a change that did
nothing costs a caller a generation, while reporting no change after modifying the database
would lose the modification.

`SET n = {...}` and `SET n += {...}` are refused with `CYPHER_UNSUPPORTED`. Whole-record
assignment has to decide what happens to properties the map omits, and the two answers
differ by exactly the data a caller would lose if the implementation chose the other one.

Setting a property on a record produced by an analyzer does not remove the analyzer's
contribution. The write adds or updates a user contribution and leaves every other
contribution in place, which is the ownership separation the root product contract
requires.

### 10.4 `DELETE` and `DETACH DELETE`

`DELETE` removes the bound nodes and relationships. Deleting a node that still has a
relationship is `CYPHER_SEMANTIC_ERROR`, because an Edge always has two non-null endpoints
and removing the node alone would leave one dangling.

`DETACH DELETE` removes the bound nodes together with every relationship incident to them.
An incident relationship whose other endpoint is in a linked source is still a record of
the root database, so deleting it is a root write and is permitted.

Deleting a record removes every contribution on it. That is different from an analyzer
refresh, which removes one owner's contribution and keeps the record while anything else
still requires it: an explicit `DELETE` is an instruction about the record, not about a
contribution.

Deleting an already-deleted record in the same query is not an error; the second delete
does nothing.

## 11. Transactions

A write executes inside a transaction, and an implementation MUST expose explicit
transaction control at its API boundary.

| Rule | Consequence |
| --- | --- |
| a transaction records the generation it began at | see the conflict rule below |
| a commit advances the generation by exactly one | a reader observes one generation or the next, never a mixture |
| a read-only transaction commits without advancing the generation | synchronization compares generations, so a read must not look like a change |
| a rollback leaves the database byte-identical | including after a partly applied statement |
| a failed commit preserves the last valid generation | the root product invariant |

If the database advanced since the transaction began, the commit MUST report a conflict and
modify nothing. It MUST NOT rebase the transaction onto the newer generation: the
transaction's reads were answered from the older one, so its writes may no longer mean what
the caller intended.

A conflict is reported as a typed error at the API boundary rather than as a diagnostic
code. It describes what the caller did, not something found in analyzed content, and the
root product contract keeps those two vocabularies apart.

### 11.1 Stopping a running query

An implementation MAY let a caller ask a running query to stop, and MUST report
`QUERY_CANCELLED` when one does. This is what a daemon's query timeout is built on.

Cancellation is **cooperative**. An implementation MUST observe the request at boundaries
where stopping is safe, and this contract requires at least:

- between the parts of a `UNION`;
- between the clauses of a part;
- between the input rows of a `MATCH`.

A single operation that does not yield between those points need not be interruptible. An
implementation MUST NOT claim a granularity it does not have: a caller that is told a query
can be stopped, and then waits through a pattern expansion that never checks, has been given
a guarantee that does not hold in the case it most needed one.

A stopped query MUST leave the last valid database generation intact. It stops rather than
partially commits, so a transaction it ran inside is rolled back like any other refusal.

`QUERY_CANCELLED` carries no source range that means anything: nothing in the query is
wrong. An implementation SHOULD report the range as the origin rather than pointing at a
token, which would send a reader looking for a mistake that is not there.

## 12. The `nostdb` namespace

NostDB-specific behavior lives in this namespace so the language itself stays
openCypher-compatible.

### 12.1 Procedures

`CALL` invokes a procedure, optionally with `YIELD` to name the columns to keep. A `CALL`
without `YIELD` and without a following `RETURN` produces the procedure's own columns.

| Procedure | Columns | Rows |
| --- | --- | --- |
| `nostdb.links()` | `source`, `alias`, `remote` | one per declared link |
| `nostdb.build_status()` | `database_generation`, `nodes`, `edges`, `links` | exactly one |
| `nostdb.evidence(node)` | `source`, `path`, `revision`, `digest`, `producer`, `producer_version`, `method`, `confidence`, `score`, `start_line`, `start_column`, `end_line`, `end_column` | one per evidence record, bounded by section 12.3 |
| `nostdb.refresh_links()` | `source`, `refreshed`, `revision` | one per declared link |

`YIELD` naming a column a procedure does not produce is `CYPHER_SEMANTIC_ERROR`, and so is
calling a procedure outside this namespace: an unknown procedure will not become known by
retrying.

`nostdb.build_status()` reports what the database records. Build coverage, which the root
product contract defines for an analysis run, is not among its columns in this contract
version; adding it later is a `query_subset_version` change rather than a silent column
addition.

`nostdb.evidence` returns metadata. Reading a source excerpt requires provider permission
and is read-only, and an unavailable source yields its metadata with a warning rather than
fabricated content.

### 12.2 Functions

| Function | Value |
| --- | --- |
| `nostdb.source(node)` | the canonical locator of the source holding the record, or `null` when the caller did not name one |
| `nostdb.source_location(node)` | the path within that source, from the record's first evidence, or `null` |
| `nostdb.source_revision(node)` | the immutable revision that evidence resolved to, or `null` |
| `nostdb.link_alias(node)` | the alias of the link the record's source was reached through, or `null` for a record of the root database |
| `nostdb.is_available(node)` | whether the source holding the record could be opened |

"First evidence" means the first evidence of the first contribution in stored order. It is
one record out of possibly many, chosen deterministically; `nostdb.evidence()` yields all
of them.

### 12.3 Bounded work

`nostdb.evidence` yields at most 256 rows per call. Every `.nostdb` file is untrusted
input, so a procedure that walked an unbounded number of stored records would hand an
attacker the query's memory budget.

### 12.4 Capability-gated procedures

`nostdb.refresh_links()` needs a source provider: refreshing a remote link means resolving
a ref to an immutable commit and fetching it. An implementation without that capability
MUST refuse the call with `CYPHER_UNSUPPORTED`, naming the missing capability.

That is the correct code rather than a semantic error, because the same query against a
build that has the capability succeeds. A caller can act on the difference: retrying an
unsupported call against a more complete build is reasonable, and retrying a semantic error
is not.

## A label no record carries is a warning, not an error

A `MATCH` naming a label the database has never held executes and returns nothing, and MUST report
`CYPHER_UNKNOWN_LABEL` as a warning on the result's diagnostics.

Zero rows is otherwise indistinguishable from zero rows: a caller cannot tell "this project has no such
thing" from "that word means nothing here". Both are legitimate and they call for opposite responses —
one is an answer and the other is a misspelling or a concept the database does not model.

A warning rather than an error, for two reasons. A label may be absent because the project genuinely has
none of that thing, and refusing would turn an ordinary empty result into a failure. And a query is
written once and run against many databases, so a label present in one and absent in another must not
make the query invalid.

The warning names the label. It MUST NOT suggest an alternative: a near-miss suggestion is a guess at
what somebody meant, and `CYPHER_UNSUPPORTED` exists precisely so nothing executes under a guessed
alternative.
