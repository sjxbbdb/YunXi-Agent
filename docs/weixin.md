# YunXi Agent 微信接入边界

## v2.1.3-hotfix.1 能力

v2.1.3-hotfix.1 在 v2.1.3 QR 登录和系统凭证边界上补齐 CLI Mock 登录验收，并完成一次 Windows 真实扫码登录验证：

- yunxi weixin login --account <name> 获取二维码、显示安全终端文本、轮询等待/扫码/确认，并明确处理过期、取消、超时、redirect、验证码和验证码阻断。
- 登录确认后，token 和数据加密密钥只写入 Windows Credential Manager；系统凭证不可用、权限失败或写入失败时登录失败，不降级到明文文件。
- .yunxi/weixin/ 只保存脱敏账户哈希、官方 endpoint、连接状态、凭证引用、workspace 标识、schema version 和创建/更新时间。
- yunxi weixin status --json 与 doctor --json 读取脱敏登录元数据和凭证可用性；不会输出 token、二维码 payload、加密密钥或原始用户标识。
- yunxi weixin logout --confirm 只删除指定微信账户的系统凭证和微信元数据，不触碰 YunXi session、persona memory、工作区其他账户或 Git 状态。
- CLI 私有登录执行 helper 已覆盖 Mock 成功、过期、取消、凭证不可用、metadata 写失败回滚和输出脱敏；生产 CLI 不暴露 mock endpoint 或 fake store 参数。
- 2026-07-27 真实验证使用 `default` 账户完成扫码确认，`status --json` 返回 `state=ready`、`credential_state=present`、`credential_backend=windows-credential-manager`，`doctor --json` 返回 `credential_store=present`、`credentials_configured=true`，新进程重读仍为 ready。

二维码文本只在交互终端显示，且会先移除 ANSI 控制序列；不会写入普通日志、JSON/JSONL、错误链、Markdown 证据或账户 JSON。

## 当前不能做什么

本版本仍不能：

- 接收、发送或流式回复微信消息；
- 启动长轮询常驻服务或后台主动推送；
- 绑定 YunXi session、接入 Agent Runtime、执行远程审批或改变 cwd、Provider、模型、sandbox、approval mode；
- 实现配对准入、消息去重、会话绑定或群聊。首期设计仅允许私聊，群聊保持关闭。

weixin serve 仍只做 workspace/Provider 配置校验，不启动长轮询。pair approve|deny 保持明确未实现。真实微信联调和真实消息闭环属于后续版本，不能把二维码登录成功宣称为聊天能力已完成。

## 真实登录验证门禁

发布或复审材料只能记录脱敏状态，不得记录二维码 payload、token、原始用户 ID、数据密钥、原始响应 body 或系统凭证明文。真实验证需要在 Windows 上完成以下步骤：

1. 运行 `target\release\yunxi.exe weixin login --account <test-name>`，只在交互终端展示二维码。
2. 使用真实微信账号扫码并确认。
3. 核对 `.yunxi/weixin/` 只出现非机密 metadata。
4. 运行 `target\release\yunxi.exe weixin status --account <test-name> --json`，确认账户、workspace 和凭证状态均为脱敏字段。
5. 运行 `target\release\yunxi.exe weixin doctor --account <test-name> --json`，确认系统凭证引用可读且 `secrets_included=false`。
6. 重新打开 shell 或重新执行 release 二进制，再次读取 status/doctor，确认凭证引用仍可诊断。

本次 `v2.1.3-hotfix.1` 已完成上述真实扫码确认。后续复审材料仍只能记录脱敏状态，不得保存二维码、token、原始账号或系统凭证明文；复审通过前不得进入 `v2.1.4`。

## 命令

~~~text
yunxi weixin login --account <name>
yunxi weixin status [--account default] [--json]
yunxi weixin doctor [--account default] [--json]
yunxi weixin serve [--account default] [--workspace <path>]
yunxi weixin pair list|approve|deny ...
yunxi weixin logout [--account default] --confirm
~~~

Provider、模型、sandbox、approval 和根 --cwd 继续由现有 YunXi CLI 配置路径解析。登录只使用固定官方 endpoint https://ilinkai.weixin.qq.com/；CLI 不接受任意 base URL。

## 安全凭证与元数据

- WeixinSecretStore 是窄化 trait；生产实现为 Windows Credential Manager，fake store 只用于测试。
- token 与数据加密密钥不进入 .yunxi/weixin/*.json；JSON 中只有哈希账户、凭证引用和非机密状态。
- Debug、Display、错误、诊断 snapshot、JSON/JSONL 和项目日志都经过脱敏边界。
- 安全凭证不可用时，登录必须失败；不写明文 token，不把二维码 payload 放入日志。
- 回滚通过选择旧 tag 完成，不重写、移动或覆盖 v2.1.3、v2.1.2 或任何历史 tag。

## 参考快照

- D:\源码\openclaw-weixin：remote https://github.com/Tencent/openclaw-weixin.git，HEAD cef0bfc390393f716903e16d50408118047f87e0。
- D:\源码\reasonix\internal\bot\weixin：参考二维码状态、超时、字段容错和 Mock 边界。

实现只复刻协议行为和测试思路，没有复制 TypeScript 或 Go 源码。

署名：开发者
