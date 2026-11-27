# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25, on merge commit `891850e4`. This card
(base `f8b41209`, measured against `w43-libc-result-alias`) was merged onto main
`0a07cb57`, after W43-INDIRECT-ACCESS-GENERAL, in the integrator's own worktree with
`CARGO_INCREMENTAL=0` and Clang 21.1.8 ([clang-version.txt](clang-version.txt)).
Every exit was captured to a file, not piped. Private logs are in the integrator
session's scratch `gate-integrate-restrict/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,254 passed / 0 failed / 11 ignored**, 284 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **172** audits |
| `unittest discover -s scripts/tests` | 0 | 330 tests |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` (after `sync`) | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 13 tests |
| release corpus run (`run_audit.py`) | 0 | — |
| `baseline_identity.py` against `w43-indirect-access-general/artifacts` | **1** | by design: this card moves the corpus |
| composition check ([compose_check.py](compose_check.py)) | 0 | **composes**; see below |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 314 follow-ups carried; **330** tests |
| `build_docs_index.py --check` | 0 | index matches |

The card's own gate recorded one failure, the ledger-tracked, load-sensitive
`TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE` cell. That test passed in this run.

## Composition (first integration of a second baseline-moving card)

This card measured its movement against `w43-libc-result-alias/artifacts`, but main
had meanwhile moved to the W43-INDIRECT-ACCESS-GENERAL baseline.
[compose_check.py](compose_check.py) compares multisets of leaves keyed by
(library, file, function, category, description, origin), plus function and unit
statuses. Preprocessed line numbers are excluded. It checks that

    (integration run − w43-indirect-access-general)  ==  (card AFTER − w43-libc-result-alias)

and the check holds exactly ([composition-check.txt](composition-check.txt)):

* 36 leaves removed and 2 added;
* 3 functions move Unknown → Safe: `mk_plugin_load_symbol`,
  `mk_signal_thread_sigpipe_safe` and `rd_regex_exec`;
* no unit moves and no violation candidates.

chunk-contracts and source are byte-identical to the indirect baseline.
contracts differs by the ten new declarations and the new `libc/regex.qcore`.

The integration run is archived as the **new combined baseline**,
[artifacts/](artifacts/) (`archive.py`; the manifest records the digests). It was
captured in the integrator's worktree
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da`,
which is the path to pass as the `--root-alias` target.

Corpus rows re-derived from it:

| Measure | Value |
|---|---|
| Functions (Safe / provisional Unsafe / Unknown) | **272 / 84 / 4,478** |
| Units | 3 Safe / 42 provisional Unsafe / 136 Unknown / 4 data-only |
| leak | 271 |
| unknown_alias | 4,725 |
| unsupported | 47,800 |
| indirect-access refusals | 17,298 leaves / 3,290 functions / 382 sole |
| return-expression refusals | 498 / 421 / 148 |

The prior row is kept in [previous-corpus-row.json](previous-corpus-row.json).

## Ledger

The ledger was merged three ways from base `f8b41209`: entry by entry for
`named_followups`, field by field for the other sections. The two sides both
changed `corpus.cycle19_raw_units`. Main's side was kept at merge time, then
rewritten from the integration run above. One entry was added from this card's two
new residuals minus the one it retired, giving **313 → 314**. The card's report
says 299 → 300, which was true on its branch.
