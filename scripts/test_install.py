#!/usr/bin/env python3
"""Offline installer integration tests. Run with: python3 scripts/test_install.py."""

import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import textwrap
import unittest


INSTALLER = Path(__file__).with_name("install.sh")
REPO = "https://github.com/umangbhadja/langstats"
TAG = "v1.2.3"
NAME = f"langstats-{TAG}-aarch64-apple-darwin"
ARCHIVE = f"{NAME}.tar.gz"


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="langstats test ")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.home = self.root / "home with spaces"
        self.tmp = self.root / "temporary files"
        self.commands = self.root / "mock commands"
        for directory in (self.home, self.tmp, self.commands):
            directory.mkdir()
        self.bin = self.home / ".local/bin/langstats"
        self.share = self.home / ".local/share/langstats"
        self.config = {}
        self.name = NAME
        self.make_archive()
        self.env = os.environ.copy()
        self.env.update(
            HOME=str(self.home),
            TMPDIR=str(self.tmp),
            PATH=f"{self.commands}{os.pathsep}{os.environ['PATH']}",
            INSTALL_TEST_ROOT=str(self.root),
        )
        self.write_command("uname", '''
            import json, os, pathlib, sys
            root = pathlib.Path(os.environ["INSTALL_TEST_ROOT"])
            config = json.loads((root / "config.json").read_text())
            print(config.get("os", "Darwin") if sys.argv[1] == "-s" else config.get("arch", "arm64"))
        ''')
        self.write_command("curl", r'''
            import json, os, pathlib, shutil, signal, sys, time
            root = pathlib.Path(os.environ["INSTALL_TEST_ROOT"])
            args = sys.argv[1:]
            url = next(arg for arg in args if arg.startswith("https://"))
            kind = "lookup" if url.endswith("/latest") else "checksum" if url.endswith(".sha256") else "archive"
            with (root / "requests.jsonl").open("a") as log:
                log.write(json.dumps({"kind": kind, "args": args}) + "\n")
            config = json.loads((root / "config.json").read_text())
            counter = root / (kind + ".count")
            count = int(counter.read_text()) + 1 if counter.exists() else 1
            counter.write_text(str(count))
            if config.get("block_checksum"):
                if kind == "checksum":
                    def stopped(signum, frame):
                        (root / "checksum.stopped").touch()
                        sys.exit(128 + signum)
                    signal.signal(signal.SIGTERM, stopped)
                    (root / "checksum.started").touch()
                    while True:
                        time.sleep(0.01)
                if kind == "archive":
                    deadline = time.monotonic() + 5
                    while not (root / "checksum.started").exists():
                        if time.monotonic() > deadline:
                            sys.exit(28)
                        time.sleep(0.01)
            if kind in config.get("fail_always", []) or (kind in config.get("fail_first", []) and count == 1):
                sys.exit(22)
            if kind == "lookup":
                sys.stdout.write(config.get("latest_url", "https://github.com/umangbhadja/langstats/releases/tag/v1.2.3"))
                sys.exit(0)
            if config.get("barrier"):
                (root / (kind + ".started")).touch()
                other = "archive" if kind == "checksum" else "checksum"
                deadline = time.monotonic() + 5
                while not (root / (other + ".started")).exists():
                    if time.monotonic() > deadline:
                        sys.stderr.write("downloads did not overlap\n")
                        sys.exit(28)
                    time.sleep(0.01)
            destination = pathlib.Path(args[args.index("-o") + 1])
            source = root / ("fixture.sha256" if kind == "checksum" else "fixture.tar.gz")
            shutil.copyfile(source, destination)
        ''')
        self.write_command("tar", f'''
            import json, os, pathlib, sys
            with (pathlib.Path(os.environ["INSTALL_TEST_ROOT"]) / "tar.jsonl").open("a") as log:
                log.write(json.dumps(sys.argv[1:]) + "\\n")
            os.execv({shutil.which('tar')!r}, ["tar", *sys.argv[1:]])
        ''')

    def write_command(self, name, source):
        path = self.commands / name
        path.write_text(f"#!{sys.executable}\n" + textwrap.dedent(source).lstrip())
        path.chmod(0o755)

    def make_archive(self, missing=()):
        members = {
            "langstats": b"#!/bin/sh\nprintf 'langstats 1.2.3\\n'\n",
            "LICENSE": b"fixture license\n",
            "NOTICE": b"fixture notice\n",
        }
        with tarfile.open(self.root / "fixture.tar.gz", "w:gz") as archive:
            for name, data in members.items():
                if name in missing:
                    continue
                member = tarfile.TarInfo(f"{self.name}/{name}")
                member.mode = 0o755 if name == "langstats" else 0o644
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))
        self.write_checksum()

    def write_checksum(self):
        digest = hashlib.sha256((self.root / "fixture.tar.gz").read_bytes()).hexdigest()
        (self.root / "fixture.sha256").write_text(f"{digest}  {self.name}.tar.gz\n")

    def existing_install(self, version="1.0.0", notice=True):
        self.bin.parent.mkdir(parents=True)
        self.bin.write_text(f"#!/bin/sh\nprintf 'langstats {version}\\n'\n")
        self.bin.chmod(0o755)
        self.share.mkdir(parents=True)
        (self.share / "LICENSE").write_text("old license\n")
        if notice:
            (self.share / "NOTICE").write_text("old notice\n")

    def run_installer(self, success=True):
        (self.root / "config.json").write_text(json.dumps(self.config))
        process = subprocess.Popen(
            ["/bin/sh", str(INSTALLER)], env=self.env,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, start_new_session=True,
        )
        try:
            stdout, stderr = process.communicate(timeout=20)
        except subprocess.TimeoutExpired:
            # A broken cleanup trap must not leave a blocked curl fixture behind.
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
            self.fail(f"installer timed out:\n{stdout}{stderr}")
        output = stdout + stderr
        if success:
            self.assertEqual(process.returncode, 0, output)
        else:
            self.assertNotEqual(process.returncode, 0, output)
        self.assertEqual(list(self.tmp.iterdir()), [], "installer leaked temporary files")
        return output

    def requests(self):
        return [json.loads(line) for line in (self.root / "requests.jsonl").read_text().splitlines()]

    def assert_installed(self):
        result = subprocess.run([str(self.bin), "--version"], capture_output=True, text=True, check=True)
        self.assertEqual(result.stdout, "langstats 1.2.3\n")
        self.assertEqual(self.bin.stat().st_mode & 0o777, 0o755)
        for name in ("LICENSE", "NOTICE"):
            self.assertEqual((self.share / name).read_text(), f"fixture {name.lower()}\n")
            self.assertEqual((self.share / name).stat().st_mode & 0o777, 0o644)

    def test_fresh_install_uses_head_parallel_downloads_and_one_extraction(self):
        self.config["barrier"] = True
        output = self.run_installer()
        self.assert_installed()
        self.assertIn("Installed langstats 1.2.3", output)
        requests = self.requests()
        self.assertEqual(sorted(request["kind"] for request in requests), ["archive", "checksum", "lookup"])
        lookup = next(request for request in requests if request["kind"] == "lookup")
        self.assertTrue(any(arg == "--head" or (arg.startswith("-") and "I" in arg) for arg in lookup["args"]))
        invocations = [json.loads(line) for line in (self.root / "tar.jsonl").read_text().splitlines()]
        extracts = [args for args in invocations if any(arg == "--extract" or (arg.startswith("-") and "x" in arg) for arg in args)]
        self.assertEqual(len(extracts), 1, invocations)
        self.assertIn("release lookup completed", output)
        self.assertIn("archive and checksum downloaded", output)
        self.assertIn("SHA-256 verified", output)
        self.assertIn("extracted and installed", output)
        self.assertIn("finished", output)

    def test_up_to_date_skips_download_and_extraction(self):
        self.existing_install("1.2.3")
        output = self.run_installer()
        self.assertIn("already up to date", output)
        self.assertEqual([request["kind"] for request in self.requests()], ["lookup"])
        self.assertFalse((self.root / "tar.jsonl").exists())

    def test_old_version_updates(self):
        self.existing_install()
        output = self.run_installer()
        self.assert_installed()
        self.assertIn("Updated langstats 1.2.3", output)

    def test_supported_platform_asset_selection(self):
        for os_name, arch, target in (
            ("Linux", "x86_64", "x86_64-unknown-linux-gnu"),
            ("Linux", "aarch64", "aarch64-unknown-linux-gnu"),
            ("Darwin", "x86_64", "x86_64-apple-darwin"),
        ):
            with self.subTest(os=os_name, arch=arch):
                self.config.update(os=os_name, arch=arch)
                self.name = f"langstats-{TAG}-{target}"
                self.make_archive()
                if self.bin.exists():
                    self.bin.unlink()
                self.run_installer()
                self.assert_installed()
                archive_requests = [request for request in self.requests() if request["kind"] == "archive"]
                self.assertIn(f"{REPO}/releases/download/{TAG}/{self.name}.tar.gz", archive_requests[-1]["args"])

    def test_unsupported_platform_stops_before_network(self):
        for setting, value, message in (
            ("os", "FreeBSD", "only macOS and Linux are supported"),
            ("arch", "riscv64", "unsupported CPU architecture"),
        ):
            with self.subTest(setting=setting):
                self.config = {setting: value}
                output = self.run_installer(success=False)
                self.assertIn(message, output)
                self.assertFalse((self.root / "requests.jsonl").exists())
                self.assertFalse(self.bin.exists())

    def test_non_executable_or_broken_previous_binary_updates(self):
        self.existing_install()
        self.bin.chmod(0o644)
        self.run_installer()
        self.assert_installed()
        self.bin.write_text("#!/bin/sh\nexit 1\n")
        self.run_installer()
        self.assert_installed()

    def test_missing_notice_repairs_current_install(self):
        self.existing_install("1.2.3", notice=False)
        self.run_installer()
        self.assert_installed()

    def test_lookup_and_downloads_retry_once(self):
        self.config["fail_first"] = ["lookup", "archive", "checksum"]
        output = self.run_installer()
        self.assert_installed()
        for kind in ("lookup", "archive", "checksum"):
            self.assertEqual(sum(request["kind"] == kind for request in self.requests()), 2)
        self.assertIn("retrying latest release lookup", output)
        self.assertIn("retrying release archive download", output)
        self.assertIn("retrying checksum download", output)

    def test_permanent_failures_preserve_install_and_clean_up(self):
        self.existing_install()
        original = self.bin.read_bytes()
        for kind in ("lookup", "archive", "checksum"):
            with self.subTest(kind=kind):
                self.config["fail_always"] = [kind]
                self.run_installer(success=False)
                self.assertEqual(self.bin.read_bytes(), original)
                self.assertEqual((self.share / "LICENSE").read_text(), "old license\n")
                self.assertEqual((self.share / "NOTICE").read_text(), "old notice\n")

    def test_sha_mismatch_preserves_existing_binary(self):
        self.existing_install()
        original = self.bin.read_bytes()
        (self.root / "fixture.sha256").write_text(f"{'0' * 64}  {ARCHIVE}\n")
        output = self.run_installer(success=False)
        self.assertIn("SHA-256 mismatch", output)
        self.assertEqual(self.bin.read_bytes(), original)

    def test_archive_failure_cancels_blocked_checksum(self):
        self.config.update(block_checksum=True, fail_always=["archive"])
        output = self.run_installer(success=False)
        self.assertIn("could not download release archive", output)
        self.assertTrue((self.root / "checksum.started").exists())
        self.assertTrue((self.root / "checksum.stopped").exists(), "checksum process was not terminated")
        self.assertFalse(self.bin.exists())

    def test_checksum_wrong_filename_rejected(self):
        (self.root / "fixture.sha256").write_text(f"{'0' * 64}  other.tar.gz\n")
        output = self.run_installer(success=False)
        self.assertIn("different archive", output)
        self.assertFalse(self.bin.exists())

    def test_malformed_checksum_rejected(self):
        for checksum in ("", "invalid\n", f"not-a-hash  {ARCHIVE}\n"):
            with self.subTest(checksum=checksum):
                (self.root / "fixture.sha256").write_text(checksum)
                self.run_installer(success=False)
                self.assertFalse(self.bin.exists())

    def test_truncated_archive_preserves_existing_install(self):
        self.existing_install()
        original = self.bin.read_bytes()
        archive = self.root / "fixture.tar.gz"
        archive.write_bytes(archive.read_bytes()[:20])
        self.write_checksum()
        output = self.run_installer(success=False)
        self.assertIn("could not extract release archive", output)
        self.assertEqual(self.bin.read_bytes(), original)
        self.assertEqual((self.share / "LICENSE").read_text(), "old license\n")
        self.assertEqual((self.share / "NOTICE").read_text(), "old notice\n")

    def test_missing_archive_members_preserve_existing_binary(self):
        self.existing_install()
        original = self.bin.read_bytes()
        for name in ("langstats", "LICENSE", "NOTICE"):
            with self.subTest(name=name):
                self.make_archive(missing=(name,))
                self.run_installer(success=False)
                self.assertEqual(self.bin.read_bytes(), original)
                self.assertEqual((self.share / "LICENSE").read_text(), "old license\n")

    def test_unexpected_latest_url_rejected(self):
        self.config["latest_url"] = "https://example.com/releases/tag/v1.2.3"
        output = self.run_installer(success=False)
        self.assertIn("unexpected latest release URL", output)
        self.assertEqual([request["kind"] for request in self.requests()], ["lookup"])

if __name__ == "__main__":
    unittest.main()
