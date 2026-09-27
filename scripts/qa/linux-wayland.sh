#!/usr/bin/env bash
# GNOME 46 screenshot portal checks on an isolated virtual desktop.
# Ubuntu 24.04 packages in addition to the installed Kiri package:
# sudo apt install --no-install-recommends gnome-shell gnome-settings-daemon \
#   xdg-desktop-portal xdg-desktop-portal-gnome pipewire wireplumber \
#   libgl1-mesa-dri libegl-mesa0 dbus-x11 at-spi2-core python3-pyatspi \
#   python3-gi python3-pil gir1.2-gtk-3.0 gir1.2-gstreamer-1.0 \
#   gir1.2-gst-plugins-base-1.0 gstreamer1.0-pipewire
set -euo pipefail

if [[ "$(uname -s)" != Linux ]]; then
  echo 'Run this check on an isolated Ubuntu 24.04 runner.' >&2
  exit 1
fi
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
executable="$(realpath "${1:-/usr/bin/kiri}")"
if [[ -n "${KIRI_LINUX_WAYLAND_QA_DIR:-}" ]]; then
  output="$KIRI_LINUX_WAYLAND_QA_DIR"
else
  output="$(mktemp -d "${TMPDIR:-/tmp}/kiri-linux-wayland-review.XXXXXXXX")"
fi
mkdir -p "$output"
output="$(realpath "$output")"
printf 'Wayland QA evidence: %s\n' "$output"
test -x "$executable"
for command in gnome-shell pipewire wireplumber dbus-run-session gsettings; do
  command -v "$command" >/dev/null
done

qa_profile=''
trap 'if [[ -n "$qa_profile" ]]; then rm -rf "$qa_profile"; fi' EXIT

# Denial is stored by the real permission-store service. Separate profiles make
# both branches reproducible without deleting or pre-granting any permissions.
for scenario in deny allow; do
  qa_profile="$(mktemp -d "${TMPDIR:-/tmp}/kiri-wayland-qa.XXXXXXXX")"
  mkdir -p "$qa_profile"/{home,config,data,cache,state,runtime}
  chmod 700 "$qa_profile/runtime"
  mkdir -p "$qa_profile/config/xdg-desktop-portal" "$output/$scenario"
  cat > "$qa_profile/config/xdg-desktop-portal/portals.conf" <<'EOF'
[preferred]
default=gnome
EOF
  env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DBUS_SESSION_BUS_ADDRESS \
    -u HYPRLAND_INSTANCE_SIGNATURE -u SWAYSOCK -u NO_AT_BRIDGE -u GSETTINGS_BACKEND \
    HOME="$qa_profile/home" XDG_CONFIG_HOME="$qa_profile/config" \
    XDG_DATA_HOME="$qa_profile/data" XDG_CACHE_HOME="$qa_profile/cache" \
    XDG_STATE_HOME="$qa_profile/state" XDG_RUNTIME_DIR="$qa_profile/runtime" \
    XDG_CURRENT_DESKTOP=GNOME XDG_SESSION_DESKTOP=gnome XDG_SESSION_TYPE=wayland \
    GDK_BACKEND=wayland WAYLAND_DISPLAY=kiri-wayland-qa \
    LIBGL_ALWAYS_SOFTWARE=1 GDK_SCALE=1 GDK_DPI_SCALE=1 \
    LANG=C.UTF-8 LC_ALL=C.UTF-8 RUST_LOG=info \
    KIRI_QA_PROFILE="$qa_profile" \
    dbus-run-session -- /usr/bin/python3 "$repository_root/scripts/qa/linux-wayland.py" \
    --executable "$executable" --output "$output/$scenario" --scenario "$scenario"
  rm -rf "$qa_profile"
  qa_profile=''
done
