"""GNOME 46 portal smoke: real app, real permission UI, isolated desktop.

Run through linux-wayland.sh. CI runs this against the same Debian package as
the X11 checks. This virtual desktop does not test recording or hardware.
No Shell unsafe mode, portal permission overrides, mocks, or Kiri test switches.

Version-specific primary interfaces:
https://github.com/GNOME/mutter/blob/46.2/data/dbus-interfaces/org.gnome.Mutter.RemoteDesktop.xml
https://github.com/GNOME/mutter/blob/46.2/data/dbus-interfaces/org.gnome.Mutter.ScreenCast.xml
https://github.com/GNOME/gnome-shell/blob/46.0/src/gnome-shell-test-tool.in
"""

import argparse
from collections import deque
import json
import os
from pathlib import Path
import subprocess
import sys
import time

import gi

gi.require_version("Gst", "1.0")
gi.require_version("GstApp", "1.0")
from gi.repository import Gio, GLib, Gst, GstApp
from PIL import Image, ImageChops, ImageStat


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--executable", required=True, type=Path)
parser.add_argument("--output", required=True, type=Path)
parser.add_argument("--scenario", choices=("allow", "deny"), required=True)
args = parser.parse_args()
profile = Path(os.environ.get("KIRI_QA_PROFILE", "/missing")).resolve()
if sys.platform != "linux" or not profile.is_dir() or os.environ.get("XDG_SESSION_TYPE") != "wayland":
    raise SystemExit("Use linux-wayland.sh to create a disposable GNOME session")
for key in ("HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_RUNTIME_DIR"):
    if not Path(os.environ[key]).resolve().is_relative_to(profile):
        raise SystemExit(f"Refusing an unisolated {key}")

output = args.output.resolve()
output.mkdir(parents=True, exist_ok=True)
library = Path(os.environ["XDG_DATA_HOME"]) / "kiri"
report = {"success": False, "environment": "GNOME 46 headless / software rendering",
          "scenario": args.scenario, "checks": [],
          "not_tested": ["recording", "hardware GPU", "multiple displays", "fractional scaling"]}
children = []
logs = []
pipeline = None
remote_path = None
screen_path = None
fixture = None
connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
context = GLib.MainContext.default()
REMOTE = "org.gnome.Mutter.RemoteDesktop"
CAST = "org.gnome.Mutter.ScreenCast"


def call(name, path, interface, method, signature=None, values=()):
    parameters = GLib.Variant(signature, values) if signature else None
    result = connection.call_sync(name, path, interface, method, parameters,
                                  None, Gio.DBusCallFlags.NONE, 10000, None)
    return result.unpack()


def launch(name, command):
    log = (output / f"{name}.log").open("wb")
    logs.append(log)
    child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    children.append((name, child))
    return child


def pump():
    while context.pending():
        context.iteration(False)


def wait_for(label, predicate, timeout=35):
    report["phase"] = label
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        pump()
        for name, child in children:
            if child.poll() is not None:
                raise RuntimeError(f"{name} exited during {label}: {child.returncode}")
        result = predicate()
        if result:
            return result
        time.sleep(0.1)
    raise RuntimeError(f"Timed out: {label}")


def pause(seconds=0.6):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        pump()
        time.sleep(0.03)


def owns(name):
    return call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus",
                "NameHasOwner", "(s)", (name,))[0]


def overview_is_closed():
    active = call("org.gnome.Shell", "/org/gnome/Shell", "org.freedesktop.DBus.Properties",
                  "Get", "(ss)", ("org.gnome.Shell", "OverviewActive"))[0]
    if isinstance(active, GLib.Variant):
        active = active.unpack()
    return active is False


def send(method, signature=None, values=()):
    return call(REMOTE, remote_path, f"{REMOTE}.Session", method, signature, values)


def key(keysym):
    send("NotifyKeyboardKeysym", "(ub)", (keysym, True))
    send("NotifyKeyboardKeysym", "(ub)", (keysym, False))
    pause(0.2)


