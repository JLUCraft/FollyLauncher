#!/usr/bin/env bash
# Real native workflow acceptance on a private bus/keyring, never the user's existing identity.
set -euo pipefail
runner=$(realpath "$0")
cd "$(dirname "$runner")/../.."
: "${TUTORIAL_OUTPUT_DIR:?Set TUTORIAL_OUTPUT_DIR to a new evidence directory}"
if [[ ${JLU_PRIVATE_TEST_BUS:-} != 1 ]]; then
  exec dbus-run-session -- env JLU_PRIVATE_TEST_BUS=1 "$runner" "$@"
fi
keyring_binary=${KEYRING_DAEMON:-$(command -v gnome-keyring-daemon || true)}
[[ -x "$keyring_binary" ]] || { echo 'A Secret Service daemon is required; set KEYRING_DAEMON or install gnome-keyring.' >&2; exit 1; }
keyring_temp=$(mktemp -d /tmp/jlucraft-native-keyring.XXXXXX)
export XDG_DATA_HOME="$keyring_temp/data" XDG_CONFIG_HOME="$keyring_temp/config" XDG_RUNTIME_DIR="$keyring_temp/runtime"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
printf '%s' 'disposable-native-test-keyring' | "$keyring_binary" --foreground --unlock --components=secrets --control-directory="$keyring_temp/control" > "$keyring_temp/daemon.log" 2>&1 &
keyring_pid=$!
trap 'kill "$keyring_pid" 2>/dev/null || true; rm -rf "$keyring_temp"' EXIT
ready=false
for attempt in $(seq 1 50); do
  if gdbus call --session --dest org.freedesktop.secrets --object-path /org/freedesktop/secrets --method org.freedesktop.DBus.Peer.Ping >/dev/null 2>&1; then ready=true; break; fi
  sleep 0.2
done
[[ "$ready" == true ]] || { cat "$keyring_temp/daemon.log" >&2; exit 1; }
MANAGEMENT_CHECK=1 node scripts/federation/capture-tutorial.mjs
