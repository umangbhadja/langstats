# Implementation plan

`langstats` scans local working trees offline and reports source-language bytes.
GitHub Linguist is the compatibility target, not a claim of exact parity.

## Boundaries

- CLI parsing and rendering stay in the binary; scanning and detection are a library.
- A bounded parallel walker yields file paths. Workers read at most a small prefix
  for binary, shebang, and modeline checks, then use file metadata for byte counts.
- Checked-in, generated Linguist metadata provides names, extensions, filenames,
  aliases, types, and colors. Metadata generation records source revision and license.
- Git ignore and attribute matching use maintained libraries where practical.
- Results sort deterministically. Warnings report incomplete scans.

## Acceptance gates

1. `cargo fmt --all -- --check`, Clippy with warnings denied, tests, release build.
2. Fixtures cover ignore rules, overrides, binary and ambiguous files, empty trees,
   symlinks, JSON contract, and deterministic ordering.
3. Release benchmarks record environment, input shape, elapsed time, throughput,
   and memory where measurement is available.
4. CI checks macOS, Linux, Windows; release workflow creates archives and checksums
   without publishing from a local development session.

## Feature commits

Plan; Rust foundation; metadata import; scanner; detection and attributes;
aggregation and output; compatibility fixtures; benchmark and tuning; CI and
release assets; documentation and audit. Commit each verified feature locally.

## Current constraints

Repository has one `README.md` and initial commit. Local Rust toolchain availability
must be checked before implementation tests. Exact Linguist parity requires upstream
fixture comparison; document measured gaps rather than asserting parity.

## Current implementation

`src/main.rs` parses options and renders reports. `src/scanner.rs` uses `ignore`
for traversal, a bounded synchronous work queue, and a fixed worker pool.
Workers read at most 8 KiB per file; byte totals come from file metadata.
`src/metadata.rs` parses checked-in Linguist data once into lookup indexes.
`src/detection.rs` applies modelines, filenames, shebangs, extensions, and a
small ambiguous-extension rule set. `src/attributes.rs` loads nested override
rules before scanning. `src/output.rs` sorts and filters aggregated counts,
then builds schema-v1 JSON or terminal output.

The attributes prepass walks the directory twice and may become a measurable
cost on very large trees. Full Git attribute semantics and upstream classifier
parity remain open, as detailed in `docs/compatibility.md`.
