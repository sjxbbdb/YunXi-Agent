# YunXi Agent v2.3.3-hotfix.10 Web 陪伴信箱安装日志

时间：2026-08-04 22:28:53 +08:00

## 安装结果

- release 来源：`D:\YunXi Agent\target\release`
- 安装目录：`D:\Apps\YunXi Agent\bin`
- PATH 解析目标：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 安装前版本：`2.3.3-hotfix.9`
- 安装后版本：`2.3.3-hotfix.10`
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260804-222641-web-companion-mailbox`

## 文件校验

- `yunxi.exe`
  - SHA-256：`9A86662B40D41AB1E976F19B653AAADD4AAEF8E4E77A298C69D4FCB0E01D05DA`
- `yunxi-agent-cli.exe`
  - SHA-256：`1A1346378642F3183D75FEFAA58A915EEA7686A8A8A8133C1DE43804715F164F`

两份安装文件的 SHA-256 均与对应 release 文件一致。

## 实机验证

1. `yunxi --version`：返回 `yunxi 2.3.3-hotfix.10`。
2. `yunxi-agent-cli.exe --version`：返回 `yunxi 2.3.3-hotfix.10`。
3. 使用安装目录中的 `yunxi.exe` 在 `127.0.0.1:17862` 启动临时 Web 验证服务。
4. `/api/health` 返回 `{"status":"ok","version":"2.3.3-hotfix.10"}`。
5. 验证完成后，只停止经路径核验的临时安装验证进程；现有 `127.0.0.1:17861` Web 服务保持运行。

## 安全边界

安装过程只复制两个明确的二进制文件。覆盖前已复制旧版本到新的时间戳备份目录并逐文件校验。未执行删除、递归清理、目录移动、force push、历史 tag 删除或用户目录修改。

署名：开发者
