#!/usr/bin/env bash
# In-process native encoder tests against a private, synthetic Pulse server.
# No desktop devices, settings, consent state, or capture library are used.
set -euo pipefail
cd "$(dirname "$0")/../.."
root=$(mktemp -d "${TMPDIR:-/tmp}/kiri-audio-qa.XXXXXX")
pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  for pid in "${pids[@]}"; do wait "$pid" 2>/dev/null || true; done
  rm -rf -- "$root"
}
trap cleanup EXIT
mkdir -p "$root/home" "$root/config" "$root/runtime" "$root/pulse"
chmod 700 "$root/runtime" "$root/pulse"
# Only this temporary server reads the cookie; authentication stays enabled.
head -c 256 /dev/urandom > "$root/cookie"
chmod 600 "$root/cookie"
cat > "$root/server.pa" <<CONFIG
load-module module-native-protocol-unix socket=$root/pulse/native auth-cookie=$root/cookie
load-module module-null-sink sink_name=kiri_test_system rate=48000 channels=2
load-module module-null-sink sink_name=kiri_test_mic_feed rate=48000 channels=2
load-module module-remap-source source_name=kiri_test_mic master=kiri_test_mic_feed.monitor
set-default-sink kiri_test_system
set-default-source kiri_test_mic
CONFIG
HOME="$root/home" XDG_CONFIG_HOME="$root/config" XDG_RUNTIME_DIR="$root/runtime" \
  pulseaudio --daemonize=no --use-pid-file=no --exit-idle-time=-1 --disallow-exit \
  --log-target="file:$root/pulse.log" -nF "$root/server.pa" &
pids+=("$!")
export PULSE_SERVER="unix:$root/pulse/native" PULSE_COOKIE="$root/cookie"
for _ in $(seq 1 100); do
  test -S "$root/pulse/native" && break
  kill -0 "${pids[0]}" 2>/dev/null || { cat "$root/pulse.log"; exit 1; }
  sleep 0.05
done
test -S "$root/pulse/native"
gst-launch-1.0 -q audiotestsrc is-live=true freq=440 volume=0.15 ! audioconvert ! pulsesink device=kiri_test_system &
pids+=("$!")
gst-launch-1.0 -q audiotestsrc is-live=true freq=880 volume=0.15 ! audioconvert ! pulsesink device=kiri_test_mic_feed &
pids+=("$!")
sleep 0.2
KIRI_LINUX_PULSE_QA=1 cargo test --locked --manifest-path src-tauri/Cargo.toml \
  native_pulse_private_server_captures_verified_sources -- --ignored --nocapture
