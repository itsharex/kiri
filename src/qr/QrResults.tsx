import { useEffect, useRef, useState } from "react";
import { QrCode, Copy, ExternalLink, Star, X, Trash2, ChevronRight } from "lucide-react";
import { api, mediaUrl, onLibraryChanged, type AssetDto, type QrCodeDto, type QrScanDto } from "../lib/ipc";
import { t, fmt } from "../i18n";
import { initialQrSelection, qrCodeCenter } from "./selection.js";
import "../ocr/text-history.css";
import "./qr.css";

export function QrDetails({ code, action, saved = false, removable = false }: {
  code: QrCodeDto; action(action: string): Promise<unknown>; saved?: boolean; removable?: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [favorite, setFavorite] = useState(saved);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const run = async (name: string) => {
    if (busy) return;
    setBusy(true); setMessage("");
    try {
      await action(name);
      if (!mounted.current) return;
      if (name === "favorite") setFavorite(true);
      if (name === "unfavorite" || name === "remove") setFavorite(false);
      setConfirmOpen(false);
      setMessage(t(name === "copy" || name === "copyImage" ? "Copied to Clipboard" : name === "favorite" ? "Saved to QR Favorites" : name === "remove" || name === "unfavorite" ? "Moved to Trash" : "Link opened"));
    } catch (error) {
      if (mounted.current) setMessage(t(typeof error === "string" ? error : "Could not complete this QR action."));
    } finally { if (mounted.current) setBusy(false); }
  };
  return <section className="qr-details" aria-label={t("QR Content")}>
    <span className="text-reader__caption">{t("QR Content")}</span>
    <div className="qr-content" tabIndex={0}>{code.text ?? t("This QR code could not be decoded. Try a clearer image.")}</div>
    {code.text && <>
      {code.host && <p className="qr-host">{t("Destination")}: <bdi>{code.host}</bdi></p>}
      {code.suspicious && <p className="qr-warning" role="note">{t("Review this content carefully. This link may be unsafe.")}</p>}
      {!code.url && code.suspicious && <p className="qr-warning">{t("Only explicit HTTP or HTTPS links without credentials can be opened.")}</p>}
      <div className="qr-actions">
        <button type="button" className="kiri-button kiri-button--primary" disabled={busy} onClick={() => void run("copy")}><Copy size={14}/>{t("Copy Text")}</button>
        {code.url && <button type="button" className="kiri-button kiri-button--secondary" disabled={busy} onClick={() => setConfirmOpen(true)}><ExternalLink size={14}/>{t("Open Link")}</button>}
        {!removable && <button type="button" className="kiri-button kiri-button--secondary" disabled={busy} onClick={() => void run(favorite ? "unfavorite" : "favorite")}><Star size={14} fill={favorite ? "currentColor" : "none"}/>{t(favorite ? "Remove Favorite" : "Save QR Code")}</button>}
        {removable && <>
          <button type="button" className="kiri-button kiri-button--secondary" disabled={busy} onClick={() => void run("copyImage")}><QrCode size={14}/>{t("Copy QR Image")}</button>
          <button type="button" className="kiri-button kiri-button--secondary" disabled={busy} onClick={() => void run("remove")}><Trash2 size={14}/>{t("Remove Favorite")}</button>
        </>}
      </div>
      {confirmOpen && <div className="qr-open-review" role="group" aria-label={t("Review Link")}>
        <p>{t("Only open links you trust.")}</p><p><bdi>{code.url}</bdi></p>
        <div className="qr-actions"><button type="button" className="kiri-button kiri-button--secondary" disabled={busy} onClick={() => setConfirmOpen(false)}>{t("Cancel")}</button><button type="button" className="kiri-button kiri-button--primary" disabled={busy} onClick={() => void run("open")}>{t("Open Link")}</button></div>
      </div>}
    </>}
    {message && <p className="qr-feedback" role="status">{message}</p>}
  </section>;
}

export function QrResults({ scan }: { scan: QrScanDto }) {
  const [selected, setSelected] = useState<number | null>(() => initialQrSelection(scan.codes));
  const [favorites, setFavorites] = useState<Set<string>>(() => new Set());
  const code = scan.codes.find(code => code.index === selected);
  return <div className="qr-results">
    <div className="qr-image-scroll"><div className="qr-image-stage">
      <img src={scan.imageUrl} alt={t("QR Source Image")} draggable={false}/>
      <div className="qr-targets" role="group" aria-label={t("Choose a QR code in the image")}>
        {scan.codes.map(code => {
          const center = qrCodeCenter(code.corners);
          return <button type="button" key={code.index} className="kiri-qr-marker" style={{ left: `${center[0] * 100}%`, top: `${center[1] * 100}%` }} aria-pressed={selected === code.index} aria-label={fmt("QR code %d", code.index + 1)} onClick={() => setSelected(code.index)}>
            <ChevronRight size={18} strokeWidth={2.5} aria-hidden="true"/>
          </button>;
        })}
      </div>
    </div></div>
    {code ? <QrDetails key={code.index} code={code} saved={!!code.text && favorites.has(code.text)} action={async action => {
      const result = await api.qrAction(scan.requestId, code.index, action);
      if (code.text && (action === "favorite" || action === "unfavorite")) setFavorites(previous => {
        const next = new Set(previous);
        if (action === "favorite") next.add(code.text!); else next.delete(code.text!);
        return next;
      });
      return result;
    }}/> :
      <div className="qr-prompt" role="status"><QrCode size={28}/><p>{t(scan.codes.length ? "Choose a QR code in the image" : "No QR codes found. Try a clearer image or a larger selection.")}</p></div>}
  </div>;
}

export function QrModal({ scan, failed, onClose, onRetry }: { scan: QrScanDto | null; failed?: boolean; onClose(): void; onRetry?(): void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { const element = dialog.current; element?.showModal(); return () => element?.close(); }, []);
  return <dialog ref={dialog} className="text-dialog qr-dialog" aria-labelledby="qr-dialog-title" onPointerDown={event => event.stopPropagation()} onContextMenu={event => event.stopPropagation()} onKeyDown={event => event.stopPropagation()} onCancel={event => { event.preventDefault(); onClose(); }}>
    <header className="text-dialog__header"><div><h2 id="qr-dialog-title">{t("QR Codes")}</h2><p>{t("Recognized locally. Choose what to copy, open, or save.")}</p></div><button type="button" className="kiri-icon-button" aria-label={t("Close")} onClick={onClose}><X size={16}/></button></header>
    {scan ? <QrResults key={scan.requestId} scan={scan}/> : <div className="text-history__empty" role={failed ? "alert" : "status"}><QrCode size={28}/><p>{t(failed ? "QR recognition failed." : "Recognizing QR Codes…")}</p></div>}
    {(failed || scan?.codes.length === 0) && onRetry && <button type="button" className="kiri-button kiri-button--secondary" onClick={onRetry}>{t("Choose Another Region")}</button>}
  </dialog>;
}

export function QrAssetDialog({ asset, onClose }: { asset: AssetDto; onClose(): void }) {
  const [scan, setScan] = useState<QrScanDto | null>(null);
  const [failed, setFailed] = useState(false);
  const request = useRef<{ id: string; promise: Promise<QrScanDto> } | null>(null);
  useEffect(() => {
    let current = true;
    if (!request.current) { const id = crypto.randomUUID(); request.current = { id, promise: api.scanQr(id, null, asset.id) }; }
    void request.current.promise.then(scan => { if (current) setScan(scan); }, () => { if (current) setFailed(true); });
    // StrictMode replays setup. Cancel belongs to the actual close action.
    return () => { current = false; };
  }, [asset.id]);
  return <QrModal scan={scan} failed={failed} onClose={() => { if (request.current) void api.cancelQr(request.current.id); onClose(); }}/>;
}

export function QrFavorites() {
  const [query, setQuery] = useState("");
  const [records, setRecords] = useState<AssetDto[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  useEffect(() => { const subscription = onLibraryChanged(() => setRevision(n => n + 1)); return () => { void subscription.then(stop => stop()).catch(() => {}); }; }, []);
  useEffect(() => {
    let current = true; setError(false); setLoading(true);
    const timer = setTimeout(() => { void api.listQrFavorites(query).then(items => {
      if (!current) return;
      setRecords(items); setSelected(id => items.some(a => a.id === id) ? id : items[0]?.id ?? null); setLoading(false);
    }, () => { if (current) { setError(true); setLoading(false); setRecords([]); } }); }, query ? 150 : 0);
    return () => { current = false; clearTimeout(timer); };
  }, [query, revision]);
  const asset = records.find(a => a.id === selected);
  return <div className="text-history">
    <aside className="text-history__sidebar"><label className="text-history__search"><QrCode size={15}/><input type="search" aria-label={t("Search QR Favorites")} placeholder={t("Search QR Favorites")} value={query} onChange={event => setQuery(event.target.value)}/></label>
      <div className="text-history__list">{records.map(asset => <button type="button" className="ocr-history__item" aria-current={asset.id === selected || undefined} key={asset.id} onClick={() => setSelected(asset.id)}><span className="text-history__excerpt">{asset.qrText}</span><time>{new Date(asset.createdAt).toLocaleString()}</time></button>)}</div>
      <p className="text-history__footnote">{t("QR images and content stay in your local library.")}</p>
    </aside>
    <main className="text-history__detail">{asset ? <><img className="qr-favorite-image" src={mediaUrl(asset.id)} alt={t("QR Source Image")}/><QrDetails key={asset.id} code={favoriteCode(asset.qrText ?? "")} saved removable action={action => api.qrFavoriteAction(asset.id, action)}/></> : <div className="text-history__empty" role={error ? "alert" : "status"}><QrCode size={28}/><p>{t(error ? "Could not load QR Favorites." : loading ? "Loading…" : query ? "No QR favorites match this search." : "Save a QR code to find and reuse it here.")}</p></div>}</main>
  </div>;
}

function favoriteCode(text: string): QrCodeDto {
  // The backend repeats URL validation before every explicit open action.
  let url: URL | null = null;
  try { if (/^https?:\/\//i.test(text) && !/[\s\\\u0000-\u001f\u007f]/.test(text)) { const parsed = new URL(text); if (!parsed.username && !parsed.password) url = parsed; } } catch { /* Plain content. */ }
  return { index: 0, corners: [], text, url: url?.href ?? null, host: url?.hostname ?? null, suspicious: url ? url.protocol === "http:" || /xn--|localhost|^\[|^\d+\.\d+\.\d+\.\d+$/.test(url.hostname) : text.includes(":") || text.includes("\u202e") };
}