def move(x, y):
    send("NotifyPointerMotionAbsolute", "(sdd)", (stream_path, float(x), float(y)))


def button(pressed):
    send("NotifyPointerButton", "(ib)", (272, pressed))  # Linux BTN_LEFT


def click(x, y):
    move(x, y)
    button(True)
    button(False)
    pause()


def controls(name):
    queue = deque([pyatspi.Registry.getDesktop(0)])
    visited = 0
    while queue and visited < 3000:
        node = queue.popleft()
        visited += 1
        try:
            if node.name == name and node.getState().contains(pyatspi.STATE_SHOWING):
                bounds = node.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
                if bounds.width > 0 and bounds.height > 0:
                    return bounds
            queue.extend(node.getChildAtIndex(i) for i in range(node.childCount))
        except Exception:
            continue
    return None


def screenshot(name):
    sample = sink.emit("try-pull-sample", 5 * Gst.SECOND)
    if sample is None:
        raise RuntimeError("Mutter/PipeWire did not provide a desktop frame")
    structure = sample.get_caps().get_structure(0)
    width, height = structure.get_value("width"), structure.get_value("height")
    buffer = sample.get_buffer()
    pixels = buffer.extract_dup(0, buffer.get_size())
    picture = Image.frombytes("RGB", (width, height), pixels, "raw", "RGB", len(pixels) // height)
    picture.save(output / name)
    return picture


def fixture_frame(name):
    picture = screenshot(name)
    samples = [picture.getpixel(point) for point in ((330, 470), (510, 470), (690, 470))]
    if all(max(abs(channel - expected) for channel in pixel) <= 1
           for pixel, expected in zip(samples, (17, 119, 221))):
        return picture
    return None


def overlay_frame(name):
    picture = screenshot(name)
    # Accessibility nodes can appear before WebKit paints. Reject a blank
    # overlay; the saved-image comparison below proves the frozen pixels.
    tones = [sum(picture.getpixel(point)) / 3 for point in ((330, 470), (510, 470), (690, 470))]
    return picture if tones[1] - tones[0] > 20 and tones[2] - tones[1] > 20 else None


def assets():
    index = library / "library.json"
    return json.loads(index.read_text()) if index.is_file() else []


def capture():
    subprocess.run([str(args.executable), "--capture"], check=True, timeout=10)


def daemon_path(name):
    for directory in ("/usr/libexec", "/usr/lib/xdg-desktop-portal"):
        path = Path(directory) / name
        if path.is_file():
            return str(path)
    raise RuntimeError(f"Missing installed portal daemon: {name}")


try:
    # These settings are stored only in this disposable profile/session bus.
    for schema, setting, value in (
        ("org.gnome.desktop.interface", "toolkit-accessibility", "true"),
        ("org.gnome.desktop.interface", "enable-animations", "false"),
        ("org.gnome.desktop.session", "idle-delay", "0"),
        ("org.gnome.desktop.screensaver", "lock-enabled", "false"),
    ):
        subprocess.run(["gsettings", "set", schema, setting, value], check=True, timeout=10)
    report["gnome_shell"] = subprocess.check_output(["gnome-shell", "--version"], text=True).strip()
    launch("pipewire", ["pipewire"])
    wait_for("PipeWire socket", lambda: (Path(os.environ["XDG_RUNTIME_DIR"]) / "pipewire-0").is_socket())
    launch("wireplumber", ["wireplumber"])
    launch("gnome-shell", ["gnome-shell", "--wayland", "--headless", "--virtual-monitor", "1280x800",
                            "--wayland-display", os.environ["WAYLAND_DISPLAY"]])
    wait_for("GNOME RemoteDesktop service", lambda: owns(REMOTE))
    wait_for("GNOME Screenshot service", lambda: owns("org.gnome.Shell.Screenshot"))
    # The bus names appear before layoutManager's startup-complete signal.
    # Its cover pane can still swallow input until this Shell 46 message.
    wait_for("GNOME startup completes", lambda: "GNOME Shell started at" in
             (output / "gnome-shell.log").read_text(errors="replace"))
    subprocess.run(["dbus-update-activation-environment", "WAYLAND_DISPLAY", "XDG_CURRENT_DESKTOP",
                    "XDG_SESSION_TYPE", "GDK_BACKEND"], check=True, timeout=10)
    launch("portal-gnome", [daemon_path("xdg-desktop-portal-gnome"), "--verbose"])
    wait_for("GNOME portal backend", lambda: owns("org.freedesktop.impl.portal.desktop.gnome"))
    launch("portal", [daemon_path("xdg-desktop-portal"), "--verbose"])
    wait_for("Desktop portal service", lambda: owns("org.freedesktop.portal.Desktop"))

    monitors = call("org.gnome.Mutter.DisplayConfig", "/org/gnome/Mutter/DisplayConfig",
                    "org.gnome.Mutter.DisplayConfig", "GetCurrentState")[1]
    if len(monitors) != 1:
        raise RuntimeError(f"Expected exactly one GNOME monitor; got {len(monitors)}")
    connector = monitors[0][0][0]
    remote_path = call(REMOTE, "/org/gnome/Mutter/RemoteDesktop", REMOTE, "CreateSession")[0]
    session_id = call(REMOTE, remote_path, "org.freedesktop.DBus.Properties", "Get", "(ss)",
                      (f"{REMOTE}.Session", "SessionId"))[0]
    if isinstance(session_id, GLib.Variant):
        session_id = session_id.unpack()
    screen_path = call(CAST, "/org/gnome/Mutter/ScreenCast", CAST, "CreateSession", "(a{sv})",
                       ({"remote-desktop-session-id": GLib.Variant("s", session_id)},))[0]
    stream_path = call(CAST, screen_path, f"{CAST}.Session", "RecordMonitor", "(sa{sv})",
                       (connector, {"cursor-mode": GLib.Variant("u", 0)}))[0]
    node_ids = []
    connection.signal_subscribe(CAST, f"{CAST}.Stream", "PipeWireStreamAdded", stream_path,
                                None, Gio.DBusSignalFlags.NONE,
                                lambda *event: node_ids.append(event[5].unpack()[0]))
    send("Start")
    wait_for("real desktop PipeWire stream", lambda: node_ids)
    Gst.init(None)
    pipeline = Gst.parse_launch(f"pipewiresrc path={node_ids[0]} ! videoconvert ! "
                                "video/x-raw,format=RGB ! appsink name=qa_sink max-buffers=1 drop=true sync=false")
    sink = pipeline.get_by_name("qa_sink")
    pipeline.set_state(Gst.State.PLAYING)
    report["checks"].append("single-monitor headless GNOME with real PipeWire desktop stream")

    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk, Gdk
    import pyatspi
    fixture = Gtk.Window(title="Kiri Wayland public QA")
    fixture.fullscreen()
    layout = Gtk.Fixed()
    fixture.add(layout)
    provider = Gtk.CssProvider()
    provider.load_from_data(b"#qa-background { background: #eeeeee; } #qa-paper { background: white; } "
                            b"#qa-ink { background: #111111; } #qa-grey { background: #777777; } "
                            b"#qa-light { background: #dddddd; } label { color: #111111; font-size: 32px; }")
    Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
    layout.set_name("qa-background")
    for name, x, y, width, height in (
        ("qa-paper", 200, 180, 800, 520), ("qa-ink", 270, 400, 120, 140),
        ("qa-grey", 450, 400, 120, 140), ("qa-light", 630, 400, 120, 140),
    ):
        patch = Gtk.EventBox()
        patch.set_name(name)
        patch.set_size_request(width, height)
        layout.put(patch, x, y)
    layout.put(Gtk.Label(label="SCREEN CAPTURE 123"), 270, 265)
    fixture.show_all()
    key(0xff1b)  # Dismiss the initial GNOME overview with a real key event.
    wait_for("GNOME overview closes", overview_is_closed)
    launch("kiri", [str(args.executable.resolve())])
    wait_for("Kiri library rendered", lambda: controls("Settings"))
    # GNOME can place the library at the top-left instead of centering it.
    click(1240, 760)  # Exposed corner of the full-screen public test window.
    source = wait_for("public fixture is visible", lambda: fixture_frame("source-desktop.png"))
    expected = source.crop((220, 240, 900, 620))
    expected.save(output / "expected-region.png")
    capture()
    wait_for("real Screenshot permission dialog", lambda: controls("Allow") and controls("Deny"))
    screenshot("portal-permission.png")
    choice = controls("Allow" if args.scenario == "allow" else "Deny")
    click(choice.x + choice.width / 2, choice.y + choice.height / 2)
    wait_for("permission dialog closes", lambda: controls("Allow") is None)

    if args.scenario == "deny":
        wait_for("denial reaches Kiri", lambda: "cancelled or denied" in (output / "kiri.log").read_text(errors="replace"))
        pause(2)
        if assets() or controls("Screenshot"):
            raise RuntimeError("Denied capture must not save an image or open an overlay")
        if controls("Allow") or controls("Deny"):
            raise RuntimeError("Denied capture opened another permission dialog")
        screenshot("denied-desktop.png")
        report["checks"].append("real portal denial reaches Kiri without saving or retrying a dialog")
    else:
        wait_for("Kiri screenshot overlay", lambda: controls("Screenshot"))
        wait_for("capture desktop content is visible", lambda: overlay_frame("capture-overlay.png"))
        key(0xff1b)
        wait_for("Escape closes capture", lambda: controls("Screenshot") is None)
        if assets():
            raise RuntimeError("Cancelled overlay unexpectedly saved an image")
        click(1240, 760)
        wait_for("public fixture is visible again", lambda: fixture_frame("repeat-source-desktop.png"))
        capture()
        wait_for("second screenshot overlay", lambda: controls("Screenshot"))
        wait_for("second capture desktop content is visible", lambda: overlay_frame("repeat-capture-overlay.png"))
        move(220, 240)
        button(True)
        pause(0.2)
        move(900, 620)
        pause(0.2)
        button(False)
        pause()
        screenshot("selected-region.png")
        key(0xff0d)
        saved = wait_for("real capture is persisted", lambda: assets() if len(assets()) == 1 else None)
        filename = Path(saved[0]["filename"])
        if filename.name != str(filename):
            raise RuntimeError("Capture filename escaped the isolated library")
        captured = Image.open(library / "Assets" / filename).convert("RGB")
        captured.save(output / "saved-capture.png")
        if captured.size != expected.size:
            raise RuntimeError(f"Capture dimensions {captured.size} do not match {expected.size}")
        difference = ImageChops.difference(captured, expected)
        error = sum(ImageStat.Stat(difference).mean) / 3
        report["capture_mean_pixel_error"] = round(error, 4)
        if error > 1.5:
            difference.save(output / "pixel-difference.png")
            raise RuntimeError(f"Wayland capture pixel mismatch: {error:.3f}")
        report["checks"].append("real portal approval, overlay cancellation, repeat capture, and saved pixels")
    report["success"] = True
except Exception as error:
    report["error"] = str(error)
    if pipeline is not None:
        try:
            screenshot("failure-desktop.png")
        except Exception:
            pass
finally:
    if pipeline is not None:
        pipeline.set_state(Gst.State.NULL)
    if remote_path is not None:
        try:
            send("Stop")
        except Exception:
            pass
    if fixture is not None:
        fixture.destroy()
    for _name, child in reversed(children):
        if child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)
    for log in logs:
        log.close()
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")

if not report["success"]:
    raise SystemExit(report["error"])
print(json.dumps(report, indent=2))
