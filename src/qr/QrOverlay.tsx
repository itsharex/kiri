import { QrCode, X } from "lucide-react";
import type { Rect } from "../annotation/geom";
import type { QrScanDto } from "../lib/ipc";
import { t } from "../i18n";
import { QrResults } from "./QrResults";

/** Read the existing frozen selection without showing a second copy of it. */
export function QrOverlay({ scan, failed, selection, bounds, scale, onClose, onOpened }: {
  scan: QrScanDto | null;
  failed: boolean;
  selection: Rect;
  bounds: Rect;
  scale: number;
  onClose(): void;
  onOpened(): void;
}) {
  // Match the backend's outward pixel rounding before mapping normalized points
  // back to the frozen display's logical coordinate space.
  const sourceRect = {
    x: Math.floor(selection.x * scale) / scale,
    y: Math.floor(selection.y * scale) / scale,
    width: scan ? scan.width / scale : (Math.ceil((selection.x + selection.width) * scale) - Math.floor(selection.x * scale)) / scale,
    height: scan ? scan.height / scale : (Math.ceil((selection.y + selection.height) * scale) - Math.floor(selection.y * scale)) / scale,
  };
  const message = failed ? "QR recognition failed." : !scan ? "Recognizing QR Codes…" : scan.codes.length ? "Choose a QR code in the image" : "No QR codes found. Try a clearer image or a larger selection.";
  return <div className="qr-overlay" onPointerDown={event => event.stopPropagation()} onContextMenu={event => event.stopPropagation()}>
    {(!scan || scan.codes.length === 0) && <div className="qr-overlay-status" role={failed ? "alert" : "status"}>
      <QrCode size={16} aria-hidden="true"/>
      <span>{t(message)}</span>
      <button type="button" className="kiri-icon-button" aria-label={t("Close")} onClick={onClose}><X size={16}/></button>
    </div>}
    {scan && <QrResults key={scan.requestId} scan={scan} sourceRect={sourceRect} viewport={bounds} onOpened={onOpened} onClose={onClose}/>}
  </div>;
}
