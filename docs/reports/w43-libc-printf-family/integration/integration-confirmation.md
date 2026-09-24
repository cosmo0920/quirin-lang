# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25, on merge commit `bc3b26c2`.
This card had base `f8b41209` and was measured against `w43-libc-result-alias`.
It was merged onto main `d2e8683a`, after W43-INDEX-TRAVERSAL, in the integrator's
own worktree with `CARGO_INCREMENTAL=0` and Clang 21.1.8. The figures are in
[gate.json](gate.json), and the private logs are in the integrator session's
scratch `gate-integrate-printf/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,301 passed / 0 failed / 11 ignored**, 286 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **174** audits |
| `unittest discover -s scripts/tests` | 0 | 330 tests |
| provider tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w43-index-traversal/integration/artifacts` | 1 | by design: this card moves the corpus |
| composition check v2 ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate integration/gate.json`, then Python | 0 / 0 | 326 follow-ups; **330** tests |

## Merge

* `contract_loader.rs`: the printf-family list and the ten restrict-prototype
  entries are both kept. The v1 declaration count goes 140 → 144, with selection
  and rejected counts unchanged at (46, 206).
* The ledger is merged entry by entry and field by field: one follow-up retired
  and three registered, **324 → 326**. The card's report says 299 → 301, which was
  true on its own branch.
* The corpus row is re-derived from this run.

## Composition

This integration adds a **transitive** clause to [compose_check.py](compose_check.py).
When a function becomes `Safe` only on the combined tree, it gains a validated
summary, so its callers' "call to uncontracted function `f`" leaves may disappear.
The check lists those leaves and removes them from the expected set. The three
earlier integrations still report `COMPOSES` under this version.

For this card, the combined run matches the expectation exactly:

| | Count |
|---|---|
| Card delta: leaves removed | 2,229 |
| Card delta: leaves added | 1,283 |
| Card delta: functions moved | 5 |
| Overlap | 0 |

**Complementary.** Two functions are `Safe` only on the combined tree, because
this card and W43-INDEX-TRAVERSAL each cleared part of their blockers:
`mk_kernel_features_print` and `rd_kafka_ssl_normalize_hostname`.

**Transitive.** Two leaves went away because `rd_kafka_ssl_normalize_hostname` is
now `Safe`: its call sites in `rd_kafka_transport_ssl_set_endpoint_id` and
`unittest_ssl`. Neither caller becomes `Safe`.

**Combined.** Seven functions move Unknown → Safe:
* `cJSON_Version`
* `cio_errno_print`
* `cmt_errno_print`
* `ctr_errno_print`
* `rd_kafka_mk_nodename`
* `mk_kernel_features_print`
* `rd_kafka_ssl_normalize_hostname`

No unit moves, and there are no violation candidates.

The combined run is archived as the new baseline in [artifacts/](artifacts/). It
was captured in the integrator worktree
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da`.

| Measure | Value |
|---|---|
| Functions (Safe / provisional Unsafe / Unknown) | **294 / 84 / 4,456** |
| Units (Safe / provisional Unsafe / Unknown / data-only) | 3 / 42 / 136 / 4 |
| `leak` | 271 |
| `unknown_alias` | 4,320 |
| `unsupported` | 45,907 |
