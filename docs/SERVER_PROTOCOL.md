# The local daemon protocol contract

Contract key: `server_protocol_version`
Current version: 1
Status: normative

The local protocol carries requests between a client and the per-user daemon over an endpoint
only the current operating-system user can reach. It is what `nostdb query --database @work`
speaks.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

This document defines the endpoint, the framing, version negotiation, the request and
response envelopes, and what makes a message invalid.
[`../fixtures/server`](../fixtures/server) is the conformance gate.

It does not own the query language, the result shape, the catalog, the graph, or the
container. Those belong to `query_subset_version`, `result_version`, `catalog_version`, and
`nostdb_format_version`. In particular, section 5.3 carries a result envelope **by reference**
and does not restate one byte of it.

### 1.1 The daemon is never required

Every operation in this protocol MUST also be reachable without it. A path-based command runs
in Embedded Mode against a file, and a readable database opens with no daemon running.

The protocol exists to manage *named* databases on a machine, not to stand between a caller
and a file. An implementation that made any file operation depend on a running daemon would
have contradicted the product contract it implements.

### 1.2 The operating system authenticates, not the protocol

There are no passwords, tokens, or credentials in this protocol, and there MUST NOT be. The
endpoint is restricted to the current user by the operating system, and a peer that reaches it
has already been authenticated by something stronger than this document could specify.

An implementation MUST reject a peer belonging to another operating-system user, and MUST NOT
offer a setting that disables that rejection. It MUST NOT add an authentication message on the
grounds that the endpoint might one day be remote: section 2 forbids the remote endpoint, so
that day requires a new version of this contract.

### 1.3 Local only

The MVP daemon MUST NOT listen on TCP, UDP, or HTTP, and MUST NOT accept a connection from
another host. This is a property of version 1 rather than an unfinished feature.

## 2. Endpoint

| Platform | Endpoint |
| --- | --- |
| Unix-like | `~/.nostdb/run/nostdb.sock`, a Unix domain socket |
| Windows | `\\.\pipe\nostdb-<user-sid>`, a named pipe |

On Unix the socket MUST be created such that only the owning user may connect, and the
directory holding it MUST NOT be world-writable. On Windows the pipe MUST carry an access
control list naming the current user.

An implementation MUST verify the restriction rather than assume it. A socket inherited from a
previous run with wider permissions is a socket that must be replaced, not reused.

### 2.1 One daemon per user

At most one daemon runs per operating-system user, enforced by an operating-system lock rather
than by checking whether the endpoint answers.

A start request that finds a healthy daemon MUST succeed and report the existing endpoint,
with `SERVER_ALREADY_RUNNING` as an informational outcome rather than a failure. Starting
something already started is what the caller wanted.

A lock held by a process that no longer exists is stale. An implementation MUST reclaim a
stale lock and MUST NOT treat a leftover socket file as proof of a running daemon: a killed
process leaves both behind, and refusing to start after a crash would require manual cleanup
that no user was told about.

## 3. Framing

A message is a 4-byte unsigned big-endian byte count followed by exactly that many bytes of
UTF-8 JSON.

Big-endian because it is the network order this project already uses in the container header,
and a second byte order in one product is a bug waiting for a different machine.

An implementation MUST refuse a frame whose declared length exceeds its configured maximum,
and MUST do so without allocating that length. The maximum MUST be at least 8 MiB.

A connection carries many messages in both directions. It is not request-per-connection: a
session in section 6 spans messages, and reconnecting for each one would make a transaction
impossible to express.

## 4. Handshake and version negotiation

The first message on a connection MUST be a `hello` from the client:

```json
{
  "message": "hello",
  "client": "nostdb-cli",
  "supported_versions": [1]
}
```

The daemon replies with the highest version both sides support:

```json
{
  "message": "welcome",
  "server_protocol_version": 1,
  "endpoint": "/home/dana/.nostdb/run/nostdb.sock"
}
```

If the sets do not intersect, the daemon MUST reply with `SERVER_PROTOCOL_UNSUPPORTED`, state
the versions it supports, and close the connection:

```json
{
  "message": "refused",
  "code": "SERVER_PROTOCOL_UNSUPPORTED",
  "supported_versions": [1]
}
```

`hello` deliberately carries no `server_protocol_version` of its own. A client that must
already know the version in order to ask which version is supported cannot negotiate at all,
which is the mistake this shape exists to avoid.

`refused` carries none either, for a stronger reason: there is no negotiated version to
state. The two sides have just established that they have none in common, and naming one
would be a claim about a language neither agreed to speak. It states `supported_versions`
instead, which is the actionable part.

Once `welcome` has named a version, every subsequent message MUST carry it, so a stray
message from a differently versioned process is detected rather than misread.

An implementation MUST NOT guess at a version outside its supported set, and MUST NOT fall
back to a best-effort parse. A refusal names the versions it has; that is enough for a client
to report something actionable.

## 5. Requests and responses

### 5.1 Request

