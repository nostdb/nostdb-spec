# The plugin installation contract

Contract key: `plugin_install_version`
Current version: 1
Status: normative

What a manager fetches, what it checks before writing anything, and what it records so a
later execution can refuse a plugin that changed.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

The installation act: resolving a source to an immutable commit, validating a tree, computing
the two digests, and the record document that carries the result.
[`../fixtures/plugin-install`](../fixtures/plugin-install) is the conformance gate.

It does not own the manifest, which is [`PLUGIN_MANIFEST.md`](PLUGIN_MANIFEST.md) and carries
`manifest_version`. It does not own the exchange stream a running plugin reads, which carries
`plugin_protocol_version`.

### 1.1 Why this is a separate contract

The manifest is the document a plugin **author** writes. The record is written by the manager
and read by the executor, and neither is an author's business.

Coupling them would mean a manager that wanted to record one more field could not do so
without changing the document every author has already written, and an author's manifest would
appear to need reissuing because a manager learned to remember something new.

### 1.2 Installation must not execute plugin code

Every check in this document happens before any byte of the plugin is written, and no step of
an installation runs anything the plugin shipped. Execution is a separate act, specified
elsewhere, and it refuses an installation whose recorded digests no longer match.

An implementation that ran the plugin to discover something about it — its version, its
capabilities, whether it works — would have already given the answer that validation exists to
decide.

## 2. The order

An installation performs these steps in this order. The order is normative: each step exists to
make the next one safe.

1. **parse the source.** The grammar is section 4 of the manifest contract, and it requires a
   `ref`. A source with no ref is refused here, before a provider is reached: whether a source
   can be resolved at all is decidable from the source, and demanding a provider first would
   send somebody to install one only to meet this refusal afterwards;
2. **resolve** the ref to one immutable commit. Everything after this uses that commit and never
   the ref;
3. **enumerate** the tree at that commit;
4. **validate** every entry against section 3, and the tree as a whole against section 4.
   Nothing has been downloaded yet;
5. **read** the manifest entry, and validate it against the manifest contract;
6. **check** the manifest's `nostdb` range against the Engine present, per section 5;
7. **read** every remaining accepted entry;
8. **compute** both digests, per section 6;
9. **compare** against an existing record for the same plugin name in the same scope, per
   section 8;
10. **write** the plugin's files and then the record, per section 9.

Step 4 precedes any download. A limit checked after the bytes have arrived is a description of
what was downloaded rather than a limit on it.

Step 6 precedes step 10 so that nothing is written under a digest that was not computed over
exactly the bytes being written.

## 3. Entry rules

Every enumerated entry an installation accepts MUST satisfy all of:

