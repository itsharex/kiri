# Linux IPC lifetime regression

A standalone headless test crate for the GObject ownership graph in the vendored
Wry 0.55.1 IPC handler. It uses real GLib/GIO objects (`glib 0.18.5`, `gio 0.18.4`),
not Rust `Rc`/`Weak` substitutes for view ownership. No application, GTK, WebKit,
display, profile, network request, or main loop starts during the tests.

From the repository root, with Rust, Python 3.11+, pkg-config and GLib/GIO
development packages installed (`libglib2.0-dev` on Debian/Ubuntu):

```sh
python3 scripts/qa/ipc-lifetime/check-source.py
env -u DISPLAY -u WAYLAND_DISPLAY cargo test --locked \
  --manifest-path scripts/qa/ipc-lifetime/Cargo.toml -- --test-threads=1
```

Add `--offline` when the locked registry dependencies are already cached.
The Linux build job runs both commands. Changes to this harness select that job;
they do not enable release packaging. The crate is QA-only and is not linked
into Kiri or a second Tauri application.

The six cases cover normal delivery and temporary-reference release, disposal
while a strong owner remains, late signals after finalization, natural release
without a cycle, a strong-capture negative control, and reentrant disposal
inside the application callback. The negative control explicitly disconnects
its intentional cycle before returning.

The source guard fingerprints the reviewed production IPC function (ignoring
indentation and blank lines) and checks every fixture dependency identity
against the application lockfile. An intentional IPC change requires reviewing
the model and updating the guard, rather than silently leaving model-only tests
green. The Rust implementation is preserved from the reviewed six-case harness.

A temporary upgraded owner protects allocation lifetime, not explicit disposal.
The application callback receives owned URI/body values; production code must
not access the WebView after invoking it. Rust `Drop` observes finalization;
GObject weak-notify alone would only establish disposal.

These tests do not establish real WebKit IPC, native window teardown, renderer
retirement, or absence of every leak. The separate focused native acceptance
is summarized in [the repair record](../../../docs/qa/linux-residual-repair.md).
