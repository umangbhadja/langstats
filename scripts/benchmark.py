#!/usr/bin/env python3
"""Reproducible release-binary scan scenarios; writes no repository fixtures."""

import argparse
import json
import pathlib
import subprocess
import tempfile
import time


def populate(root, count, payload, depth=1):
    for index in range(count):
        folder = root
        if depth > 1:
            folder = root / f"d{index % depth}"
            folder.mkdir(exist_ok=True)
        (folder / f"file{index}.rs").write_bytes(payload)


def populate_mixed_repo(root):
    """Build a deterministic repository-shaped fixture with ignored and binary files."""
    (root / ".gitignore").write_text("build/\nnode_modules/\n")
    (root / "web").mkdir()
    (root / "web" / ".gitignore").write_text("generated/\n")
    source = [
        ("src/rust", ".rs", 120, (84, 512, 4096), b"fn f() {}\n"),
        ("src/python", ".py", 90, (64, 1024, 2048), b"print(1)\n"),
        ("web/js", ".js", 90, (128, 1024, 4096), b"let x = 1;\n"),
        ("web/css", ".css", 40, (256, 2048), b"a { color: red; }\n"),
        ("web/templates", ".html", 40, (512, 4096), b"<p>Hello</p>\n"),
    ]
    for directory, extension, count, sizes, seed in source:
        folder = root / directory
        folder.mkdir(parents=True)
        for index in range(count):
            size = sizes[index % len(sizes)]
            (folder / f"file{index}{extension}").write_bytes(
                (seed * (size // len(seed) + 1))[:size]
            )
    for directory, extension, count, payload in [
        ("docs", ".md", 60, b"# Documentation\n" * 20),
        ("assets", ".rs", 40, b"\0" + b"x" * 1023),
        ("vendor", ".js", 30, b"let x = 1;\n" * 20),
        ("build", ".rs", 100, b"fn f() {}\n" * 100),
        ("node_modules", ".js", 100, b"let x = 1;\n" * 100),
        ("web/generated", ".js", 100, b"let x = 1;\n" * 100),
    ]:
        folder = root / directory
        folder.mkdir(parents=True)
        for index in range(count):
            (folder / f"file{index}{extension}").write_bytes(payload)


def measure(binary, root, args=()):
    start = time.perf_counter()
    run = subprocess.run([str(binary), "--json", *args, str(root)],
                         capture_output=True, check=True)
    elapsed = time.perf_counter() - start
    summary = json.loads(run.stdout)["summary"]
    files = summary["files_scanned"]
    size = summary["total_bytes"]
    return elapsed, files, size


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", default="target/release/langstats")
    parser.add_argument("--mixed-fixture", type=pathlib.Path,
                        help="create only the mixed repository fixture in an empty directory")
    args = parser.parse_args()
    binary = pathlib.Path(args.binary).resolve()
    if not binary.is_file():
        raise SystemExit(f"release binary missing: {binary}")
    if args.mixed_fixture is not None:
        root = args.mixed_fixture
        if root.exists() and (not root.is_dir() or any(root.iterdir())):
            raise SystemExit(f"fixture directory is not empty: {root}")
        root.mkdir(parents=True, exist_ok=True)
        populate_mixed_repo(root)
        print(root)
        return
    print("scenario,seconds,files_scanned,bytes_counted,files_per_second,bytes_per_second")
    cases = [
        ("small", 20, b"fn f() {}\n", 1),
        ("medium", 1000, b"fn f() {}\n" * 20, 10),
        ("large", 5000, b"fn f() {}\n" * 20, 50),
        ("many-small", 3000, b"x", 10),
        ("large-source", 8, b"fn f() {}\n" * 100_000, 1),
        ("deep", 1000, b"fn f() {}\n", 100),
    ]
    with tempfile.TemporaryDirectory(prefix="langstats-bench-") as temp:
        base = pathlib.Path(temp)
        for name, count, payload, depth in cases:
            root = base / name
            root.mkdir()
            populate(root, count, payload, depth)
            emit(name, measure(binary, root))
            if name == "medium":
                emit("cached-medium", measure(binary, root))
                emit("single-thread", measure(binary, root, ("--threads", "1")))
        binary_root = base / "many-binary"
        binary_root.mkdir()
        populate(binary_root, 2000, b"\0" + b"x" * 1024)
        emit("many-binary", measure(binary, binary_root))
        ignored = base / "many-ignored"
        ignored.mkdir()
        (ignored / ".gitignore").write_text("ignored/\n")
        (ignored / "ignored").mkdir()
        populate(ignored / "ignored", 2000, b"fn f() {}\n")
        populate(ignored, 100, b"fn f() {}\n")
        emit("many-ignored", measure(binary, ignored))
        mixed = base / "mixed-repository"
        mixed.mkdir()
        populate_mixed_repo(mixed)
        emit("mixed-repository-first", measure(binary, mixed))
        emit("mixed-repository-repeat", measure(binary, mixed))


def emit(name, sample):
    elapsed, files, size = sample
    print(f"{name},{elapsed:.6f},{files},{size},{files / elapsed:.0f},{size / elapsed:.0f}", flush=True)


if __name__ == "__main__":
    main()
