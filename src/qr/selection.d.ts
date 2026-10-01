export function initialQrSelection(codes: { index: number }[]): number | null;
export function qrCodeCenter(corners: [number, number][]): [number, number];
export function qrContentType(code: { host: string | null; text: string | null; url: string | null }): "Link" | "Text" | "WeChat";
export function qrLooksLikeLink(text: string): boolean;
