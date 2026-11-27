# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25, on merge commit `e7ef0887`. The card
had already merged main `f8b41209`, so the merge had no conflicts. The run used
the integrator's own worktree, built from cold, with Clang 21.1.8 present
([clang-version.txt](clang-version.txt)). Every exit was captured to a file, not
piped. Private logs are in the integrator session's scratch `gate-integrate-naming/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,216 passed / 0 failed / 11 ignored**, 281 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **170** audits |
| `unittest discover -s scripts/tests` | 0 | 330 tests |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 13 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 298 follow-ups carried; **330** tests |
| `build_docs_index.py --check` | 0 | index matches |

## Corpus arbiter

A fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0) was
compared with `docs/reports/w43-libc-result-alias/artifacts/` using
`baseline_identity.py --root-alias`, which also exited 0. Diagnostics,
chunk-contracts and source are **byte-identical**.

The run was also compared with the CONTRACT-INHERIT-ADOPTION integration run in
this same checkout:

* **source-effects** is byte-identical;
* **contracts** differs only in these places:
  * `qcore-provider-libevent:monkey/memory.qcore` and
    `qcore-provider-librdkafka:librdkafka/memory.qcore` are now keyed by
    `crate:module`. Each elaborated text is **token for token identical** to the
    hand-written text it replaces;
  * the manifest now references those two entries;
  * the inclusion record gains the eight chains (`rd_malloc`, `rd_calloc`,
    `rd_free`, `rd_strdup`, `rd_strndup`, `mk_mem_alloc`, `mk_mem_alloc_z`,
    `mk_mem_free`).

No verdict, leaf or coverage moves.
