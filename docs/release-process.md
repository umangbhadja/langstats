# Release process

No release has been published. Workflows are prepared for future tagged builds.

1. Run format, Clippy, tests, metadata consistency, and release build locally.
2. Update `Cargo.toml` version and `CHANGELOG.md`; commit.
3. Create and push a matching `v<version>` tag **only when release is approved**.
4. `.github/workflows/release.yml` tests six native runners, builds archives
   containing binary, `LICENSE`, and `NOTICE`, writes per-archive SHA-256 files,
   verifies contents, writes `SHA256SUMS`,
   and creates a GitHub Release. This workflow has not run yet.
5. Download all archives locally; run `python scripts/verify_release.py dist`.
6. Generate Homebrew formula with
   `python scripts/generate_homebrew.py <version> dist Formula/langstats.rb`.
   Review URLs and checksums, then `brew audit --strict Formula/langstats.rb`
   and `brew install ./Formula/langstats.rb` after release assets are live.
   Formula generation requires all four Unix archives. Generator URLs,
   checksums, target selection, and `ruby -c` passed with synthetic fixtures;
   `brew audit` and install have not run.

Local package test:

```sh
cargo build --release
python scripts/package_release.py target/release/langstats aarch64-apple-darwin 0.1.0 dist
python scripts/verify_release.py dist
```

Use the actual host target instead of `aarch64-apple-darwin` on other systems.
Do not label a binary with an architecture it was not built for.
On 2026-10-13, local ARM64 and x64 macOS release binaries were packaged and
the two resulting archives passed `verify_release.py`. A synthetic Windows
`.exe` archive passed naming, layout, and checksum checks; no Windows binary
was built locally. Actual Linux and Windows release assets require CI.