| Rule | Why |
| --- | --- |
| the path is relative | an absolute path names something outside the plugin |
| no path segment is `..` | a plugin names something inside itself |
| no path segment is empty, `.`, or ends in a space or a period | these do not round-trip across every filesystem an install must work on |
| the path contains no NUL, and no character below `U+0020` | a control character in a path is not a name anybody typed |
| the path is not an absolute Windows path or a reserved device name | `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, with or without an extension |

An entry breaking any of these is refused with `PLUGIN_SOURCE_INVALID`. The whole installation
is refused: a tree with one escaping path is not a tree with one fewer usable file, because
whoever wrote it meant something by that path and an implementation cannot know what.

Rejected rather than skipped, for the reason an escaping `output_paths` entry is rejected rather
than clamped: skipping would install a plugin that is not the one the author published, and
nothing would say which parts are missing.

### 3.1 A subdirectory install narrows the tree

When the source names a subdirectory, only entries beneath it are part of the plugin, and each
entry's path within the plugin is its path with that prefix removed. Entries outside it are not
validated and are not digested: they are not part of what was installed.

A subdirectory naming nothing in the tree is refused with `PLUGIN_SOURCE_INVALID`. An empty
plugin is not a plugin, and reporting success for one would install nothing under a name a
project then depends on.

### 3.2 The manifest must be present

The plugin's root MUST contain `nostdb-plugin.json`, at the top level of the plugin — not
nested. Its absence is refused with `PLUGIN_SOURCE_INVALID` rather than
`PLUGIN_MANIFEST_INVALID`: a source with no manifest is not a plugin with a bad one, and
telling an author their manifest is invalid when they have none sends them to edit a file that
does not exist.

## 4. Archive limits

An installation MUST refuse a tree exceeding any of these, with `PLUGIN_LIMIT_EXCEEDED`:

| Limit | Value |
| --- | --- |
| entries in the plugin | 4096 |
| bytes in one entry | 8 MiB (8 388 608) |
| bytes in the plugin | 64 MiB (67 108 864) |
| path length, in bytes | 1024 |
| path depth, in segments | 32 |

These are fixed rather than configurable. A limit a project can raise is one an install can ask
it to raise, and the request would arrive attached to the plugin that wants it.

The values bound the work an installation does before it can decide anything, which is what
makes a hostile source a refusal rather than a resource exhaustion. They are not a security
boundary on a plugin that has already been approved and run: section 1.2 of the manifest
contract says plainly that there is no sandbox.

A limit is reported separately from an invalid path because the fixes differ: a tree over a
limit is trimmed or split by its author, and a tree with an escaping path is corrected.

## 5. The Engine range

`nostdb` in a manifest is a **range**, and this is its grammar:

```ebnf
range      = comparator , { " " , comparator } ;
comparator = operator , version ;
operator   = ">=" | ">" | "<=" | "<" | "=" ;
version    = number , "." , number , "." , number ;
number     = "0" | ( non_zero , { digit } ) ;
```

Every comparator MUST hold. A range is a conjunction, so `>=0.1.0 <0.2.0` admits `0.1.7` and
excludes `0.2.0`.

Versions compare component by component, numerically: `0.10.0` is above `0.9.0`. An
implementation MUST NOT compare them as strings.

Deliberately small. No caret, no tilde, no wildcard, no pre-release, and no build metadata. Each
of those is a shorthand whose meaning differs between ecosystems, and a plugin author who wrote
`^0.1.0` expecting one ecosystem's reading would get another's. A conjunction of comparators has
one reading everywhere.

A range an implementation cannot parse is refused with `PLUGIN_MANIFEST_INVALID`, because it is a
malformed manifest member. A range that parses and excludes the Engine present is refused with
`PLUGIN_INCOMPATIBLE`, because the manifest is correct and this build is not the one it is for.

The check is repeated before execution. An Engine can be upgraded after a plugin was installed,
and the plugin does not learn of it.

## 6. The two digests

Both are `sha256:` followed by 64 lower-case hexadecimal characters.

### 6.1 The manifest digest

The SHA-256 of the manifest's bytes **exactly as received**, before any parsing and without
reserialization.

Digesting a reserialized manifest would make a formatting change look like a content change
and — worse — could make a content change invisible, because two different documents can
serialize identically once a reader has normalized them.

### 6.2 The tree digest

Over the entries the installation accepted, in ascending byte order of their plugin-relative
paths, the SHA-256 of the concatenation, for each entry, of:

```text
<plugin-relative path as UTF-8> LF <lower-case hex SHA-256 of the entry's bytes> LF
```

The manifest is one of those entries and is included. A tree digest that excluded it would
leave the manifest covered only by its own digest, and an implementation comparing trees would
report two installations identical when their manifests differed.

Byte order of paths, not a locale collation: a digest that depended on the installing machine's
locale would differ between two machines installing the same commit, and the disagreement would
look exactly like tampering.

The path and not the mode. A file's executable bit is not covered, which is stated here rather
than left to be discovered: a mode change is invisible to this digest, and a future version that
covers it is a `plugin_install_version` bump.

### 6.3 What each one is for

The manifest digest detects an edited **request** — a plugin that widened what it asks for.

The tree digest detects edited **code behind an unchanged request**, which is the more dangerous
of the two precisely because nothing about the plugin's stated intent would have changed and a
user re-reading the manifest would find it identical.

Recording only the tree digest would technically cover both, since the manifest is in the tree.
Two are recorded because they answer different questions, and an implementation that can say
*which* one changed can tell a user whether the plugin asked for more or merely became a
different plugin.

## 7. Install locations

| Scope | Record | Plugin files |
| --- | --- | --- |
| project | `<project>/.nostdb/plugins/installed.json` | `<project>/.nostdb/plugins/<name>/` |
| global | `<home>/.nostdb/plugins/installed.json` | `<home>/.nostdb/plugins/<name>/` |

`<name>` is the manifest's `name`, used verbatim. It is already constrained to lower-case
dotted segments, so it is a safe directory name on every filesystem and needs no escaping —
which is one of the reasons the manifest constrains it.

A project installation takes precedence over a global one of the same name, for the reason a
project-local Engine takes precedence over a global one: a project that pinned something did so
on purpose.

## 8. Reinstalling

An installation whose plugin name and scope match an existing record is a **reinstall**.

| Situation | Outcome |
| --- | --- |
| the recorded commit and both digests match | nothing is written, and the installation reports that it was already installed |
| the recorded commit differs | the record and the files are replaced |
| the recorded commit matches and either digest differs | refused with `PLUGIN_DIGEST_MISMATCH` |

The third row is the one that matters. A commit is immutable, so the same commit yielding
different bytes means something between the host and this machine is not what it was, and the
installation MUST NOT proceed. Replacing the record would overwrite the only evidence that
anything had changed.

An implementation MUST NOT offer a flag that installs over a digest mismatch. A user cannot
evaluate that question, and a flag that exists to be passed when a check fails is a check
nobody has.

Moving to a different commit is not a mismatch. That is a user asking for a different version
of the plugin, and the request names the commit it wants.

## 9. The record document

```json
{
  "plugin_install_version": 1,
  "installed": [
    {
      "name": "org.nostdb.view-webgpu",
      "repository": "https://github.com/nostdb/plugins",
      "commit": "0f1e2d3c4b5a69788796a5b4c3d2e1f009182736",
      "subdirectory": "reference/view-webgpu",
      "manifest_digest": "sha256:9e1f...",
      "tree_digest": "sha256:41ab...",
      "scope": "project",
      "manifest_version": 1,
      "plugin_version": "1.0.0",
      "approved_permissions": {
        "graph_read": true,
        "database_write": false,
        "output_paths": [".nostdb/out/**"],
        "network_hosts": []
      }
    }
  ]
}
```

Every member is required, except `subdirectory`, which is absent when the source named none.

| Member | Meaning |
| --- | --- |
| `plugin_install_version` | the version of this contract the record was written under |
| `name` | the manifest's `name`, and the key a project pins |
| `repository` | the canonical repository URL, with no ref and no fragment |
| `commit` | the immutable commit the ref resolved to, 40 lower-case hexadecimal characters |
| `subdirectory` | the plugin's directory within the repository |
| `manifest_digest` | section 6.1 |
| `tree_digest` | section 6.2 |
| `scope` | `project` or `global`, matching where the record lives |
| `manifest_version` | the manifest version this plugin declared |
| `plugin_version` | the manifest's `version`, so a user can see what they have without reading the manifest |
| `approved_permissions` | the manifest's `permissions` as approved, verbatim |

`repository` carries no ref, because the ref is not what was installed — `commit` is, and
recording both would invite a reader to trust the one that can move.

`approved_permissions` is a copy and not a reference. Execution is checked against what was
approved, and a reference into the plugin's own manifest would be checked against whatever the
manifest says today, which is precisely the thing a digest exists to detect changing.

`installed` is ordered by `name`, in ascending byte order, so that two managers installing the
same set of plugins in different orders produce the same file. A record file that differed only
by insertion order would show as a change in every diff and in every backup.

### 9.1 Refused records

An implementation MUST reject, rather than repair, each of the following with
`PLUGIN_RECORD_INVALID`, except an unsupported version, which is
`PLUGIN_RECORD_VERSION_UNSUPPORTED`:

| Condition | Why |
| --- | --- |
| `plugin_install_version` absent, or not a supported version | the version is what makes every other rule interpretable |
| `installed` absent, or not an array | a record that did not say is not one recording nothing |
| a required member of an entry is absent | the same |
| two entries share a `name` | one name resolving to two installations has no answer |
| `installed` is not in ascending `name` order | a canonical document with a second valid spelling is not canonical |
| a digest is not `sha256:` and 64 lower-case hexadecimal characters | a digest an implementation cannot compare is not one |
| `commit` is not 40 lower-case hexadecimal characters | a ref recorded where a commit belongs is a pin that can move |
| `scope` is neither `project` nor `global`, or does not match its file | a record claiming the other scope's precedence |
| `approved_permissions.database_write` is `true` | only the Engine writes `.nostdb`, and a record granting it would be checked at execution |

A malformed record is not repaired, for the same reason a malformed manifest is not: this file
is the evidence of what a user approved, and an implementation that rewrote it into something
acceptable would be deciding on their behalf what they had agreed to.

## 10. Consent

An explicit `plugin add` is authorization to install what it names. Nothing further is asked
about *whether* to install.

What is asked is **where**, when the invocation does not say and the session can ask:

| Situation | Outcome |
| --- | --- |
| the scope is given | it is used |
| omitted, in a project, interactive | ask, recommending project |
| omitted, in a project, non-interactive | project |
| omitted, outside a project | global |

Recommending project scope is the same argument as everything else in this document preferring
the project: a plugin installed for one project does not appear in another that never asked
for it.

Defaulting to project rather than asking in a non-interactive session is deliberate. The
alternative — refusing for want of an answer — would make every unattended install depend on a
person being present, and the narrower of the two scopes is the safe one to choose without
being told.

## 11. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/plugin-install`](../fixtures/plugin-install). Each fixture pairs with an
`.expected` file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `record/valid/` | `accept` | the record is read |
| `record/invalid/` | `reject` | the record is refused with the declared code |
| `range/` | `admit` or `exclude` | the range parses, and admits or excludes the declared engine version |
| `range-invalid/` | `reject` | the range does not parse |
| `tree/` | `accept` or `reject` | the entry list is accepted, or refused with the declared code |

A tree fixture declares an optional `subdirectory` and a list of `entries`, each with a `path`
and a byte count. It declares no content, because every rule in sections 3 and 4 is decidable
from an enumeration — which is the property that lets an implementation refuse a tree before
downloading it.

An entry may carry `repeat`, meaning that many entries with an index appended to the path. It is
an encoding for this suite and not a concept in this contract: without it, the fixture for the
entry-count limit would be four thousand lines and nobody would read it.

An accepted tree fixture declares `accepted_entries`, the number of entries that are part of the
plugin. For a subdirectory install that is fewer than the tree holds, and asserting it is what
distinguishes narrowing the tree from ignoring the subdirectory.

No fixture installs anything, and none contains a plugin. Every one tests what reading a
document, a range, or an entry list can decide — which is the boundary the manifest suite stops
at, for the same reason: a suite that installed a plugin to test installation would be
executing the thing this contract exists to keep from executing.
