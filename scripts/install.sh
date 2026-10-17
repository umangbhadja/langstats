#!/bin/sh
set -eu

repo=https://github.com/umangbhadja/langstats

fail() {
  printf 'langstats installer: %s\n' "$1" >&2
  exit 1
}

download_once() {
  if [ "$3" = 'release archive' ] && [ -t 2 ]; then
    curl -fL --progress-bar --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 "$1" -o "$2"
  else
    curl -fsSL --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 "$1" -o "$2"
  fi
}

download() {
  if ! download_once "$@"; then
    printf 'langstats installer: retrying %s download\n' "$3" >&2
    download_once "$@" || fail "could not download $3"
  fi
}

command -v curl >/dev/null 2>&1 || fail 'curl is required'
: "${HOME:?HOME is required}"

case "$(uname -s)" in
  Darwin) os=apple-darwin ;;
  Linux) os=unknown-linux-gnu ;;
  *) fail 'only macOS and Linux are supported; use the PowerShell installer on Windows' ;;
esac

case "$(uname -m)" in
  arm64|aarch64) arch=aarch64 ;;
  x86_64|amd64) arch=x86_64 ;;
  *) fail 'unsupported CPU architecture' ;;
esac

bin_dir="$HOME/.local/bin"
share_dir="$HOME/.local/share/langstats"
installed_version=
if [ -x "$bin_dir/langstats" ]; then
  installed_version=$("$bin_dir/langstats" --version 2>/dev/null) || installed_version=
fi

printf 'Checking langstats...\n' >&2
latest_url=$(curl -fsSL --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 \
  -o /dev/null -w '%{url_effective}' "$repo/releases/latest") || {
  printf 'langstats installer: retrying latest release lookup\n' >&2
  latest_url=$(curl -fsSL --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 \
    -o /dev/null -w '%{url_effective}' "$repo/releases/latest") || fail 'could not find latest release'
}
case "$latest_url" in
  "$repo/releases/tag/v"*) tag=${latest_url##*/} ;;
  *) fail 'unexpected latest release URL' ;;
esac
case "$tag" in
  v[0-9]*) ;;
  *) fail 'unexpected release tag' ;;
esac
case "$tag" in
  *[!A-Za-z0-9._-]*) fail 'unexpected release tag' ;;
esac

if [ "$installed_version" = "langstats ${tag#v}" ] &&
   [ -f "$share_dir/LICENSE" ] && [ -f "$share_dir/NOTICE" ]; then
  printf 'langstats %s is already up to date at %s\n' "${tag#v}" "$bin_dir/langstats"
  exit 0
fi

command -v tar >/dev/null 2>&1 || fail 'tar is required'
command -v install >/dev/null 2>&1 || fail 'install is required'
command -v mktemp >/dev/null 2>&1 || fail 'mktemp is required'
if command -v sha256sum >/dev/null 2>&1; then
  hash_command=sha256sum
elif command -v shasum >/dev/null 2>&1; then
  hash_command=shasum
else
  fail 'sha256sum or shasum is required'
fi

target="$arch-$os"
name="langstats-$tag-$target"
archive="$name.tar.gz"
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/langstats-install.XXXXXXXX") || fail 'could not create temporary directory'
trap 'rm -rf "$tmp_dir"' 0

url="$repo/releases/download/$tag/$archive"
printf 'Downloading langstats %s...\n' "${tag#v}" >&2
download "$url" "$tmp_dir/$archive" 'release archive'
download "$url.sha256" "$tmp_dir/$archive.sha256" checksum

read -r expected checksum_name < "$tmp_dir/$archive.sha256" || fail 'invalid checksum file'
[ "$checksum_name" = "$archive" ] || fail 'checksum file names a different archive'
if [ "$hash_command" = shasum ]; then
  actual=$(shasum -a 256 "$tmp_dir/$archive") || fail 'could not hash archive'
else
  actual=$(sha256sum "$tmp_dir/$archive") || fail 'could not hash archive'
fi
actual=${actual%% *}
[ "$actual" = "$expected" ] || fail 'release archive SHA-256 mismatch'

for file in langstats LICENSE NOTICE; do
  tar -xOf "$tmp_dir/$archive" "$name/$file" > "$tmp_dir/$file" || fail "missing $file in archive"
done

if [ -f "$bin_dir/langstats" ]; then
  action=Updated
else
  action=Installed
fi
mkdir -p "$bin_dir" "$share_dir"
install -m 0644 "$tmp_dir/LICENSE" "$share_dir/LICENSE"
install -m 0644 "$tmp_dir/NOTICE" "$share_dir/NOTICE"
install -m 0755 "$tmp_dir/langstats" "$bin_dir/langstats"

printf '%s langstats %s to %s\n' "$action" "${tag#v}" "$bin_dir/langstats"
case ":${PATH:-}:" in
  *":$bin_dir:"*) ;;
  *) printf 'Add %s to PATH, then run langstats --version\n' "$bin_dir" ;;
esac
