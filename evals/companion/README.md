# YunXi Companion Evaluation Harness

This directory contains the offline, deterministic v2.0.0 release corpus. It
retains all 31 v1.9.4 scenarios and their golden thresholds as the v2 quality
gate.

It is intentionally small and reviewable: JSONL scenarios are embedded by the
`yunxi-agent-eval` crate, evaluated with Rust rule judges, and emitted as a
structured JSON/JSONL summary by `yunxi eval companion`.

Run the full suite after building:

```powershell
yunxi eval companion
yunxi --json eval companion
yunxi --jsonl eval companion
```

The suite has 31 scenarios across persona consistency, memory precision,
relationship continuity, proactive boundaries, and control operations. It does
not call a live provider, a cloud judge, Python runtime, or an external
service. Temporary build/test output belongs under `target` and is cleaned at
the end of the release workflow.

Metrics include persona consistency, memory precision, false positives, missed
writes, forbidden writes, recall accuracy, relationship continuity, proactive
boundary violations, tool approval bypasses, and control regression rate.

The scenario schema is documented in `schemas/scenario.schema.json`; the
result shape is documented in `schemas/result.schema.json`. The small expected
thresholds are in `golden/companion_expected_metrics.json`.
