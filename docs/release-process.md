# Release process

A `v<version>` tag triggers the release workflow, including publication of a
public GitHub Release. Do not push a tag until the release is approved.

1. Run format, Clippy, tests, metadata consistency, and release build locally.
2. Confirm `Cargo.toml` and `Cargo.lock` versions match the planned tag;
   update the changelog and commit before creating the tag.
3. After the release commit is merged, create the matching `v<version>` tag
   from up-to-date `main`, **only when release is approved**. Do not tag a
   feature branch or pull request head.
4. `.github/workflows/release.yml` tests six native runners, builds archives
   containing binary, `LICENSE`, and `NOTICE`, writes per-archive SHA-256 files,
   verifies contents, writes `SHA256SUMS`,
   and creates a GitHub Release. Verify its actual result after tagging.
5. Download all archives locally; run `python scripts/verify_release.py dist`.
6. Verify the one-line installers in `README.md` on all six supported platforms.
   These are the supported install and update method. Re-run each against an
   existing installation to confirm it updates. Verify the one-line uninstall
   commands remove the installed files and Windows user `PATH` entry.

Local package test:

```sh
cargo build --release --locked
python scripts/package_release.py target/release/langstats aarch64-apple-darwin 0.2.2 dist
python scripts/verify_release.py dist
```

Use the actual host target instead of `aarch64-apple-darwin` on other systems.
Do not label a binary with an architecture it was not built for.

The workflow checks the release tag against the Cargo version. The preflight
also requires the tagged commit to be an ancestor of `main`. Configure protected
tags and repository approval rules separately; ancestry alone is not approval.
Only the publishing job receives `contents: write`.

Archives contain exactly three unique, nonempty regular files: the platform
binary, `LICENSE`, and `NOTICE`. Unix binaries must have an executable bit.
The verifier validates each sidecar checksum and writes `SHA256SUMS` only after
all archives pass. Matching checksums detect corruption; they do not establish
publisher authenticity. Download through the trusted repository release URL.

After publication, download all six actual assets, verify them, and exercise
install, update, and uninstall on each supported runner. Windows symlink tests
may skip when the runner denies symlink creation; inspect test output before
claiming that coverage passed.
