# NostDB contract versions

Every NostDB contract carries its own version and evolves independently. A bump
in one version never implies a bump in another, and no implementation may couple
two of them.

`versions.json` is the machine-readable form of this table. The conformance suite
checks that the two agree and that every contract marked `specified` names a file
that exists.

## Registry

| Contract key | Current | Supported | Status | Specified in |
| --- | --- | --- | --- | --- |
| `nost_language_version` | 4 | 4 | specified | [docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) |
| `nostdb_format_version` | 3 | 3 | specified | [docs/NOSTDB_FORMAT.md](docs/NOSTDB_FORMAT.md) |
| `query_subset_version` | 1 | 1 | specified | [docs/QUERY_SUBSET.md](docs/QUERY_SUBSET.md) |
| `settings_version` | 1 | 1 | specified | [docs/SETTINGS.md](docs/SETTINGS.md) |
| `credentials_version` | 1 | 1 | deferred | not yet specified |
| `catalog_version` | 1 | 1 | specified | [docs/CATALOG.md](docs/CATALOG.md) |
| `result_version` | 2 | 2 | specified | [docs/RESULT.md](docs/RESULT.md) |
| `provider_protocol_version` | 1 | 1 | specified | [docs/PROVIDER_PROTOCOL.md](docs/PROVIDER_PROTOCOL.md) |
| `plugin_protocol_version` | 1 | 1 | specified | [docs/PLUGIN_PROTOCOL.md](docs/PLUGIN_PROTOCOL.md) |
| `manifest_version` | 1 | 1 | specified | [docs/PLUGIN_MANIFEST.md](docs/PLUGIN_MANIFEST.md) |
| `plugin_install_version` | 2 | 2 | specified | [docs/PLUGIN_INSTALL.md](docs/PLUGIN_INSTALL.md) |
| `view_exchange_version` | 1 | 1 | specified | [docs/VIEW_EXCHANGE.md](docs/VIEW_EXCHANGE.md) |
| `server_protocol_version` | 1 | 1 | specified | [docs/SERVER_PROTOCOL.md](docs/SERVER_PROTOCOL.md) |
| `change_set_version` | 1 | 1 | specified | [docs/CHANGE_SET.md](docs/CHANGE_SET.md) |

A `deferred` contract has a reserved key and an agreed starting version, but no
authored contract yet. Reserving the key now is what keeps a later contract from
inventing a competing version field.

## Why the versions are independent

A single product version would force unrelated churn. Changing the `.nostdb`
container layout must not invalidate a `.nost` file, and adding a plugin action
must not renumber the daemon protocol.

Consequences an implementation must honor:

- report every supported version separately, as `nostdb --version --json` does;
- accept a file or message whose contract version is supported, regardless of
  the versions of unrelated contracts;
- refuse a contract version above the highest supported with an explicit
  diagnostic rather than a best-effort parse;
- never invalidate a cache because an unrelated component changed.

## Adding a version

1. Raise `current` for that one key, and extend `supported` only if the older
   version still round-trips.
2. Record the change in the contract document that owns the key.
3. Add fixtures covering the new version and the now-unsupported case.
4. Leave every other key untouched.

## Recorded bump: `nost_language_version` 3 to 4, `nostdb_format_version` 2 to 3, and `result_version` 1 to 2

Three keys moved for one change and independently of each other: a property value
may be an object, so a schema field may declare one, a container must store one,
and a result envelope must be able to return one.

`nost_language_version` 4 adds an anonymous object type in field position, an
object literal in value position, a repeatable array suffix, and an optional
separator between two fields or two properties. See
[docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) section 1.1.

`nostdb_format_version` 3 adds a map value tag and makes a list element a value
rather than a scalar. See [docs/NOSTDB_FORMAT.md](docs/NOSTDB_FORMAT.md).

**Every one of the three lists exactly one version**, and the format's list was
briefly two.

For the language, listing 3 would be wrong even though every version 4 syntax is
additive: a reader accepting `@nost 3` must refuse the syntax version 3 had no
production for, or the number it read governed nothing. Gating syntax on a declared
version is real machinery, and it buys a one-line edit to a file that is usually
generated from the database anyway.

The format listed 2 and 3 for one revision, reasoned as protecting user-owned
contributions no analyzer can rebuild. **That reasoning describes a released
product, and this one is not released.** No database exists whose loss would be a
user's rather than a developer's, so the compatibility cost a version branch in the
schema reader and a version field on the container to feed it, and bought nothing.
Both are gone.

