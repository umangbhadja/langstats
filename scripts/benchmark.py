#!/usr/bin/env python3
"""Reproducible release-binary scan scenarios; writes no repository fixtures."""

import json
import pathlib
import subprocess
import sys
import tempfile
import time


def populate(root, count, payload, depth=1):
    for index in range(count):
        folder = root
        if depth > 1:
            folder = root / f"d{index % depth}"
            folder.mkdir(exist_ok=True)
        (folder / f"file{index}.rs").write_bytes(payload)


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
    binary = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "target/release/langstats").resolve()
    if not binary.is_file():
        raise SystemExit(f"release binary missing: {binary}")
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


def emit(name, sample):
    elapsed, files, size = sample
    print(f"{name},{elapsed:.6f},{files},{size},{files / elapsed:.0f},{size / elapsed:.0f}", flush=True)


if __name__ == "__main__":
    main()
