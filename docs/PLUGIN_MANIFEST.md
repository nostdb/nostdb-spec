# The plugin manifest contract

Contract key: `manifest_version`
Current version: 1
Status: normative

Every plugin contains `nostdb-plugin.json`. It states what the plugin is, what it runs, and
what it is asking to be allowed to do.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

The manifest document, the GitHub source grammar an installation resolves, and what an
installation records. [`../fixtures/plugin`](../fixtures/plugin) is the conformance gate.

It does not own the exchange stream a running plugin reads, which carries
`plugin_protocol_version` and evolves separately. A plugin declares which protocol version
it speaks; what that protocol *is* belongs to its own contract.

### 1.1 A manifest is a request, not a grant

Everything under `permissions` is what the plugin is **asking** for. Nothing here authorizes
anything: the user approves an installation, the approval is recorded, and execution is
checked against what was recorded rather than against what the manifest says today.

The distinction is the whole safety story. A plugin that could widen its own permissions by
editing its manifest would be a plugin that grants itself whatever it likes, and the digest
recorded at installation is what makes an edit visible.

### 1.2 Installation must not execute plugin code

An implementation validates paths, archive limits, manifest compatibility, and digests
**before** anything runs, and running is a separate act afterwards.

A manager that executed first would have already lost, because the validation exists to
decide whether executing is safe and a plugin that has run has had its answer.

## 2. Document shape

```json
{
  "manifest_version": 1,
  "name": "org.nostdb.view-webgpu",
  "version": "1.0.0",
  "nostdb": ">=0.1.0 <0.2.0",
  "entrypoint": {
    "command": ["bin/nostdb-view"]
  },
  "protocol_version": 1,
  "actions": [
    { "name": "view", "ai_usage": "none" }
  ],
  "permissions": {
    "graph_read": true,
    "database_write": false,
    "output_paths": [".nostdb/out/**"],
    "network_hosts": []
  }
}
```

Every member is required. A manifest that omits `permissions` is not one asking for nothing;
it is one that did not say, and an implementation cannot tell those apart.

### 2.1 `name`

A reverse-DNS-style identifier: lower-case segments separated by dots, at least two.

Namespaced because a plugin's name is how a user refers to it and how a project pins it, and
two authors independently choosing `viewer` would make one project's pin ambiguous.

### 2.2 `nostdb`

The Engine versions this plugin works with, as a range. An installation refuses a plugin
whose range excludes the Engine present.

Checked at installation *and* before execution, because an Engine can be upgraded after a
plugin was installed and the plugin does not learn of it.

### 2.3 `entrypoint.command`

An **argument vector**, never a string.

A manifest comes from a repository somebody else wrote. A string a shell interprets is that
author choosing what runs — including what runs instead. An implementation MUST NOT pass this
through a shell, and MUST reject a manifest whose `command` is a string rather than an array.

The first element is a path relative to the plugin directory. It MUST NOT be absolute and
MUST NOT contain a `..` segment: a plugin names something inside itself, and one naming
`/bin/sh` or `../../../usr/bin/env` is naming something it did not ship.

## 3. Permissions

| Field | Type | Meaning |
| --- | --- | --- |
| `graph_read` | boolean | receives authorized graph data through the exchange |
| `database_write` | boolean | MUST be `false` |
| `output_paths` | array of string | where it may write, as project-relative globs |
| `network_hosts` | array of string | hosts it may reach; empty means none |

### 3.1 `database_write` is always false

Only the Engine writes `.nostdb`. The field exists so that a manifest requesting it can be
**refused by name** rather than being silently ignored — an author who asked for it has a
misunderstanding worth correcting, and ignoring the request would leave them believing it
was granted.

A plugin never receives a `.nostdb` parser API and MUST NOT read or write the binary format
directly. A viewer that parsed the container would be a second reader of a format with
exactly one.

### 3.2 `output_paths` are project-relative and bounded

Each entry MUST be relative and MUST NOT contain a `..` segment. An absolute path or an
escaping one is rejected rather than clamped, because clamping would silently grant
something adjacent to what was asked for and the author would not know which.

An empty list means the plugin writes nothing.

### 3.3 `network_hosts` is an allowlist, and empty by default

A plugin that names no host reaches none. Listing `*` is not a wildcard and is rejected: a
plugin that wants the whole network is asking for something a user cannot meaningfully
approve, and a field that can express it makes refusing harder rather than easier.

## 4. The GitHub source

```text
https://github.com/<owner>/<repository>[?ref=<git-ref>][#<subdirectory>]
```

With no `ref`, the manager resolves the default branch **once** and records the commit. Every
later action uses the recorded commit, so a plugin does not change underneath a project that
installed it.

A `subdirectory` MUST be relative and MUST NOT escape the repository.

## 5. What an installation records

- the canonical repository URL;
- the exact resolved commit;
- the plugin subdirectory;
- the manifest digest;
- the source tree digest;
- the selected scope, project or global;
- the approved permissions.

Two digests rather than one. The manifest digest detects an edited request; the tree digest
detects edited code behind an unchanged request, which is the more dangerous of the two
because nothing about the plugin's stated intent would have changed.

Execution refuses an installation whose digests no longer match.

## 6. Rejected manifests

An implementation MUST reject, rather than repair, each of the following with
`PLUGIN_MANIFEST_INVALID`, except an unsupported version, which is
`PLUGIN_MANIFEST_VERSION_UNSUPPORTED`:

| Condition | Why |
| --- | --- |
| `manifest_version` absent, or not a supported version | the version is what makes every other rule interpretable |
| a required member is absent | a manifest that did not say is not one asking for nothing |
| `name` is not two or more lower-case dotted segments | a name is how a project pins a plugin |
| `entrypoint.command` is a string, or an empty array | a string a shell interprets is the author choosing what runs |
| the command path is absolute or contains `..` | a plugin names something inside itself |
| `database_write` is `true` | only the Engine writes `.nostdb`, and refusing by name beats ignoring |
| an `output_paths` entry is absolute or contains `..` | clamping would grant something adjacent to what was asked |
| `network_hosts` contains `*` | a plugin wanting the whole network is asking for what nobody can approve |
| an action declares no name, or an unknown `ai_usage` | an action nobody can budget for |

## 7. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/plugin`](../fixtures/plugin). Each fixture pairs with an `.expected` file of
`key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `manifest/valid/` | `accept` | the manifest is read |
| `manifest/invalid/` | `reject` | the manifest is refused with the declared code |
| `source/valid/` | `accept` | the source parses and normalizes to the declared form |
| `source/invalid/` | `reject` | the source is refused |

No fixture installs anything. Every one tests what reading a document or a string can decide,
which is the same boundary the change-set and locator suites stop at — and here it is not
only a convenience: a suite that installed a plugin to test installation would be executing
the thing this contract exists to keep from executing.
