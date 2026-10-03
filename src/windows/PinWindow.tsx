import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { mediaUrl, onAssetContentChanged, onPinOnTop } from "../lib/ipc";
import { t } from "../i18n";
import "./pin-window.css";

export function PinWindow({ id }: { id: string }) {
  const [onTop, setOnTop] = useState(true);
  const [busy, setBusy] = useState(false);
  const topState = useRef({onTop: true, generation: 0, busy: false});
  const [error, setError] = useState(false);
  const [imageFailed, setImageFailed] = useState(false);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    const subscription = onAssetContentChanged((assetId) => {
      if (assetId === id) { setImageFailed(false); setRevision((value) => value + 1); }
    });
    return () => { void subscription.then((dispose) => dispose()).catch(() => {}); };
  }, [id]);
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape" || ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "w")) {
        event.preventDefault(); void getCurrentWindow().close();
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, []);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    // Every new pin is created with always_on_top(true). The native getter can
    // briefly report false while the window manager is applying that request.
    // Track successful Kiri pin actions instead of freezing that startup snapshot.
    void onPinOnTop(() => {
      if (disposed) return;
      topState.current.generation += 1;
      topState.current.onTop = true;
      setOnTop(true);
      setError(false);
    }).then(stop => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch(() => {});
    return () => { disposed = true; unlisten?.(); };
  }, []);
  const toggleTop = async () => {
    const state = topState.current;
    if (state.busy) return;
    state.busy = true;
    const generation = state.generation;
    const next = !state.onTop;
    setBusy(true); setError(false);
    try {
      await getCurrentWindow().setAlwaysOnTop(next);
      // A library repin received while this request was pending is newer.
      if (state.generation === generation) {
        state.onTop = next;
        setOnTop(next);
      }
    }
    catch { if (state.generation === generation) setError(true); }
    finally { state.busy = false; setBusy(false); }
  };
  return <div className="pin-window">
    <header className="pin-window__header">
      <strong>{t("Screenshot Reference")}</strong>
      <div className="pin-window__actions">
        <button className="kiri-pin-action" type="button" disabled={busy} onClick={() => void toggleTop()}>{t(onTop ? "Unpin" : "Pin on Top")}</button>
        <button className="kiri-pin-action" type="button" onClick={() => void getCurrentWindow().close()}>{t("Close")}</button>
      </div>
    </header>
    {error && <p className="pin-window__error" role="alert">{t("Could not change window pinning on this desktop.")}</p>}
    <main className="pin-window__image">
      {imageFailed ? <p role="alert">{t("Can't read this file")}</p> :
        <img key={revision} src={`${mediaUrl(id)}?v=${revision}`} alt={t("Pinned screenshot")} onError={() => setImageFailed(true)} />}
    </main>
  </div>;
}
