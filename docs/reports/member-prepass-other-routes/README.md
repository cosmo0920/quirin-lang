# MEMBER-PREPASS-OTHER-ROUTES

The member-facts pre-pass of gateway rule 12n now runs on the current and
no-build routes, a checkpoint carries the run's facts, and the scope file's
exclusions are re-checked against the run's own sources. No verdict moves on
any route.

* **Card:** MEMBER-PREPASS-OTHER-ROUTES (card-high), wave 48 round 28.
* **Base:** local `main` `d708641bc`; rebased onto `711287771` (after
  FB-QUANT-CONTRACT-CENSUS, LEAN-4-35-TOOLCHAIN and
  LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED).
* **Authority:** the ledger entry `MEMBER-PREPASS-OTHER-ROUTES` and its
  DISCHARGE; rule 12n; user decisions R1 and F1 (2026-10-14); the
  integrator's concern about `member-facts-scope.json` (part 2 of the brief).
* **Prediction:** [PREDICTION.md](PREDICTION.md), frozen before any
  implementation code (`e014a6cef`, `1db404d20` after the rebase).
* **Rule:** 12n amended in place. No new rule id.
* **Lean:** `QCore.MemberPrepassRoutes`, audited in
  `proof/AuditMemberPrepassRoutes.lean` (Lean 324 → 325).

## 1. What changed

| Piece | Where | What it does |
| --- | --- | --- |
| Current route | `current_route.rs`, `CurrentAudit::set_member_prepass` | Before any unit, runs `member_prepass::run` over the whole `--compile-database` plus the Monkey units the route analyses. A selection chooses the units analysed, not the program. Each unit is analysed with the run's facts installed, and `finish` folds over them. |
| No-build route | `NoBuildAudit::run_member_prepass` | The same pre-pass over the selected units, under the compile records they are analysed as. The route has no profile, so nothing is excluded. |
| Checkpoint carrier | `checkpoint.rs` (schema 5), `checkpoint_run.rs` | The run identity carries `member_facts_sha256`. A changed run set recomputes every unit and never composes with another (`member facts differs`). `global.member_fold` carries the fold's environment (`FoldEnvironment`: member-invoking and result-state declarations, owned-result functions) and the run's closure of each invoked member, with the pre-pass section. `contracts.callback_members` carries each unit's member and state facts. |
| Composer | `compose.rs` (`member_fold_section`) | Folds the carried inputs exactly as the whole run's `finish` does (`callback_member_fold::fold_in`). It writes the `callback_members` section and prints the `callback members:` line. It never rewrites rows, so a fold that would demote one is refused. Checkpoints without the run's facts still refuse a member-invoking run. |
| Fold | `callback_member_fold.rs` | `fold_with` is now `fold_in` over a serialisable `FoldEnvironment` and the run closures. The behaviour is unchanged. |
| Scope file | `contract/profiles/fluent-bit/member-facts-scope.json` (schema 2), `member_prepass::load_scope` | Names `evidence_revision` (`66910c10…`) and, per entry, a typed evidence. A file of another schema, without the revision, or with an entry without evidence is refused. |
| Re-check | `member_prepass::exclusion_files`, the fixed-point loop in `member_prepass::run` | Re-verifies every exclusion against the run's sources. `tokens-absent`: the unit's raw text, every file Clang resolves for it under its own command (`-M -MG`, dependency flags stripped, nothing written), and the named generator sources. A file source is read with the files its preprocessor directives resolve under the unit's command; a directory source contributes every file under it. None of these may name a final token, and every unresolved include must be named as generated. `not-linked`: the named link lines, beside the `CMakeCache.txt` above the unit, name the archive and never the wrapper. The check runs only at a fixed point of the reading, against that point's tokens. A dropped exclusion is read like any other unit, and an unreadable one opens every member. The section's `scope` records the revision and every held and dropped entry. |

The pre-pass section's `QCORE_MEMBER_PREPASS_SECTION` write, the console line
and the hand-off constants' resolution (rule 13l) are shared by all three
routes (`member_prepass::announce`). The current route's line joins the
preamble a checkpoint composer prints.

**A defect found and fixed during the card.** The first re-check ran in
every round. The bootstrap round's tokens include unnormalised member path
segments (`cb`, `log_cb`). They dropped 88 sound exclusions and left 34 units
unread on the corpus (`private/mpor/a1-prepass-corpus.json`). The final
tokens have 69 entries, none generic. The check now runs only at a fixed
point (`9a59fe155`).

### Scope evidence per entry

All written by [`scripts/write_scope.py`](scripts/write_scope.py). Every
fragment and recorded `why` is kept byte for byte.

