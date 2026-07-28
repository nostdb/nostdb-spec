# The provider protocol contract

Contract key: `provider_protocol_version`
Current version: 1
Status: normative

A provider retrieves bytes and metadata from somewhere the Engine cannot reach on its own.
It runs out of process, and this document defines the conversation between the two.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

This document defines the request and reply messages, the two provider roles, the
`github://` locator grammar, and how a credential reaches a provider without being written
down. [`../fixtures/provider`](../fixtures/provider) is the conformance gate.

It does not own what the retrieved bytes mean. A provider returns bytes; only the Engine
interprets `.nost` or `.nostdb`. A provider that parsed either would be a second parser of a
format that has exactly one.

### 1.1 Out of process, and why that is the default

A provider is a separate executable. Third-party providers MUST run out of process unless
separately approved for in-process use.

The reason is not performance. A provider is the component that holds a credential and talks
to a network, and it is the component most likely to come from somebody other than whoever
shipped the Engine. Keeping it behind a process boundary means a provider cannot read the
Engine's memory, cannot reach a database handle, and cannot outlive the request it was
started for.

### 1.2 Retrieval is not interpretation

A provider answers three questions and no others: what does this locator resolve to, what
does that snapshot contain, and what are the bytes of one entry.

It does not decide which entries are interesting, does not analyze, and does not report a
graph. Section 15.1 of the product contract splits this into two roles precisely so a
provider that can hand over a repository's source is not thereby able to hand over a graph.

## 2. Transport

One request per line, one reply per line, both JSON, over the provider's standard input and
standard output. Standard error is for diagnostics and MUST NOT carry a reply.

A line-delimited stream is chosen over a framed binary protocol because a provider is
expected to be written in whatever language its ecosystem favors, and every one of them can
write a line of JSON.

Binary content does not travel in JSON. A `read` reply names a length and the bytes follow
the newline as an opaque run, which keeps a megabyte blob from being base64-inflated by a
third on every hop.

### 2.1 Every message carries the version

```json
{"provider_protocol_version": 1, "request": "...", ...}
```

A provider MUST refuse a request whose version it does not implement, and MUST refuse it
with a reply rather than by exiting. An Engine that gets no reply cannot tell a version
mismatch from a crash, and the two need different things from whoever hits them.

## 3. Roles

A provider declares which roles it implements, and MAY implement both.

| Role | Answers |
| --- | --- |
| `source` | resolve a source locator, enumerate its entries, read one entry |
| `graph_store` | resolve a graph locator, materialize a read-only artifact |

Section 15.1 keeps these apart because analyzing a repository and federating a published
graph are different permissions over the same host. A deployment may want the second without
the first.

## 4. Requests

### 4.1 `handshake`

```json
{"provider_protocol_version": 1, "request": "handshake"}
```

Reply:

```json
{"provider_protocol_version": 1, "reply": "handshake",
 "provider": "github", "provider_version": "1.0.0",
 "roles": ["source", "graph_store"]}
```

The Engine MUST send this first and MUST NOT send anything else until it has a reply. A
provider that answers a `read` before agreeing on a version has already guessed what the
request meant.

### 4.2 `resolve`

```json
{"provider_protocol_version": 1, "request": "resolve",
 "locator": "github://example/payments/?ref=main",
 "credential": {"ref": "github.work"}}
```

Reply:

```json
{"provider_protocol_version": 1, "reply": "resolve",
 "snapshot": "0f1e2d3c4b5a69788796a5b4c3d2e1f009182736",
 "canonical_locator": "github://example/payments/?ref=main",
 "cached": false}
```

`snapshot` is an immutable identifier — for GitHub, a commit. A branch or tag MUST be
resolved to one before anything is enumerated or read, and every later request in the same
build or query MUST use that one snapshot.

`canonical_locator` is the locator normalized per section 6. It MAY differ from the
requested one when a browser URL was accepted, and the Engine stores the canonical form,
because a locator is a link's identity and two spellings of one identity is two links.

`cached` reports that the snapshot came from a cache rather than from the host. Section 16.3
permits serving a cached immutable snapshot while the host is unreachable and requires
saying so. A provider MUST NOT report `false` for a snapshot it did not confirm.

### 4.3 `enumerate`

```json
{"provider_protocol_version": 1, "request": "enumerate", "snapshot": "0f1e2d..."}
```

Reply:

```json
{"provider_protocol_version": 1, "reply": "enumerate",
 "entries": [{"path": "src/main.rs", "bytes": 412, "content_id": "b1946ac9..."}]}
```

`content_id` identifies the entry's bytes within the host's own scheme — a Git blob ID for
GitHub. It is **not** a content digest the Engine may trust: section 16.3 uses it to decide
what to *avoid downloading*, and every artifact that is downloaded receives an independent
cryptographic digest before it is opened.

Enumeration MUST precede reading where the host allows it. Fetching a file to discover its
size is the cost this reply exists to avoid.

### 4.4 `read`

