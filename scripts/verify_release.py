#!/usr/bin/env python3
"""Verify release archives and checksums; write combined SHA256SUMS."""

import hashlib
import pathlib
import stat
import sys
import tarfile
import zipfile


def main():
    folder = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "dist")
    (folder / "SHA256SUMS").unlink(missing_ok=True)
    archives = sorted(list(folder.glob("*.tar.gz")) + list(folder.glob("*.zip")))
    if not archives:
        raise SystemExit("no release archives")
    sums = []
    for archive in archives:
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        expected = (folder / f"{archive.name}.sha256").read_text().split()
        if expected != [digest, archive.name]:
            raise SystemExit(f"checksum mismatch: {archive}")
        prefix = archive.name.removesuffix(".tar.gz").removesuffix(".zip")
        exe = "langstats.exe" if archive.suffix == ".zip" else "langstats"
        expected_members = {f"{prefix}/{name}" for name in (exe, "LICENSE", "NOTICE")}
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as package:
                members = package.namelist()
                if len(members) != 3 or set(members) != expected_members:
                    raise SystemExit(f"unexpected archive members: {archive}")
                for member in package.infolist():
                    kind = stat.S_IFMT(member.external_attr >> 16)
                    if member.is_dir() or kind not in (0, stat.S_IFREG) or member.file_size == 0:
                        raise SystemExit(f"invalid archive payload: {archive}")
                    if not package.read(member):
                        raise SystemExit(f"empty archive payload: {archive}")
        else:
            with tarfile.open(archive, "r:gz") as package:
                members = package.getnames()
                if len(members) != 3 or set(members) != expected_members:
                    raise SystemExit(f"unexpected archive members: {archive}")
                for member in package.getmembers():
                    if not member.isfile() or member.size == 0 or member.mode & 0o7000:
                        raise SystemExit(f"invalid archive payload: {archive}")
                    if member.name == f"{prefix}/{exe}" and not member.mode & 0o111:
                        raise SystemExit(f"binary is not executable: {archive}")
                    with package.extractfile(member) as payload:
                        if len(payload.read()) != member.size:
                            raise SystemExit(f"truncated archive payload: {archive}")
        sums.append(f"{digest}  {archive.name}")
    (folder / "SHA256SUMS").write_text("\n".join(sums) + "\n")
    print(f"verified {len(archives)} archives")


if __name__ == "__main__":
    main()
