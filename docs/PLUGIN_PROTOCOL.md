# The plugin protocol contract

Contract key: `plugin_protocol_version`
Current version: 1
Status: normative

What the manager says to a running plugin, what a plugin may say back, and what must be true
before either speaks.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

The transport, version negotiation, the handshake, invoking an action, the exchange handoff, and
what makes a message invalid. [`../fixtures/plugin-protocol`](../fixtures/plugin-protocol) is the
conformance gate.

It does not own the manifest, which is [`PLUGIN_MANIFEST.md`](PLUGIN_MANIFEST.md), nor what an
installation records, which is [`PLUGIN_INSTALL.md`](PLUGIN_INSTALL.md). It carries an exchange
artifact **by reference** and does not specify one byte of what is inside it — see section 6.

### 1.1 This is not a sandbox

A plugin runs as the user who invoked it, with that user's files and that user's network. The
process boundary is the whole of the isolation, and this document MUST NOT be described as
providing more.

What it does buy is stated plainly, because it is real and it is worth having: a plugin cannot
read the Engine's memory, cannot reach a database handle, cannot open a `.nostdb`, and cannot
outlive the request it was started for.

Every rule below about permissions is therefore a rule about what the **manager** does — what it
hands over and what it accepts back — and not a restraint on what a plugin is capable of doing.
An implementation that presented them as restraints would be describing a sandbox it does not
have.

### 1.2 The approval is the authority, never the manifest on disk

Everything the manager checks before and during an invocation is checked against the record
`plugin_install_version` describes, not against the `nostdb-plugin.json` sitting in the plugin's
directory.

That file is what the plugin *currently says* it wants. The record is what a user agreed to. A
plugin that could widen its permissions by editing its own manifest would grant itself whatever
it liked, and the recorded digests exist so that an edit is visible rather than effective.

### 1.3 After the digest check, the installed manifest *is* the approved manifest

The record does not duplicate the whole manifest. It carries what a user needs to see without the
plugin present — the permissions they approved, the version, the commit — and not the entrypoint,
the declared actions, or the Engine range.

Those are read from the installed `nostdb-plugin.json`, and section 3 puts the digest check
**before** every step that needs them. That ordering is the whole of it: once the recorded
manifest digest holds, the file on disk is the approved bytes, and reading it is reading what was
approved rather than trusting what is there.

An implementation MUST NOT read the manifest before verifying its digest, and MUST NOT fall back
to reading it when a digest cannot be computed. Section 1.2 is not a rule against reading that
file; it is a rule against reading it *unverified*, which is a different thing and the difference
is one step in an ordered list.

Duplicating the manifest into the record instead would have made the record a second copy of a
document that already exists, with two ways for them to disagree and no rule for which wins.

## 2. Transport

The plugin's standard input carries requests, one per line, as UTF-8 JSON with no embedded
newline. Its standard output carries replies the same way. A reply MAY be followed by a fixed run
of bytes, and only where this document says so.

Standard error belongs to the plugin and is not part of this protocol. An implementation SHOULD
let it reach the user rather than capture it into a buffer nothing reads: a misbehaving plugin
with nowhere to complain is one nobody can diagnose.

This is the framing [`PROVIDER_PROTOCOL.md`](PROVIDER_PROTOCOL.md) section 2 already defines, and
deliberately so. A second framing would be a second set of framing bugs, and the subtle one — a
buffered line reader consuming part of a content run while looking for a newline — is a bug worth
solving once.

### 2.1 The manager starts the process, and never a shell

The entrypoint is the argument vector the manifest declared, resolved against the plugin's
installed directory. An implementation MUST pass it directly to the operating system and MUST NOT
route it through a shell, a shell builtin, or any string the operating system will re-split.

The manifest comes from a repository somebody else wrote. A string a shell interprets is that
author choosing what runs — including what runs *instead*.

### 2.2 Every message carries the version

Every request and every reply states `plugin_protocol_version`. A message without it is invalid;
an implementation MUST NOT infer a version from a message's shape, because two versions that
differ in meaning may not differ in shape.

## 3. Before anything starts

The manager performs all of these, in this order, before the plugin process exists. Each one can
refuse, and refusing costs nothing because nothing has run.

1. **the plugin is installed.** A name with no record in either scope is refused with
   `PLUGIN_REQUIRED`, naming the plugin and, when the manager knows one, the command that would
   install it. One code covers both directions — a user who named a missing plugin and an action
   that needs one the user did not name — because a caller branches on the code and never on the
   message, so a message that sometimes carries an install command and sometimes cannot is not a
   difference a caller can act on;
2. **both recorded digests still hold.** The manager recomputes them over the installed
   directory, by the derivations in `PLUGIN_INSTALL.md` section 6, and refuses
   `PLUGIN_DIGEST_MISMATCH` when either has moved. This is the check every digest in that record
   was written down for;
