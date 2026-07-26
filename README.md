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

## Current status

This repository is initialized as root Stage 1 scaffolding. The executable
grammar, format contract, protocol schemas, examples, and conformance fixtures
are authored in Stage 2 and are not present yet.

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
