# Contributing

Open an [issue](https://github.com/umangbhadja/langstats/issues) for behavior
changes. Keep patches focused; add a fixture for detection changes. Run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release
```

Document Linguist compatibility differences and benchmark performance changes
with `scripts/benchmark.py`. Do not edit generated metadata directly; follow
`docs/metadata-updates.md`. Contributions use the MIT project license.
