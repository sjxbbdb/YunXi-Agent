# YunXi Agent 微信接入边界

## v2.1.5 能力

v2.1.5 在 v2.1.4/hotfix 状态持久化、诊断、安全账户生命周期和旧登录账户 metadata 到 `WeixinStateStore` 安全初始化基础上，新增前台私聊长轮询接纳层：

- yunxi weixin login --account <name> 获取二维码、显示安全终端文本、轮询等待/扫码/确认，并明确处理过期、取消、超时、redirect、验证码和验证码阻断。
- 登录确认后，token 和数据加密密钥只写入 Windows Credential Manager；系统凭证不可用、权限失败或写入失败时登录失败，不降级到明文文件。
- .yunxi/weixin/ 只保存脱敏账户哈希、官方 endpoint、连接状态、凭证引用、workspace 标识、schema version 和创建/更新时间。
- yunxi weixin status --json 与 doctor --json 读取脱敏登录元数据和凭证可用性；不会输出 token、二维码 payload、加密密钥或原始用户标识。
- yunxi weixin logout --confirm 只删除指定微信账户的系统凭证和微信元数据，不触碰 YunXi session、persona memory、工作区其他账户或 Git 状态。
- CLI 私有登录执行 helper 已覆盖 Mock 成功、过期、取消、凭证不可用、metadata 写失败回滚和输出脱敏；生产 CLI 不暴露 mock endpoint 或 fake store 参数。
- 2026-07-27 真实验证使用 `default` 账户完成扫码确认，`status --json` 返回 `state=ready`、`credential_state=present`、`credential_backend=windows-credential-manager`，`doctor --json` 返回 `credential_store=present`、`credentials_configured=true`，新进程重读仍为 ready。
- `crates/yunxi-agent-storage` 提供独立版本化 `WeixinStateStore`，状态文件使用同目录临时文件、文件级 sync 和同卷替换更新；启动诊断会识别未完成临时文件候选，但不会把半成品当作有效状态。
- 状态 store 记录脱敏账户、workspace hash、官方 endpoint、Credential Manager 引用、游标占位、回执、会话绑定占位、reply context 引用、待投递元数据、pair request 和加密 pending inbound 引用。
- `status --json` 与 `doctor --json` 增加 state schema、account lock state、pending inbound/delivery count、pair request count 和最后一次脱敏错误；不输出完整本地路径。
- 已有旧账户 metadata 且 state 文件缺失时，`status --json` 和 `doctor --json` 会用旧 metadata 中的非机密字段与凭证引用一次性创建当前 schema 的最小状态；首次 JSON 输出 `state_store_migration="initialized_from_legacy_metadata"`，后续新进程重读输出 `state_store_migration="already_current"`。
- 初始化不会读取、复制或输出 token、二维码 payload、原始 user ID、原始 peer ID、data key、context token 或系统凭证明文。
- 已存在当前 state 时不会覆盖 pair、pending inbound、delivery、cursor、last error 等运行状态；遇到未来 schema 或损坏 state 时拒绝覆盖并返回脱敏诊断；遇到损坏 metadata 时 `doctor --json` 返回结构化安全错误且不创建错误 state。
- 凭证引用存在但系统凭证不可用时，state 仍可由非机密 metadata 初始化；`doctor --json` 会标记凭证不可用或缺失，不写明文回退。
- `yunxi weixin serve` 是前台服务入口，启动前复用 workspace/provider 解析、旧 metadata 初始化、系统凭证检查、数据密钥/加密 pending queue 检查和账户锁。
- serve 循环调用 iLink `getupdates`，使用 `WeixinStateStore` 中的 `get_updates_buf` 游标，尊重服务端 timeout hint，并对空轮询、网络错误和服务端错误执行带 jitter 的有界退避。
- 入站消息先归一化为只含 account hash、peer hash、message id hash、direct-message key、时间、secret reference 和 kind 的 `WeixinInboundEnvelope`；stdout、stderr、JSON、状态和日志不输出原始账号、peer、message id、context token、正文、附件 URL、本地绝对 workspace 或系统凭证 target。
- 已准入 peer 的私聊文本写入加密 pending inbound；陌生私聊文本只生成短时、不透明 pair request；群消息、自消息、附件和未知消息只进入脱敏跳过计数。
- 每个 getupdates 批次把新游标、receipt、pending inbound、pair request、连接状态和最后一次脱敏错误写入同一次 state-store 原子提交；加密队列或保存失败时游标不推进。
- 同一 account、peer hash、message id hash 已存在 receipt 或 pending inbound 时幂等跳过，不创建第二个 pending inbound。
- token 过期或凭证失效时，serve 写入 `suspended`/`credential_expired` 等脱敏健康状态并停止轮询，不自动删除账户或凭证。
- `pair list|approve|deny` 只处理本地状态 store 中的不透明 request ID、脱敏账户、peer hash、过期时间和状态，不执行远程审批。
- `logout --confirm` 遇到活动账户锁会拒绝；服务停止后只删除指定账户微信凭证引用、微信状态和微信 metadata，不删除 YunXi session、persona memory、工作区文件、其他账户或历史报告。

