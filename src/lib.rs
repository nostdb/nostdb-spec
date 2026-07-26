#![forbid(unsafe_code)]

//! `nostdb-spec` publishes contracts, not behavior.
//!
//! This crate deliberately exposes no runtime API. It exists so that the
//! conformance suite under `tests/` can be built and run with the standard Rust
//! command set, and so that `cargo` can verify the repository the same way CI
//! does.
//!
//! The artifacts this repository owns are files, not functions:
//!
//! - `grammar/nost.ebnf` is the normative `.nost` grammar.
//! - `grammar/nost.pest` is an executable reference encoding of that grammar.
//! - `docs/NOST_LANGUAGE.md` is the normative `.nost` language contract.
//! - `docs/NOSTDB_FORMAT.md` is the normative `.nostdb` format contract.
//! - `format/nostdb-header.json` describes the `.nostdb` header for machines.
//! - `diagnostics.json` is the spec-owned diagnostic code registry.
//! - `versions.json` is the independent contract version registry.
//! - `fixtures/` is the normative conformance suite.
//!
//! Parsing, comment-preserving CST construction, canonical formatting, storage,
//! synchronization, analysis, and query execution belong to `nostdb-core`. Adding
//! any of them here would create a second implementation of a contract this
//! repository is supposed to define once.
