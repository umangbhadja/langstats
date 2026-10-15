#!/usr/bin/env python3
"""Package one already-built target and write its SHA-256 checksum."""

import hashlib
import io
import pathlib
import sys
import tarfile
import zipfile


def main():
    if len(sys.argv) != 5:
        raise SystemExit("usage: package_release.py BINARY TARGET VERSION OUTPUT_DIR")
    binary = pathlib.Path(sys.argv[1])
    target = sys.argv[2]
    version = sys.argv[3].removeprefix("v")
    output = pathlib.Path(sys.argv[4])
    if not binary.is_file() or not version:
        raise SystemExit("binary missing or version empty")
    output.mkdir(parents=True, exist_ok=True)
    exe = "langstats.exe" if "windows" in target else "langstats"
    archive_name = f"langstats-v{version}-{target}"
    data = binary.read_bytes()
    if "windows" in target:
        archive = output / f"{archive_name}.zip"
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
            package.writestr(f"{archive_name}/{exe}", data)
    else:
        archive = output / f"{archive_name}.tar.gz"
        with tarfile.open(archive, "w:gz") as package:
            info = tarfile.TarInfo(f"{archive_name}/{exe}")
            info.size = len(data)
            info.mode = 0o755
            package.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (output / f"{archive.name}.sha256").write_text(f"{digest}  {archive.name}\n")
    print(archive)


if __name__ == "__main__":
    main()
