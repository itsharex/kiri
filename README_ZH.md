<div align="center">
  <img src="src-tauri/icons/128x128.png" width="112" alt="Kiri 应用图标">
  <h1>Kiri</h1>
  <p>截图、文字识别和录屏，素材保存在本机。</p>
  <p>
    <a href="https://kiri.yuxino.cn">官网</a>
    · <strong>简体中文</strong>
    · <a href="README.md">English</a>
  </p>
</div>

Kiri 支持 macOS、Windows 和 Linux。按 `⇧⌘A`（macOS）或 `Shift+Ctrl+A`（Windows / Linux X11），选择窗口或区域，即可截图、标注、识别文字或录屏。Linux Wayland 请使用「截图」按钮，或在桌面设置中将快捷键绑定到 `kiri --capture`。截图会复制到剪贴板；截图、MP4 和 GIF 保存在本地素材库。

<!-- project-demo-v1 -->
<h2 align="center">演示</h2>

https://github.com/user-attachments/assets/13742f07-1845-4201-9295-39f83515547f

<p align="center">查看截图标注、文字识别、录屏和素材库的操作过程。4K 演示，配有中文旁白和字幕。</p>
<p align="center"><a href="https://kiri.yuxino.cn/#demo">Watch in English</a> · <a href="https://kiri.yuxino.cn/zh/#demo">观看中文版</a> · <a href="docs/demos/full-tour-4k.json">录制说明</a></p>
<!-- /project-demo-v1 -->

## 功能

- **截图**：选择窗口或区域后，可以裁剪、绘图、添加文字或打马赛克。截图使用鼠标所在的显示器，也支持 macOS 上其他应用的原生全屏空间。截图复制到剪贴板，同时存入本地素材库；新建截图的标注可以重新打开修改。
- **文字识别**：提取屏幕或已有截图中的文字，默认在本机识别；使用可选的远程 OCR 时，每次上传前都会确认。识别结果保存在「文字」中，可以搜索、复制和回看原图。
- **录屏**：将指定区域录成 MP4 或无声 GIF，可选择系统声音、麦克风、指针和点击高亮。开始前点击倒计时圆环或按 Esc 可取消。麦克风检测会显示输入设备和实时音量，持续五秒，不保存测试音频。
- **视频剪辑（macOS 和 Windows）**：裁剪、重排同一视频中的片段，调整片段速度，添加定时标注、图片贴图或隐私遮挡。剪辑自动保存在本机，重新打开原片即可继续；导出另存为 MP4，在最后保存前可以取消。操作、画质设置和平台限制见[视频剪辑指南](docs/video-editing.zh-CN.md)。
- **导入素材**：本地 PNG、JPEG、WebP、MP4 和 MOV 文件也可以加入素材库，原文件不受影响。每次最多导入 32 个文件；图片上限为 32 MB、每边 8192 像素，并受解码内存限制，视频上限为 8 GB。
- **本地素材库**：按名称或标签查找素材，添加收藏、重命名或从回收站恢复。可在设置中将素材库迁移到其他本机目录或外接盘。

每个视频工程只编辑一个源文件，不能把导入的多个视频合成一条时间轴。Windows 片段变速会同时改变音调，画面效果要求原视频不含旋转标记；Kiri 录屏符合此条件。

Linux 主要面向 Ubuntu 24.04 / GNOME，并兼容 X11。Wayland 捕获目前要求只连接一台显示器，使用区域框选。Linux 录屏暂不支持声音或点击高亮，通过托盘或系统快捷键暂停、停止。本地 OCR 使用已安装的 Tesseract 英文、简体中文和日文数据；视频支持播放与 GIF 转换，暂不开放视频剪辑和导出。依赖、授权流程和平台说明见 [Linux 指南](docs/linux.md)。

## 下载与安装

