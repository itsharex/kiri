"""Installed Linux app smoke test on Xvfb, using real desktop capture pixels.

Invoke through linux-native.sh. This does not exercise GNOME/Wayland portals,
recording, multiple monitors, or fractional scaling.
"""

import argparse
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import tkinter as tk

from PIL import Image, ImageChops, ImageGrab, ImageStat


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--executable", required=True, type=Path)
parser.add_argument("--output", required=True, type=Path)
args = parser.parse_args()
profile = Path(os.environ.get("KIRI_QA_PROFILE", "/missing")).resolve()
if sys.platform != "linux" or not profile.is_dir() or os.environ.get("XDG_SESSION_TYPE") != "x11":
    raise SystemExit("Use linux-native.sh to create an isolated Linux desktop")
for variable in ("HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR"):
    if not Path(os.environ[variable]).resolve().is_relative_to(profile):
        raise SystemExit(f"{variable} must be inside the disposable QA profile")

output = args.output.resolve()
output.mkdir(parents=True, exist_ok=True)
library = Path(os.environ["XDG_DATA_HOME"]) / "kiri"
report = {
    "success": False,
    "session": "X11 / Xvfb / Openbox / 1280x800 / scale 1",
    "checks": [],
    "not_tested": ["GNOME Wayland and portal consent", "recording", "multiple monitors", "fractional scaling"],
}
process = None
manager = None
fixture = None
logs = []


def command(*arguments, check=True, binary=False):
    return subprocess.run(arguments, check=check, capture_output=True, text=not binary, timeout=12)


def wait_for(description, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if fixture is not None:
            fixture.update()
        if process is not None and process.poll() is not None:
            raise RuntimeError(f"Kiri exited during {description}: {process.returncode}")
        result = predicate()
        if result:
            return result
        time.sleep(0.1)
    raise RuntimeError(f"Timed out: {description}")


def pause(seconds=0.6):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if fixture is not None:
            fixture.update()
        time.sleep(0.05)


def windows():
    result = command("xdotool", "search", "--onlyvisible", "--pid", str(process.pid), check=False)
    return result.stdout.split() if result.returncode == 0 else []


def geometry(window):
    result = command("xdotool", "getwindowgeometry", "--shell", str(window), check=False)
    return dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)


def overlay():
    for window in windows():
        bounds = geometry(window)
        if int(bounds.get("WIDTH", 0)) >= 1278 and int(bounds.get("HEIGHT", 0)) >= 798:
            return window
    return None


def screenshot(name):
    picture = ImageGrab.grab().convert("RGB")
    picture.save(output / name)
    return picture


def assets():
    index = library / "library.json"
    if not index.is_file():
        return []
    return json.loads(index.read_text(encoding="utf-8"))


def start():
    global process
    log = (output / f"app-{len(logs) + 1}.log").open("wb")
    logs.append(log)
    process = subprocess.Popen([str(args.executable.resolve())], stdout=log, stderr=subprocess.STDOUT)
    wait_for("visible library window", windows)
    pause(1)


def stop(child):
    if child is not None and child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)


def show_fixture():
    fixture.lift()
    fixture.focus_force()
    fixture.update()
    pause()


def open_capture():
    command("xdotool", "key", "--clearmodifiers", "ctrl+shift+a")
    window = wait_for("full-display capture overlay", overlay)
    # The native window maps before the WebKit page has installed its listeners.
    pause(1)
    return window


def compare(actual, expected, description):
    if actual.size != expected.size:
        raise RuntimeError(f"{description}: got {actual.size}, expected {expected.size}")
    difference = ImageChops.difference(actual.convert("RGB"), expected.convert("RGB"))
    error = sum(ImageStat.Stat(difference).mean) / 3
    if error > 1.5:
        difference.save(output / "pixel-difference.png")
        raise RuntimeError(f"{description}: mean pixel error {error:.3f}")
    return round(error, 4)


