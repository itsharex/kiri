import { isTextComposition } from "../annotation/text-composition.js";

/** Keep text selections, native editors and IME keys with the browser. */
export function canCopyCaptureOnKeyDown(event, actions) {
  return !event.defaultPrevented && !isTextComposition(event) &&
    (event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey &&
    event.key.toLowerCase() === "c" &&
    !event.target?.closest?.("input,textarea,select,[contenteditable],[role='textbox'],[role='dialog']") &&
    !actions.hasTextSelection() && actions.canCopy();
}

/** Copy the capture only when the browser has no text-copy target. */
export function installViewerCopyShortcut(target, actions) {
  const onKeyDown = (event) => {
    if (!canCopyCaptureOnKeyDown(event, actions)) return;
    event.preventDefault();
    if (!event.repeat) actions.copy();
  };
  target.addEventListener("keydown", onKeyDown);
  return () => target.removeEventListener("keydown", onKeyDown);
}
