#!/usr/bin/env bash
# Run only against a fresh virtual desktop and throwaway user profile.
set -euo pipefail

if [[ "$(uname -s)" != Linux ]]; then
  echo "Linux native QA requires Linux with Xvfb and a window manager." >&2
  exit 1
fi

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
executable="$(realpath "${1:-$repository_root/src-tauri/target/release/kiri}")"
output="${KIRI_LINUX_QA_DIR:-$repository_root/linux-native-review}"
mkdir -p "$output"
output="$(realpath "$output")"
test -x "$executable"
for command in xvfb-run dbus-run-session openbox xdotool xclip xprop; do
  command -v "$command" >/dev/null
done

qa_profile="$(mktemp -d "${TMPDIR:-/tmp}/kiri-linux-qa.XXXXXXXX")"
trap 'rm -rf "$qa_profile"' EXIT
mkdir -p "$qa_profile"/{home,config,data,cache,state,runtime}
chmod 700 "$qa_profile/runtime"

# No Kiri runtime test switches: HOME/XDG isolate the ordinary production
# library while Xvfb provides real X11 windows for the native capture backend.
env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DBUS_SESSION_BUS_ADDRESS \
  -u HYPRLAND_INSTANCE_SIGNATURE -u SWAYSOCK \
  HOME="$qa_profile/home" \
  XDG_CONFIG_HOME="$qa_profile/config" \
  XDG_DATA_HOME="$qa_profile/data" \
  XDG_CACHE_HOME="$qa_profile/cache" \
  XDG_STATE_HOME="$qa_profile/state" \
  XDG_RUNTIME_DIR="$qa_profile/runtime" \
  XDG_SESSION_TYPE=x11 \
  XDG_CURRENT_DESKTOP=Openbox \
  GDK_BACKEND=x11 GDK_SCALE=1 GDK_DPI_SCALE=1 \
  LIBGL_ALWAYS_SOFTWARE=1 GSETTINGS_BACKEND=memory \
  LANG=C.UTF-8 LC_ALL=C.UTF-8 \
  KIRI_QA_PROFILE="$qa_profile" \
  xvfb-run --auto-servernum --server-args='-screen 0 1280x800x24 -nolisten tcp' \
  dbus-run-session -- /usr/bin/python3 "$repository_root/scripts/qa/linux-native.py" \
  --executable "$executable" --output "$output"
