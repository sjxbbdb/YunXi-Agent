# YunXi Agent v2.0.6 Composer And Dialog ConPTY Evidence

- Capture window: 2026-07-20 17:29:22 to 17:29:56 +08:00
- Provider: DeepSeek
- Model: `deepseek-chat`
- Terminal: Windows ConPTY through `node-pty@1.1.0`
- Renderer/parser: `@xterm/headless@5.5.0`
- Collector: `scripts/conpty/v206`
- Raw evidence: `docs/reports/evidence/frames/v206-conpty`
- Release `yunxi.exe` SHA-256: `678D0637CE854CE8AF7F032B82B1A5848BD11D40D4E7EBAD91ACABABA2AAE675`
- Release compatibility binary SHA-256: `82A01F48E2F310BB601CA8174D054173AF550532390C365868D4C57D78E7FEAB`

## Boundary

The collector starts the real release `yunxi.exe` with `--provider-live`,
`--backend yunxi`, `--model deepseek-chat`, `useConpty: true`, and provider
streaming enabled. `@xterm/headless` interprets terminal output; it does not
create YunXi runtime, input, approval, or transcript events.

Provider credentials are inherited from the normal local configuration or
environment. They are not included in actions, frames, the manifest, or this
report. Every persisted evidence file is sanitized and scanned for GitHub,
Bearer, API-key, and `sk-` token shapes before verification.

## Scenarios

| Scenario | Terminal | Checkpoints | Output bytes | Evidence SHA-256 |
| --- | --- | ---: | ---: | --- |
| `ordinary` | 120x40 | 4 | 7,419 | `0113530f4fc2e3c6c3bbe58554b6197f690998b6201d944460f73915a6c15f20` |
| `multiline` | 120x40 | 5 | 8,682 | `89c0ad61a796bc43be4fa89ad42e2df75bbc1cf1cd9797927f3daf0355ce1894` |
| `crlf` | 120x40 | 5 | 10,221 | `ca4ea351bb2a2cc6f31a9ab1b20c5e894315305654b00bc01a26cd25c5ab8ff4` |
| `long` | 80x24 | 5 | 21,082 | `a48dd424ea57af7d3c001dd6a4abf95e700ae90898826c46e98158b78be22e70` |
| `ime` | 120x40 | 5 | 7,739 | `2871df12014253730144b998facb9426b9a3e9e4246013cf8a068ebda84fcd52` |
| `stream-cancel` | 120x40 | 6 | 10,115 | `1bb9051979f2ab7ef1b8665893ebf7cd32f1fa9395dc0f575aa06cc2f7d09993` |
| `approval-restore` | 120x40 | 7 | 14,220 | `13c8ace88a9657856aa63407959b7569e1ebeb7f5ee86d4199c27c0dc54d061e` |
| `final-single` | 120x40 | 5 | 7,391 | `a123f9584604ed013809e91b932dd2f5917faed9ed5102c7fde7d517c6484654` |

The multi-line and CRLF checkpoints require `cells=0` while all lines remain in
Composer, proving that embedded newlines do not submit partial turns. The long
scenario pastes more than 1,800 characters at 80x24 and requires the tail and
footer to remain visible. The IME-style scenario includes committed Chinese,
an emoji ZWJ family, and a combining grapheme.

The stream scenario queues a next-turn draft, observes active-turn draft
status, sends Ctrl+C, verifies restoration, and submits that exact draft. The
approval scenario queues a draft before the tool approval request, observes
the decline-by-default overlay, sends `N`, verifies exact restoration, and
submits the recovered draft. The final scenario requires exactly one visible
`[assistant] YUNXI_FINAL_SINGLE_OK` occurrence after delta/final/completed.

## Verification

`npm.cmd run verify --prefix scripts\conpty\v206` recomputes all eight hashes,
validates metadata, scenario order, required labels, actions, frame markers,
normalization, dimensions, exact final occurrence count, and secret scans. It
returns `ok=true, scenarios=8` without a network request. The historical
`scripts/conpty/v205` verifier also returns `ok=true, scenarios=8` against its
unchanged evidence set.

署名：开发报告撰写者
