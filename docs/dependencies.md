# Dependency review

Production crates: `clap` (CLI parsing), `ignore` (Git-aware traversal),
`globset` (attribute-pattern matching), `serde` and `serde_json` (stable JSON).
`tempfile` is test-only. The lockfile pins transitive versions.

On 2026-10-13, `cargo metadata --offline --locked --format-version 1` reported
53 packages including `langstats`; none lacked a license expression. All
dependency expressions offer MIT, Apache-2.0, or Unlicense terms; one
`unicode-ident` expression also includes Unicode-3.0 terms, and one `bstr`
expression offers LGPL-2.1-or-later as an alternative. This is a metadata
review, not a legal opinion. The Linguist metadata notice is in `NOTICE`.

Repeat license review whenever `Cargo.lock` changes. No runtime crate fetches
occur in the released executable.
