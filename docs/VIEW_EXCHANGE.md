# The viewer exchange contract

Contract key: `view_exchange_version`
Current version: 1
Status: normative

What a viewer plugin receives, and the layout of the `view.data.bin` a browser fetches.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

The media type `application/vnd.nostdb.view+bin`, its container layout, and every section a viewer
may read. [`../fixtures/view-exchange`](../fixtures/view-exchange) is the conformance gate.

It does not own the protocol carrying it, which is
[`PLUGIN_PROTOCOL.md`](PLUGIN_PROTOCOL.md) and states that adding a media type is not a change to
it. It does not own `.nostdb`, which is [`NOSTDB_FORMAT.md`](NOSTDB_FORMAT.md) and which a plugin
never receives.

### 1.1 A separate contract because a payload is not a handoff

The plugin protocol fixes how an artifact is handed over: where it is, how long, and what its
digest is. This fixes what is inside one.

Keeping them apart is what lets a renderer improve without renegotiating a protocol, and lets the
protocol grow a second handoff without invalidating a renderer. They are the first two contracts in
this project whose separation was designed before either was needed rather than discovered
afterwards.

### 1.2 This is not `.nostdb`

A viewer never reads the binary database and never receives a path into one. This container is
written *for* a viewer, from data the Engine already decided it was authorized to send.

The two formats are deliberately unalike, and the difference is the point: `.nostdb` is a
transactional store optimized for correctness under mutation, and this is a read-only snapshot
optimized for a renderer that has to draw it. A single format serving both would serve neither.

## 2. Why columnar and index-addressed

`docs/PRD.md` section 24.3 requires instanced node and edge rendering, incremental decoding, and no
allocation of the full graph as DOM elements. Those requirements fix the shape of this format more
than any aesthetic preference does.

- **columns, not records.** A renderer uploads a buffer per attribute. A record layout would make it
  walk the whole graph to gather one attribute, and copy it into the buffer it wanted in the first
  place;
- **edges name endpoints by index**, not by opaque identifier. An edge that named identifiers would
  make a renderer build a hash map over every node before it could draw a single line, which is
  exactly the work a million-edge graph cannot afford;
- **sections are independently locatable**, so a viewer may decode geometry and draw before it has
  read evidence metadata it does not need yet.

The opaque identifier is still present, in its own section, because source navigation needs it. What
changes is that a renderer never has to *resolve* one to draw.

## 3. Layout

```text
offset 0                      header, 32 bytes
32                            section table, section_count entries of 16 bytes
                              section payloads, in any order after the table
```

| Offset | Width | Field |
| --- | --- | --- |
| 0 | 8 | magic, the ASCII bytes `NOSTVIEW` |
| 8 | 2 | `view_exchange_version`, currently 1 |
| 10 | 2 | `section_count` |
| 12 | 4 | `node_count` |
| 16 | 4 | `edge_count` |
| 20 | 4 | `source_count` |
| 24 | 4 | CRC-32C of bytes 0 through 23 |
| 28 | 4 | reserved, and MUST be zero |

A section table entry:

| Offset | Width | Field |
| --- | --- | --- |
| 0 | 2 | section kind |
| 2 | 2 | reserved, and MUST be zero |
| 4 | 4 | payload offset from the start of the file |
| 8 | 4 | payload length in bytes |
| 12 | 4 | CRC-32C of the payload |

All integers are unsigned, fixed width, and little-endian, which is the rule
`NOSTDB_FORMAT.md` section 4 fixes and for the same reasons: a file is byte-identical across
platforms, a fixture is portable, and the magic detects a byte-swapped or text-mangled file with no
runtime probe.

The counts are in the header rather than only in the sections because a viewer allocates its buffers
before it decodes: reading a count out of a section would mean either two passes or a growing
allocation, and section 24.3 forbids the second at scale.

### 3.1 The counts and the sections must agree

A reader MUST refuse a file where a section's length is not what its kind and the header's counts
imply. A `node_labels` section of 40 bytes with a `node_count` of 3 is a disagreement, and a reader
that trusted either one over the other would draw a graph that is not the one it was sent.

## 4. Sections

| Kind | Name | Payload |
| --- | --- | --- |
| 1 | `strings` | length-prefixed UTF-8, see 4.1 |
| 2 | `node_ids` | `node_count` × 4-byte string index |
| 3 | `node_labels` | `node_count` × 4-byte string index, the primary label |
| 4 | `node_sources` | `node_count` × 4-byte source index |
| 5 | `edge_endpoints` | `edge_count` × two 4-byte node indices, source then target |
| 6 | `edge_relations` | `edge_count` × 4-byte string index |
| 7 | `sources` | `source_count` × see 4.2 |
| 8 | `evidence` | see 4.3 |

