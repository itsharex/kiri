// Only the isolated harness config aliases this file into QrResults.
export type { AssetDto, QrCodeDto, QrScanDto } from "../../src/lib/ipc";
const base = "/src-tauri/tests/fixtures/qr/";
let favorites = [{id:"url",createdAt:0,qrText:"https://example.org/kiri-safe"},{id:"text",createdAt:0,qrText:"Kiri QR 测试"}];
const listeners = new Set<() => void>();
export const mediaUrl = (id: string) => base + id + ".png";
export const onLibraryChanged = async (callback: () => void) => { listeners.add(callback); return () => listeners.delete(callback); };
const log = (name: string) => { const output=document.querySelector("#qa-actions");if(output)output.textContent=name; };
export const api = {
  qrAction: async (_request: string, index: number, action: string) => { log(`Mock action: ${index} / ${action}`);return action === "favorite" ? {id:"fixture-favorite"} : null; },
  listQrFavorites: async (query: string) => favorites.filter(f => f.qrText.toLowerCase().includes(query.toLowerCase())),
  qrFavoriteAction: async (id: string, action: string) => { log(`Mock action: ${id} / ${action}`); if(action==="remove"){favorites=favorites.filter(f=>f.id!==id);listeners.forEach(f=>f());} },
  scanQr: async () => { throw "Harness does not perform native scans."; },
  cancelQr: async () => {},
};
