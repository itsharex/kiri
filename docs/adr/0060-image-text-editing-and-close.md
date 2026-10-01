# Image text editing and unsaved close / 图片文字编辑与关闭保护

Status: Accepted — 2026-09-30. Refs #54.

While an annotation textarea is focused, Ctrl/Cmd+Z and redo belong to its
native text history. Canvas history applies after leaving the text editor.
Escape cancels only the current text edit, preserving committed marks; a
second Escape cancels the capture. IME composition owns its Enter/Escape.
Shift+Enter adds a line, Enter commits (and still completes a capture).
Show a localized hint next to the textarea.

Closing the saved-image editor via Cancel, Escape, Cmd/Ctrl+W or the native
window close warns only when marks, a crop or pending text differ from the
saved baseline. Offer Save & close, Discard, and Keep editing. Failed saves
retain the window and show the error. Undoing to the baseline removes the
warning; existing saved marks and empty new text are not dirty. Save As exports
a copy without changing the library baseline, so edits still require a close
decision after export. Only Save updates that baseline. Export keeps its immutable interaction lock.
Video already owns autosave and close protection; preserve that contract.

文字输入框获得焦点时，撤销与重做使用原生文字历史；退出输入后再操作画布历史。
Esc 先取消本次文字编辑，保留已提交标注；再按 Esc 取消捕获。输入法组合态自行处理
Enter/Esc。Shift+Enter 换行，Enter 提交，截图仍按现有约定完成捕获；输入框附近显示三语提示。

已保存图片的编辑器关闭时，仅在标注、裁剪或尚未提交文字相对保存基线有变化时询问。
提供保存并关闭、放弃及继续编辑；保存失败保持窗口并显示错误。撤销回基线无需提示，
原有标注与新空输入框不算修改；另存仅导出副本，素材库的修改仍需关闭确认，仅保存更新基线。
视频现有自动保存与关闭保护保持不变。

## Native close completion — 2026-10-01

Tauri's installed `onCloseRequested` wrapper calls window destroy after the
handler allows closing. The image-close capability grants only
`core:window:allow-destroy` to `editor-*` callers, matching the separate existing
video-close capability. Default and other window permissions remain unchanged.
This is a caller boundary, not a target-label scope; the editor uses the SDK
current window only. Keep the same dirty/save-failure guard. SDK/IPC tests do
not establish native ACL acceptance; replay clean close, Keep editing, Discard
and Save & close on the exact packaged application.

用户已确认图片编辑窗口权限修复；新增独立 image-close 能力，仅 editor-* 调用窗口
获得 destroy，不加入默认能力。这是调用窗口边界，不能误称为目标 label 限制；
应用关闭路径只调用当前窗口。未保存与保存失败保护保持不变，exact 包仍须原生实测。

## Composition lifecycle guards — 2026-10-01

The inline textarea tracks composition start/end and clears its marker on blur.
The key guard combines that marker with isComposing and keyCode 229, so a
confirming key whose WebView flags are false is still owned by the IME while
composition is active. Screenshot capture-phase Escape, saved-image shortcuts
and video project capture-phase save/close use the same guard. Native text
undo remains the browser editor's history; this does not add a replacement text history.

Pure key-dispatch tests establish event ownership, not real input-method or
WebKit undo acceptance. Replay Chinese/Japanese composition and native Undo on
the exact package before claiming those paths passed.

## Native textarea value ownership — 2026-10-01

Cloud X11 acceptance of source25870a5 confirmed that Ctrl+Z left typed
`alpha betax` unchanged, while the same key injection removed `x` in Mousepad.
Allowing the default key action alone did not establish working text Undo.
React's controlled textarea also synchronizes its `defaultValue` on each
input. WebKit's textarea setter changes light-DOM text, and its
`childrenChanged` path marks the change as non-user editing. Avoid those
unnecessary DOM mutations while a native text edit is active.

WebKitGTK's GTK3 `KeyBindingTranslator` forwards GTK text-view bindings and
lists copy/cut/paste but no Undo/Redo command. Leaving Ctrl+Z's default alone
is insufficient in a standalone WebView. The focused textarea now asks its
own document to execute native `undo`/`redo`; a successful command prevents
the key default to avoid two edits on other platforms. If the command is
unavailable or the history is empty, the platform default remains available.
Composition keys never enter this path. This uses browser editing commands,
without IPC, new permissions, or an application-managed text history.

Primary source: [WebKit GTK key translation](https://github.com/WebKit/WebKit/blob/81c183eff1e72ae9aab6c55e6460996af1d8a3b6/Source/WebKit/UIProcess/gtk/KeyBindingTranslator.cpp).
The controlled-DOM mutation was measured (7 writes while typing `test123`,
versus 0 after); whether it alone invalidated history on the cloud GTK runtime
has not been established. Native package regression covers the combined fix.

Initialize the live textarea value once on attachment, without a controlled
value/defaultValue prop. Observe input into the existing draft model; leave
the browser's own text history and selection intact. Key the editor by mark
ID so switching annotations initializes an independent text editor. Project
revision reloads already end the edit. This does not introduce a custom text
history, change IME ownership, or change Return/Shift+Return/Escape behavior.

The component regression observes textarea light-DOM mutations during typing,
Undo/Redo, multiline edits and close protection. The installed X11 test also
reads the real focused WebKitGTK textarea through accessibility, injects keys,
and verifies Undo changes text, Redo restores it, and first Escape preserves
previous canvas history. Browser dispatch/DOM tests are not native acceptance;
the new exact package must pass this test and the cloud original scenario.

云X11已确认25870a5的文本Ctrl+Z无效，同键在Mousepad可撤销。输入框只在进入本次
编辑时初始化，后续不把React模型反写回原生DOM；仍监听输入更新草稿，不自制撤销栈。
聚焦输入框内Ctrl/Cmd+Z调用浏览器原生Undo/Redo，成功后阻止默认重复执行；不新增权限。
切换标注使用独立输入框，Enter完成截图、Shift+Enter换行、两级Esc与IME约定保留。
真实输入法及新包原生验收仍须分别记录，不能用浏览器结果替代。
