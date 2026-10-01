export function initialQrSelection(codes) {
  return codes.length === 1 ? codes[0].index : null;
}

export function qrPolygon(corners, width, height) {
  return corners.map(([x, y]) => `${x * width},${y * height}`).join(" ");
}
