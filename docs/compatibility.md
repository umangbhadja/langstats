# Linguist compatibility

Target: GitHub Linguist behavior described in its documentation. Metadata comes
from pinned commit in `docs/metadata-updates.md`. A local comparison against
`github-linguist` gem v9.7.0 matched all 53 checked-in fixtures on 2026-10-13.
The 53/53 figure means exact equality of **nonzero bytes per language** in
these 53 committed Git fixtures. It does not measure per-file labels, ignored
file counts, JSON shape, or general accuracy across repositories.

Implemented: names, extensions, compound extensions, exact filenames, aliases,
colors, interpreters, binary-prefix exclusion, selected modelines, several
ambiguous-extension rules, programming/markup filtering, `.gitignore`, and
subset of nested `.gitattributes` Linguist overrides. Common vendor directories,
documentation directories, and minified JS/CSS are excluded. An explicit
negative attribute rule can re-enable its corresponding excluded path. Minified
JS/CSS is both generated and vendored upstream, so both attributes must be
cleared to include it.

Known differences:

- Ambiguous extensions beyond the explicit `.rs`, `.h`, `.m`, `.md`, `.html`, and
  `.php` rules use a stable candidate
  fallback rather than Linguist's heuristic corpus or Bayesian classifier.
- Attribute patterns support ordinary globs, nesting, and last-match precedence.
  Git's full attribute grammar, escapes, macros, `!attr` resets, and ancestor
  rules above the scan root are not yet supported.
- Generated/vendor/documentation defaults are a deliberately small rule set,
  not Linguist's complete rule lists.
- Detection reads at most 8 KiB. Modelines at file end, binary markers after
  that prefix, and content heuristics requiring full files can differ.
- Local working-tree scan differs from GitHub's default-branch snapshot. Empty
  files are assigned a language and file count but add zero bytes.

The implementation tests cover representative local fixtures. Run
`python3 scripts/compare_linguist.py target/release/langstats` after installing
the upstream `github-linguist` executable to compare 53 checked-in cases.
The comparison used an isolated Ruby 3.3.0 build, ICU 78.3, and pinned
`github-linguist` v9.7.0. System Ruby 2.6 could not compile its
`charlock_holmes` dependency against that ICU. The corpus initially exposed
`.html` (dropped) and `.php` (misclassified as Hack); both were fixed and
regression-tested. The expanded corpus also exposed ambiguous `node` shebangs
and minified-file exclusion interactions; both were fixed with focused tests.
Broader classifier and attribute parity remain unverified.
