# nostdb-spec Agent Instructions

## Inheritance

This repository is a child of the NostDB root superproject. The root `AGENTS.md`
at <https://github.com/nostdb/nostdb> is the governing contract.

This file only narrows the root rules for the specification boundary. It must
not weaken any root product, safety, or ownership boundary. If this file and the
root contract appear to conflict, the root contract wins, the current valid
behavior stays unchanged, and the exact conflict is recorded in the root
`IMPLEMENTATION_PROGRESS.md`.

## Language policy

Write everything in this repository in English only.

This covers documentation, source code, identifiers, comments, rustdoc, test
names, commit messages, branch names, pull request titles and bodies, issue
text, diagnostics, error messages, log records, configuration, fixtures, and
example `.nost` content.

This rule holds regardless of the language a request is written in. Do not add a
translated copy of a document unless the user explicitly asks for one.

## Ownership boundary

`nostdb-spec` owns executable contracts and owns no runtime.

Permitted:

- the executable `.nost` grammar;
- the `.nostdb` format contract, including header, versioning, endianness,
  checksum, generation, and recovery rules;
- versioned provider, plugin, and server protocol schemas;
- examples and conformance fixtures;
- fixture tooling whose only purpose is proving the contracts are
  self-consistent and that fixtures match the declared grammar.

Prohibited:

- a parser, storage engine, synchronizer, analyzer, or query engine;
- any `.nostdb` writer, because only `nostdb-core` writes `.nostdb`;
- a CLI, daemon, provider, or plugin implementation;
- a copy of the root PRD;
- runtime code copied in from another NostDB repository or from any legacy
  implementation.

Fixture tooling is not a licence to grow a second parser. If a check needs real
parsing behavior, it belongs in `nostdb-core` and consumes these fixtures from
there.

## Contract versioning

The `.nost` language, `.nostdb` format, settings, provider protocol, plugin
protocol, and server protocol versions evolve independently. Every contract
carries its own explicit version field. Never couple two of those versions, and
never let one version bump imply another.

Every fixture states the exact contract version it exercises.

## Rust standards

Rust code in this repository uses Rust stable and Edition 2024. Public APIs
require explicit error types and rustdoc. Use `#![forbid(unsafe_code)]` where
practical; required `unsafe` code needs a separate ADR with documented safety
invariants and a Miri or equivalent verification plan before implementation.
Libraries use `tracing`, do not write directly to stdout, and do not panic for
ordinary errors.

Every Rust change must pass:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Do not add a dependency without documenting its purpose, maintenance status, and
license.

## Repository verification

Run before every commit:

```bash
./scripts/verify-repository.sh
```

The verifier is non-mutating. Extend it as contracts land rather than replacing
it with a manual checklist.

## Testing expectations

Specification work is only complete with fixtures that an implementation can
fail against. Cover valid syntax, invalid syntax, comment preservation, parser
recovery with source ranges, and golden round trips. Cover link declarations
with and without aliases, duplicate aliases, duplicate sources, and cycles.
Cover unsupported and unreadable format versions explicitly.

A fixture that no implementation can fail is documentation, not a conformance
test. Label it accordingly.

## Safety and external actions

- Do not create remote repositories, add remotes, push to a new remote, publish
  packages, create releases, or modify registries without explicit user
  authorization.
- Never place credentials, passwords, tokens, private keys, or PEM content in
  files, fixtures, diagnostics, or command output.
- Do not use destructive Git commands or broad deletion.
- Preserve existing user changes and never revert them without authorization.
- Treat every example and fixture as untrusted input for the implementations
  that consume it. A fixture intended to be hostile must be labelled as such.

## Stage workflow

Implementation sequencing is tracked in the root `IMPLEMENTATION_PROGRESS.md`,
not in this repository. Do not begin a later Stage during a setup-only request,
and do not mark a Stage `DONE` until every Acceptance Criterion passes.
