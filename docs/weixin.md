# YunXi Agent 微信接入边界

## v2.1.2 能力

`v2.1.2` 只建立可测试的工程和协议边界：

- `crates/yunxi-agent-weixin` 提供窄化领域 facade、秘密值脱敏、结构化 iLink 错误、二维码/轮询/发送/typing/上传 URL serde 模型，以及固定生产端点的 HTTP 客户端。
- `yunxi weixin login|status|doctor|serve|pair|logout` 提供帮助、参数校验和诚实的可用性输出。
- `status --json`、`doctor --json` 和 `pair list --json` 只输出非秘密状态。
- `IlinkHttpClient` 固定生产端点为 `https://ilinkai.weixin.qq.com/`；仅测试构造器接受 loopback Mock 地址，CLI 不提供任意 base URL。
- 离线 `wiremock` 测试覆盖必要请求头、游标、请求 ID、API 错误、错误 JSON、超时、响应大小上限、数字/字符串 message ID 和脱敏边界。

## 当前不能做什么

本版本尚不能：

- 真实扫码登录或保存微信凭证；
- 真实接收、发送或流式回复微信消息；
- 启动长轮询常驻服务或后台主动推送；
- 绑定 YunXi session、执行远程审批或让远程消息改变 cwd、Provider、模型、sandbox、approval mode；
- 使用群聊。首期设计仅允许私聊，群聊保持关闭。

因此，`login`、`serve`、`pair approve|deny` 和经确认的 `logout` 会返回明确的“尚未实现”错误，而不是伪造成功。`status`、`doctor` 和 `pair list` 是安全的本地只读骨架命令。

## 命令

```text
yunxi weixin login [--account default]
yunxi weixin status [--account default] [--json]
yunxi weixin doctor [--account default] [--json]
yunxi weixin serve [--account default] [--workspace <path>]
yunxi weixin pair list|approve|deny ...
yunxi weixin logout [--account default] --confirm
```

Provider、模型、sandbox、approval 和根 `--cwd` 继续由现有 YunXi CLI 配置路径解析。`serve --workspace` 只验证并规范化未来服务工作区，不调用 `Agent::run_with_backend_stream`，也不发起微信网络请求。

## 协议与秘密边界

- 生产请求使用官方 iLink endpoint、`AuthorizationType: ilink_bot_token`、`X-WECHAT-UIN`、`iLink-App-Id`、`iLink-App-ClientVersion` 和每请求 `X-YunXi-Request-Id`。
- 每个请求有显式超时；响应按流读取并受 1 MiB 默认上限约束。
- HTTP status、腾讯错误码、错误 JSON、超时、网络失败和协议缺失字段归一为 `WeixinApiError`。
- token、二维码 payload、验证码、`context_token`、轮询游标、原始用户 ID 和原始消息正文不进入 Debug、Display、错误链或诊断 JSON snapshot。
- wire serde 必须携带协议秘密字段，但生产代码不记录原始请求/响应 body。

## 参考快照

- `D:\源码\openclaw-weixin`：remote `https://github.com/Tencent/openclaw-weixin.git`，HEAD `cef0bfc390393f716903e16d50408118047f87e0`。
- `D:\源码\reasonix\internal\bot\weixin`：参考二维码状态、游标、数字/字符串 message ID 和 Mock 边界。

实现只复刻协议行为和测试思路，没有复制 TypeScript 或 Go 源码。凭证存储、账户状态、真实登录、长轮询、会话绑定、远程审批、流式回信和真实微信联调属于后续版本。

署名：开发者
