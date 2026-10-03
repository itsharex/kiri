"""Smoke-test the extracted ZIP and installed NSIS app on disposable Windows CI."""

import json
import os
from pathlib import Path
import re
import subprocess
import time

from pywinauto import Desktop, keyboard, mouse


if os.name != "nt" or os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("Use an isolated Windows CI desktop")

output = Path("windows-release-review")
output.mkdir(exist_ok=True)
report = {"success": False, "checks": []}
desktop = Desktop(backend="uia")
process = None


def find(name, timeout=35, scroll=False):
    deadline = time.monotonic() + timeout
    previous = None
    stable = 0
    while time.monotonic() < deadline:
        needs_scroll = scroll
        if process.poll() is not None:
            raise RuntimeError(f"Kiri exited early: {process.returncode}")
        for window in desktop.windows(process=process.pid, visible_only=True):
            for control in window.descendants():
                try:
                    if (control.element_info.control_type != "Button" or
                            not re.fullmatch(name, control.window_text()) or not control.is_enabled()):
                        continue
                    if not control.is_visible() and scroll:
                        # WebView2 exposes the off-screen button in UIA. Asking
                        # that element to focus or scroll into view targets the
                        # settings pane; wheel events at the window gutter can
                        # miss its CSS scroll container.
                        try:
                            control.iface_scroll_item.ScrollIntoView()
                        except Exception:
                            control.set_focus()
                    if control.is_visible():
                        bounds = control.rectangle()
                        needs_scroll = False
                        current = (control.window_text(),
                                   (bounds.left, bounds.top, bounds.right, bounds.bottom))
                        if bounds.width() > 0 and bounds.height() > 0:
                            stable = stable + 1 if previous == current else 1
                            previous = current
                            if stable >= 3:
                                return control
                except Exception:
                    pass
        if needs_scroll:
            # Keep a physical-scroll fallback for WebView2 versions without
            # UIA ScrollItem support. Aim inside the page, not at its gutter.
            window = desktop.windows(process=process.pid, visible_only=True)[0]
            window.set_focus()
            bounds = window.rectangle()
            mouse.scroll(coords=(bounds.left + int(bounds.width() * 0.75),
                                 bounds.top + int(bounds.height() * 0.5)), wheel_dist=-4)
        time.sleep(0.2)
    raise RuntimeError(f"Visible control not found: {name}")


def snapshot(label):
    windows = desktop.windows(process=process.pid, visible_only=True)
    evidence = {"label": label, "windows": []}
    for index, window in enumerate(windows):
        filename = f"{label}-{index}.png"
        window.capture_as_image().save(output / filename)
        evidence["windows"].append({"screenshot": filename, "controls": [
            {"text": control.window_text(), "type": control.element_info.control_type,
             "bounds": [control.rectangle().left, control.rectangle().top,
                        control.rectangle().right, control.rectangle().bottom],
             "visible": control.is_visible(),
             "enabled": control.is_enabled()}
            for control in window.descendants()
        ]})
    report.setdefault("evidence", []).append(evidence)


def stop():
    global process
    if process and process.poll() is None:
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True)
        process.wait(timeout=15)
    process = None


def smoke(executable, update_button, label):
    global process
    if not executable.is_file():
        raise RuntimeError(f"Missing {label} executable")
    process = subprocess.Popen([str(executable)])
    try:
        settings = find("Settings")
        # UIA can expose WebView controls before startup activation completes.
        # Bring the native window forward before the one physical click, and
        # retain both frames instead of silently retrying a missed click.
        settings.top_level_parent().set_focus()
        settings = find("Settings")
        snapshot(f"{label.split()[0]}-before-settings")
        settings.click_input()
        find(update_button, scroll=True)
        snapshot(f"{label.split()[0]}-settings")
        report["checks"].append(f"{label} launches and shows the correct update route")
        keyboard.send_keys("^+a")
        find("Screenshot")
        keyboard.send_keys("{ESC}")
        report["checks"].append(f"{label} opens and cancels native capture")
    except Exception:
        try:
            snapshot(f"{label.split()[0]}-failure")
        except Exception as error:
            report["screenshot_error"] = str(error)
        report["windows"] = []
        for window in desktop.windows(process=process.pid, visible_only=True):
            try:
                report["windows"].append([
                    {"text": control.window_text(), "visible": control.is_visible(),
                     "enabled": control.is_enabled()}
                    for control in window.descendants()
                ])
            except Exception:
                pass
        raise
    finally:
        stop()


try:
    portable = Path(os.environ["KIRI_QA_PORTABLE_EXE"])
    installed = Path(os.environ["KIRI_QA_INSTALLED_EXE"])
    if not (portable.parent / "kiri.portable").is_file():
        raise RuntimeError("Extracted portable marker is missing")
    if (installed.parent / "kiri.portable").exists():
        raise RuntimeError("Installed copy contains a portable marker")
    smoke(portable, "Open Releases Page", "portable ZIP")
    if sorted(path.name for path in portable.parent.iterdir()) != ["kiri.exe", "kiri.portable"]:
        raise RuntimeError("Portable copy wrote unexpected files beside the executable")
    smoke(installed, "Check for Updates", "NSIS installation")
    report["success"] = True
except Exception as error:
    report["error"] = str(error)[:1500]
finally:
    stop()
    (output / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")

if not report["success"]:
    raise SystemExit(report["error"])
