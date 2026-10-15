# Release process

A `v<version>` tag triggers the release workflow, including publication of a
public GitHub Release. Do not push a tag until the release is approved.

1. Run format, Clippy, tests, metadata consistency, and release build locally.
2. Confirm `Cargo.toml` version matches the planned tag; update the changelog
   and commit. For `v0.1.0`, the Cargo version is already `0.1.0`.
3. Create and push a matching `v<version>` tag **only when release is approved**.
4. `.github/workflows/release.yml` tests six native runners, builds archives
   containing binary, `LICENSE`, and `NOTICE`, writes per-archive SHA-256 files,
   verifies contents, writes `SHA256SUMS`,
   and creates a GitHub Release. Verify its actual result after tagging.
5. Download all archives locally; run `python scripts/verify_release.py dist`.
6. Generate Homebrew formula with
   `python scripts/generate_homebrew.py <version> dist Formula/langstats.rb`.
   Review URLs and checksums, then run `brew audit --strict Formula/langstats.rb`,
   `brew install ./Formula/langstats.rb`, and `brew test langstats` after
   release assets are live. Formula generation requires all four Unix archives.
   Generator URLs, checksums, target selection, and `ruby -c` passed with
   synthetic fixtures;
   `brew audit`, install, and test have not run.

Local package test:

```sh
cargo build --release --locked
python scripts/package_release.py target/release/langstats aarch64-apple-darwin 0.1.0 dist
python scripts/verify_release.py dist
```

Use the actual host target instead of `aarch64-apple-darwin` on other systems.
Do not label a binary with an architecture it was not built for.
On 2026-10-13, local ARM64 and x64 macOS release binaries were packaged and
the two resulting archives passed `verify_release.py`. A synthetic Windows
`.exe` archive passed naming, layout, and checksum checks; no Windows binary
was built locally. Actual Linux and Windows release assets require CI.

## Cross-platform and publication audit, 2026-10-13

At the time of this audit, the CI and release matrices used matching Rust
target triples and runner
architectures: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`,
`aarch64-pc-windows-msvc`, and `x86_64-pc-windows-msvc`. GitHub documents the
selected [runner labels](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job),
but neither workflow had run remotely as of 2026-10-13. Action
references are pinned to full commits obtained from the official action tags.
All jobs have `contents: read` except the publishing job, which has
`contents: write` for the future release creation step.

On this macOS ARM64 host, native and Rosetta x64 macOS tests and release builds
passed. `cargo check --release --locked --tests --target <triple>` passed for
both Linux and both Windows targets. These checks compile source and tests but
do not link or execute them. A Linux x64 release link attempt failed because
the macOS linker rejected GNU options (`--as-needed`, `-Bstatic`, and others).
A Windows x64 release link attempt failed because `link.exe` is absent. Docker
CLI exists but its daemon is unavailable. Native Linux and Windows runners
were therefore still needed for builds, tests, symlink behavior, and archives.
The Windows symlink regression test reports when the runner denies symlink
creation; that case remains unverified even if the rest of the test suite passes.

The release workflow injects `${{ github.ref_name }}` as `RELEASE_TAG` and
`check_release_version.py` requires the exact `v`-prefixed Cargo version;
`v0.1.0` passed locally and `v0.2.0` was rejected. `package_release.py`
uses `.tar.gz` with `langstats` on Unix and `.zip` with `langstats.exe` on
Windows. `verify_release.py` checks each archive's SHA-256 and the expected
binary, `LICENSE`, and `NOTICE` members. The final publish job still needs a
real six-target release run and inspection of downloaded assets.

The Homebrew generator chooses ARM64 or Intel macOS and Linux URLs using the
same archive names as the workflow. It verifies each local SHA-256 before
writing a formula with MIT metadata and a `--version` test. Four-architecture
syntax and URL construction passed with two real macOS archives plus two
synthetic Linux entries; the generated macOS SHA-256 values matched the real
local archives. There are no real Linux release archives yet. The configured
GitHub repository returned HTTP 404 to this environment on 2026-10-13, before
the repository became public. After publication, download all six real assets, verify
`SHA256SUMS`, generate the formula from the four real Unix archives, run
`ruby -c`, `brew audit --strict`, and install and test it on both macOS
architectures and Linux where available.

## CI after public repository merge

[PR #1](https://github.com/umangbhadja/langstats/pull/1) reported passing
quality checks for all six native runner and target pairs, plus the metadata
check. These checks include format, Clippy, tests, and release builds. They do
not verify the tag-triggered release workflow, downloaded archives, or
installation. Windows symlink coverage may be skipped if the runner denies
symlink creation; inspect that test's output before claiming it passed.
