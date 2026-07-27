# The named database catalog contract

Contract key: `catalog_version`
Current version: 1
Status: normative

The catalog maps stable local names to databases on one machine, for one operating-system
user. It is what makes `nostdb query --database @work` resolve to a file without the caller
knowing where that file is.

The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

## 1. What this document owns

This document defines the on-disk catalog: its location, its shape, what makes one invalid,
and how it survives interruption. [`../fixtures/catalog`](../fixtures/catalog) is the
conformance gate.

It does not own the daemon that reads it, the protocol that carries a name, the graph, or
the settings of any project a named database belongs to. Those belong to
`server_protocol_version`, to `nostdb_format_version`, and to `settings_version`.

### 1.1 The catalog is a convenience, never a requirement

A path-based command MUST NOT consult the catalog. `nostdb query ./project` works with no
catalog, no daemon, and no entry, and adding a catalog MUST NOT change what it does.

This is the direction the product contract fixes: Embedded Mode is the default and the
catalog is an addition to it. A build that made a path resolve through the catalog would have
turned an optional local service into a dependency of opening a file.

### 1.2 A name is not an identity

A catalog entry is a local nickname for a path. It is not a database identifier, and moving
a database does not carry its name along.

Two consequences follow, and both are deliberate:

- the same database MAY be named twice, under two names, in one catalog;
- the same name on two machines MAY mean unrelated databases.

Nothing in a `.nostdb` records the name it is catalogued under, so nothing has to be
rewritten when a name changes. An implementation MUST NOT write a catalog name into a
database.

### 1.3 An entry is a claim, not a guarantee

Registering a name does not prove the target exists, is readable, or is a database. Those are
decided when it is opened, and the answers change while the catalog sits still.

An implementation MUST NOT reject a catalog because an entry's target is absent. Section 4
of this document is the whole reason: a removable disk that is currently unplugged is not a
malformed catalog, and treating it as one would make every other name on the machine
unusable.

## 2. Location

```text
~/.nostdb/catalog.json
```

The catalog is per operating-system user. An implementation MUST resolve `~` to the current
user's home directory and MUST NOT read or write another user's catalog.

There is exactly one catalog per user. There is no project-level catalog, and no merge:
unlike settings, which merge a global document under a project one, a name means one thing
per machine per user.

## 3. Document shape

```json
{
  "catalog_version": 1,
  "databases": {
    "work": {
      "path": "/home/dana/projects/work/.nostdb/root.nostdb"
    }
  }
}
```

Both members are required. A catalog with no names is written as an empty `databases`
object, not as an absent member: absence would be indistinguishable from a truncated
document.

### 3.1 `databases`

A JSON object whose keys are names and whose values are entry objects.

An object rather than an array, because the name is the key. An array would allow two
entries claiming one name and leave the reader to decide which wins.

### 3.2 Names

A name MUST match:

```text
[A-Za-z0-9][A-Za-z0-9_-]*
```

Names are compared exactly, so two names differing only in case are two names.

The leading `@` used on the command line is **not** part of the name. `@work` on a command
line refers to the name `work`, and a catalog key that begins with `@` is invalid: it would
mean either a name that can never be typed or one that has to be typed twice.

The character set excludes path separators, whitespace, and `.` deliberately. A name travels
through command lines and protocol messages, and one that could be mistaken for a path is one
an implementation would eventually resolve as a path.

### 3.3 `path`

The absolute path of the database file the name refers to.

An implementation MUST reject a relative path. This is the opposite of the rule in
`settings_version`, which requires the database path to be **relative** to the project, and
the reason for the difference is that the two documents answer to different things: a project
setting is committed and shared, so it must not carry one machine's layout, while a catalog
is per user and per machine and is resolved from whatever working directory the caller
happened to be in. A relative path in a catalog has no anchor.

An implementation MUST NOT canonicalize the stored path in a way that requires the target to
exist. Registering a name for a database on a disk that is not currently mounted is
legitimate, and section 1.3 requires it to keep working.

Absoluteness is judged by the rules of the platform the implementation runs on, so
`/home/dana/db.nostdb` and `C:\Users\Dana\db.nostdb` are each absolute on their own platform
and neither is portable to the other. A catalog is per machine and per user and is not a
document to be copied between platforms; section 1.2 already says the same name on two
machines may mean unrelated databases.

## 4. A missing or unreadable target

An entry whose target cannot be opened stays in the catalog, and the failure is reported
against the operation that tried to use it, not against the catalog.

| Situation | Result |
| --- | --- |
| the name is not in the catalog | the operation fails; the catalog is valid |
| the target is absent or unreadable | that operation fails; every other name still resolves |
| the target is not a readable database | the container contract decides, with its own code |

A catalog MUST remain usable when one of its entries does not. Refusing the document because
one path is stale would mean a single unplugged disk takes every named database on the
machine with it.

## 5. Serialized mutation

Catalog writes MUST be serialized, and a catalog MUST NOT be left partially written.

An implementation MUST write a complete replacement document and move it into place, so a
reader sees either the previous catalog or the next one. A reader that finds a truncated
document MUST report `CATALOG_INVALID` rather than repairing it or treating the readable
prefix as the catalog.

Two processes MAY attempt a mutation at once. The last complete write wins, and neither
process may observe a document that is the concatenation of both.

## 6. Rejected documents

An implementation MUST reject, rather than repair, each of the following with
`CATALOG_INVALID`, except an unsupported version, which is `CATALOG_VERSION_UNSUPPORTED`:

| Condition | Why |
| --- | --- |
| `catalog_version` absent, or not a supported version | the version is what makes every other rule interpretable |
| the document is not a JSON object | there is nothing to read |
| `databases` absent, or not an object | an absent member is indistinguishable from a truncated file |
| a name does not match the form in section 3.2 | a name that cannot be typed is a name nobody can use |
| a name begins with `@` | the sigil belongs to the command line, not to the name |
| an entry is not an object, or its `path` is absent | an entry with no target is not a claim about anything |
| `path` is not absolute | a catalog is resolved from an arbitrary working directory, so a relative path has no anchor |
| `path` is empty | the same reason, with nothing to report |

Rejection reports every problem found rather than the first, so a hand-edited catalog can be
fixed in one pass.

A duplicate name is not in the table, because JSON gives it no single meaning: a document
with two `"work"` keys is refused by the reader for the same reason two entries in an array
would be, and an implementation MUST NOT silently keep the last one.

## 7. Versions and unknown fields

`catalog_version` MUST be present and MUST be a positive integer. A version this build does
not read is refused with `CATALOG_VERSION_UNSUPPORTED`.

An implementation MUST preserve an unknown member it did not write when it rewrites the
catalog. A newer build MAY have added a member this one does not understand, and dropping it
on the next `catalog remove` would make the older build a downgrade that silently discards
configuration.

This is the same preservation rule `settings_version` states, and for the same reason. It
does **not** extend to an unknown member inside a name that the document's own rules reject:
a malformed entry is refused before preservation applies.

## 8. Conformance

An implementation conforms when it reproduces every declared outcome in
[`../fixtures/catalog`](../fixtures/catalog). Each fixture pairs with an `.expected` file of
`key = value` lines.

| Directory | `outcome` | Requirement |
| --- | --- | --- |
| `valid/` | `accept` | the document is read and every name resolves to its stated path |
| `invalid/` | `reject` | the document is refused with the declared code |

An `accept` outcome is not a promise that any target exists. Section 1.3 is the reason: these
fixtures test what can be decided by reading the document, and whether a disk is mounted is
not one of those things.
