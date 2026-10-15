#!/usr/bin/env python3
"""Compare checked-in fixtures with installed upstream github-linguist CLI."""

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile


def command(*args, cwd=None):
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True, check=True).stdout


def main():
    ours = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "target/release/langstats").resolve()
    upstream = shutil.which("github-linguist")
    if upstream is None:
        raise SystemExit("github-linguist executable missing; install upstream gem in a separate development environment")
    print(command(upstream, "--version").strip(), file=sys.stderr)
    cases = json.loads(pathlib.Path("tests/compatibility/cases.json").read_text())
    mismatches = 0
    with tempfile.TemporaryDirectory(prefix="langstats-compat-") as base:
        for case in cases:
            root = pathlib.Path(base) / case["name"]
            root.mkdir()
            command("git", "init", "-q", str(root))
            command("git", "config", "user.name", "Fixture", cwd=root)
            command("git", "config", "user.email", "fixture@example.invalid", cwd=root)
            for name, content in case["files"].items():
                file = root / name
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text(content)
            command("git", "add", "-A", cwd=root)
            command("git", "commit", "-qm", "fixture", cwd=root)
            actual = json.loads(command(str(ours), "--json", str(root)))
            actual = {entry["name"]: entry["bytes"] for entry in actual["languages"] if entry["bytes"]}
            expected = json.loads(command(upstream, str(root), "--json"))
            expected = {name: data["size"] for name, data in expected.items()}
            if actual != expected:
                mismatches += 1
                print(f"MISMATCH {case['name']}: ours={actual} upstream={expected}")
            else:
                print(f"MATCH {case['name']}")
    print(f"{len(cases) - mismatches}/{len(cases)} fixtures matched")
    if mismatches:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