```json
{
  "server_protocol_version": 1,
  "request_id": "r1",
  "session_id": "s1",
  "operation": "query",
  "database": "work",
  "statement": "MATCH (n) RETURN count(n)"
}
```

`request_id` is opaque to the daemon and MUST be echoed. It MUST be unique among a
connection's outstanding requests, because responses MAY arrive in any order: a client that
pipelines two requests and gets one reply back has no way to tell which one it answers
otherwise.

`database` is a catalog **name**, without the `@` sigil, as `catalog_version` section 3.2
defines it. This protocol MUST NOT accept a filesystem path in `database`. A path is Embedded
Mode's business, and accepting one here would make the daemon a second route to a file with
different rules.

### 5.2 Operations

| `operation` | Effect |
| --- | --- |
| `open_session` | starts a session and returns its `session_id` |
| `close_session` | ends a session, rolling back an open transaction |
| `query` | runs one statement against the named database |
| `begin`, `commit`, `rollback` | explicit transaction control within a session |
| `status` | reports the daemon's endpoint, uptime, and session count |
| `shutdown` | stops the daemon after ending its sessions |

An unknown `operation` MUST be refused by name rather than ignored.

### 5.3 Response

```json
{
  "server_protocol_version": 1,
  "request_id": "r1",
  "outcome": "ok",
  "result": {}
}
```

For a `query`, `result` MUST be a result envelope exactly as `result_version` defines it,
carried verbatim.

This contract states no field of that envelope. The daemon calls the Engine, receives the
envelope the Engine built, and forwards it. A daemon that assembled its own would be a second
implementation of a published shape, and the two would drift on the first change to either.

A failed request carries `"outcome": "error"` and the diagnostics the Engine produced. The
daemon MUST NOT translate a diagnostic code into one of its own, and MUST NOT add a code for a
failure the Engine already named.

## 6. Sessions and transactions

A session is the unit of isolation. Requests in one session see one consistent view; requests
in different sessions MUST NOT observe each other's uncommitted work.

A transaction lives inside a session and MUST NOT span two. Closing a session with an open
transaction MUST roll it back: a client that disconnects mid-transaction has not decided to
commit, and treating a dropped connection as consent is how a partial write becomes permanent.

A dropped connection MUST end its sessions. The daemon MUST reclaim a session whose client is
gone rather than holding its resources until shutdown.

Writes affect only the database the session targets. Linked databases are read-only from it,
which `query_subset_version` already requires and this protocol does not relax.

## 7. Limits and timeouts

An implementation MUST enforce, and MUST make configurable:

- a maximum frame size, at least 8 MiB;
- a query timeout;
- a maximum number of concurrent sessions;
- a per-session memory or result-size ceiling.

A request stopped by a limit MUST report which limit stopped it. "Failed" without the limit
leaves a caller to guess between a bug and a ceiling, and those have opposite fixes.

Reaching a limit MUST leave the last valid database generation intact.

## 8. Rejected messages

An implementation MUST refuse, rather than repair, each of the following. An unsupported
version is `SERVER_PROTOCOL_UNSUPPORTED`; the rest are refused as malformed for the stated
reason and carry no code of their own, because a code is a contract with a caller and a
client that cannot frame a message correctly is not yet a caller.

| Condition | Why |
| --- | --- |
| `supported_versions` does not intersect the daemon's | the two sides have no language in common |
| a post-handshake message omits `server_protocol_version` | a message from a differently versioned process must not be misread |
| the first message is not `hello` | negotiation has not happened, so nothing after it is interpretable |
| a frame's declared length exceeds the maximum | an attacker MUST NOT be able to name an allocation |
| the frame body is not a JSON object | there is nothing to read |
| `request_id` is absent or repeats among outstanding requests | a response could not be matched to its request |
| `operation` is absent or unknown | guessing which was meant is how a typo becomes a write |
| `database` holds a path rather than a name | the daemon is not a second route to a file |
| a request names an unknown `session_id` | the isolation the session guaranteed no longer exists |
| the peer belongs to another operating-system user | the endpoint's only authentication is the operating system's |

## 9. Versions

`server_protocol_version` is negotiated once per connection and MUST NOT change within one.

A version this build does not speak is refused with `SERVER_PROTOCOL_UNSUPPORTED` in both
directions: an older client reaching a newer daemon and a newer client reaching an older one
get the same answer, with the supported set attached.

This version evolves independently of every other contract. A change to the result envelope,
the catalog, or the container MUST NOT renumber it, and renumbering it MUST NOT invalidate a
database.

## 10. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/server`](../fixtures/server). Each fixture pairs with an `.expected` file of
`key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `valid/` | `accept` | the message is well formed at the negotiated version |
| `invalid/` | `reject` | the message is refused for the declared reason |

The fixtures are message documents, not transcripts. Framing, endpoint permissions, the
one-daemon lock, and session isolation are behavioral and cannot be expressed as a JSON
document; `nostdb-server` proves those with its own tests, and section 8 lists what each one
must refuse.
