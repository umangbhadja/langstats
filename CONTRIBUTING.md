# Contributing

Open an issue for behavior changes. Keep patches focused; add a fixture for
detection changes. Run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --locked
```

Document Linguist compatibility differences and benchmark performance changes
with `scripts/benchmark.py`. Do not edit generated metadata directly; follow
`docs/metadata-updates.md`. Contributions use the MIT project license.
