# langstats

Fast, offline language statistics for local directories and repositories.
Counts source bytes by language, inspired by GitHub's language bar.

## Features

- Parallel, bounded-memory directory scan; no runtime network or GitHub account.
- Git ignore rules, hidden-file controls, safe default symlink behavior.
- Pinned GitHub Linguist names, extensions, filenames, aliases, types, and colors.
- Nested `.gitattributes` overrides for key Linguist attributes.
- Terminal bars and versioned, deterministic JSON.

```text
$ langstats .
Repository: .
Files analyzed: 12
Total bytes: 28146

Languages

Rust                98.08%  █████████████████████████████████████████████████
Ruby                 1.92%  █
```

Numbers above illustrate output shape; actual results depend on directory.

## Install

No release has been published yet. From a checkout of this repository, build
from source:

```sh
cargo build --release --locked
./target/release/langstats .
```

Stable Rust 1.85 or newer is required. In PowerShell on Windows, run
`.\target\release\langstats.exe .` after building.

## Use

```sh
langstats .
langstats --json path/to/repo
langstats --threads 1 --include-hidden --include-ignored .
langstats --format json --sort name --top 5 .
```

| Option | Meaning |
| --- | --- |
| `[PATH]` | Directory; default `.` |
| `-h, --help`, `-V, --version` | Help and version |
| `--json`, `--format terminal\|json` | Output format |
| `--top N` | Show first N languages after sorting |
| `--threads N` | Worker count, 1–256 |
| `--include-ignored` | Disable ignore filtering |
| `--include-hidden` | Include hidden entries |
| `--no-color` | Disable terminal color |
| `--quiet`, `--verbose` | Reduce terminal text or show diagnostics |
| `--sort bytes\|percentage\|name\|files` | Display order; percent and bytes rank identically |
| `--min-percent PERCENT`, `--min-bytes BYTES` | Filter displayed languages |
| `--fail-on-empty` | Exit 1 if no source bytes were counted |

JSON example:

```json
{
  "schema_version": 1,
  "tool": {"name": "langstats", "version": "0.1.0"},
  "repository": {"path": ".", "name": "."},
  "summary": {"files_scanned": 1, "files_counted": 1, "total_bytes": 13, "language_count": 1, "warnings_truncated": 0},
  "languages": [{"name": "Rust", "bytes": 13, "percentage": 100.0, "files": 1, "color": "#dea584"}],
  "warnings": []
}
```

Percentages use counted bytes before display filters. Binary files and
non-source language types are excluded. Empty files can contribute file counts
but no bytes. Errors and scan warnings go to stderr; JSON stdout stays valid.
See [schema](docs/json-schema.md).

## Detection and compatibility

Metadata derives from [GitHub Linguist](https://github.com/github-linguist/linguist),
commit `e30359651f214629c697f1eab0eb167dc108e972`, under MIT terms in
`NOTICE`. Detection is **not fully Linguist compatible**. It uses a bounded
8 KiB prefix, selected content rules, and a documented attribute subset.
`.gitignore` works in Git and ordinary directories. `.git` internals and
symlinks are skipped. Relevant `.gitattributes` overrides include
`linguist-language`, `linguist-detectable`, `linguist-generated`,
`linguist-vendored`, and `linguist-documentation`. See
[compatibility details](docs/compatibility.md) and [metadata updates](docs/metadata-updates.md).

## Platforms and performance

CI is configured for macOS ARM64/x64, Linux ARM64/x64, and Windows ARM64/x64.
macOS ARM64 and x64 builds and tests passed locally (x64 through Rosetta);
CI has not run. Linux and Windows remain unverified on native hosts.
The 53 checked-in fixtures matched pinned GitHub Linguist v9.7.0 locally;
this compares nonzero language byte totals for those fixtures only.
On an Apple M1 MacBook Air, a warmed release scan of a 5,000-file,
1,000,000-byte fixture took 0.044–0.047 s, with peak RSS around 5.1 MB.
Filesystem cache state was not controlled.
See [methodology and all scenarios](docs/performance.md). These figures are
observations, not guarantees.

## Develop

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --locked
python3 scripts/benchmark.py target/release/langstats
```

Dependencies: `clap` parses CLI; `ignore` handles traversal and Git ignore;
`globset` matches supported attribute patterns; `serde`/`serde_json` provide
versioned JSON. `tempfile` is test-only. See [dependency review](docs/dependencies.md),
[architecture](docs/architecture.md),
[contributing](CONTRIBUTING.md), and [release process](docs/release-process.md).

## Project

[Security policy](SECURITY.md) · [Support](SUPPORT.md) · [Changelog](CHANGELOG.md)

Roadmap: broader upstream compatibility corpus, full Git attribute semantics,
classifier parity, and native Linux/Windows CI validation.
Licensed MIT; bundled Linguist metadata attribution is in `NOTICE`.
