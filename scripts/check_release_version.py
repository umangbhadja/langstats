#!/usr/bin/env python3
"""Fail if a release tag differs from Cargo package version."""

import pathlib
import re
import sys

tag = sys.argv[1] if len(sys.argv) > 1 else ""
manifest = pathlib.Path("Cargo.toml").read_text()
package = manifest.split("[package]", 1)[1].split("\n[", 1)[0]
match = re.search(r'^version\s*=\s*"([^"]+)"', package, re.MULTILINE)
if match is None:
    raise SystemExit("Cargo package version missing")
version = match.group(1)
if tag != f"v{version}":
    raise SystemExit(f"tag {tag!r} does not match Cargo version v{version}")
print(f"release version verified: {tag}")
