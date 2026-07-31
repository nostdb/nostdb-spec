#!/usr/bin/env bash

# Non-mutating verification for nostdb-spec.
#
# Stage 1 checks repository scaffolding only. Stage 2 extends this script with
# grammar, format-contract, protocol-schema, and conformance-fixture checks.

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repository_root=$(CDPATH= cd -- "$script_dir/.." && pwd)

cd "$repository_root"

required_files="
AGENTS.md
CLAUDE.md
README.md
LICENSE
.gitignore
.editorconfig
.github/workflows/verify.yml
Cargo.toml
Cargo.lock
rust-toolchain.toml
src/lib.rs
VERSIONS.md
versions.json
diagnostics.json
grammar/nost.ebnf
grammar/nost.pest
format/nostdb-header.json
docs/NOST_LANGUAGE.md
docs/NOSTDB_FORMAT.md
docs/QUERY_SUBSET.md
docs/RESULT.md
docs/SETTINGS.md
fixtures/cypher/supported
fixtures/cypher/unsupported
fixtures/cypher/semantic
fixtures/nost/valid
fixtures/nost/invalid-syntax
fixtures/nost/invalid-semantic
fixtures/nostdb/header
fixtures/settings/valid
fixtures/settings/invalid
fixtures/settings/merge
fixtures/result/valid
fixtures/result/invalid
"

for required_file in $required_files; do
  if [ ! -e "$required_file" ]; then
    echo "missing required file: $required_file" >&2
    exit 1
  fi
done

# LICENSE is verbatim upstream text and is intentionally not whitespace-scanned.
# Fixtures are excluded: they are test data, and a future fixture may need
# deliberately unusual whitespace. Rust sources are covered by `cargo fmt`.
checked_text_files="
AGENTS.md
README.md
VERSIONS.md
.gitignore
.editorconfig
.github/workflows/verify.yml
Cargo.toml
rust-toolchain.toml
versions.json
diagnostics.json
grammar/nost.ebnf
grammar/nost.pest
format/nostdb-header.json
docs/NOST_LANGUAGE.md
docs/NOSTDB_FORMAT.md
docs/QUERY_SUBSET.md
docs/RESULT.md
docs/SETTINGS.md
scripts/verify-repository.sh
"

for checked_file in $checked_text_files; do
  if grep -nE '[[:blank:]]+$' "$checked_file"; then
    echo "trailing whitespace found in: $checked_file" >&2
    exit 1
  fi
done

if [ ! -L CLAUDE.md ] || [ "$(readlink CLAUDE.md)" != "AGENTS.md" ]; then
  echo "CLAUDE.md must be a symlink to AGENTS.md" >&2
  exit 1
fi

if ! grep -q '^ *Apache License$' LICENSE; then
  echo "LICENSE must be the Apache License, Version 2.0" >&2
  exit 1
fi

if ! grep -q '^ *Version 2\.0, January 2004$' LICENSE; then
  echo "LICENSE must be the Apache License, Version 2.0" >&2
  exit 1
fi

git diff --check

# The conformance harness. The crate is test-only and exposes no runtime API, so
# these commands verify the contracts and fixtures rather than a library surface.
# The viewer exchange fixtures are generated, and the generator is the readable form of what the
# bytes are. Regenerating must reproduce every one: a fixture edited by hand would be a byte array
# nobody could extend, and a generator that had drifted from its output would be a document
# describing a file it no longer writes.
if command -v node >/dev/null 2>&1; then
  before=$(find fixtures/view-exchange -name '*.bin' -exec shasum -a 256 {} + | LC_ALL=C sort)
  node fixtures/view-exchange/generate.mjs >/dev/null
  after=$(find fixtures/view-exchange -name '*.bin' -exec shasum -a 256 {} + | LC_ALL=C sort)
  if [ "$before" != "$after" ]; then
    echo "the viewer exchange fixtures differ from what generate.mjs writes" >&2
    exit 1
  fi
  echo "view exchange: every fixture matches its generator"
else
  echo "view exchange: node is absent, so the generator was not re-run" >&2
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required: the conformance suite is the normative gate" >&2
  exit 1
fi

cargo fmt --check
# `--locked`, so a manifest whose version moved without its lock fails here rather than in a release.
# Release 0.1.5 lost four build jobs to exactly that in a sibling repository, whose verifier ran no
# cargo command at all — and this one ran `cargo check`, which refreshes the lock as a side effect and
# would have reported a pass on the same disagreement.
cargo check --locked --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features

echo "nostdb-spec verification passed"
