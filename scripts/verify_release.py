#!/usr/bin/env python3
"""Verify release archives and checksums; write combined SHA256SUMS."""

import hashlib
import pathlib
import sys
import tarfile
import zipfile


def main():
    folder = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "dist")
    archives = sorted(list(folder.glob("*.tar.gz")) + list(folder.glob("*.zip")))
    if not archives:
        raise SystemExit("no release archives")
    sums = []
    for archive in archives:
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        expected = (folder / f"{archive.name}.sha256").read_text().split()[0]
        if digest != expected:
            raise SystemExit(f"checksum mismatch: {archive}")
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as package:
                members = package.namelist()
                exe = "langstats.exe"
        else:
            with tarfile.open(archive, "r:gz") as package:
                members = package.getnames()
                exe = "langstats"
        if len(members) != 1 or not members[0].endswith(f"/{exe}"):
            raise SystemExit(f"unexpected archive members: {archive}")
        sums.append(f"{digest}  {archive.name}")
    (folder / "SHA256SUMS").write_text("\n".join(sums) + "\n")
    print(f"verified {len(archives)} archives")


if __name__ == "__main__":
    main()
