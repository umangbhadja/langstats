# Performance method

Build with `cargo build --release`, then run
`python3 scripts/benchmark.py target/release/langstats`.

The script creates deterministic source and binary fixtures in a temporary
directory. Cases cover small, medium, large, many small, large source, deep,
many binary, many ignored, cached repeated scan, and one-worker scan.
Each case is measured once with wall-clock `perf_counter`; rerun multiple times
for distribution estimates. Throughput uses `files_scanned` and counted bytes.
Peak memory is not measured by this portable script. Filesystem cache state
is uncontrolled except the repeated medium case. Results are local observations,
not universal speed claims.

## Local measurement, 2026-10-13

Apple M1 MacBook Air, 8 cores, 16 GB RAM; macOS 26.1; Rust 1.99.0; release
profile in `Cargo.toml`; temp fixtures on local filesystem. Second full script
run (first run's first process took 0.601 s, likely cold launch; no causal
profiling was done):

| Scenario | Files scanned | Counted bytes | Elapsed | Files/s |
| --- | ---: | ---: | ---: | ---: |
| small | 20 | 200 | 0.00456 s | 4,390 |
| medium | 1,000 | 200,000 | 0.01663 s | 60,139 |
| cached medium | 1,000 | 200,000 | 0.01559 s | 64,164 |
| single thread | 1,000 | 200,000 | 0.02114 s | 47,299 |
| large | 5,000 | 1,000,000 | 0.05801 s | 86,197 |
| many small | 3,000 | 3,000 | 0.03562 s | 84,223 |
| large source | 8 | 8,000,000 | 0.00476 s | 1,682 |
| deep | 1,000 | 10,000 | 0.01914 s | 52,235 |
| many binary | 2,000 | 0 | 0.02381 s | 83,999 |
| many ignored | 100 | 1,000 | 0.00651 s | 15,364 |

`many ignored` contains 2,000 additional files skipped by ignore rules. The
8 MB source case reads only 8 KiB per file; bytes/s there reflects metadata
length, not disk read throughput. Memory peak was not measured. Two script
runs are insufficient for statistically stable performance claims.
