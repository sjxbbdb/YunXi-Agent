# YunXi Agent v1.9.1 System32 Session Path Hotfix 报告

时间：2026-07-17 20:57:45 +08:00

## 问题

用户从 Windows PowerShell 的 `C:\Windows\System32` 启动已安装的 `yunxi`，交互输入后运行失败：

```text
failed to create session directory .\.yunxi\sessions: 拒绝访问。 (os error 5)
```

CLI 原先把未显式提供的 `--cwd` 默认解析为 `.`，`FileSessionStore` 随后固定使用
`<cwd>\.yunxi\sessions`。因此，从普通用户无写权限的受保护目录启动时，会话在模型或静态
provider 返回回复前就保存失败。

## 修复

- 将 CLI 的 `cwd` 参数从隐式的 `PathBuf(".")` 改为 `Option<PathBuf>`，区分用户显式指定和默认启动。
- 新增 `workspace::resolve_cli_cwd`：
  - 显式 `--cwd` 原样保留，不做静默回退；
  - 未显式指定时，先使用当前工作目录并预检 `.yunxi\sessions`；
  - 仅在系统返回 `PermissionDenied` 时回退到 `%USERPROFILE%`，其次为 `$HOME`；
  - 非权限类错误保持可见，避免掩盖损坏目录、路径冲突等真实问题；
  - 用户目录回退不可用或不可写时返回带有两个路径的明确错误。

修改路径：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\workspace.rs`
- `D:\YunXi Agent\docs\reports\2026-07-17-205745-yunxi-agent-v1-9-1-system32-session-path-hotfix-report.md`

## 验证

- 新增 4 个单元回归测试：正常当前目录、权限拒绝回退、非权限错误不隐藏、缺少用户目录回退。
- `cargo fmt --check` 通过。
- `cargo test` 全仓通过。
- Debug 实际进程从 `C:\Windows\System32` 启动：
  - 状态为 `completed`；
  - 返回 `YunXi autonomous runtime accepted prompt: 晚上好`；
  - 用户目录下生成 1 个会话文件；
  - 未创建 `C:\Windows\System32\.yunxi`。
- Release 安装后再次从 `C:\Windows\System32` 冒烟：结果同上。
- 安装位置：`D:\Apps\YunXi Agent\bin`。
- `yunxi.exe` SHA-256：`11E2D0E644E0150A624B6678C6F851C323585C2F6B91EB8838132B92340F2E29`。
- `yunxi-agent-cli.exe` SHA-256：`977B7B976E6BF4458DDF7198D85D7765B866A3A8EA17A722E4263A03AA5A764B`。

## 发布约束

- 热修复继续报告产品版本 `1.9.1`，不占用后续功能版本号。
- 使用新的 annotated `v1.9.1-hotfix.1` tag 发布。
- 原 annotated `v1.9.1` 及更早 tag 不删除、不移动、不重写。
- 发布 ref 更新必须为 non-force。

开发者