本 README 说明当前源码中的功能；已发布的安装包、各版本包含的平台和功能，请查看 [GitHub Releases](https://github.com/yuxino/kiri/releases) 及相应更新记录。

从 v1.4.9 起，macOS 与 Windows 安装版的设置页支持手动检查、下载并安装经过签名验证的更新；每一步都需要你明确点击，Kiri 不会后台检查或静默安装。v1.4.8 及更早版本需要先从 Releases 手动安装一次 v1.4.9 或更新版本，之后才能使用应用内更新。

日常更新请用**设置 → 关于 → 检查更新**。下载时显示已下载大小和可用的百分比，随后验证签名。Windows 点击「安装并重启」后，Kiri 会短暂关闭，更新完成自动重新打开，无需手动卸载现有 NSIS 版本；macOS 安装后再点击重启。手动运行下载的安装包则可能出现卸载／重装选项。

- **macOS 14+**：下载 Universal `.dmg`（Apple 芯片与 Intel），把 `Kiri.app` 拖入“应用程序”。截图与录屏需要“屏幕与系统音频录制”权限；点击高亮才需要“输入监控”。麦克风录制需要 macOS 15+。
- **Windows 11（x64）**：提供 x64 安装包，完整捕获流程的真机验收进度见[路线图](ROADMAP.md)。运行 `.exe` 安装程序；屏幕捕获不需要额外系统授权，麦克风权限由 Windows 隐私设置控制。安装程序未经过 Authenticode 签名，SmartScreen 可能提示警告。
- **Linux（Ubuntu 24.04 x64）**：使用 Release 中的 `.deb` 附件；对于尚未发布的源码版本，可从对应的成功 [构建](https://github.com/yuxino/kiri/actions/workflows/build.yml) 下载 `kiri-linux-deb` artifact。若下载的是 artifact ZIP，先解压，再执行 `sudo apt install ./kiri_VERSION_amd64.deb`（将 `VERSION` 换为实际版本）。Linux 更新需要手动下载并安装新版软件包。GNOME portal 配置、Wayland 快捷键和平台说明见 [Linux 指南](docs/linux.md)。

Windows 免安装使用：下载 `Kiri-<version>-Windows-x64-Portable.zip`，解压后直接运行 `kiri.exe`。此版本无需安装，但素材库和设置仍保存在 Windows 用户目录中；移动 ZIP 不会带走这些数据。绿色版的更新入口会打开 Releases，请手动下载新版 ZIP；不会运行 NSIS 安装更新。

Windows 安装器跟随系统语言，支持简体中文、英文和日文，安装、更新与卸载提示均已翻译；应用内语言可在设置中单独选择。Kiri 的半身立绘与公共安装样式统一在 [desktop-installer](https://github.com/yuxino/desktop-installer) 维护。

macOS 发布包使用项目维护的本地自签名身份，未使用 Developer ID 签名或 Apple 公证。首次启动若被拦截，请按住 Control 点按 `Kiri.app` 并选择“打开”，或在“系统设置 → 隐私与安全性”中选择“仍要打开”。

## 隐私

素材、OCR 和编码默认都在本机处理。远程 OCR 完全可选，API Key 保存在 macOS 钥匙串、Windows 凭据管理器或 Linux Secret Service 中，每次请求都需要明确点击“发送”或“重试”。

可重编辑截图会在本地保存未加标注的源图；其中可能仍有被马赛克或图形遮住的像素。保存裁剪会同时移除框外像素。macOS 的 MP4 录屏、合并、缩略图和 GIF 生成使用 AVFoundation 与 ImageIO；Windows 使用 Media Foundation 与系统图像组件；Linux 使用系统 GStreamer 库和插件。所有平台都不下载 FFmpeg，媒体处理始终在本机完成。

## 从源码运行

需要 Rust 1.88+、Node.js 20.19+（或 22.12+）和 pnpm。macOS 需要 Xcode Command Line Tools；Windows 需要 MSVC C++ 构建工具；Linux 构建前请安装 [Ubuntu 系统依赖](docs/linux.md#build-from-source)。

```bash
git clone https://github.com/yuxino/kiri.git
cd kiri
pnpm install
pnpm tauri dev
pnpm tauri build --no-bundle
```

macOS 开发版还需要稳定的签名身份。请通过 Tauri 命令运行或构建；普通 `cargo build` 生成的二进制不包含前端资源。

## 快捷键

macOS、Windows 与 Linux X11 可在**设置 → 通用**中修改或恢复截图快捷键，使用 Control、Alt 或 Command 搭配字母或数字。若新组合被其他应用占用，Kiri 会保留原快捷键。Wayland 请在桌面设置中绑定 `kiri --capture`，Kiri 不会修改合成器的快捷键。下列原生截图快捷键为默认值。

- **⇧⌘A**（macOS）/ **Shift+Ctrl+A**（Windows / Linux X11）：开始捕获
- **Esc**：取消截图或倒计时；macOS 和 Windows 录屏时停止录制
- **Return**：确认截图
- **C**：在截图编辑器中裁剪
- **⌘F**（macOS）/ **Ctrl+F**（Windows / Linux）：搜索素材库
- **⌘Z / ⇧⌘Z**（macOS）/ **Ctrl+Z / Shift+Ctrl+Z**（Windows / Linux）：撤销 / 重做

Linux 录屏可通过 `kiri --toggle-recording-pause` 暂停或继续，通过 `kiri --stop-recording` 停止；也可以将这两个命令绑定到系统快捷键。

## 致谢

感谢 [@kerwin2046](https://github.com/kerwin2046) 在 [PR #20](https://github.com/yuxino/kiri/pull/20) 中提供 Linux 初始支持，也感谢他为这些代码投入的时间与心力。

另见 [隐私说明](PRIVACY_ZH.md)、[路线图](ROADMAP.md)、[贡献指南](CONTRIBUTING.md)、[安全策略](SECURITY.md) 与[文档索引](docs/README.md)。

[MIT](LICENSE) © 2026 yuxino
