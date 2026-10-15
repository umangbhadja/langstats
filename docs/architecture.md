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