Sections 1 through 7 are **required**. `evidence` is optional, because a graph may carry none and
an empty section is not the same statement as an absent one.

A reader MUST refuse an unknown section kind rather than skip it. A viewer that skipped one would
render a graph missing whatever that section carried and report success, and the reader cannot know
whether what it skipped was decorative.

A section kind MUST NOT appear twice.

### 4.1 `strings`

```text
u32 count
count × ( u32 byte length, that many bytes of UTF-8 )
```

A string index is a position in this table. Index 0 is reserved for the empty string and a writer
MUST emit it, so a reader has an index meaning "nothing stated" without a sentinel value that could
be mistaken for a real one.

Every other index MUST be less than `count`. An index out of range is a refusal, not a fallback to
the empty string: a renderer showing a blank label where the file said something is a renderer
lying about its input.

### 4.2 `sources`

One entry per source, and this is the scoped source identity `docs/PRD.md` section 24.2 requires:

```text
u32 locator      string index of the canonical source locator
u32 alias        string index, or 0 when the link declared none
u8  kind         0 root, 1 local link, 2 remote link
u8  status       0 available, 1 unavailable
u16 reserved     MUST be zero
```

Index 0 MUST be the root, whose `kind` is 0 and whose `status` is 0. Every node names a source, so a
viewer can show which graph an item came from without inferring it.

`status` 1 is the **broken-link marker** section 24.2 requires. An unavailable link keeps its
declaration and its entry: the product contract requires an unavailable source to remain declared
and yield reachable partial results, so removing its entry would report it as never having been
declared.

### 4.3 `evidence`

Optional, and sparse rather than one entry per node, because most nodes in a large graph carry none
and a dense column would spend four bytes per node to say so:

```text
u32 count
count × ( u32 node index, u32 path string index, u32 line, u32 column )
```

Ordered by node index, ascending, and a node index MUST NOT repeat. A reader MAY therefore binary
search it, which is what makes source navigation on a click affordable without a second index.

A line or column of 0 means unstated. This is metadata for navigation and not a claim about the
source: a viewer opens what it names and does not assert that anything is still there.

## 5. Disconnected components stay disconnected

Nothing in this format can express a relationship that is not an edge in the graph, and that is
deliberate. `docs/PRD.md` section 24.2 requires disconnected components to remain disconnected in
one result, and a format with a "root" or "parent" field a layout could hang everything from would
make violating that the easy path.

A viewer laying out several components MUST NOT invent an edge between them, and MUST NOT report a
component count as though it were a graph property the Engine stated. It is a property of what the
viewer received.

## 6. Bounded reading

A reader MUST apply all of these before allocating on a count it read from the file, and MUST refuse
with `VIEW_EXCHANGE_INVALID` rather than attempting the work:

| Bound | Value |
| --- | --- |
| file bytes | 512 MiB |
| `section_count` | 16 |
| `node_count` | 4 194 304 |
| `edge_count` | 33 554 432 |
| `source_count` | 65 536 |
| strings in the table | 4 194 304 |
| bytes in one string | 65 536 |

A section whose offset or length falls outside the file, or which overlaps another section, is a
refusal. So is a payload whose CRC-32C does not match.

The node and edge bounds are four times the largest tier `docs/PRD.md` section 24.3 names, so a
graph the product intends to render is never near them. They exist to bound a hostile or truncated
file, not to cap a legitimate one.

### 6.1 `VIEW_CAPACITY_EXCEEDED` is not a refusal of this format

`VIEW_EXCHANGE_INVALID` means the bytes are not a readable exchange.

`VIEW_CAPACITY_EXCEEDED` means the bytes were fine and *this viewer on this machine* cannot render
them — which section 24.3 requires it to report rather than crash. The same file may exceed one
machine's capacity and not another's, so it is a fact about a renderer and never about the file.

A viewer MUST NOT report `VIEW_EXCHANGE_INVALID` for a graph it merely found too large, and MUST NOT
report `VIEW_CAPACITY_EXCEEDED` for a file it could not read. The two send a user to different
places: one to whoever produced the file, and one to a smaller graph or a better machine.

## 7. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/view-exchange`](../fixtures/view-exchange). Each fixture pairs with an `.expected`
file of `key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `container/valid/` | `accept` | the container is read, and its counts are the declared ones |
| `container/invalid/` | `reject` | the container is refused with the declared code |

A fixture is a `.bin` file, written by the generator in
[`../fixtures/view-exchange/README.md`](../fixtures/view-exchange/README.md) so that a reader can
see how each one was built rather than having to reverse-engineer a byte array.
