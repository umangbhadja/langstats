# Linguist metadata

`data/languages.json` derives from GitHub Linguist `lib/linguist/languages.yml`
at commit `e30359651f214629c697f1eab0eb167dc108e972`.
Linguist is MIT licensed; its copyright notice lives in `NOTICE`.

Update by pinning a new upstream commit, downloading `languages.yml` from that
commit, running `ruby scripts/update-linguist-metadata.rb languages.yml`,
reviewing the diff and license, updating this revision, then running tests.
No network access occurs during CLI execution.

Only names, types, groups, colors, extensions, filenames, aliases, and
interpreters are imported. Linguist's heuristic and classifier rules are not
contained in this file.
