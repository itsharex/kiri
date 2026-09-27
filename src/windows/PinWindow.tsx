import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { mediaUrl, onAssetContentChanged } from "../lib/ipc";
import { t } from "../i18n";
import "./pin-window.css";

export function PinWindow({ id }: { id: string }) {
  const [onTop, setOnTop] = useState(true);
  const [busy, setBusy] = useState(false);
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
  const toggleTop = async () => {
    if (busy) return;
    setBusy(true); setError(false);
    try { await getCurrentWindow().setAlwaysOnTop(!onTop); setOnTop(!onTop); }
    catch { setError(true); }
    finally { setBusy(false); }
  };
  return <div className="pin-window">
    <header className="pin-window__header">
      <strong>{t("Screenshot Reference")}</strong>
      <div className="pin-window__actions">
        <button type="button" disabled={busy} onClick={() => void toggleTop()}>{t(onTop ? "Unpin" : "Pin on Top")}</button>
        <button type="button" onClick={() => void getCurrentWindow().close()}>{t("Close")}</button>
      </div>
    </header>
    {error && <p className="pin-window__error" role="alert">{t("Could not change window pinning on this desktop.")}</p>}
    <main className="pin-window__image">
      {imageFailed ? <p role="alert">{t("Can't read this file")}</p> :
        <img key={revision} src={`${mediaUrl(id)}?v=${revision}`} alt={t("Pinned screenshot")} onError={() => setImageFailed(true)} />}
    </main>
  </div>;
}