| Entries | Units | Evidence |
| --- | --- | --- |
| `src-cpp/` | 12 | `not-linked`: `src/CMakeFiles/fluent-bit-{bin,shared}.dir/link.txt` name `librdkafka.a`, never `rdkafka++` |
| six generated parsers | 6 (absent) | `tokens-absent`, sources: their two grammars; generated: the two parser headers |
| three parser drivers | 3 | `tokens-absent`, sources: the grammars; generated: `*_parser.h`, `*_lex.h` |
| `fluent-bit.c`, `flb_dump.c` | 2 | `tokens-absent`, sources: libbacktrace's `backtrace.h`, `backtrace-supported.h.in`; generated: both headers |
| LuaJIT `src/`, `lj_vm.S` | 76 + 1 (absent) | `tokens-absent`, source: the whole `lib/luajit-7152e154/src` tree; generated: the seven `buildvm` / version headers |

## 2. Tests and proofs

`crates/qcore-audit/tests/member_prepass_routes.rs` uses a fixture whose
`use_err` calls a member invocation of a member no unit stores into. It is
`Safe` exactly when the unit decides over the run's closed, empty set (E1):

* the current route decides over the run's set;
* the no-build route decides, and an unreadable selected unit naming a token
  opens it;
* a checkpointed run composes byte-identically to the whole run, and so does a
  standalone `checkpoint compose`; the same checkpoints stripped of the run's
  facts are refused;
* an exclusion that still holds keeps its unit outside;
* an exclusion whose unit now stores into the member is dropped and read, so
  its candidate joins the set;
* an exclusion whose unit gained an unresolved include is dropped, its
  preprocessing fails, and it opens every member.

The unit tests in `member_prepass.rs` cover the dependency-rule parser,
directive extraction, scope-file refusals and the link-line evidence.
`checkpoint_cli.rs` pins schema 5.

`QCore.MemberPrepassRoutes` proves the following.

* An evidence file naming a token drops the exclusion
  (`a_named_token_drops_the_exclusion`), and so does failed evidence.
* A dropped exclusion reads as its unit.
* **A stale exclusion never closes a member**
  (`a_stale_exclusion_never_closes_a_member`).
* Exclusions that hold change nothing under C's rule that a unit naming no
  token stores nothing (`held_exclusions_keep_the_run_set`).
* A composer without the run's facts refuses
  (`a_checkpoint_without_facts_refuses`).
* **A composition that succeeds publishes exactly the run's fold**
  (`the_composed_decision_is_the_runs`).
* A run whose units decided over the carried closure always composes
  (`a_run_decided_over_its_facts_composes`).

The axioms are only `propext`, `Quot.sound` and `Classical.choice`.

## 3. Measurement

Every route selected the null property. BEFORE `before` (`82ae7914…`) was
built from `git archive d708641bc` in `private/mpor/base-src`, with this
worktree's `third_party/libevent` copied in
([`scripts/base-submodule.py`](scripts/base-submodule.py)). AFTER `a2`
(`c6f21efe…`) was built from `d52fed96a`. Each corpus `run.json` names its
binary. All runs went through [`scripts/before.sh`](scripts/before.sh),
[`scripts/after-chain.sh`](scripts/after-chain.sh) and
[`scripts/summarize_runs.py`](scripts/summarize_runs.py), with results in
[artifacts/summary.json](artifacts/summary.json).

| Route | BEFORE → AFTER | Report sha256 | Null rows | Wall (s, inside the lock) |
| --- | --- | --- | --- | --- |
| Corpus | 936 / 9 / 3,889 → **same** | `71387bd3…`, byte-identical | identical; 1,113 / 1,435 / 2,355 / 18 Unsafe | 673.3 → 673.1 |
| System | 93 / 4 / 351 → **same** | `263e93ab…`, byte-identical | identical; 84 / 125 / 245 | 94.6 → 192.8 |
| Slice | 52 / 1 / 347 / 4 Pending → **same** | `3e0c6ccf…`, byte-identical | identical; 71 / 103 / 232 | 50.4 → 150.3 |

On every route, only `callback_members` changes in the sidecar (base-tree
paths normalised):

* **Corpus:** only its `prepass` changes. The pinned route's pre-pass reads
  987 of 1,444 units in 2 rounds over 69 final tokens, none unread. Its new
  `scope` record holds all 14 entries (100 units) at evidence revision
  `66910c10`, with none dropped.
* **System (tree `046accf6`) and slice (tree `34155955`):** these gain the
  `prepass` section, and their `members` are now the run's:
  * system: 1,419 units, 966 read; slice: 1,421 units, 968 read;
  * none unread, all 100 exclusions held;
  * `rko_op_cb` closed over 27, `rko_serve` over 4;
  * the rk_conf members as on the corpus;
  * `rko_u.offset_commit.cb` and `rko_u.terminated.cb` open, for the union
    reasons the corpus shows;
  * BEFORE, every member read "closed, empty" over the selection's own
    units, which hold no librdkafka unit;
  * 0 rows demoted ([system](artifacts/callback-members-system.json),
    [slice](artifacts/callback-members-slice.json)).

**Checkpoints on a real route.** The system route, run whole and with
`--checkpoint-dir` (15 checkpoints, no null property, since checkpoints refuse
it), composes a report and sidecar byte-identical to the whole run. The
report is `263e93ab…`, equal to the null-property run's. The fold demotes 0
rows. Wall time is 179.0 s whole and 175.1 s checkpointed.

