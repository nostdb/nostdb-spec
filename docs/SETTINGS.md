# The settings contract

Contract key: `settings_version`
Current version: 1
Status: normative

Settings hold a project's and a user's **operational** configuration: where the
database lives, whether the human-readable representation is materialized, what
an analysis run is allowed to spend, how a declared link is reached, and which
plugin serves an action.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are
normative.

## 1. What this document owns

This document defines `settings.json` at both scopes, the merge between them, and
what an implementation must do with a field it does not recognize.
[`../fixtures/settings`](../fixtures/settings) is the conformance gate.

It does **not** own credentials, the named-database catalog, or the plugin
manifest. Those carry `credentials_version`, `catalog_version`, and
`manifest_version`, and each evolves independently.

### 1.1 Settings are not the graph

The single most important rule here is what settings do *not* contain.

A link is **semantically** declared in `.nostdb`, and in `.nost` when that is
materialized. Settings mirror the same link only to carry the operational detail
a graph file must not hold: a credential reference, a timeout, a refresh policy.

An alias is part of the semantic declaration, so an alias MUST NOT appear in
settings. So must no secret: settings carry a credential *reference* by name and
never a token, a password, a private key, or PEM content.

The reason is that a graph file is shared and reviewed, and an operational file
is local to a machine. Putting an alias in settings would make the same link mean
different things on two checkouts; putting a secret in settings would leak it
into whatever backs up that machine.

## 2. Location and scope

```text
<project>/.nostdb/settings.json     project scope
~/.nostdb/settings.json             user-global scope
```

The active project is the nearest ancestor directory containing
`.nostdb/settings.json`. An explicit `--project` or `--database` argument
overrides that search.

A file holding operational state MUST be readable and writable only by the
current operating-system user, where the platform supports such permissions.

## 3. Document shape

```json
{
  "settings_version": 1,
  "database": {
    "path": "root.nostdb",
    "nost": false
  },
  "analysis": {
    "ai_mode": "auto",
    "max_input_tokens": null,
    "max_output_tokens": null,
    "max_cost_usd": null,
    "on_budget_exceeded": "ask"
  },
  "links": [
    {
      "source": "./packages/child",
      "credential_ref": null,
      "refresh": "manual",
      "timeout_ms": 10000
    }
  ],
  "federation": {
    "max_link_depth": 16,
    "max_link_databases": 256,
    "link_open_timeout_ms": 10000
  },
  "cache": {
    "user": true
  },
  "plugins": {
    "view": "org.nostdb.view-webgpu"
  }
}
```

`settings_version` is the only required member. Every section below is optional,
and an absent section means its defaults apply, so an empty configured project
writes `{"settings_version": 1}` and nothing else.

### 3.1 `database`

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `path` | string | `"root.nostdb"` | the database file, resolved against the `.nostdb` directory |
| `nost` | boolean | `false` | whether the canonical `.nost` is materialized |

`path` MUST be relative and MUST NOT escape the `.nostdb` directory. An absolute
path, a `..` segment, or a path naming a directory is rejected. The rule exists
because settings are read from a repository someone else may have written, and a
path is the one field there that could otherwise name any file on the machine.

`nost: true` materializes and maintains `.nostdb/root.nost`. Setting it to
`false` removes only the generated file the setting names; it never removes the
database, and it never removes an unrelated `.nost` file a person wrote.

### 3.2 `analysis`

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `ai_mode` | `"off"`, `"auto"`, `"full"` | `"auto"` | how much AI enrichment is permitted |
| `max_input_tokens` | integer or null | `null` | hard input-token ceiling |
| `max_output_tokens` | integer or null | `null` | hard output-token ceiling |
| `max_cost_usd` | string or null | `null` | advisory cost ceiling, as a decimal string |
| `on_budget_exceeded` | `"ask"`, `"stop"`, `"continue_without_ai"` | `"ask"` | what to do at the ceiling |

A token limit is normative and MUST NOT be exceeded by starting another batch.
`max_cost_usd` is advisory unless the active provider reports reliable pricing,
and it is a string so a currency amount is never carried as a binary float.

`null` means unlimited. A negative number is rejected; zero is permitted and
means no AI work may start.

### 3.3 `links`

Each entry mirrors one semantic link declaration.

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `source` | string | required | the canonical locator, which is the link identity |
| `credential_ref` | string or null | `null` | a name in `credentials.json`, never a secret |
| `refresh` | `"manual"` | `"manual"` | when a remote snapshot advances |
| `timeout_ms` | integer | `10000` | how long opening this source may take |
| `resolved_commit` | string or null | `null` | the immutable commit this link last resolved to |
| `resolved_digest` | string or null | `null` | the content digest of what that commit yielded |

`resolved_commit` and `resolved_digest` are **snapshot metadata, not identity**. The
configured `source` remains what the link *is*; these record what it last pointed at. That
separation is the reason they live here rather than in the graph: an alias is semantic and a
resolved commit is operational, and putting a commit in a shared graph file would make two
checkouts disagree about a link that is identical in both.

An implementation MUST NOT advance either field as a side effect of a query. Only an explicit
refresh records a newer commit, which is what keeps a branch from moving underneath a build.

An entry MUST NOT carry an alias. An entry carrying one is rejected, rather than
ignored, because silently dropping it would leave two files disagreeing about
what the link is called.

Two entries with the same `source` are rejected.