The refusal is unchanged and is the part [docs/PRD.md](../docs/PRD.md) section 12
actually requires: an unsupported version is reported with the version in the
diagnostic rather than decoded on a guessed layout. Narrowing the range also made a
row of the format contract's comparison table reachable for the first time — a
version *below* the minimum, which no header could express while the minimum was
the lowest version ever written.

`result_version` 2 adds one value form, `{"object": {...}}`, and widens a list to
hold any value. It lists **2 alone**, and the reason is what `supported` means in
this registry: the versions *this implementation accepts as input*. Nothing in
NostDB reads a result envelope — it only produces them — so listing 1 would claim
a reader that does not exist, which is the same defect as a value declared and
never produced.

Version 1 is therefore not a compatibility promise but a description of what
earlier builds emitted. A consumer still holding a version 1 reader meets
`{"object": ...}` as an **unknown tag** and refuses it, which is why the form is
tagged rather than bare: an unknown tag is refusable, and a bare object would have
been silently misread as one of the tagged forms.

`query_subset_version` deliberately does **not** move. Returning a property whose
value is an object is the envelope's business; reaching *inside* one from a query
would be new query syntax, and none is added here. See
[docs/QUERY_SUBSET.md](docs/QUERY_SUBSET.md).

## Recorded bump: `nost_language_version` 2 to 3, and `nostdb_format_version` 1 to 2

Both moved in one revision, for the same underlying change and independently of
each other: a contribution's owner is one string.

`nost_language_version` 3 replaced three keyword owner forms —
`analyzer "<name>" "<version>"`, `ai "<digest>"`, and `user` — with one string
whose kind follows from the name, and removed the version an analyzer owner
carried. `producer_version` is now always stated, because no owner supplies one
to inherit. See [docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) section 1.2.

`nostdb_format_version` 2 replaced three tagged owner shapes with one interned
name. See [docs/NOSTDB_FORMAT.md](docs/NOSTDB_FORMAT.md) section 13.2.

Neither lists its predecessor as supported, and that is the decision rather than
an oversight. There is no reader for the earlier owner spellings, so listing them
would promise a parse no implementation can deliver — and a version 1 database
read by a version 2 reader would decode until it reached an owner byte and then
report an unknown tag, which is what a *corrupt* file reports. Refusing at the
header instead says what is true: a database to rebuild.

Two keys moved together and neither implies the other. A reader supporting one
and not the other is a coherent implementation, which is the independence this
registry exists to keep.

## Recorded bump: `nost_language_version` 1 to 2

Version 2 removed the module declaration, introduced schema declarations,
changed the node and edge forms, made the record identifier a reserved property
key holding a prefixed UUID, and added contribution and evidence blocks. See
[docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) section 1.3.

`supported` lists 2 alone. Version 1 does not round-trip through version 2,
because a version 1 file requires a module block that version 2 has no
declaration for, so listing 1 as supported would promise a parse no
implementation can deliver. A version 1 file is `NOST_VERSION_UNSUPPORTED`.

No other key moved. In particular `nostdb_format_version` stayed at 1: the
container gained no section and changed no layout, which is exactly the
independence this registry exists to keep.

## Recorded bump: `plugin_install_version` 1 to 2

Version 1 recognised a plugin by a **path**: a source named a subdirectory with a fragment, and any
directory holding `nostdb-plugin.json` was installable. Version 2 recognises one by a **declaration**:
a repository must carry `nostdb.plugins.json` at its root, that file maps names to directories, and a
source's fragment names a key in it.

`supported` is `[2]` and not `[2, 1]`. The two cannot round-trip in either direction: a version-1
source has no index and version 2 refuses it, and a version-2 fragment names a key that version 1
would read as a directory path and fail to find. Listing 1 as supported would mean an implementation
could accept a repository nobody had declared as a plugin source, which is the whole thing the bump
exists to stop.

### Why this is a bump and not a correction

Section 4 of the manifest contract was **corrected** in place earlier, when it began requiring `?ref=`,
on the grounds that nothing had shipped against the old reading. That argument is not available here
and was not used: `plugin_install_version` 1 is published, in 0.1.0 and 0.1.1, and reported by both.

What is true is that no published build could install a plugin at all — the GitHub provider was never
bundled in a release, so every install refused for want of a provider before reaching any of this. That
makes the practical cost of the bump zero, and it is not a reason to pretend the version did not
change. A version is what an implementation reports it can do, and this changes what it can do.
