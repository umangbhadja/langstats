#!/usr/bin/env python3
"""Offline regression tests for public release packaging and verification."""

import hashlib
import io
from pathlib import Path
import stat
import subprocess
import sys
import tarfile
import tempfile
import unittest
import warnings
import zipfile


ROOT = Path(__file__).resolve().parent.parent


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="langstats-release-test-")
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.prefix = "langstats-v1.2.3-aarch64-apple-darwin"

    def run_script(self, name, *args):
        return subprocess.run(
            [sys.executable, str(ROOT / "scripts" / name), *map(str, args)],
            cwd=ROOT, capture_output=True, text=True, timeout=30,
        )

    def checksum(self, archive):
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n")

    def make_tar(self, defect):
        archive = self.folder / f"{self.prefix}.tar.gz"
        with tarfile.open(archive, "w:gz") as package:
            for name in ("langstats", "LICENSE", "NOTICE"):
                member = tarfile.TarInfo(f"{self.prefix}/{name}")
                data = b"payload\n"
                member.mode = 0o755 if name == "langstats" else 0o644
                if name == "langstats":
                    if defect == "empty":
                        data = b""
                    elif defect == "not-executable":
                        member.mode = 0o644
                    elif defect in ("symlink", "hardlink"):
                        member.type = tarfile.SYMTYPE if defect == "symlink" else tarfile.LNKTYPE
                        member.linkname = "outside"
                    elif defect == "special-mode":
                        member.mode = 0o4755
                    elif defect == "traversal":
                        member.name = "../langstats"
                member.size = len(data) if member.isfile() else 0
                package.addfile(member, io.BytesIO(data))
                if defect == "duplicate" and name == "langstats":
                    package.addfile(member, io.BytesIO(data))
        self.checksum(archive)
        return archive

    def test_malformed_tar_payloads_rejected_with_matching_checksums(self):
        for defect in ("empty", "not-executable", "symlink", "hardlink", "special-mode", "traversal", "duplicate"):
            with self.subTest(defect=defect):
                self.make_tar(defect)
                result = self.run_script("verify_release.py", self.folder)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertFalse((self.folder / "SHA256SUMS").exists())

    def test_malformed_zip_payloads_rejected(self):
        for defect in ("empty", "symlink", "duplicate", "traversal"):
            with self.subTest(defect=defect):
                archive = self.folder / "langstats-v1.2.3-x86_64-pc-windows-msvc.zip"
                prefix = archive.stem
                with zipfile.ZipFile(archive, "w") as package:
                    for name in ("langstats.exe", "LICENSE", "NOTICE"):
                        member = zipfile.ZipInfo(f"{prefix}/{name}")
                        data = b"payload\n"
                        if name == "langstats.exe":
                            if defect == "empty":
                                data = b""
                            elif defect == "symlink":
                                member.create_system = 3
                                member.external_attr = (stat.S_IFLNK | 0o777) << 16
                            elif defect == "traversal":
                                member.filename = "../langstats.exe"
                        package.writestr(member, data)
                        if defect == "duplicate" and name == "langstats.exe":
                            with warnings.catch_warnings():
                                warnings.simplefilter("ignore", UserWarning)
                                package.writestr(member, data)
                self.checksum(archive)
                result = self.run_script("verify_release.py", self.folder)
                self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_package_round_trip_and_checksum_rejection(self):
        binary = self.folder / "binary"
        binary.write_bytes(b"release fixture\n")
        for target in ("aarch64-apple-darwin", "x86_64-pc-windows-msvc"):
            result = self.run_script("package_release.py", binary, target, "1.2.3", self.folder)
            self.assertEqual(result.returncode, 0, result.stderr)
        result = self.run_script("verify_release.py", self.folder)
        self.assertEqual(result.returncode, 0, result.stderr)
        sums = self.folder / "SHA256SUMS"
        self.assertEqual(len(sums.read_text().splitlines()), 2)
        archive = next(self.folder.glob("*.zip"))
        archive.write_bytes(archive.read_bytes() + b"corruption")
        result = self.run_script("verify_release.py", self.folder)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertFalse(sums.exists())

    def test_unsafe_package_inputs_rejected(self):
        binary = self.folder / "binary"
        binary.write_bytes(b"fixture\n")
        for target, version in (("../../outside", "1.2.3"), ("aarch64-apple-darwin", "../../outside")):
            result = self.run_script("package_release.py", binary, target, version, self.folder)
            self.assertNotEqual(result.returncode, 0)
        binary.write_bytes(b"")
        result = self.run_script("package_release.py", binary, "aarch64-apple-darwin", "1.2.3", self.folder)
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
