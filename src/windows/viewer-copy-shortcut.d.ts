interface CaptureCopyKey {
  key: string;
  defaultPrevented?: boolean;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
  isComposing?: boolean;
  keyCode?: number;
  target?: EventTarget | null;
  nativeEvent?: { isComposing?: boolean; keyCode?: number; target?: EventTarget | null };
}

export function canCopyCaptureOnKeyDown(event: CaptureCopyKey, actions: {
  canCopy(): boolean;
  hasTextSelection(): boolean;
}): boolean;

export function installViewerCopyShortcut(target: Window, actions: {
  canCopy(): boolean;
  hasTextSelection(): boolean;
  copy(): void;
}): () => void;
