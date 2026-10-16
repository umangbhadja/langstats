#!/bin/sh
set -eu

: "${HOME:?HOME is required}"

bin="$HOME/.local/bin/langstats"
share="$HOME/.local/share/langstats"

rm -f "$bin"
if [ -d "$share" ] && [ ! -L "$share" ]; then
  rm -f "$share/LICENSE" "$share/NOTICE"
  rmdir "$share" 2>/dev/null || :
fi

printf 'Uninstalled langstats from %s\n' "$bin"