try:
    manager_log = (output / "openbox.log").open("wb")
    logs.append(manager_log)
    manager = subprocess.Popen(["openbox"], stdout=manager_log, stderr=subprocess.STDOUT)
    wait_for("Openbox window manager", lambda: "window id" in command(
        "xprop", "-root", "_NET_SUPPORTING_WM_CHECK", check=False).stdout)

    # A separate ordinary X11 program owns this public test window. Kiri must
    # capture the screen through its shipping backend, not an injected image.
    fixture = tk.Tk()
    fixture.title("Kiri Linux QA public pattern")
    fixture.overrideredirect(True)
    fixture.geometry("1280x800+0+0")
    canvas = tk.Canvas(fixture, width=1280, height=800, highlightthickness=0, background="#eeeeee")
    canvas.pack(fill="both", expand=True)
    canvas.create_text(80, 80, text="KIRI LINUX DESKTOP QA", anchor="w", fill="#222222", font=("Sans", 30))
    canvas.create_rectangle(200, 180, 1000, 700, fill="#ffffff", outline="")
    canvas.create_text(270, 290, text="SCREEN CAPTURE 123", anchor="w", fill="#111111", font=("Sans", 32))
    for x, shade in ((270, "#111111"), (450, "#777777"), (630, "#dddddd")):
        canvas.create_rectangle(x, 400, x + 120, 540, fill=shade, outline="")
    fixture.update()

    start()
    screenshot("library-launch.png")
    if assets():
        raise RuntimeError("QA must start with an empty isolated library")
    report["checks"].append("installed app opens a visible library in an empty isolated profile")

    show_fixture()
    open_capture()
    screenshot("capture-overlay.png")
    command("xdotool", "key", "--clearmodifiers", "Escape")
    wait_for("Escape closes the overlay", lambda: overlay() is None)
    if assets():
        raise RuntimeError("Cancelled capture unexpectedly saved an asset")
    report["checks"].append("native global shortcut opens capture; Escape cancels without saving")

    show_fixture()
    desktop = screenshot("source-desktop.png")
    expected = desktop.crop((220, 240, 900, 620))
    expected.save(output / "expected-region.png")
    open_capture()
    command("xdotool", "mousemove", "220", "240", "mousedown", "1")
    pause(0.15)
    command("xdotool", "mousemove", "--sync", "900", "620")
    pause(0.15)
    command("xdotool", "mouseup", "1")
    pause()
    screenshot("selected-region.png")
    command("xdotool", "key", "--clearmodifiers", "Return")
    saved = wait_for("Return saves one captured image", lambda: assets() if len(assets()) == 1 else None)
    wait_for("successful capture closes its overlay", lambda: overlay() is None)
    asset = saved[0]
    if asset["kind"] != "image" or (asset["pixelWidth"], asset["pixelHeight"]) != (680, 380):
        raise RuntimeError(f"Unexpected capture metadata: {asset}")
    filename = Path(asset["filename"])
    if filename.name != str(filename):
        raise RuntimeError("Capture filename must remain inside the isolated library")
    captured = Image.open(library / "Assets" / filename).convert("RGB")
    captured.save(output / "saved-capture.png")
    report["capture_mean_pixel_error"] = compare(captured, expected, "saved native screenshot")
    report["checks"].append("region drag and Return save the exact pixels from the real X11 desktop")

    clipboard = command("xclip", "-selection", "clipboard", "-t", "image/png", "-o", binary=True).stdout
    clipboard_image = Image.open(io.BytesIO(clipboard)).convert("RGB")
    report["clipboard_mean_pixel_error"] = compare(clipboard_image, captured, "clipboard image")
    clipboard_image.save(output / "clipboard.png")
    report["checks"].append("clipboard contains the captured PNG")

    stop(process)
    process = None
    start()
    if assets() != saved:
        raise RuntimeError("Capture library changed after restarting the installed app")
    screenshot("library-reopened.png")
    report["checks"].append("saved capture persists after restarting the installed app")
    report["success"] = True
except Exception as error:
    report["error"] = str(error)
    try:
        screenshot("failure-desktop.png")
        report["windows"] = {window: geometry(window) for window in windows()} if process else {}
    except Exception:
        pass
finally:
    stop(process)
    if fixture is not None:
        fixture.destroy()
    stop(manager)
    for log in logs:
        log.close()
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

if not report["success"]:
    raise SystemExit(report["error"])
print(json.dumps(report, indent=2))
