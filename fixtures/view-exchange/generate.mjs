// Writes every viewer exchange fixture.
//
// Kept as one readable script rather than as byte arrays in a test, so that a fixture's
// construction is something a reader can follow and a new one is something they can add.
//
// Nothing here is normative. docs/VIEW_EXCHANGE.md is.

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const MAGIC = "NOSTVIEW";

// CRC-32C, the same polynomial the container contract fixes. Table-driven so the fixtures are
// cheap to regenerate.
const TABLE = (() => {
  const table = new Uint32Array(256);
  for (let index = 0; index < 256; index += 1) {
    let value = index;
    for (let bit = 0; bit < 8; bit += 1) {
      value = value & 1 ? (value >>> 1) ^ 0x82f63b78 : value >>> 1;
    }
    table[index] = value >>> 0;
  }
  return table;
})();

function crc32c(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

const SECTION = {
  strings: 1,
  node_ids: 2,
  node_labels: 3,
  node_sources: 4,
  edge_endpoints: 5,
  edge_relations: 6,
  sources: 7,
  evidence: 8,
};

function u32(values) {
  const buffer = Buffer.alloc(values.length * 4);
  values.forEach((value, index) => buffer.writeUInt32LE(value >>> 0, index * 4));
  return buffer;
}

/** The string table: a count, then each string length-prefixed. Index 0 is the empty string. */
function strings(list) {
  const parts = [u32([list.length])];
  for (const text of list) {
    const bytes = Buffer.from(text, "utf8");
    parts.push(u32([bytes.length]), bytes);
  }
  return Buffer.concat(parts);
}

/** One source entry: locator, alias, kind, status, reserved. */
function sources(entries) {
  const buffer = Buffer.alloc(entries.length * 12);
  entries.forEach((entry, index) => {
    const at = index * 12;
    buffer.writeUInt32LE(entry.locator, at);
    buffer.writeUInt32LE(entry.alias ?? 0, at + 4);
    buffer.writeUInt8(entry.kind, at + 8);
    buffer.writeUInt8(entry.status ?? 0, at + 9);
    buffer.writeUInt16LE(0, at + 10);
  });
  return buffer;
}

function evidence(entries) {
  const parts = [u32([entries.length])];
  for (const entry of entries) {
    parts.push(u32([entry.node, entry.path, entry.line ?? 0, entry.column ?? 0]));
  }
  return Buffer.concat(parts);
}

/**
 * Assembles a container. `mutate` receives the finished buffer and the section table offsets, so a
 * rejection fixture can break exactly one thing and leave everything else valid — which is what
 * makes it a test of that one rule.
 */
function container({ version = 1, counts, sections, reserved = 0, mutate }) {
  const kinds = Object.keys(sections);
  const header = Buffer.alloc(32);
  header.write(MAGIC, 0, "ascii");
  header.writeUInt16LE(version, 8);
  header.writeUInt16LE(kinds.length, 10);
  header.writeUInt32LE(counts.nodes, 12);
  header.writeUInt32LE(counts.edges, 16);
  header.writeUInt32LE(counts.sources, 20);
  header.writeUInt32LE(crc32c(header.subarray(0, 24)), 24);
  header.writeUInt32LE(reserved, 28);

  const table = Buffer.alloc(kinds.length * 16);
  let offset = 32 + table.length;
  const payloads = [];
  kinds.forEach((kind, index) => {
    const payload = sections[kind];
    const at = index * 16;
    table.writeUInt16LE(SECTION[kind], at);
    table.writeUInt16LE(0, at + 2);
    table.writeUInt32LE(offset, at + 4);
    table.writeUInt32LE(payload.length, at + 8);
    table.writeUInt32LE(crc32c(payload), at + 12);
    payloads.push(payload);
    offset += payload.length;
  });

  const file = Buffer.concat([header, table, ...payloads]);
  return mutate ? mutate(file, { tableAt: 32, entrySize: 16, kinds }) : file;
}

/** A small graph: three nodes, two edges, one root source and one unavailable remote link. */
function sample(extra = {}) {
  const table = [
    "",
    "n_0198a1b2c3d47e5f8a9b0c1d2e3f4a5b",
    "n_0198a1b2c3d47e5f8a9b0c1d2e3f4a5c",
    "n_0198a1b2c3d47e5f8a9b0c1d2e3f4a5d",
    "Function",
    "Module",
    "CALLS",
    "file:///project",
    "github://example/payments/?ref=main",
    "payments",
    "src/login.rs",
  ];
  return {
    counts: { nodes: 3, edges: 2, sources: 2 },
    sections: {
      strings: strings(table),
      node_ids: u32([1, 2, 3]),
      node_labels: u32([4, 4, 5]),
      // The third node came from the linked source, which is what scoped identity carries.
      node_sources: u32([0, 0, 1]),
      edge_endpoints: u32([0, 1, 1, 2]),
      edge_relations: u32([6, 6]),
      sources: sources([
        { locator: 7, kind: 0, status: 0 },
        { locator: 8, alias: 9, kind: 2, status: 1 },
      ]),
      ...extra,
    },
  };
}

const write = (directory, name, bytes, expected) => {
  mkdirSync(join(here, directory), { recursive: true });
  writeFileSync(join(here, directory, `${name}.bin`), bytes);
  writeFileSync(join(here, directory, `${name}.expected`), expected);
};

// Accepted.
write("container/valid", "three_nodes_two_edges", container(sample()),
  "outcome = accept\nnodes = 3\nedges = 2\nsources = 2\nevidence = 0\n" +
  "note = A root graph and one unavailable remote link, so scoped identity and the broken-link marker are both exercised.\n");

write("container/valid", "with_evidence", container(sample({
    evidence: evidence([{ node: 0, path: 10, line: 12, column: 5 }]),
  })),
  "outcome = accept\nnodes = 3\nedges = 2\nsources = 2\nevidence = 1\n" +
  "note = Sparse, and ordered by node index, so a viewer may binary search it for source navigation.\n");

write("container/valid", "two_disconnected_components", container({
    counts: { nodes: 4, edges: 2, sources: 1 },
    sections: {
      strings: strings(["", "a", "b", "c", "d", "Function", "CALLS", "file:///project"]),
      node_ids: u32([1, 2, 3, 4]),
      node_labels: u32([5, 5, 5, 5]),
      node_sources: u32([0, 0, 0, 0]),
      // Two pairs, and nothing joining them. Nothing in this format could join them.
      edge_endpoints: u32([0, 1, 2, 3]),
      edge_relations: u32([6, 6]),
      sources: sources([{ locator: 7, kind: 0, status: 0 }]),
    },
  }),
  "outcome = accept\nnodes = 4\nedges = 2\nsources = 1\nevidence = 0\n" +
  "note = Two components with no edge between them. A viewer that invented one would be reporting a relationship the graph does not have.\n");

write("container/valid", "an_empty_graph", container({
    counts: { nodes: 0, edges: 0, sources: 1 },
    sections: {
      strings: strings(["", "file:///project"]),
      node_ids: u32([]),
      node_labels: u32([]),
      node_sources: u32([]),
      edge_endpoints: u32([]),
      edge_relations: u32([]),
      sources: sources([{ locator: 1, kind: 0, status: 0 }]),
    },
  }),
  "outcome = accept\nnodes = 0\nedges = 0\nsources = 1\nevidence = 0\n" +
  "note = A configured project with nothing built yet. Empty is a graph, not an error, and a viewer that refused it would refuse every new project.\n");

// Rejected. Each breaks one rule and leaves the rest valid.
const reject = (name, bytes, note) =>
  write("container/invalid", name, bytes,
    `outcome = reject\ncode = VIEW_EXCHANGE_INVALID\nnote = ${note}\n`);

{
  const file = container(sample());
  file.write("NOSTVIEX", 0, "ascii");
  reject("bad_magic", file, "The magic detects a byte-swapped or text-mangled file with no runtime probe.");
}
{
  const file = container({ ...sample(), version: 2 });
  reject("unsupported_version", file, "The version is what makes every other rule interpretable, so it is read before anything else.");
}
{
  const file = container(sample());
  file.writeUInt32LE(0xdeadbeef, 24);
  reject("header_checksum_does_not_match", file, "The header carries the counts a reader allocates on, so a corrupt one is caught before any allocation.");
}
{
  const file = container(sample());
  // Flip a byte inside the first payload, leaving its recorded checksum alone.
  const payloadAt = 32 + 7 * 16;
  file[payloadAt] ^= 0xff;
  reject("payload_checksum_does_not_match", file, "Per-section checksums, so a reader knows which section is wrong rather than only that something is.");
}
{
  const file = container(sample());
  file.writeUInt32LE(9, 12);
  reject("node_count_disagrees_with_the_section", file, "A node_labels section of three entries with a node_count of nine is a disagreement, and trusting either over the other would draw a graph nobody sent.");
}
{
  const file = container(sample());
  // Point the first section past the end of the file.
  file.writeUInt32LE(file.length + 64, 32 + 4);
  reject("section_offset_outside_the_file", file, "Bounded reading: an offset outside the file is refused before any read is attempted.");
}
{
  const file = container(sample());
  file.writeUInt32LE(0xffff_0000, 32 + 8);
  reject("section_length_overflows", file, "A length that would run past the end is refused rather than clamped: clamping would decode a truncated section as a shorter graph.");
}
{
  const file = container(sample());
  // Two sections claiming the same bytes.
  file.writeUInt32LE(file.readUInt32LE(32 + 4), 32 + 16 + 4);
  file.writeUInt32LE(file.readUInt32LE(32 + 8), 32 + 16 + 8);
  file.writeUInt32LE(file.readUInt32LE(32 + 12), 32 + 16 + 12);
  reject("sections_overlap", file, "Overlapping sections mean one of the two is being read as something it is not.");
}
{
  const file = container(sample());
  file.writeUInt16LE(99, 32);
  reject("unknown_section_kind", file, "Refused rather than skipped. A viewer that skipped one would render a graph missing whatever it carried and report success.");
}
{
  const file = container(sample());
  // Two entries with the same kind.
  file.writeUInt16LE(file.readUInt16LE(32), 32 + 16);
  reject("duplicate_section_kind", file, "One kind twice has no rule for which wins.");
}
{
  const sections = { ...sample().sections };
  delete sections.sources;
  const file = container({ counts: { nodes: 3, edges: 2, sources: 2 }, sections });
  reject("a_required_section_is_absent", file, "Every node names a source, so a container without the sources section describes items whose origin cannot be shown.");
}
{
  const base = sample();
  const file = container({
    ...base,
    sections: { ...base.sections, node_labels: u32([4, 4, 999]) },
  });
  reject("string_index_out_of_range", file, "Refused, not defaulted to the empty string: a renderer showing a blank label where the file said something is lying about its input.");
}
{
  const base = sample();
  const file = container({
    ...base,
    sections: { ...base.sections, edge_endpoints: u32([0, 1, 1, 7]) },
  });
  reject("edge_endpoint_out_of_range", file, "An endpoint past the node count is the one corruption that would otherwise read out of a renderer's buffer.");
}
{
  const base = sample();
  const file = container({
    ...base,
    sections: {
      ...base.sections,
      sources: sources([
        { locator: 8, alias: 9, kind: 2, status: 1 },
        { locator: 7, kind: 0, status: 0 },
      ]),
    },
  });
  reject("source_zero_is_not_the_root", file, "Index 0 is the root by definition, so a container whose first source is a link has no root to attribute anything to.");
}
{
  const base = sample();
  const file = container({
    ...base,
    sections: {
      ...base.sections,
      evidence: evidence([
        { node: 2, path: 10, line: 1, column: 1 },
        { node: 0, path: 10, line: 2, column: 1 },
      ]),
    },
  });
  reject("evidence_is_out_of_order", file, "Ascending node index is what lets a viewer binary search it. Out of order, a search would miss entries that are present.");
}
{
  const file = container(sample());
  file.writeUInt32LE(1, 28);
  reject("reserved_is_not_zero", file, "A reserved field is where a later version puts something. A writer setting it now would make that version unable to tell an old file from a new one.");
}
{
  const file = container(sample()).subarray(0, 20);
  reject("truncated_before_the_header_ends", Buffer.from(file), "A file shorter than its own header cannot state what it is.");
}

console.log("view exchange fixtures written");
