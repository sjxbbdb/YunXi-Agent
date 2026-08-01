# YunXi Weixin evaluation suite

This suite is the offline and real-condition checklist for the v2.2.0 merged Weixin line.

The default `yunxi eval weixin` run is offline only:

- it does not read Windows Credential Manager;
- it does not perform iLink network requests;
- it does not read or write token, context token, raw user id, raw message text, provider wire, or data key evidence.

The `real_integration_checklist` and manual restart recovery gates are explicit `status=not_run` markers in the offline harness. They are not counted as passed live evidence. They must be satisfied with separately redacted evidence before a `v2.2.0` release tag can be created.

Covered gates:

- protocol mock and iLink inbound classification;
- state schema migration boundary for delivery spool and remote-control requests;
- pairing and slash-command remote control;
- final-text delivery segmentation and restart-recovery checklist with manual gates marked `not_run` unless real evidence exists;
- safety diagnostics and explicit real iLink/Provider validation checklist.

署名：开发报告撰写者