```json
{"provider_protocol_version": 1, "request": "read",
 "snapshot": "0f1e2d...", "path": "src/main.rs"}
```

Reply, followed immediately by exactly `bytes` bytes of content:

```json
{"provider_protocol_version": 1, "reply": "read", "bytes": 412}
```

The Engine MUST read exactly that many bytes and MUST NOT treat what follows as a line. A
provider that writes fewer has failed the request; one that writes more has corrupted the
stream and the Engine MUST close it rather than resynchronize, because a stream whose
framing is wrong cannot be trusted to say so.

### 4.5 `materialize`

```json
{"provider_protocol_version": 1, "request": "materialize", "snapshot": "0f1e2d..."}
```

Reply:

```json
{"provider_protocol_version": 1, "reply": "materialize",
 "bytes": 81920, "content_digest": "sha256:..."}
```

Belongs to the `graph_store` role. The artifact is read-only, and the Engine MUST verify
`content_digest` against the bytes it received before opening it. A provider is not trusted
to have got this right; the digest is what the Engine checks, not what it accepts.

## 5. Refusals

```json
{"provider_protocol_version": 1, "reply": "error",
 "code": "PROVIDER_SOURCE_UNAVAILABLE", "message": "the host did not answer"}
```

| Code | Meaning |
| --- | --- |
| `PROVIDER_PROTOCOL_UNSUPPORTED` | the request's version is not implemented |
| `PROVIDER_REQUEST_INVALID` | the request is malformed or names an unknown kind |
| `PROVIDER_LOCATOR_INVALID` | the locator is not one this provider reads |
| `PROVIDER_CREDENTIAL_REQUIRED` | the source needs a credential that was not supplied |
| `PROVIDER_CREDENTIAL_REJECTED` | the host refused the credential |
| `PROVIDER_SOURCE_UNAVAILABLE` | the host could not be reached, or has no such snapshot |
| `PROVIDER_LIMIT_EXCEEDED` | a host quota or rate limit was reached |

A refusal is a reply. A provider MUST NOT exit to signal one, and MUST NOT write a partial
success followed by an error.

`PROVIDER_SOURCE_UNAVAILABLE` leaves a link **declared**. The product contract requires an
unavailable source to keep its declaration and yield reachable partial results, so this is
not a build failure and an implementation MUST NOT treat it as one.

### 5.1 A message is written for a person and read by nobody else

`message` is human-readable and carries no structure a caller may branch on. `code` is the
signal. A caller matching on message text would break the first time the wording improved.

A `message` MUST NOT contain a credential, a token, a URL carrying either, or a header that
would reveal one. Section 15.3 forbids raw credentials in logs, command output, caches,
settings, links, plugin locks, `.nost`, and `.nostdb`; a provider diagnostic is the same
kind of place and is left out of that list only because providers did not exist when it was
written.

## 6. The `github://` locator

```text
github://<owner>/<repository>/<path>?ref=<git-ref>
```

| Part | Case | Notes |
| --- | --- | --- |
| `owner` | canonicalized to lower case | GitHub treats it case-insensitively |
| `repository` | canonicalized to lower case | the same |
| `path` | preserved | a repository's own paths are case-sensitive |
| `ref` | preserved | a Git ref is case-sensitive |

`ref` is required. An implementation MUST NOT default it to a branch name, because the
default branch of a repository can change and a locator is an identity.

Reserved characters are percent-encoded. A credential MUST NOT appear anywhere in a locator.

An empty `path` names the repository root and is written with a trailing `/`.

### 6.1 Browser URLs

A provider MAY accept `https://github.com/<owner>/<repository>` and the `tree` and `blob`
forms, and MUST normalize to the canonical grammar before storing or comparing. Accepting
one and storing it unnormalized would give one repository two identities and one link two
rows.

## 7. Credentials

A request carries `{"ref": "<name>"}` and never a secret. The provider resolves the name
through its own configured resolver — an environment variable, an OS credential store, a
protected key path, or a process-memory-only prompt.

The Engine does not read the secret, so it cannot leak one it never had. That is the point
of passing a name: a component that never holds a credential cannot be the component that
writes it somewhere.

A provider that needs a credential and was given none MUST reply
`PROVIDER_CREDENTIAL_REQUIRED` rather than attempting an anonymous request. A silent
downgrade to anonymous access turns a permissions problem into a "repository not found",
which sends whoever hits it looking in the wrong place.

## 8. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/provider`](../fixtures/provider). Each fixture pairs with an `.expected` file
of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `locator/valid/` | `accept` | the locator parses, and normalizes to the declared canonical form |
| `locator/invalid/` | `reject` | the locator is refused with the declared code |
| `message/valid/` | `accept` | the message is one this version defines |
| `message/invalid/` | `reject` | the message is refused with the declared code |

No fixture reaches a network. Every one of them tests what can be decided from a document or
a string, which is the same reason the change-set fixtures stop where they do: a suite that
needed a live third-party service would be a suite nobody could run.