**The pre-pass alone.** [`scripts/prepass.sh`](scripts/prepass.sh) runs
`examples/member_prepass_run` at `--jobs 4`
([artifacts/prepass-alone.json](artifacts/prepass-alone.json)):

| Database | Time | Units read | Exclusions |
| --- | --- | --- | --- |
| corpus, without Monkey | 133.6 s | 963 of 1,415 | 100 held |
| system | 112.4 s | 966 of 1,419 | 100 held |
| slice | 110.5 s | 968 of 1,421 | 100 held |

This is the added wall time on the system and slice routes: about 98 and
100 s. Wall times were taken with up to five cards sharing the host.

**The rebased tip** (`a3`, built from the tip after the rebase onto
`711287771`): binary `8f8636b3…` from `78097ac6f`, every route with the null property
([`scripts/rebased.py`](scripts/rebased.py),
[artifacts/rebased.json](artifacts/rebased.json)).

* **Corpus:** 936 / 9 / 3,889. The report is `82214412…`, byte-identical
  to main's after LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED, and the null rows are
  `5e854d2a…`, identical. Wall time 674.0 s; `run.json` names `a3`.
* **System:** 93 / 4 / 351 at 191.0 s.
* **Slice:** 52 / 1 / 347 / 4 Pending at 151.0 s.
* **System and slice report bytes** (`53076b12…`, `64757594…`) carry this
  worktree's profile path, so they differ from main's recorded digests,
  which also differed from this worktree's BEFORE at the same base. The null
  rows match main's (`23316f8b…`, `ed8de3d2…`).
* **Row for row** ([`scripts/rows_equal.py`](scripts/rows_equal.py)): every
  unit's status and every function's status and diagnostics are identical to
  `a2`'s, on system (448 rows) and slice (404). The corpus is identical to
  BEFORE (4,834 rows).
* **Pre-pass:** reads 987 / 966 / 968 units, none unread, with all 100
  exclusions held.
* **Checkpointed system run:** composes byte-identically again
  (`53076b12…`, 15 checkpoints).

## 4. Prediction against outcome

* **Verdicts:** as predicted, none moves and every report and null row is
  byte-identical.
* **Exclusions:** as predicted, all 14 entries hold on all three trees.
  They had to be re-verified at the system's and slice's own revisions, and
  they were.
* **Wall time:** within the predicted ranges: system 192.8 s (150–230
  predicted), slice 150.3 s (110–200).
* **Not predicted:** the first re-check's bootstrap-token defect (§1),
  caught on the pre-pass-alone run before any arbiter run, and the system
  sidecar's member closures, which BEFORE read over the selection alone
  (closed and empty). No row rested on them.

## 5. Records and follow-ups

* **Ledger 784 → 785:**
  * `MEMBER-PREPASS-OTHER-ROUTES` retired: its DISCHARGE holds, with the
    pre-pass on both routes over their compile databases, the checkpoint
    carrier, and the measurements on system and slice.
  * Registered `MEMBER-SCOPE-RECHECK-RAW-TEXT`: the re-check reads raw text
    and generator sources, not an excluded unit's preprocessed text.
  * Registered `CHECKPOINT-COMPOSER-DEMOTING-FOLD-REFUSES`: a composer
    refuses a fold that would demote, rather than rewriting rows.
  * Script: [`scripts/update_ledger.py`](scripts/update_ledger.py).
* **Now available:** `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION` names this
  card as its prerequisite; the run's set is now read on the system and
  slice routes.
* **Also updated:** spec C04.19.3 (the run's set), the checkpoint format
  page (`docs/development/audit-checkpoints.md`, schema 5), the rule router
  row 12n, and the increment paragraph in `wave-48-part8.md`.

## 6. Gate

The standing gate ran at `4417f7a58` (the records commit on the rebased
tip) under one exclusive hold of the shared heavy lock, 58 GB free at the
lock. Scripts: [`scripts/gate-run.sh`](scripts/gate-run.sh),
[`gate-whole.sh`](scripts/gate-whole.sh), [`gate.sh`](scripts/gate.sh) and
[`finish-gate.sh`](scripts/finish-gate.sh). Logs are in
`private/mpor/gate-g1/`; exits are in [gate/](gate/) and the summary in
[gate.json](gate.json).

| Step | Exit | Count |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update | 0 | |
| `cargo fmt --check` | 0 | |
| `cargo clippy --locked --all-targets -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 500 units, 513 summaries, **8,108 passed**, 0 failed, 15 ignored, no rerun (`private/mpor/tests-g1/`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` (Lean 4.35.0) | 0 | **325** audits (324 → 325: `QCore.MemberPrepassRoutes`) |
| Python suite before the matrix regeneration | 1 | expected: it reads the committed matrix's Lean count (324) |
| `build_matrix.py . --gate gate.json` (09a) | 0 | |
| Python suite (09b) | 0 | 374 tests OK |
| docs index check / regeneration / check | 0 / 0 / 0 | |

The provider gate was not run: nothing under `providers/` or
`crates/qcore-schemas/` changed.