3. **the Engine range still admits this build.** Refused with `PLUGIN_INCOMPATIBLE`. Checked
   again here rather than trusted from installation, because an Engine can be upgraded afterwards
   and the plugin does not learn of it;
4. **the action is one the record's manifest declared.** Refused with `PLUGIN_ACTION_UNKNOWN`
   before the process starts, so a plugin is never launched to be told no.

### 3.1 The installed directory is effectively read-only

Recomputing the tree digest over the installed directory means a plugin that writes into its own
directory fails its **next** invocation, with `PLUGIN_DIGEST_MISMATCH`.

That consequence is stated rather than left to be discovered. A plugin writes to its approved
`output_paths`, which are project-relative and outside its own directory; one that treats its
installation as scratch space will appear to work once and then refuse.

It is not enforced, and section 1.1 is why: nothing stops a plugin from writing there. The digest
does not prevent it, it detects it.

## 4. Handshake

The manager MUST send this first and MUST NOT send anything else until it has a reply.

```json
{"plugin_protocol_version": 1, "request": "handshake"}
```

Reply:

```json
{"plugin_protocol_version": 1, "reply": "handshake",
 "plugin": "org.nostdb.view-webgpu", "plugin_version": "1.0.0",
 "actions": ["view"]}
```

### 4.1 The handshake must agree with what was approved

The manager MUST refuse with `PLUGIN_IDENTITY_MISMATCH` when the reply's `plugin` differs from the
recorded name, or when `actions` contains an action the record's manifest did not declare.

A running process claiming an action nobody approved is claiming a capability nobody agreed to. The
digests cover the bytes on disk and this covers what the process says about itself, which are two
different claims: a plugin whose files are exactly as installed can still answer this question
untruthfully.

A reply naming **fewer** actions than the manifest declared is not a mismatch. A plugin may
implement less than it advertised, and an invocation of a missing one is refused with
`PLUGIN_ACTION_UNKNOWN` — a smaller problem than a plugin claiming more.

`plugin_version` is informational and MUST NOT be compared. It is what the process says it is,
recorded so a diagnostic can name it; the record's `plugin_version` is what was installed, and a
mismatch between them means an edited manifest, which the digests already detect.

### 4.2 A plugin that does not answer is not a plugin

An implementation MUST apply a bounded wait to the handshake and MUST treat a plugin that does not
answer within it as unusable rather than as slow, refusing with `PLUGIN_FAILED`.

A process on disk with the right name and the wrong behavior is more dangerous than nothing there
at all, which is the same rule the Skill applies to resolving an Engine.

## 5. Invoke

```json
{"plugin_protocol_version": 1, "request": "invoke", "action": "view",
 "exchange": {"kind": "artifact",
              "media_type": "application/vnd.nostdb.graph+json",
              "path": "/tmp/nostdb-exchange-1a2b/graph.json",
              "bytes": 81920,
              "content_digest": "sha256:9e1f..."},
 "output_directory": "/project/.nostdb/out",
 "options": {}}
```

Reply:

```json
{"plugin_protocol_version": 1, "reply": "invoke", "status": "complete",
 "outputs": ["view.html", "view.data.bin"]}
```

| Member | Meaning |
| --- | --- |
| `action` | one the handshake named |
| `exchange` | section 6. **Absent** when the approval does not grant `graph_read` |
| `output_directory` | an existing directory the plugin may write into, absolute |
| `options` | action-specific, and opaque to this contract |
| `status` | `complete` or `partial` |
| `outputs` | paths the plugin wrote, relative to `output_directory` |

An `exchange` that is absent because `graph_read` was not approved is the permission meaning
something. A manager that supplied one anyway would have made the field decorative, and a user who
declined graph access would have been told something untrue.

### 5.1 What the manager does with `outputs`

Each entry MUST be relative, MUST NOT contain a `..` segment, and MUST match an approved
`output_paths` glob. An entry that does not is refused with `PLUGIN_FAILED`, and the invocation is
reported as failed rather than partial.

The manager checks what it was **told**; it does not police the filesystem. A plugin that wrote
somewhere it did not declare has done so, and section 1.1 is why this contract cannot pretend
otherwise. What the check buys is that a plugin cannot get the manager to *treat* an undeclared
file as a legitimate output — which is what a later action would go on to read.

An unreported file is not an output. A manager MUST NOT discover outputs by listing the directory:
that would make every stale file from a previous run part of this run's result.

### 5.2 `partial` is not `complete`

A plugin that finished some of its work reports `partial` and lists what it wrote. An
implementation MUST NOT report a partial invocation as a success, and MUST NOT delete what a
partial invocation produced — the outputs it named are real, and discarding them would throw away
work the plugin did and reported honestly.

This is the same rule enrichment follows for a partial semantic pass. Reporting partial work as
complete is the failure that makes a status field worthless.

