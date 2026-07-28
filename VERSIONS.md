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
| `nost_language_version` | 2 | 2 | specified | [docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) |
| `nostdb_format_version` | 1 | 1 | specified | [docs/NOSTDB_FORMAT.md](docs/NOSTDB_FORMAT.md) |
| `query_subset_version` | 1 | 1 | specified | [docs/QUERY_SUBSET.md](docs/QUERY_SUBSET.md) |
| `settings_version` | 1 | 1 | specified | [docs/SETTINGS.md](docs/SETTINGS.md) |
| `credentials_version` | 1 | 1 | deferred | not yet specified |
| `catalog_version` | 1 | 1 | specified | [docs/CATALOG.md](docs/CATALOG.md) |
| `result_version` | 1 | 1 | specified | [docs/RESULT.md](docs/RESULT.md) |
| `provider_protocol_version` | 1 | 1 | specified | [docs/PROVIDER_PROTOCOL.md](docs/PROVIDER_PROTOCOL.md) |
| `plugin_protocol_version` | 1 | 1 | deferred | not yet specified |
| `manifest_version` | 1 | 1 | specified | [docs/PLUGIN_MANIFEST.md](docs/PLUGIN_MANIFEST.md) |
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

## Recorded bump: `nost_language_version` 1 to 2

Version 2 removed the module declaration, introduced schema declarations,
changed the node and edge forms, made the record identifier a reserved property
key holding a prefixed UUID, and added contribution and evidence blocks. See
[docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) section 1.1.

`supported` lists 2 alone. Version 1 does not round-trip through version 2,
because a version 1 file requires a module block that version 2 has no
declaration for, so listing 1 as supported would promise a parse no
implementation can deliver. A version 1 file is `NOST_VERSION_UNSUPPORTED`.

No other key moved. In particular `nostdb_format_version` stayed at 1: the
container gained no section and changed no layout, which is exactly the
independence this registry exists to keep.
