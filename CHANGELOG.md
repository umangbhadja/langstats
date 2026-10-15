# Changelog

## 0.1.0

- Initial offline Rust CLI, pinned Linguist metadata, bounded parallel scan,
  documented JSON v1, attribute subset, and local benchmark harness.
- Six-platform CI and release workflow added.
- Correct HTML and PHP/Hack detection against pinned Linguist v9.7.0;
  53 local comparison fixtures now match, including ambiguous `node` shebangs
  and minified-file exclusion interactions.
- Pin GitHub Actions revisions and add a mixed-repository release benchmark.
- Make grouped-language colors and capped warning lists deterministic across
  parallel scans.
