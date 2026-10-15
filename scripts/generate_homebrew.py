#!/usr/bin/env python3
"""Generate Homebrew formula only from verified release archives."""

import hashlib
import pathlib
import sys

TARGETS = {
    "arm_macos": "aarch64-apple-darwin",
    "intel_macos": "x86_64-apple-darwin",
    "arm_linux": "aarch64-unknown-linux-gnu",
    "intel_linux": "x86_64-unknown-linux-gnu",
}


def main():
    if len(sys.argv) != 4:
        raise SystemExit("usage: generate_homebrew.py VERSION DIST_DIR OUTPUT_FILE")
    version, dist, output = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    if version.startswith("v"):
        version = version[1:]
    assets = {}
    for label, target in TARGETS.items():
        archive = dist / f"langstats-v{version}-{target}.tar.gz"
        if not archive.is_file():
            raise SystemExit(f"missing {archive}")
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        checksum = (dist / f"{archive.name}.sha256").read_text().split()[0]
        if digest != checksum:
            raise SystemExit(f"checksum mismatch: {archive}")
        url = f"https://github.com/umangbhadja/langstats/releases/download/v{version}/{archive.name}"
        assets[label] = f'    url "{url}"\n    sha256 "{digest}"'
    text = f'''class Langstats < Formula
  desc "Fast, offline language statistics for local directories"
  homepage "https://github.com/umangbhadja/langstats"
  version "{version}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
{assets["arm_macos"]}
    else
{assets["intel_macos"]}
    end
  end

  on_linux do
    if Hardware::CPU.arm?
{assets["arm_linux"]}
    else
{assets["intel_linux"]}
    end
  end

  def install
    bin.install "langstats"
    pkgshare.install "LICENSE", "NOTICE"
  end

  test do
    assert_match "langstats", shell_output("#{{bin}}/langstats --version")
  end
end
'''
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text)
    print(output)


if __name__ == "__main__":
    main()