## 6. The exchange

Graph data reaches a plugin as a **read-only artifact the Engine wrote**, never as a `.nostdb` and
never as a file the plugin may modify.

| Member | Meaning |
| --- | --- |
| `kind` | `artifact`. Reserved so a later version can add another handoff |
| `media_type` | what is inside it |
| `path` | absolute, and readable by the plugin for the life of the invocation |
| `bytes` | its exact length |
| `content_digest` | `sha256:` and 64 lower-case hexadecimal characters |

A plugin MUST verify `content_digest` before interpreting the artifact. The manager is not asking
to be trusted; it is stating what it wrote so the plugin can check.

The manager MUST remove the artifact after the invocation ends, including when it failed. An
artifact left behind is authorized graph data sitting in a temporary directory after the
authorization ended.

### 6.1 The media type is what evolves, not this protocol

`plugin_protocol_version` fixes the handoff. It does not fix what is handed over.

Version 1 defines one media type, `application/vnd.nostdb.graph+json`: a versioned JSON document
carrying the graph a plugin was authorized to read. It is deliberately unremarkable — it exists so
the handoff can be exercised end to end, and so a plugin author has something to read on day one.

A media type a plugin does not recognize is refused with `PLUGIN_REQUEST_INVALID`, naming the type
it was given. Adding a media type is not a change to this contract, which is the whole reason the
handoff and the payload are separated: the viewer's binary format arrives as another media type and
nothing here is replaced.

An implementation MUST NOT infer the media type from the path's extension. A file named `.json`
that is not the type the manager declared is a disagreement worth reporting, not one worth
resolving by guessing.

## 7. Refusals

A plugin refuses with a reply, never by exiting:

```json
{"plugin_protocol_version": 1, "reply": "error",
 "code": "PLUGIN_ACTION_UNKNOWN", "message": "this build implements only `view`"}
```

| Code | Raised by | Meaning |
| --- | --- | --- |
| `PLUGIN_PROTOCOL_UNSUPPORTED` | either | the message's version is not implemented |
| `PLUGIN_REQUEST_INVALID` | plugin | the request is malformed, names an unknown kind, or carries a media type the plugin does not read |
| `PLUGIN_ACTION_UNKNOWN` | either | the action is not one this plugin implements |
| `PLUGIN_IDENTITY_MISMATCH` | manager | the handshake disagrees with what was approved |
| `PLUGIN_REQUIRED` | manager | a plugin an action needs has no record in either scope |
| `PLUGIN_FAILED` | either | the action did not complete, or the plugin broke this protocol |

A plugin MUST NOT exit to signal a refusal, and MUST NOT write a partial success followed by an
error. An implementation that read a success and then found an error would have to decide which of
the two the plugin meant, and there is no answer to that.

A plugin that exits without replying is `PLUGIN_FAILED` with its exit status in the message, which
is the manager's account of what happened rather than the plugin's.

### 7.1 A malformed reply is not an invalid request

`PLUGIN_REQUEST_INVALID` is a plugin's complaint about a message it was **sent**. A malformed
**reply** — an unknown status, an absent outputs list, an error with no code, a code outside the
registry, or a reply answering a request that was not asked — is the plugin breaking this protocol,
and is `PLUGIN_FAILED`.

The distinction is which side has the defect, and it is worth a separate code because it is worth a
different response. A caller seeing `PLUGIN_REQUEST_INVALID` has a manager to fix or a version to
reconcile; one seeing `PLUGIN_FAILED` has a plugin to report to its author. Collapsing them would
tell a user to look in the wrong place.

`PLUGIN_PROTOCOL_UNSUPPORTED` is the exception that is deliberately raised by either side: a version
disagreement is not a defect in either, and whichever side received the message is the one that can
say so.

Every fixture in `message/invalid/` declares the `role` of the message it holds, so this rule is
checked rather than assumed.

### 7.1 A message is written for a person

`message` is human-readable and carries no structure a caller may branch on. `code` is the signal.

A `message` MUST NOT contain a credential, a token, or a path carrying either. A plugin diagnostic
reaches a user's terminal and a log, which are two of the places the product contract forbids a
secret from reaching.

## 8. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/plugin-protocol`](../fixtures/plugin-protocol). Each fixture pairs with an
`.expected` file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `message/valid/` | `accept` | the message is read, and its kind is the declared one |
| `message/invalid/` | `reject` | the message is refused with the declared code |
| `handshake/` | `accept` or `reject` | the reply agrees with the approved record, or is refused with the declared code |

No fixture starts a process. Every one tests what reading a message or comparing it against a
record can decide — which is where this suite has to stop, and the reason is the same one the
installation suite gives: a conformance suite that started a plugin to test starting one would be
executing arbitrary code to check the rules that decide whether to.
