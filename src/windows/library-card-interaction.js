export function getLibraryCardInteraction({
  selectionActive,
  selected,
  menuOpen,
  editingTitle,
  highlighted,
}) {
  return {
    opensOnClick: !selectionActive && !menuOpen && !editingTitle,
    showsActions: highlighted || selected || menuOpen,
  };
}

export function getLibraryCardPrimaryAction(kind) {
  return kind === "image"
    ? { icon: "pencil.tip", title: "Edit", opensEditor: true }
    : { icon: "eye", title: "View", opensEditor: false };
}

export function getMenuFocusIndex(key, current, itemCount) {
  if (itemCount <= 0) return -1;
  if (key === "Home") return 0;
  if (key === "End") return itemCount - 1;
  if (key === "ArrowDown") return (current + 1 + itemCount) % itemCount;
  if (key === "ArrowUp") return current < 0 ? itemCount - 1 : (current - 1 + itemCount) % itemCount;
  return current;
}

export function getLibraryMenuPosition({ x, y, width, height, viewportWidth, viewportHeight }) {
  const pad = 10;
  const gap = 4;
  const maxLeft = Math.max(pad, viewportWidth - width - pad);
  const maxTop = Math.max(pad, viewportHeight - height - pad);
  const left = Math.min(Math.max(x + width > viewportWidth - pad ? x - width : x, pad), maxLeft);
  const below = y + gap;
  const above = y - height - gap;
  const preferredTop = below + height <= viewportHeight - pad || above < pad ? below : above;
  return { left, top: Math.min(Math.max(preferredTop, pad), maxTop) };
}

export function getLibraryContentPoint({
  clientX,
  clientY,
  rectLeft,
  rectTop,
  clientLeft,
  clientTop,
  scrollLeft,
  scrollTop,
}) {
  return {
    x: clientX - rectLeft - clientLeft + scrollLeft,
    y: clientY - rectTop - clientTop + scrollTop,
  };
}

export function getLibraryBandRect({ x0, y0, x1, y1 }) {
  return {
    x: Math.min(x0, x1),
    y: Math.min(y0, y1),
    w: Math.abs(x1 - x0),
    h: Math.abs(y1 - y0),
  };
}

export function getAvailableShortcutLabel(shortcutStatus) {
  return shortcutStatus?.status === "enabled" && shortcutStatus.label
    ? shortcutStatus.label
    : null;
}
