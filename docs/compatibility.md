# Linguist compatibility

Target: GitHub Linguist behavior described in its documentation. Metadata comes
from pinned commit in `docs/metadata-updates.md`. This project has **not** run a
meaningful upstream fixture corpus and does **not** claim full compatibility.

Implemented: names, extensions, compound extensions, exact filenames, aliases,
colors, interpreters, binary-prefix exclusion, selected modelines, several
ambiguous-extension rules, programming/markup filtering, `.gitignore`, and
subset of nested `.gitattributes` Linguist overrides. Common vendor directories,
documentation directories, and minified JS/CSS are excluded. An explicit
`-linguist-vendored`, `-linguist-documentation`, or `-linguist-generated` rule
re-enables those paths.

Known differences:

- Ambiguous extensions beyond `.rs`, `.h`, `.m`, `.md` use a stable candidate
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

The implementation tests cover representative local fixtures. An upstream
comparison workflow and larger corpus remain required before release claims.
