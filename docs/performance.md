# Performance method

Build with `cargo build --release`, then run
`python3 scripts/benchmark.py target/release/langstats`.

The script creates deterministic source and binary fixtures in a temporary
directory. Cases cover small, medium, large, many small, large source, deep,
many binary, many ignored, cached repeated scan, and one-worker scan.
Each case is measured once with wall-clock `perf_counter`; rerun multiple times
for distribution estimates. Throughput uses `files_scanned` and counted bytes.
Peak memory is not measured by this portable script. On macOS, measure an
individual case with `/usr/bin/time -l target/release/langstats --json PATH`;
`maximum resident set size` is in bytes. Filesystem cache state is uncontrolled
except the repeated medium case. Results are local observations, not universal
speed claims.

## Local measurement, 2026-10-13

Apple M1 MacBook Air, 8 cores, 16 GB RAM; macOS 26.1; Rust 1.99.0; release
profile in `Cargo.toml`; temp fixtures on local filesystem. Final binary's
second full script run (first process took 0.631 s; no causal
profiling was done):

| Scenario | Files scanned | Counted bytes | Elapsed | Files/s |
| --- | ---: | ---: | ---: | ---: |
| small | 20 | 200 | 0.00499 s | 4,005 |
| medium | 1,000 | 200,000 | 0.01699 s | 58,870 |
| cached medium | 1,000 | 200,000 | 0.01644 s | 60,846 |
| single thread | 1,000 | 200,000 | 0.02182 s | 45,830 |
| large | 5,000 | 1,000,000 | 0.05701 s | 87,711 |
| many small | 3,000 | 3,000 | 0.03633 s | 82,580 |
| large source | 8 | 8,000,000 | 0.00559 s | 1,431 |
| deep | 1,000 | 10,000 | 0.01833 s | 54,557 |
| many binary | 2,000 | 0 | 0.02470 s | 80,986 |
| many ignored | 100 | 1,000 | 0.00675 s | 14,823 |

`many ignored` contains 2,000 additional files skipped by ignore rules. The
8 MB source case reads only 8 KiB per file; bytes/s there reflects metadata
length, not disk read throughput. Two script runs are insufficient for
statistically stable performance claims.

## Release binary and peak RSS

After the compatibility fixes, a separate deterministic fixture under
`/private/tmp/langstats-release-bench-5000` contained 5,000 Rust files in 50
directories, each 200 bytes (1,000,000 counted bytes). Five independent
processes ran `target/release/langstats --json` with `/usr/bin/time -l` and a
Python `perf_counter` around the process. Peak RSS is macOS's
`maximum resident set size`. The first launch below was slower; filesystem
cache and background load were not controlled, so it is **not** a cold-cache
measurement.

| Launch | Elapsed | Files/s | Counted bytes/s | Peak RSS |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 0.122100 s | 40,950 | 8,189,986 | 5,079,040 B |
| 2 | 0.045905 s | 108,921 | 21,784,199 | 5,095,424 B |
| 3 | 0.044169 s | 113,202 | 22,640,463 | 5,128,192 B |
| 4 | 0.045581 s | 109,696 | 21,939,106 | 5,128,192 B |
| 5 | 0.046419 s | 107,715 | 21,543,019 | 5,226,496 B |

The scanner reads at most an 8 KiB prefix per file; counted bytes/s therefore
does not mean disk read throughput. The attributes prepass walks paths a second
time; no change was justified by these local measurements.
