#!/usr/bin/env python3
"""Bind the ownership-model tests to the reviewed Wry IPC implementation.

Deliberately fail closed when that function or its GLib/GIO dependency graph
changes. Review the lifetime model before updating the expected fingerprint.
This is a source guard, not a Rust parser or a native WebKit test.
"""
from pathlib import Path
import hashlib
import tomllib

root = Path(__file__).resolve().parents[3]
harness = Path(__file__).resolve().parent
source = (root / "src-tauri/vendor/wry/src/webkitgtk/mod.rs").read_text()
start = '  fn attach_ipc_handler(webview: WebView, attributes: &mut WebViewAttributes) {'
end = '\n  #[cfg(any(debug_assertions, feature = "devtools"))]'
begin = source.index(start)
function = source[begin:source.index(end, begin)]
normalized = "\n".join(line.strip() for line in function.splitlines() if line.strip())
expected = "33549930a5e738760aa771badd59b7b9156e271261d889958a8c115a8a4387bc"
if hashlib.sha256(normalized.encode()).hexdigest() != expected:
    raise SystemExit("Wry IPC source changed: review the ownership harness and source contract")

app_packages = tomllib.loads((root / "src-tauri/Cargo.lock").read_text())["package"]
fixture_packages = tomllib.loads((harness / "Cargo.lock").read_text())["package"]
identities = {(p["name"], p["version"], p.get("source"), p.get("checksum")) for p in app_packages}
for package in fixture_packages:
    if package["name"] == "kiri-weak-ipc-lifetime-tests":
        continue
    identity = (package["name"], package["version"], package.get("source"), package.get("checksum"))
    if identity not in identities:
        raise SystemExit(f"Harness dependency differs from application lock: {identity[:2]}")
print("PASS: reviewed Wry IPC function and exact application dependency identities")