`refresh` accepts only `"manual"` in version 1. An automatic policy would let a
query advance a remote ref, which the root product contract forbids; the field
exists so a later version can add a policy without changing the shape.

### 3.4 Orphan and missing entries

A settings entry whose `source` matches no link declared in the database is an
**orphan**. It is ignored, and `ORPHAN_LINK_SETTINGS` is reported. It is not an
error: a link removed from the graph leaves its operational entry behind, and
refusing to open the project over that would be worse than saying so.

The reverse case, a declared link with no settings entry, is not a diagnostic at
all. The declaration is authoritative and the entry supplies defaults.

A read-only open MUST NOT write settings to fill in a missing entry. It uses the
defaults and reports the mismatch. Only an explicit state-changing command
reconciles the two, through the multi-file journal, because writing to a file the
user did not ask to change is exactly what a read-only operation must not do.

### 3.5 `federation`

| Field | Type | Default | Counts |
| --- | --- | --- | --- |
| `max_link_depth` | integer | `16` | links followed from the root, which is depth zero |
| `max_link_databases` | integer | `256` | linked databases opened, **excluding** the root |
| `link_open_timeout_ms` | integer | `10000` | milliseconds per source |

Exceeding a limit yields a structured partial-result warning rather than an
error. Each MUST be positive.

`max_link_databases` excludes the root deliberately, so that it counts the same
thing as `linked_databases_opened` in the result envelope. A limit counting one
thing while the number reported beside it counted another would be a trap: a
caller comparing them would conclude the limit was off by one.

### 3.6 `cache`

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `user` | boolean | `true` | whether the user-global cache tier is read |

An implementation caches derived analysis in two tiers, read project first and then
user. This field turns the second off for one project.

The project tier has no field, because a project that could not cache its own
derived work would have nothing to turn off — the tier lives inside the project
and is discarded with it. The user tier is different: it is shared across every
project the same operating-system user builds, and a project may have reason not
to read from something another project wrote.

Setting `user` to `false` MUST NOT be read as a privacy guarantee about the other
direction. Nothing here says what an implementation writes; the tier a write goes
to is an implementation decision, and a future team cache is out of scope for this
version.

### 3.7 `plugins`

An object mapping an action name to the plugin that serves it. A value is a
plugin name, never a path and never a command line, because settings must not be
able to name an executable to run.

## 4. The merge

Global settings load first. Project settings override them **by defined field**.

"By defined field" is the whole rule, and it is deliberately not a recursive JSON
merge. A field present in the project document replaces the global value for that
field alone; a field absent from the project document keeps the global value; and
a field absent from both takes its default.

`links` is one field, so a project that defines `links` replaces the global list
entirely rather than appending to it. Merging two link lists would require
deciding what to do when both scopes name one source with different timeouts, and
every answer to that is a surprise to somebody.

An implementation MUST NOT merge a value into a deeper structure than the field
it belongs to. Given a global `{"database": {"nost": true}}` and a project
`{"database": {"path": "other.nostdb"}}`, the result has `path` from the project
and `nost` from the global, because `database.path` and `database.nost` are two
defined fields.

## 5. Versions and unknown fields

`settings_version` MUST be present and MUST be a positive integer.

| Situation | Required behavior |
| --- | --- |
| version is supported | read normally |
| version is above the highest supported, opened read-only | read what is understood, **preserve** every unknown field, and report `SETTINGS_VERSION_UNSUPPORTED` as a warning |
| version is above the highest supported, and a command would write | refuse with `SETTINGS_VERSION_UNSUPPORTED`, unless every unknown field's meaning can be preserved through the write |
| version is below the lowest supported | refuse with `SETTINGS_VERSION_UNSUPPORTED` |

An unknown field inside a supported version is preserved on write and otherwise
ignored. Preservation is what lets one machine run a newer build without
destroying configuration for another machine running an older one; the moment a
writer drops what it does not recognize, downgrading becomes lossy.

Refusing to write a newer document is the same rule seen from the other side. A
writer that cannot state what an unknown field means cannot promise the document
still means the same thing afterwards.

## 6. Rejected documents

An implementation MUST reject, rather than repair, each of the following with
`SETTINGS_INVALID`:

| Condition | Why |
| --- | --- |
| `settings_version` absent, zero, negative, or not an integer | the version is what makes every other rule interpretable |
| the document is not a JSON object | there is nothing to read fields from |
| `database.path` is absolute, escapes `.nostdb`, or names a directory | a path from an untrusted repository must not reach outside the project |
| a link entry carries an alias | an alias is semantic and belongs in the graph |
| a link entry has no `source`, or two entries share one | the locator is the link's identity |
| a numeric limit is negative, or a federation limit is zero | there is no sensible reading of it |
| a field has the wrong JSON type | guessing a conversion is how a typo becomes a silent behavior change |

Rejection is an error with a message naming the field. Repairing a malformed
document in place would change configuration the user did not ask to change.

## 7. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/settings`](../fixtures/settings). Each fixture pairs with an
`.expected` file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `valid/` | `accept` | the document is read |
| `invalid/` | `reject` | the document is refused |
| `merge/` | `merge` | a `.global.json` and a `.project.json` produce the declared result |

A `merge` fixture declares its result as `.expected.json`, which is the whole
effective document after the merge and after defaults are applied. Comparing
whole documents rather than named fields is what keeps a fixture from passing
while some other field quietly changed.
