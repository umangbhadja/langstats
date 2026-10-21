#!/bin/sh
set -eu

repo=https://github.com/umangbhadja/langstats
started=$(date +%s)

elapsed() {
  printf 'langstats installer: %s in %ss\n' "$1" "$(($(date +%s) - $2))" >&2
}

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
lookup_started=$(date +%s)
latest_url=$(curl -fsSLI --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 \
  -o /dev/null -w '%{url_effective}' "$repo/releases/latest") || {
  printf 'langstats installer: retrying latest release lookup\n' >&2
  latest_url=$(curl -fsSLI --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 \
    -o /dev/null -w '%{url_effective}' "$repo/releases/latest") || fail 'could not find latest release'
}
elapsed 'release lookup completed' "$lookup_started"
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
  elapsed 'finished' "$started"
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
checksum_pid=
cleanup() {
  if [ -n "$checksum_pid" ]; then
    kill "$checksum_pid" 2>/dev/null || :
    wait "$checksum_pid" 2>/dev/null || :
  fi
  rm -rf "$tmp_dir"
}
trap cleanup 0
trap 'exit 130' INT
trap 'exit 143' TERM

url="$repo/releases/download/$tag/$archive"
printf 'Downloading langstats %s...\n' "${tag#v}" >&2
download_started=$(date +%s)
# Track curl itself so cleanup can stop it without leaving an orphan process.
curl -fsSL --connect-timeout 10 --max-time 60 --speed-limit 1024 --speed-time 15 \
  "$url.sha256" -o "$tmp_dir/$archive.sha256" &
checksum_pid=$!
download "$url" "$tmp_dir/$archive" 'release archive'
if wait "$checksum_pid"; then
  checksum_pid=
else
  checksum_pid=
  printf 'langstats installer: retrying checksum download\n' >&2
  download_once "$url.sha256" "$tmp_dir/$archive.sha256" checksum || fail 'could not download checksum'
fi
elapsed 'archive and checksum downloaded' "$download_started"

verify_started=$(date +%s)
read -r expected checksum_name < "$tmp_dir/$archive.sha256" || fail 'invalid checksum file'
[ "$checksum_name" = "$archive" ] || fail 'checksum file names a different archive'
if [ "$hash_command" = shasum ]; then
  actual=$(shasum -a 256 "$tmp_dir/$archive") || fail 'could not hash archive'
else
  actual=$(sha256sum "$tmp_dir/$archive") || fail 'could not hash archive'
fi
actual=${actual%% *}
[ "$actual" = "$expected" ] || fail 'release archive SHA-256 mismatch'
elapsed 'SHA-256 verified' "$verify_started"

install_started=$(date +%s)
tar -xf "$tmp_dir/$archive" -C "$tmp_dir" \
  "$name/langstats" "$name/LICENSE" "$name/NOTICE" || fail 'could not extract release archive'
for file in langstats LICENSE NOTICE; do
  [ -f "$tmp_dir/$name/$file" ] || fail "missing $file in archive"
done

if [ -f "$bin_dir/langstats" ]; then
  action=Updated
else
  action=Installed
fi
mkdir -p "$bin_dir" "$share_dir"
install -m 0644 "$tmp_dir/$name/LICENSE" "$share_dir/LICENSE"
install -m 0644 "$tmp_dir/$name/NOTICE" "$share_dir/NOTICE"
install -m 0755 "$tmp_dir/$name/langstats" "$bin_dir/langstats"
elapsed 'extracted and installed' "$install_started"

printf '%s langstats %s to %s\n' "$action" "${tag#v}" "$bin_dir/langstats"
elapsed 'finished' "$started"
case ":${PATH:-}:" in
  *":$bin_dir:"*) ;;
  *) printf 'Add %s to PATH, then run langstats --version\n' "$bin_dir" ;;
esac
