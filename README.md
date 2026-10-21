# langstats

Fast, offline language statistics for local directories and repositories.
Counts source bytes by language, inspired by GitHub's language bar.

## Features

- Parallel directory scan with a bounded work queue; no runtime network or GitHub account.
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

## Install or update

The one-line installer is the supported installation and update method. Re-run
the same command to check for the latest published release. It downloads and
updates an older installation; if the installed version is current and its
bundled files are present, it leaves the installation untouched.

On macOS or Linux (ARM64 or x86-64):

```sh
curl -fsSL https://raw.githubusercontent.com/umangbhadja/langstats/main/scripts/install.sh | sh
```

The installer verifies the release archive's SHA-256, places `langstats` in
`~/.local/bin`, and places `LICENSE` and `NOTICE` in `~/.local/share/langstats`.
Add `~/.local/bin` to your `PATH` if prompted. It requires `curl`, `tar`, and
either `shasum` or `sha256sum` when installing or updating. Interactive
terminals show download progress. The installer checks the release version with
a HEAD request, downloads the archive and checksum concurrently, and extracts
the archive once. Elapsed times identify time spent checking the release,
downloading, verifying SHA-256, and installing; failures are reported.

On Windows (x64 or ARM64), run in PowerShell:

```powershell
irm https://raw.githubusercontent.com/umangbhadja/langstats/main/scripts/install.ps1 | iex
```

This verifies the release ZIP's SHA-256, installs `langstats.exe`, `LICENSE`,
and `NOTICE` under `%LOCALAPPDATA%\Programs\langstats`, and adds that directory
to your user `PATH`. Installing or updating requires `curl.exe`; download
progress appears in PowerShell. Release lookup uses HEAD, archive and checksum
downloads run concurrently, and elapsed times appear for each installation stage.
SHA-256 verification remains required; failures are reported.

## Uninstall

On macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/umangbhadja/langstats/main/scripts/uninstall.sh | sh
```

On Windows, run in PowerShell:

```powershell
irm https://raw.githubusercontent.com/umangbhadja/langstats/main/scripts/uninstall.ps1 | iex
```

These remove only the installed binary, `LICENSE`, and `NOTICE`. They leave
other files in the installation directories untouched. The Windows command
also removes the install directory from your user `PATH`.

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
  "tool": {"name": "langstats", "version": "0.2.2"},
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
Attribute files and rules are loaded into memory; memory use grows with their size.
`.gitignore` works in Git and ordinary directories. `.git` internals and
descendant symlinks are skipped. A symlink supplied as the scan root is
resolved. Scan a stable tree: concurrent file replacement is not isolated.
Relevant `.gitattributes` overrides include
`linguist-language`, `linguist-detectable`, `linguist-generated`,
`linguist-vendored`, and `linguist-documentation`. See
[compatibility details](docs/compatibility.md) and [metadata updates](docs/metadata-updates.md).

## Platforms and performance

CI defines native build and test jobs for macOS ARM64/x64, Linux ARM64/x64,
and Windows ARM64/x64, plus installer regression and smoke tests. Consult
current workflow results for platform validation.
The 53 checked-in fixtures matched pinned GitHub Linguist v9.7.0 locally;
this compares nonzero language byte totals for those fixtures only.
On an Apple M1 MacBook Air, a warmed release scan of a 5,000-file,
1,000,000-byte fixture took 0.044–0.047 s, with peak RSS around 5.1 MB.
Filesystem cache state was not controlled.
See [methodology and all scenarios](docs/performance.md). These figures are
observations, not guarantees.

## Develop

Building from source for development requires stable Rust 1.85 or newer.

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
and classifier parity.
Licensed MIT; bundled Linguist metadata attribution is in `NOTICE`.
