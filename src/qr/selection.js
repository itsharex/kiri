export function initialQrSelection() {
  return null;
}

export function qrCodeCenter(corners) {
  // Diagonals meet at the projected center even when a code is in perspective.
  const [a, b, c, d] = corners;
  const r = [c[0] - a[0], c[1] - a[1]];
  const s = [d[0] - b[0], d[1] - b[1]];
  const denominator = r[0] * s[1] - r[1] * s[0];
  if (Math.abs(denominator) > Number.EPSILON) {
    const t = ((b[0] - a[0]) * s[1] - (b[1] - a[1]) * s[0]) / denominator;
    return [a[0] + t * r[0], a[1] + t * r[1]];
  }
  return [corners.reduce((sum, p) => sum + p[0], 0) / 4, corners.reduce((sum, p) => sum + p[1], 0) / 4];
}
