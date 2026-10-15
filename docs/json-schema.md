# JSON schema version 1

`--json` and `--format json` emit one JSON object. Field names and numeric meanings
remain stable within schema version 1; breaking changes increment `schema_version`.

- `tool`: name and Cargo package version.
- `repository`: supplied path (lossy UTF-8 for non-Unicode names) and final component.
- `summary.files_scanned`: regular files opened or attempted after ignore filtering.
- `summary.files_counted`: regular files assigned a counted language, including empty files.
- `summary.total_bytes`: sum of counted file metadata lengths before display filters.
- `summary.language_count`: number of counted languages before display filters.
- `summary.warnings_truncated`: number of warnings beyond the first 100.
- `languages`: deterministic list after `--top`, `--min-bytes`, `--min-percent`,
  and `--sort`; each entry has name, byte count, unrounded percentage, file count,
  and optional Linguist color.
- `warnings`: up to 100 sorted diagnostics.

Percentage denominator is `summary.total_bytes`; zero total yields 0.0 for every
language. JSON stdout never contains ANSI codes or diagnostics.
