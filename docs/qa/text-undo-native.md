# 标注文字 Undo：原生证据与原包复验

应用修复 d73339e 保留 textarea 的浏览器输入历史，并显式执行原生
undo/redo，成功后阻止重复默认动作。组合输入仍由 IME 处理；未增加权限。
Enter 提交文字并完成截图、Shift+Enter 换行、两级 Esc 是 ADR0060 既定约定。

25870a5 云 Debian13/Xfce/X11 的两条键注入路径确认 Ctrl+Z 无效；相同注入在
Mousepad 可撤销。新包实际 source 04774860，deb SHA256
`2178811ed8ad3ec42a0200c8f15fae58088235f67b3291dcc9663c9facf35c22`，
artifact11156722519 / build run36853144607。独立云原生窄验已经确认 abc 与
alpha beta 两组 Ctrl+Z 改变文字、Ctrl+Shift+Z 恢复，且输入框保留。
这不是实体 mixed DPI 或正式 Ubuntu/GNOME 硬件验收；真实 IME 仍待验。

该运行的 Ubuntu X11 测试在输入文字前因 GI 同名方法调用错误失败：
`Atspi.Accessible.get_text() takes exactly 1 argument (3 given)`。
2efd1a6 改用显式 `Atspi.Text.get_text`。没有修改产品或放宽任何断言。
原运行保持 failure，同包 Wayland 四档通过不能替代 X11。

用 build.yml 的 `profile=recheck-linux-x11` 和原 `linux_candidate_run_id`
仅复验安装后的 X11。不会重新编译、生成 deb、运行其他平台或重复 Wayland。
复验必须核对原包 artifact/manifest/hash、repo/run/source，以及应用与 build
源码相同。只接受原 Linux job 的唯一失败步骤是安装后 X11 验收，且 cargo
check/test、打包和安装成功；默认成功候选复用门槛不变。原失败被写入 provenance。
新的 run/report 分别标注原包 source 和 QA harness source，不改写原运行结论。

复验完整执行原截图/OCR/剪贴板/录制全帧/暂停时间/持久化断言，并增加聚焦输入框
真实键注入的 Undo/Redo 与两级 Esc/既有矩形历史。只读获取文本必须调用正确的
AT-SPI Text 接口；测试没有通过时不能拿 DOM dispatch 或旧包通过替代。
