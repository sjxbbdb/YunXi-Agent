# YunXi Agent v2.0.7 Interaction Focus ConPTY Evidence

- Verification time: 2026-07-21 07:16:46 +08:00
- Binary: `D:\YunXi Agent\target\release\yunxi.exe`
- Version: `yunxi 2.0.7`
- Provider/model: DeepSeek / `deepseek-chat`
- Terminal: real Windows ConPTY through locked `node-pty@1.1.0`
- Collector: `D:\YunXi Agent\scripts\conpty\v207`
- Frames: `D:\YunXi Agent\docs\reports\evidence\frames\v207-conpty`

The first restricted-network attempt reached the TUI but returned
`YX-PROVIDER-001`; it was not accepted as evidence. The collector was then run
from an approved real-network process. Credentials were inherited by the child
process and were not printed or persisted. All saved frames passed the
collector's GitHub-token, bearer-token, API-key, and `sk-` redaction scans.

## Result

| Scenario | Checkpoints | SHA-256 |
| --- | ---: | --- |
| ordinary | 4 | `fd7cd641fb83d601044124efe66b5427c78793cf5f871ba28df5acf9cfa835e9` |
| multiline | 5 | `986b74e41da270aadd4a03af3d78dd49310b38d22fa0ad55efdeddd7cbf301f1` |
| crlf | 5 | `832a955c047274b9a6a7c8debc29de4ec1cf43ea798617d2de225a9b539c4d7d` |
| long | 5 | `837c1e8857746d20a728dcdf4c960ed8fc292fd254cdb073343b410c0db80dc3` |
| ime | 5 | `b7a0d4dd40ea5d7b5af66b9ebee51ccaffb9dc192e16e3a8e92b46c5e523850e` |
| stream-cancel | 6 | `b85ed891307c54e7468729acb17e8d11f9e3550abf166220ef7c65c9dacf141a` |
| approval-restore | 7 | `61f04a5801429df0dd174397350637f276c4c8447017dd701ffba47edfb29c17` |
| final-single | 5 | `09b0e0d08c4c9ee167982c439deed02298e024a9d9e5b8aa214ec4027af66e9b` |

`npm.cmd run verify --prefix scripts\conpty\v207` returned
`ok=true, scenarios=8`. The unchanged historical v206 verifier also returned
`ok=true, scenarios=8`.

署名：开发报告撰写者