二维码文本只在交互终端显示，且会先移除 ANSI 控制序列；不会写入普通日志、JSON/JSONL、错误链、Markdown 证据或账户 JSON。

## 当前不能做什么

本版本仍不能：

- 绑定 YunXi session、创建 YunXi session、接入 Agent Runtime 或触发 Agent dispatch；
- 调用 Provider、执行工具、改变 cwd、Provider、模型、sandbox 或 approval mode；
- 发送微信消息、调用 sendmessage、生成或流式合并微信回复；
- 执行远程审批、微信文字命令控制、追问或取消命令桥接；
- 支持群聊、主动推送、附件解析、媒体上传、联系人抓取、Hook、逆向协议或第二套 Agent。

weixin serve 只完成私聊长轮询、准入、排队、pair request 和幂等接纳；真实聊天闭环属于后续版本，不能把登录成功、状态 store 完成或 pending inbound 接纳宣称为 Runtime 对话能力已完成。

## 真实登录验证门禁

发布或复审材料只能记录脱敏状态，不得记录二维码 payload、token、原始用户 ID、数据密钥、原始响应 body 或系统凭证明文。真实验证需要在 Windows 上完成以下步骤：

1. 运行 `target\release\yunxi.exe weixin login --account <test-name>`，只在交互终端展示二维码。
2. 使用真实微信账号扫码并确认。
3. 核对 `.yunxi/weixin/` 只出现非机密 metadata。
4. 运行 `target\release\yunxi.exe weixin status --account <test-name> --json`，确认账户、workspace 和凭证状态均为脱敏字段。
5. 运行 `target\release\yunxi.exe weixin doctor --account <test-name> --json`，确认系统凭证引用可读且 `secrets_included=false`。
6. 重新打开 shell 或重新执行 release 二进制，再次读取 status/doctor，确认凭证引用仍可诊断。

`v2.1.5` 保留上述真实扫码登录前置能力、v2.1.4 状态 store 诊断和旧账户 metadata 懒初始化，并新增前台私聊长轮询接纳层。后续复审材料仍只能记录脱敏账户 hash、状态、计数、退出码和是否触发网络；不得保存二维码、token、原始账号、联系人、消息正文、context token、data key 或系统凭证明文。

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
- 回滚通过选择旧 tag 完成，不重写、移动或覆盖 v2.1.3-hotfix.1、v2.1.3、v2.1.2 或任何历史 tag。

## 参考快照

- D:\源码\openclaw-weixin：remote https://github.com/Tencent/openclaw-weixin.git，HEAD cef0bfc390393f716903e16d50408118047f87e0。
- D:\源码\reasonix\internal\bot\weixin：参考二维码状态、超时、字段容错和 Mock 边界。

实现只复刻协议行为和测试思路，没有复制 TypeScript 或 Go 源码。

署名：开发者
