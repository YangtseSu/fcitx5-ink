#!/bin/sh
# SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
# SPDX-License-Identifier: GPL-3.0-or-later
# Selecting Ink must not open the panel: handwriting is a module addon with no
# input method entry. The hotkey is the only entry point.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
build="$root/build"
tmp=$(mktemp -d)
client_pid=

cleanup() {
  if [ -n "${client_pid:-}" ]; then
    kill "$client_pid" 2>/dev/null || true
    wait "$client_pid" 2>/dev/null || true
  fi
  rm -rf "$tmp"
}
trap cleanup EXIT INT TERM

mkdir -p "$tmp/runtime" "$tmp/config/fcitx5" "$tmp/cache" "$tmp/state" "$tmp/data"
chmod 700 "$tmp/runtime"

cat > "$tmp/config/fcitx5/profile" <<'EOF'
[Groups/0]
Name=Default
Default Layout=us
DefaultIM=keyboard-us

[Groups/0/Items/0]
Name=ink
Layout=

[Groups/0/Items/1]
Name=keyboard-us
Layout=

[GroupOrder]
0=Default
EOF

cat > "$tmp/panel-stub" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" >> "$PANEL_LOG"
EOF
chmod +x "$tmp/panel-stub"

cc "$root/tests/commit-client.c" -o "$tmp/client" \
  $(pkg-config --cflags --libs gtk+-3.0)

if ! dbus-run-session -- xvfb-run -a env \
  XDG_RUNTIME_DIR="$tmp/runtime" \
  XDG_CONFIG_HOME="$tmp/config" \
  XDG_DATA_HOME="$tmp/data" \
  XDG_CACHE_HOME="$tmp/cache" \
  XDG_STATE_HOME="$tmp/state" \
  XDG_DATA_DIRS="$build/addon/test-data" \
  GTK_IM_MODULE=fcitx \
  XMODIFIERS=@im=fcitx \
  FCITX5_INK_PANEL="$tmp/panel-stub" \
  PANEL_LOG="$tmp/panel.log" \
  sh -c '
    set -eu
    fcitx5 -d --replace >"$1" 2>&1
    for _ in $(seq 1 50); do
      fcitx5-remote --check && break
      sleep 0.1
    done
    "$2" >"$3" 2>/dev/null &
    echo $! >"$4"
    sleep 2
    xdotool search --name fcitx5-ink-test windowfocus 2>/dev/null || true
    sleep 1
    fcitx5-remote -s ink >/dev/null 2>&1 || true
    sleep 1
    if [ -e "$5" ]; then
      echo "selecting ink spawned the panel" >&2
      cat "$5" >&2
      exit 1
    fi
    xdotool key --clearmodifiers ctrl+alt+h
    sleep 1
    fcitx5-remote -e 2>/dev/null || true
  ' sh "$tmp/fcitx.log" "$tmp/client" "$tmp/client.log" "$tmp/client.pid" \
    "$tmp/panel.log" >"$tmp/test.log" 2>&1; then
  cat "$tmp/test.log" >&2
  exit 1
fi

client_pid=$(cat "$tmp/client.pid" 2>/dev/null || true)
grep -q -- '--socket' "$tmp/panel.log"
grep -q 'fcitx5-ink/panel-input.sock' "$tmp/panel.log"
echo "selecting ink did not open the panel; hotkey did"
