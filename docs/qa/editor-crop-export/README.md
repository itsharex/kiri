# 待保存裁剪与导出保护 / Pending crop and export protection

修复 Save As 仅导出副本却清除素材库 dirty 的前端错误；保留待保存裁剪时允许
切换标注工具，回到 Crop 保留裁剪撤销／重做，取消裁剪保留标注。

## 原图条件与版本

四张 PNG 都是未编辑的实际组件截图：Mac arm64 上的 headless Chromium，
800×600、backing scale 1、英文编辑器、同一公开网格图。素材库 fixture 初始为中文。
这不是 Kiri 原生 GUI、实体显示器或原生分数缩放验收。

- Before：从 #71 CI 36741052118 下载的生产前端，source
  `6cbb8469e27e316d821f443f11a632934a562371`，含旧 head `b7ea616`。
- After：本机生产前端，renderer harness/source
  `bd25381ad2ab9a563f5c4a4f624175782c5ca481`，应用修改来自 `bea5f80`、`48a7182`。
  之后本目录和说明提交不改变应用代码。
- 文档画布 500×300，源图 1000×600，裁剪文档坐标 `(50,40,300,180)` 对应源像素
  `(100,80,600,360)`。矩形标注原坐标 `(80,60,80,50)`，导出平移后为 `(30,20,80,50)`。
- Crop before：进入裁剪后其他工具禁用；after：同框保留并可绘制矩形。
- Save As before/after：先绘制同一矩形、创建同一裁剪框、成功另存，再点 Cancel。
  Before 错误地关闭；after 显示未保存素材库修改的决策。
- 原图 SHA256 在 `image-sha256.json`；公开前已逐图查看，无用户内容。

## 回归与限制

`test_image_crop.py` 使用真实生产组件，隔离 native IPC。24 组：三语 ×
800×600/1280×720 × 1/1.25/1.5/2 backing scales；检查裁剪与标注各自 Undo/Redo、
工具切换、取消导出、Cancel crop/Escape 保留标注、源像素映射、最终 Save PNG 600×360。
后续键盘补验让聚焦工具、Save As、Cancel crop 的 Enter 执行控件自身动作，不触发全局 Save。
导出整数尺寸和 cropPixels 精确断言；浮点标注坐标容差 0.0001 文档单位。

更新后的文字／导出 fixture 20 条通过，包括导出后继续编辑、再次导出仍有关闭保护、
撤销回原始素材库基线。**撤回旧“Save As 更新保存基线”的断言**：后端始终只导出副本。
组合 OCR 6 条和工具栏 63＋36 条也通过；对应 JSON 随图归档。

原生 QA 此前确认 `plugin:window|destroy not allowed by ACL`：图片可保存但窗口未关闭。
用户于 2026-10-01 明确确认权限修复；新增独立 image-close 能力，只授予 editor-*
调用窗口 destroy，不改变默认或其他窗口能力。SDK／IPC 回归覆盖 close-request
到 destroy 的实际调用顺序与确认阻止；不能作为原生 ACL 已通过的证据。
最终 exact 包的关闭、真实文本 Undo、实际 IME 待复验。旧 JSON 保留原版本边界。
