# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-25, on merge commit `2d0b77e1`
plus a ledger repair commit. This card was merged onto main `d3cd8d81`, after
W43-LIBC-RESULT-ALIAS, in the integrator's own worktree with Clang 21.1.8
present ([clang-version.txt](clang-version.txt)). Every exit was captured to a
file, not piped. The private logs are in the integrator session's scratch
directory `gate-integrate-adoption/`.

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,208 passed / 0 failed / 11 ignored**, 280 summaries |
| core fmt, clippy `-D warnings` | 0 / 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | — |
| `check_lean_audits.py` | 0 | **170** audits |
| `unittest discover -s scripts/tests` (on the merge commit) | 1 | 36 errors/failures: unparseable matrix, see below |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **117 passed / 0 / 0**, 39 summaries |
| provider fmt, clippy `-D warnings` | 0 / 0 | clean |
| package mutation control (isolated, `--test-threads=1`) | 0 | passed |
| `providers/contracts.py check` | 0 | — |
| `providers/package.py build` + `verify` × 5 | 0 | all five packages |
| `unittest discover -s providers/tests` | 0 | 12 tests |
| after the ledger repair: `unittest discover -s scripts/tests` | 0 | 330 tests |
| `build_matrix.py . --gate integration/gate.json`, then `unittest discover -s scripts/tests` | 0 / 0 | 299 follow-ups carried; **330** tests |
| `build_docs_index.py --check` | 0 | index matches |

## Ledger repair (integrator error, disclosed)

The merge commit `2d0b77e1` carried unresolved conflict markers in
`docs/reviews/r0_3/acceptance_matrix.json`:

* The integrator's entry-level merge script declined to write, because it
  found a conflict in the `corpus` section as well as `validation`.
* The integrator did not notice that it had declined, and committed git's
  conflicted file.
* The Rust, Lean, provider and corpus steps do not read the matrix and passed.
  The Python evidence validators could not parse it and failed.

The repair merged the ledger three ways, field by field (base `dfe10c92`):

* main's `corpus` verdict fields (W43-LIBC-RESULT-ALIAS) are combined with the
  card's `corpus.contract_modules` 107 → 108; the two sides touched different
  fields;
* the card's three residuals are added and `CONTRACT-INHERIT-ADOPTION` is
  retired, so the count goes from **297 to 299**;
* `validation` is regenerated from this gate.

No conflict marker remains anywhere in the tree. The card's report says
294 → 296, which was true on its branch.

## Corpus arbiter

A fresh release run (`w43-return-expression/scripts/run_audit.py`, exit 0) was
compared with `docs/reports/w43-libc-result-alias/artifacts/` using
`baseline_identity.py --root-alias`, which also exited 0:

* **diagnostics, chunk-contracts and source are byte-identical**: no verdict,
  leaf or coverage moves;
* **source-effects** is identical modulo the checkout-path identity digests;
* **contracts** differs only by expected additions. The integrator checked each:
  * a new `contract_inclusion` record with the three inheritance chains;
  * the schema-2 manifest, which names `providers/contract-load-path.json`;
  * the Fluent Bit sources now keyed by `crate:module`;
  * the elaborated `fluent_bit/resource_memory.qcore`, which is **token for token
    identical** to the hand-written text it replaces once comments are ignored;
  * the new `fluent_bit/resource_string.qcore` (`flb_strdup`, `flb_strndup`);
  * `libc/string.qcore` equal to the baseline text with `strndup` appended.
