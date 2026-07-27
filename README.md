# nostdb-spec

`nostdb-spec` owns the executable contracts of NostDB: the `.nost` language
grammar, the `.nostdb` format contract, the provider, plugin, and server
protocol schemas, worked examples, and conformance fixtures.

NostDB is a clean-slate, local-first Property Graph Database for software
environments.

## Boundary

This repository is contract-only and contains no runtime.

It owns:

- the `.nost` grammar in executable form;
- the `.nostdb` file format contract;
- versioned provider, plugin, and server protocol schemas;
- examples and conformance fixtures that implementations verify against.

It does not own:

- a parser, storage engine, synchronizer, analyzer, or query engine;
- any `.nostdb` writer, because only `nostdb-core` writes `.nostdb`;
- a CLI, daemon, provider, or plugin implementation.

## Contracts

| Contract | Version | Normative document |
| --- | --- | --- |
| `.nost` language | `nost_language_version = 2` | [docs/NOST_LANGUAGE.md](docs/NOST_LANGUAGE.md) |
| `.nostdb` container | `nostdb_format_version = 1` | [docs/NOSTDB_FORMAT.md](docs/NOSTDB_FORMAT.md) |
| query subset | `query_subset_version = 1` | [docs/QUERY_SUBSET.md](docs/QUERY_SUBSET.md) |

Supporting artifacts:

- [`grammar/nost.ebnf`](grammar/nost.ebnf) is the normative, generator-neutral
  grammar. [`grammar/nost.pest`](grammar/nost.pest) is an executable reference
  encoding of it.
- [`format/nostdb-header.json`](format/nostdb-header.json) describes the container
  header for machines.
- [`versions.json`](versions.json) and [`VERSIONS.md`](VERSIONS.md) are the
  independent contract version registry.
- [`diagnostics.json`](diagnostics.json) is the diagnostic code registry.
- [`fixtures/`](fixtures) is the conformance suite, and it is normative.

## Current status

The `.nost` language, `.nostdb` container, and query subset contracts are specified
with a conformance suite. The settings, credentials, catalog, result-envelope,
provider, plugin, manifest, and server protocol contracts have reserved version keys
but are not authored yet; see [VERSIONS.md](VERSIONS.md).

No parser, storage engine, formatter, or query engine lives here. `nostdb-core`
implements those and proves conformance by passing this suite.

## Product contract

The normative product contract is the PRD in the root NostDB superproject at
<https://github.com/nostdb/nostdb>. That repository may be private; request
access if the link does not resolve.

This repository keeps no copy of the PRD. A divergent child copy would create
two competing contracts.

## Independent versioning

The `.nost` language, `.nostdb` format, settings, provider protocol, plugin
protocol, and server protocol each carry an explicit version and evolve
independently. No contract in this repository couples two of those versions.

## Verify

```bash
./scripts/verify-repository.sh
```

Continuous integration runs the same verifier on every push and pull request, so
a local pass and a CI pass check identical invariants.

## License

Apache-2.0. See [LICENSE](LICENSE).

The executable grammar and conformance fixtures are Apache-2.0 so that any
implementation may verify itself against them. `nostdb-core`, `nostdb-cli`, and
`nostdb-server` carry SSPL-1.0 and are described as source-available rather
than open source.
