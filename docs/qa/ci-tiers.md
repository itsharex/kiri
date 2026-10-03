# CI 分层 / CI checks and requested acceptance packages

历史实测 run36815503794：墙钟17分07秒、约66 runner分钟；Windows17分04秒，
Mac x64 11分17秒，Linux8分23秒，Mac arm64 6分02秒，renderer5分18秒。
纯release-note文档main提交36666407353也曾触发14分22秒全量。
这不是新工作流的性能承诺，新流程尚待远程运行。

| 触发 | 必需检查 | 包／原生桌面验收 |
| --- | --- | --- |
| 普通main／PR，纯文档或证据 | Node、TS/Vite、CI策略／provenance测试、汇总门禁 | 无 |
| 前端／renderer harness修改 | 上述＋实际renderer回归 | 无 |
| 平台专用Rust修改 | 上述＋对应平台编译／测试，Mac保留两架构check | 无 |
| 共享Rust／Cargo／capability修改 | 上述＋三平台原生检查及Mac两架构check | 无 |
| 手动quick | renderer及所有原生检查 | 无 |
| 手动linux-package | 快速、renderer、Linux check/test | 一个deb，安装／依赖检查；不启动X11或GNOME桌面 |
| PR标签`ci:linux-package` | 保留普通PR的全部必需检查，追加Linux check/test | 一个deb，安装／依赖检查；不请求Portal权限 |
| 手动linux | 快速、renderer、Linux check/test | 一个deb、X11及同包Wayland四scale |
| 手动windows | 快速、renderer、Windows Rust/media | NSIS／portable与桌面控件、快捷键、安装烟测 |
| 手动macos | 快速、renderer、Mac测试 | 两架构release compile，不产生可运行／签名Mac包 |
| 手动full | 所有既有检查 | 完整既有验收矩阵，集成时仍可显式申请 |
| 正式v*标签／手动release | 版本（标签）、renderer及三平台原生检查 | 新deb安装／依赖检查、Windows签名包与原生验收、两架构Mac编译；不选择Linux设置型桌面验收 |
| 手动recheck-linux＋原run ID | 快速检查及严格source comparison | 复验原成功Linux包，Wayland四scale，不重建 |

PR按完整候选相对base的diff选择检查，避免同一PR的文档后续提交掩盖尚未验证的代码。
diff未知时执行所有原生检查及renderer，仍不意外打包。汇总门禁拒绝失败、取消、缺失、
错误plan输出及应执行任务的跳过。未修改GitHub branch protection配置；维护者可要求
稳定的`CI quality gate` context。

```bash
gh workflow run build.yml --ref qa/kiri-integrated-acceptance -f profile=linux-package
gh workflow run build.yml --ref qa/kiri-integrated-acceptance -f profile=linux
gh workflow run build.yml --ref qa/kiri-integrated-acceptance -f profile=full
gh workflow run build.yml --ref qa/kiri-integrated-acceptance \
  -f profile=recheck-linux -f linux_candidate_run_id=36815503794
```

The exact `ci:linux-package` label opts a PR into package build/install/inspection
on label changes and subsequent source pushes while present. Other PR labels can
also restart the normal checks because label events use the existing concurrency
group. Removing the package label returns the PR to ordinary source checks.
Package-only runs skip both X11 and GNOME Wayland acceptance; they do not launch
Kiri, change GNOME settings, or grant Portal access. Check the plan and job steps
before citing an artifact as desktop evidence. The package manifest records the
actual PR merge checkout, which can differ from the branch head.

Fresh manual packages require their own run/attempt manifest, like PR/tag packages.
Explicit reuse still checks original successful Linux build, repository, artifact,
source and exact deb SHA. Application/build changes still reject reuse; existing
run-ID-only dispatch remains compatible. Workflow/harness-only differences remain
recorded as before. No credential scope or workflow permission is expanded.

Release publication and trusted local Mac signing are unchanged. A compile is
not signed native Mac acceptance; virtual desktops are not physical mixed DPI.
The b8c35876 candidate already in cloud QA and its package identity stay unchanged.

Release tags use the `release` profile. It keeps all native checks and packages,
including signed Windows installer/portable desktop checks and fresh Linux package
build/install/provenance inspection, without running Linux X11/GNOME harnesses that
configure accessibility or screen-lock settings. Those desktop tests remain
available through the unchanged explicit `linux`, `full` and recheck profiles.
Skipped desktop checks are not passes. Carry candidate desktop evidence forward
only when production sources are unchanged apart from the release version;
record the accepted hardware/simulation limits and verify each new package.
