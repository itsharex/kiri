#!/usr/bin/env bash
# In-process native encoder tests against a private, synthetic Pulse server.
# No desktop devices, settings, consent state, or capture library are used.
set -euo pipefail
cd "$(dirname "$0")/../.."
for command in pulseaudio gst-launch-1.0 cargo timeout; do
  command -v "$command" >/dev/null || { echo "Missing audio QA dependency: $command" >&2; exit 127; }
done
# Compile before starting the owned server; the timeout below bounds tests only.
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-run
root=$(mktemp -d "${TMPDIR:-/tmp}/kiri-audio-qa.XXXXXX")
pids=()
cleanup() {
  # A hung-server test can be interrupted by timeout before its Rust guard runs.
  for pid in "${pids[@]}"; do kill -CONT "$pid" 2>/dev/null || true; done
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
for pid in "${pids[@]}"; do
  kill -0 "$pid" 2>/dev/null || { echo "A synthetic audio source failed to start" >&2; exit 1; }
done
export KIRI_LINUX_PULSE_QA=1 KIRI_LINUX_PULSE_QA_SERVER_PID="${pids[0]}"
run_capture() {
  if [[ -n "${KIRI_LINUX_MEDIA_QA_DIR:-}" ]]; then
    KIRI_LINUX_MEDIA_QA_DIR="$KIRI_LINUX_MEDIA_QA_DIR/$1" \
      timeout 45s cargo test --locked --manifest-path src-tauri/Cargo.toml \
      native_pulse_private_server_captures_verified_sources -- --ignored --nocapture --test-threads=1
  else
    timeout 45s cargo test --locked --manifest-path src-tauri/Cargo.toml \
      native_pulse_private_server_captures_verified_sources -- --ignored --nocapture --test-threads=1
  fi
}
# Both clean startup and recovery must preserve PCM timing. Fault injection
# must not be a prerequisite for a passing recording or hide a startup defect.
run_capture clean
timeout 45s cargo test --locked --manifest-path src-tauri/Cargo.toml \
  native_pulse_private_server_cancel_survives_a_hung_service -- --ignored --nocapture --test-threads=1
run_capture after-hang
