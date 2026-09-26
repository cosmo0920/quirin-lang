# STABILITY-REEVAL #5 — the cadence's fifth firing (wave 45 close)

**Card:** `STABILITY-REEVAL` #5, wave 45. **Base:** local `main` at `8c1f15eb`.
**Scope:** everything merged since firing #4's base `6dc484ad` through
`8c1f15eb`: the rest of wave 40 and waves 41–45 (89 first-parent merges; wave
45 weighted highest). **Method:** the four steps of
[operations](../../agents/operations.md#maintenance-cadence).
**Baseline:** [firing #4](../stability-reeval-4/README.md).

This card is an **audit**. It weakens no gate, edits no frozen archive, changes
no verdict rule and lands **no Rust, no Lean and no test**. Every claim is
labelled **MEASURED** (a command was run or a line read, cited) or **INFERRED**.
Seven read-only sub-audits ran in this worktree with cargo forbidden (the
firing-#4 tally; Pending and plugins; checkpoints, 10g and the memo; gateway
rules 5j–5l and 10c–10f; merges and mechanical edits; waves 41–44 with a Lean
binding census; Fluent Bit evidence policy). Every conclusion used below was
re-read at its cited line by this card, or is attributed to its sub-audit in
those words. Where a sub-audit's inference decided a registration, this card
**measured it with a one-off probe** (§2.1): a temporary integration test over
the gateway's synthetic `q_*`/`r_*` test contracts, run once and removed. Its
source and output are shipped in [`probes/`](probes/); it moved no file of the
tree.

`main` moved to `33558c67` during this card: a records-only commit that
corrected the summary count this card found (§4.1). The card did not merge it.

**Headline.** One MEASURED admitting-direction hole (a double member release
published `Safe`, §2.1), one MEASURED Pending forgery (§3.1), and 38 further
findings registered; firing #4's 23 residuals tally **12 discharged, 11 open**,
and the discharged twelve are exactly the ones an implementing card's scope
named.

---

## 1. Tally over firing #4's twenty-three residuals

MEASURED (sub-audit, removal commits found by diffing the id set per commit,
because `git log -S` misses removals whose names other entries still quote;
this card re-read the mechanisms marked ✓).

### 1.1 The twelve gone — all discharged, every successor registered

| Name | Removed by | Verdict at `8c1f15eb` |
|---|---|---|
| `OWNERSHIP-COPY-OP-UNPRODUCED` | `OWNED-FIELD-ROUTE-REACH` (`bfcf5601`, merge `b3ec447b`) | Discharged ✓: `CopyBind` emits `OwnershipOp::Copy` on the route (`crates/qcore-core/src/std_buffer.rs:1108-1113`); one route only (`COPY-BIND-ROUTE-SCOPE`) |
| `OWNED-FIELD-IR-REFUSALS-UNREACHED` | same | Discharged on its documentation arm (spec :1182 "carried but unreached"); successor `AGGREGATE-OWNER-POSITION-UNREACHABLE` |
| `OWNED-FIELD-BOUNDARIES-ARE-ABSENCE` | same | Discharged: each boundary refuses by its own name (`compiler/tests/owned_field_route_reach.rs:355,376,419`) |
| `LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION` | `COMPOSED-WINDOW-REACH` (`36f8b8ec`, merge `619005b3`) | Discharged ✓: `composed_window_reach.rs:715` reads `LoopWindows.lean` and checks every agreement row; **gap** registered as `LOOPWINDOWS-LEND-CLASS-MAPPING-UNCHECKED` (§3.3) |
| `LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY` | same | Discharged: the theorem was removed; one stale citation (`WAVE-45-RECORD-ERRATA` item 4) |
| `COMPOSED-VIEW-WINDOW-INERT` | same | Discharged; the card also corrected firing #4's reading (the function is live at top level, its depth half empty) |
| `WINDOW-FIELD-ARM-UNREACHED` | same | Discharged on the refusal arm (four Field\* arms refuse by name, `std_buffer.rs:1438-1451`); successor `WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED` |
| `TSAN-HEADER-INVISIBILITY-UNENFORCED` | `TSAN-FAILURE-ARMS` (`8412fc86`, merge `026314c8`) | Discharged: a textual tripwire and a standing mutation (`compiler/tests/task_route_tsan.rs:1174,1252`) |
| `PROOF-LEG-DIGESTS-UNCROSSCHECKED` | `CT-PROOF-LEG-3` (`99414f80`) | Discharged ✓: `compiler/tests/proof_leg_digests.rs:42` compares five legs' Lean and Rust digests |
| `F02-AGREEMENT-TEST-SURFACED-BY-PROSE-ONLY` | `fa7ab464` (user commit, merged with `b5f45079`) | Discharged by removing the premise ✓: the Windows pins equal the Linux ones (`windows_threads.rs:31,36` = `f02_trace_relation.rs:96-97`) and the test is not ignored; the run is unbound (`WINDOWS-F02-RUN-REVISION-UNBOUND`) |
| `INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE` | `WAVE-40-CLOSE` (`3310c0e4`) | Discharged on its rule arm (operations, Integration); the path it names has since drifted (`INTEGRATION-GATE-PATH-RULE-DRIFT`) |
| `CT-PROOF-FOURTH-CASE-UNTRACKED` | `CT-PROOF-LEG-3` | Discharged: `CT-PROOF-F02-CANDIDATES-EXHAUSTED` registered |

No retirement was without discharge; no successor is unregistered.

### 1.2 The eleven still registered

| Name | Verdict |
|---|---|
| `CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND` | Open. No commit touched `socket_footprint.rs` or `SocketFootprint.lean`; the digest mechanism now exists and is not applied here |
| `CT42-ARM-UNREACHABILITY-UNPINNED` | Open |
| `CT07-F02-PREMISE-PIN-IS-ERROR-BLIND` | Open ✓ (`compiler/tests/ct07_proof_leg.rs:592` still `.is_err()`) |
| `PROOF-GATED-LIVENESS-CHECK-FILE-SCOPED` | Open, both gaps (`ct_corpus.rs` ~:8037-8068), now over five cases |
| `CLANG-RECORDING-UNBOUND-TO-ITS-RUN` | Documentation arm met (`validation.md:63-75`); mechanical arm open by decision |
| `UNCARDED-BLIND-TO-MERGE-PROSE` | Open; `reconciliation_sweep.py`'s docstring still overstates |
| `MERGED-GATE-ROW-LEAN-HAND-EDIT` | Open; see `STATUS-LATEST-GATE-STALE` (§4.2) |
| `INTEGRATION-TIP-CITATION-UNREACHABLE` | Open. Wave 45's 44 integration-gate SHAs are all ancestors (practice fixed), but four wave-41 files cite non-ancestors ✓ (`git merge-base --is-ancestor` exits 1 for `72a648f1`, cited by ct10-source-layout, ct24-cleanup-task-composition and ct41-sync-model, and for `7d2c1fa6`, tsan-failure-arms) |
| `GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT` | Open; **recurred** at `8c1f15eb` (§4.1), amended by this card |
| `RECONCILIATION-VERIFY-CLAIM-PERISHABLE` | Open, re-scoped in place to its second arm |
| `LEDGER-TSAN-BOUNDARY-SENTENCES-STALE` | Partly discharged: the two rule sentences are rewritten (`buffers-records.md:159-168`); `ct07_proof_leg.rs:58`, `ct07-two-thread-proof-leg/README.md:143` and the CT07/CT46 manifest notes still cite the retired name (with in-text supersession notes) |

**Tally: 12 discharged · 0 retired without discharge · 11 open (three of them
partly discharged).** This card retires none: no present entry was measured
fully discharged.

Of firing #3's six carried entries, four remain (`MATRIX-RECORDED-BY-ABSENT-67`,
`LEGACY-CFG-NODE-FIXTURES-SPELL-EVERY-FIELD`, `CALL-FRAMES-NO-SOURCE-FIXTURE`,
`W37-SUBSTITUTION-SET-EQUALITY-UNARCHIVED`); `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`
went with `fa7ab464` (discharged) and `MAIN-GUARD-PACK-REFS-UNPINNED` with
`d7b39fac` (moot: the hooks were deleted; successor `MAIN-ADVANCE-POLICY-ONLY`).
Entries without `recorded_by`: **59 of 472** (MEASURED, this card), down from
67 by retirements only; no entry added since `6dc484ad` lacks it.

### 1.3 Brief-reach, re-measured — the discriminator is an implementing card's scope

MEASURED (sub-audit, `git log -S<name> 6dc484ad..HEAD -- docs`): **all twelve**
discharged names were named in the scope of the card that removed them (ten in
a wave-41 card's report from its first commit — `COMPOSED-WINDOW-REACH`'s
skeleton lists its four under "Card brief" — one by the wave-40 close, one
incidentally by `fa7ab464`). **None of the eleven open ones** reached an
implementing card; seven reached only a close, a compaction or the
restructure, and four reached no document at all. The wave-40 close's six
"tooling-card material" names discharged only where one rode inside an
implementing card (`PROOF-LEG-DIGESTS-UNCROSSCHECKED` in `CT-PROOF-LEG-3`).

Firing #4 said a residual is discharged when a close writes it into a card's
brief. This firing sharpens it (INFERRED from 12/12 against 0/11): **being named
by a close is not enough; being in an implementing card's scope is.** Waves 42–45
had no wave close report, which is why this cadence's own tooling residuals
aged.

---

## 2. Step 1 — reachability of changed fail-closed arms

### 2.1 The probe, and the one measured soundness hole

The gateway sub-audit inferred eight admitting-direction candidates by reading.
This card measured the four that decide a registration's severity with a probe
(`probes/zz_sr5_probe.rs.txt`, run as a temporary `crates/qcore-c-gateway/tests/`
target, `CARGO_INCREMENTAL=0 cargo test -p qcore-c-gateway --test zz_sr5_probe
-- --nocapture`, exit 0, output `probes/probe.log`; the file was then removed
and `git status` showed only this report directory). `Safe` over these
synthetic contracts promotes nothing.

| Probe row | Result | Reading |
|---|---|---|
| `5k-control-destroy`: `node_destroy(n);` | Safe | control |
| **`5k-member-then-destroy`**: `q_nullable_free(n->buf); node_destroy(n);` with `node_destroy` = `if (n->buf) q_nullable_free(n->buf); q_nullable_free(n);` | **Safe**, and **unit verdict Safe** through `analyze_unrestricted_c11` | `n->buf` is released twice. **MEASURED admitting hole** → `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` |
| `10c-control-unconditional` / `10c-control-if` | Safe / Unsafe (leak) | controls |
| `10c-ternary`, `10c-and`, `10c-or`, `10c-ternary-free` | Unknown ("resource call with a formal embedded in an unmodeled expression") | the sub-audit's expression-statement candidate is **refuted**: fail-closed |
| `10f-control-double-free-one-name` | Unsafe (DoubleRelease) | control |
| `10f-two-names`: two `mk_list_entry_first` names of one element, each unlinked and freed | Unknown (`UnknownAlias f/e`) | the foreign-unlink alias candidate is **refuted**: fail-closed |
| **`10d-root-released-after-publication`**: publish `e` into `q->items`, then `q_free(q)` | **Safe** | admitted under an unstated container-teardown premise → `PUBLICATION-ROOT-RELEASED` |

The 5k hole's mechanism, re-read: P4's admissible release accepts any callee
whose contract in force releases its only parameter
(`crates/qcore-c-gateway/src/legacy_member_release.rs:377`), inferred summaries
are installed in that environment (`legacy_summary.rs:272-275`), and the
call-site guard refuses only a *tracked* sub-custody.
`tests/unit/legacy_member_release.rs:201` pins the opposite order only. It is the
same family as the double member release the integrator repaired at `5275d58f`:
that repair made the member release the last use of its place *within the body*;
the second release here lives in a callee's summary. Corpus reach is
unmeasured (INFERRED to be rare: the shape needs a caller that frees a member
and then calls the destructor).

### 2.2 Other admitting-direction candidates, INFERRED and registered

From the gateway sub-audit, each re-read at its cited lines, none probed:
`COUNTED-USER-CONDITIONAL-OPERAND` (the counted-user walker applies calls behind
`?:`/`&&`/`||` on every path, and reads `f(a)(b)` as non-opaque against 10c's
rule 2), `CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER` (5l(2)'s binding over a formal
escapes 10d's formal check at `legacy_container_publication.rs:518-519`),
`DECLARED-NEUTRAL-UNVERIFIED-AT-CALL` (MEASURED by reading
`legacy_counted_users.rs:645-647`: declared neutrality returns before any body
check), `SCALAR-READ-PARENTHESISED-DOUBLE-DEREF`,
`GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK` (5l's premise text is stale
since 10f binds the element) and `STATIC-ROOT-SHADOW-READER` (low).

The sub-audit also confirmed the two holes the integrator already fixed are
pinned: the double member release (`5275d58f`,
`tests/unit/legacy_member_release.rs:105,201`) and the skipped `(f)(args)`
(`56829d8e`, `tests/parenthesised_callee.rs:133-185,393-457`). Its sweep of the
other statement forms (`(void)f(x)`, `p->fn(x)`, `(*fp)(x)`, comma, `({…})`,
`sizeof`, compound literals) found each classified or refused.

### 2.3 Arms no fixture drives

MEASURED by the sub-audits' greps of the reason text over every test
directory (spot-checked by this card; e.g. the counted-user `PATH_BOUND` at
`legacy_counted_users.rs:1003-1004` is named by no test). Every arm below fails
closed by reading; deleting any turns no test red.

* **Gateway, wave 45** → `GATEWAY-W45-REFUSAL-ARMS-UNREACHED`: ten 10e arms,
  10d "projection cannot name" and the four container-effect guards on
  borrowed-identity routes, 5l checked-spec `Unreadable`; and
  `tests/refused_call_hand_on.rs:113`, named a local-container case, refuses for
  nullability instead. Driven arms (5j, 5k, 10c, 5l, most of 10d/10f) are
  tabled in the sub-audit's output, summarised in the entry.
* **Condition desugaring, wave 44** → `CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED`:
  six variants (`legacy_condition_desugar.rs:140-184`) whose reason keys occur in
  no file but their source (MEASURED, this card), and six "unexplained" leaves in
  `w44-branch-condition-calls/residuals.txt:17-21` its README omits.
* **Revision mode** → `PIN-NOT-IN-LOCK-ARM-UNREACHED`; **subcommand plugins** →
  `SUBCOMMAND-FAILURE-ARMS-UNREACHED`; **checkpoints and 10g** →
  `CHECKPOINT-REFUTED-DIGEST-MISMATCH-UNDRIVEN` (the refuted-table digest *is* in
  the `.qo` identity, `checkpoint.rs:191-196`, but no test drives a mismatch, and
  eleven further resume/compose refusals are undriven) and
  `REFUTED-TWO-PHASE-ROUTE-UNTESTED` (the two-phase route has unit tests only;
  `--jobs` independence with a non-empty table is a slice observation).
* **Driven, for the record** (MEASURED by the sub-audits, cited in their
  outputs): all three Pending reasons end to end through `quirin-audit`
  (`pending_review_report.rs:118`, `contract_revision_audit.rs:187`,
  `user_contract_plugins.rs:382,425`); plugin timeout/crash/malformed → Unknown;
  checkpoint resume on stale identity, tampered and re-pinned files, with
  Pending surviving the round trip (`checkpoint_cli.rs`); the held refuted call
  is Unknown, never Pending (`refuted_contract_withdrawal.rs:466`).

**The Clang guard family grew from 750 to 1,005** early-returning tests
(MEASURED by the waves 41–44 sub-audit with `git grep -c`), and every
end-to-end Pending test above is one of them. That is
`RETURN-CALL-TESTS-CLANG-GUARDED-GREEN`'s family, not a new residual; this gate
recorded Clang present (§6).

---

## 3. Step 2 — model proof versus observation

### 3.1 The four-value verdict

* **Proved** (Lean, of a model): `QCore.VerdictSemigroup` — join associative,
  commutative, idempotent with `Safe` as identity, `pending_never_folds_to_safe`,
  `pending_only_from_a_human_resolvable_reason`, the firewall theorems; 34
  theorems in `AuditVerdictSemigroup.lean`. `QCore.ReasonCategory` pins the three
  human-resolvable categories by `rfl`.
* **Observed** (Rust tests on enumerated inputs): the order
  (`crates/qcore-schemas/src/verdict.rs:51-57`, derived `Ord` in declaration
  order, pinned by `the_order_is_safe_pending_unknown_unsafe`), `join = max`,
  `is_safe` only for `Safe`. The binding is **transcription** (a doc comment and
  mirroring tests; no digest).
* **Pending never reaches Safe:** MEASURED — no production path found where it
  does (every gateway positive gate is `== Safe`; the only 4→3 projection maps
  Pending to Unknown; Pending is counted separately in every tally).
* **Pending only from a named reason:** **false in one measured case.** A source
  that spells a held-call prefix publishes Pending: probe rows `held-pin-prefix`
  and `held-review-prefix` are Pending where `zz(0);` is Unknown, and a unit
  holding only the first is a Pending unit verdict
  → `HELD-PREFIX-SOURCE-FORGES-PENDING`. The classification is lexical
  (`legacy_c.rs:7156-7171`, `legacy_verdict.rs:76-78`); the comment "which is
  `Unknown`" at `legacy_pin_hold.rs:37-39` and
  `W45-REVIEW-PREFIX-COLLISION-UNPINNED`'s "so no verdict moves" became false
  when held calls moved to Pending. Two INFERRED siblings:
  `PENDING-BYPASSES-UNIT-SAFE-GATES` (the unread-definition and scope gates run
  on the Safe arm only, `legacy_c.rs:2111-2186`; the probe could not engage the
  gate through the API without a selection) and
  `PLUGIN-UNSAFE-MISBOUND-EVIDENCE-PENDING` (MEASURED order at
  `plugin_firewall.rs:218-241`: an Unsafe claim is Pending before its evidence
  binding is checked).
* **The bare `Verdict::Pending` deserialization question** (the coordinator's):
  `Verdict` derives `Deserialize` and `Verdict::Pending` is public; **no
  production code serde-deserializes a `Verdict`** (MEASURED, rg). But four
  read-back paths parse a status string `"Pending"` with no reason code
  (`checkpoint.rs:443-449`, `compose.rs`, `user_plugins.rs:459-462`,
  `revision_compare.rs:188-191`) while `subcommand_plugins.rs:616-623` demands a
  human-resolvable code or answers Unknown. Only the non-Safe direction is
  affected. Spec §L21.2 (`QCORE_SPEC_r0_3.md:1295`) says "no other `Pending` can
  be constructed", true of `ReasonedVerdict` only
  → `PENDING-UNTYPED-OUTSIDE-REASONEDVERDICT`.

### 3.2 Revision mode, plugins, checkpoints, 10g, the memo

* **Revision mode.** Missing lock, source change and contract change → Pending
  (`contract-pin-mismatch`); malformed or unknown-version lock → the audit is
  refused; v1 lock migrates; flexible contracts are a recorded assumption by the
  rule's choice. `pin --check` exits 1 on mismatch and 2 on refusal, both tested.
  Undriven arms in §2.3.
* **Checker plugins.** Safe needs a `CoreCertificate`, issued only after a
  trusted `WitnessChecker` replay; the CLI supplies no checker, so every plugin
  Safe is Pending on the command line; a plugin link joins upward only
  (`user_plugins.rs:484-488`). **No uncertified Safe is published** (MEASURED,
  sub-audit).
* **Subcommand plugins.** `classify_claim` has no certified arm at all; the route
  writes no `.qo` and feeds nothing to the matrix. `W45-SUBCOMMAND-HOST-REPORT-TRUST`
  is **accurate but under-scoped** (INFERRED from the code it cites): the host
  Safe also reaches the aggregate verdict and the invocation record, a subset
  report gives an aggregate Safe over the subset, and the result carries none of
  the host's profile, scope or assumptions. Put on the integrator list (§7), not
  re-registered.
* **Checkpoints.** Composition independence is **proved** in
  `QCore.Checkpoints` (`compose_eq_whole`, consecutive groups in plan order) under
  the premise `Determined` (the identity captures every input), and **observed**
  byte-identical on a two-group fixture at jobs 1 and 4. Two MEASURED code
  defects in the two-phase interaction: `CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES`
  (both phases write `D/<label>.qo`, `checkpoint_run.rs:117`,
  `current_route.rs:273-279`) and `CHECKPOINT-PHASE-ONE-REPORT-PUBLISHED` (phase 1
  composes into the published report before phase 2 runs,
  `checkpoint_run.rs:169-180`).
* **Rule 10g.** `--jobs` independence is proved in a model under the stated
  premise `byMembership` and observed once on the slice. The table is **not
  checked to be a fixed point**: the premise "a held call can only make a
  refutation undecided, never create one" is stated unproved
  (`gateway.md:877-879`), the checkpoint route discards phase 2's own table
  (`current_route.rs:279`), and nothing compares published with applied
  refutations → `REFUTED-TABLE-FIXPOINT-UNCHECKED`. Held calls are Unknown, never
  Pending (MEASURED: `stale-evidence` is not human-resolvable).
* **The memo removal.** `removing_the_memo_changes_no_answer`
  (`proof/QCore/MemoKey.lean:148-151`) is the symmetric form of `run_eq_map` and
  says exactly what the removal needs; it is a transcription, and with no memo
  the premise it rests on is moot. The two deleted tests tested only the memo;
  the kept interleaving test gained absolute assertions. MEASURED clean.
  The baseline comparison (`w45-memo-memory/evidence/`) is byte-identical on
  diagnostics at the same Fluent Bit revision.

### 3.3 The transcription family, sized

Firing #4 asked for a census of every "proved in Lean" claim against what binds
it to Rust. MEASURED (sub-audit; this card re-verified the count, 70, with
`git diff --name-status --diff-filter=A 6dc484ad HEAD -- proof/QCore`, and the
two mechanical bindings): of **70 Lean modules added** since `6dc484ad`,
**2 are mechanically bound** (`ScalarCalls`, read by
`compiler/tests/source_scalar_calls.rs:430`; `Ct50LifetimeIdentity`, digest
cross-checked), **40 are transcriptions a Rust doc comment names, 27 are
transcriptions only a report names, and 1 (`EventObservation`) names no
counterpart**. Only the two Cycle 18 routes disclose "review-maintained" in a
verdict's assumptions. This is trust boundary 28 quantified, not a new trust
boundary → `LEAN-PARITY-TRANSCRIPTION-CENSUS`.

A sharper instance: `proof/QCore/IndexTraversal.lean:484-508` models the
repaired `out[m++]` hole with the recogniser's output as a `Bool` input, so the
hole (which lived in computing that flag, `typed_field_access.rs:2162-2175`) is
invisible to any theorem → `LEAN-MODELS-TAKE-RECOGNISER-OUTPUT-AS-PREMISE`. And
the one wave-41 mechanical binding, LoopWindows, never builds a `Lend` event in
its shared grammar (`composed_window_reach.rs:746-790`) →
`LOOPWINDOWS-LEND-CLASS-MAPPING-UNCHECKED`.

Per-row assumptions are absent from published audit rows: the slice's
provisional Unsafe rows rest on the synchronous-proxy premise and on unchecked
neutrality, and the corpus's libc Safes on declared contracts, all disclosed at
run level only (MEASURED: `resource-diagnostics.json` unit rows have no
assumptions key; `report.rs:301-304` writes them for no-build units only)
→ `AUDIT-ROW-ASSUMPTIONS-UNRECORDED`.

---

## 4. Step 3 — semantic edits by merges, hotfixes and mechanical work

### 4.1 Found by this card and already corrected at `33558c67`

`8c1f15eb` corrected the W45-AUDIT-SUBCOMMAND-PLUGINS integration note to "333
summaries". MEASURED on this card's gate at equal code: **331** strict
`^test result: ` lines (the runner's totals line), **333** loose; the extra two
are the test names beginning `result_` — the artifact
`GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT` names. The integrator corrected the
note and the matrix copy to 331 at `33558c67`, crediting this card. **Judged:
the process still lacks a guard** (the figures are hand-summed into the note),
so this card appends the recurrence and names the guard as the discharge:
records take the summary figure only from the runner's totals line or
`summary.json`, and `build_matrix.py --gate` refuses a note that disagrees.

### 4.2 The wave-45 merges and the named mechanical work

MEASURED (sub-audit, combined diffs; the cited hunks re-read by this card):

* **Integrator-own code in wave-45 merges:** 2 of 23 merges carry any; both
  keep meaning (`7de5edf6` replaces a literal with the constant of equal value;
  `7546ff88` is a pure union of hooks).
* **Rule renumbering 10f → 10g:** clean. Every reference in code, Lean, spec,
  rules, increments and matrix points at the right rule; the refuted-contract
  card renumbered itself before merging.
* **Pin 47 → 49** (`b981c5c4`): a module count in
  `providers/tests/test_contract_load_path.py`, not a verdict; the two modules
  came from W45-FB-RETRY-LIFETIME; now 50 after `storage_backlog.qcore`, and 50
  reproduces from `providers/contract-load-path.json`.
* **Memo removal:** a pure performance change (§3.2).
* **Tree-walk conversions** (W45-TEST-HYGIENE): no pinned count changed; the
  helper lists `git ls-files` under the directory it is given and fails closed
  without git; no standing-gate step runs outside a checkout (CI checks out;
  `providers/package.py` compiles no test). **Constraint 29 introduces no
  git-less gate dependency** (MEASURED). One conversion,
  `providers/conformance/tests/packages.rs:766-795`, now pins tracked templates
  while `package.py:25` globs the working tree — a test that mirrors a
  production walk, which rule 29 says keeps its walk. Already covered by
  `W45-PRODUCTION-WALKS-READ-WORKING-TREE`; put on the integrator list.
* **Semantic edits without a card, both refusing-direction and recorded, neither
  with Lean:** `7edfd6b9` (sizeof type fold) → `SIZEOF-TYPE-FOLD-NO-LEAN`; merge
  `b89e0a45` plus re-pin `b79aa07e` (an Any-stepped exit is not a published
  derived result, `legacy_c.rs:10191-10194`) → `TRAVERSAL-DERIVED-EXIT-STEPPED-ANY-NO-LEAN`.
* **`fa7ab464`** (Windows portability) relaxed production integrity digests for
  CRLF — `bundle.rs:82-92` on Windows only, `pinned_lifecycle.rs:242` and
  `native_platform_matrix.rs:585-592` on every host — with no record naming them
  → `CHECKOUT-CRLF-DIGEST-ACCEPTANCE-UNSCOPED`; its Windows F02 run records no
  revision → `WINDOWS-F02-RUN-REVISION-UNBOUND`.
* **The census-sink guard** added by the integrator at `fc0fafcb` runs only
  under `--features indirect-census` in CI; the standing gate never runs it,
  and main carried an unlisted sink from `66ce7c7f` to `5b7942bb`
  → `FEATURE-GATED-SINK-GUARD-OUTSIDE-STANDING-GATE`.
* **Citations:** every SHA in wave 45's 21 `integration/gate.json` files is an
  ancestor of `main`; card reports still cite squashed WIP tips, and one README
  calls a production return-type change "test-only"
  → `CARD-TIP-CITATION-SQUASHED-WIP`. The integration rule's path
  (`integration-gate.json`) is not the path any card since wave 43 uses
  (`integration/gate.json`; 54 against 15 old) → `INTEGRATION-GATE-PATH-RULE-DRIFT`.
* **STATUS's "Latest validated gate"** (`docs/STATUS.md:95-104`) still
  publishes the 2026-09-21 gate (5,043 / 165 / 336 / 123)
  → `STATUS-LATEST-GATE-STALE`. Reported, not edited: DOCS-COMPACT-9 owns that
  prose.
* Six smaller record statements that disagree with their own evidence are
  collected in `WAVE-45-RECORD-ERRATA`.

---

## 5. Step 4 — later cards against earlier premises

* **W45-USER-CONTRACT-PLUGINS against W45-CONTRACT-REVISION-MODE and
  -DEFERRED-REVIEW:** moving held calls from Unknown to Pending made the earlier
  cards' "a prefix collision can only refuse" false (§3.1).
* **W45-CONTAINER-ROOT-REACH against 5l:** the foreign unlink binds an element
  5l assumes untracked → `GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK`.
* **W45-REFUTED-CONTRACT-WITHDRAWAL against W45-AUDIT-CHECKPOINTS:** the digest
  joined the identity correctly (schema 2), but the two phases share one path
  and phase 1 publishes first (§3.2).
* **W45-MEMO-MEMORY against 5k:** no premise moved (since `def46837` the memo
  answered only on full key equality).
* **Fluent Bit, the provisional-Unsafe policy.** MEASURED (sub-audit, this card
  re-reading each cited line): **no wave-45 record presents a provisional Unsafe
  or a local native observation as an upstream defect.** No native run supplies
  an Unsafe: the slice's only Unsafe rows are OR-1 and OR-7, static
  `counted_user` refutations. Four wording and premise gaps are registered:
  `NATIVE-OBSERVATION-UPSTREAM-WORDING` (STATUS:44 "two source-confirmed
  upstream double closes", a W43 observation, and `wave-45.md:28`),
  `W45-FB-SYNCHRONOUS-PREMISE-UNEVIDENCED` (the OR-1 premise's source search is
  not archived), `EP-CENSUS-PREMISE-ON-UNDECIDED-OBLIGATION`, and
  `PROVISIONAL-UNSAFE-VALIDATION-STANDARD-UNSTATED` (a user policy decision:
  whether "validated" means projection-validated under named premises or proved
  feasible). The re-pin from `864c3d97` to `046accf6` is recorded with byte
  identity per file; one discharged observation entry is stale at the new pin
  (`WAVE-45-RECORD-ERRATA` item 5). Two plan cards are prose-only
  → `FB-PLAN-CARDS-UNTRACKED`.
* **Platform:** a macOS run of the standing gate (`wave-45.md:20`) recorded 13
  Rust and 15 Python failures no entry tracks → `MACOS-GATE-FAILURES-UNTRACKED`.

---

## 6. Gate

Run detached by [`scripts/gate.sh`](scripts/gate.sh) at **`cb07510c`** (the base
`8c1f15eb` plus this card's documentation-only skeleton, so the code measured
is the base's byte for byte), `CARGO_INCREMENTAL=0`, every exit in its own file,
no pipe; logs in the worktree-private `measure/gate-cb07510c/`; exits and the
Clang recording copied into [`gate/`](gate/).

| Step | Exit | Figure |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a2` |
| `python3 scripts/run_test_binaries.py` | 0 | **319 units, 331 summaries; 5,995 passed; 0 failed; 11 ignored** (`private/test-binaries/20260926T161112Z-1623871/`) — equal to the W45-AUDIT-SUBCOMMAND-PLUGINS integration gate, as it must be |
| `cargo clean --profile dev` | 0 | |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **Passed 207 Lean audits**; `ls proof/Audit*.lean` = 207 |
| `python3 -m unittest discover -s scripts/tests` (base) | 0 | **Ran 364, OK** |
| `python3 scripts/build_docs_index.py --check` | 0 | |
| `clang --version` (this worktree) | 0 | Ubuntu clang version 21.1.8 (6ubuntu1) |
| probe (§2.1), once, then removed | 0 | 22 `PROBE` rows, `probes/probe.log` |
| `scripts/register.py` | 0 | `named_followups` 472 → 512 (+40 −0), one amendment |
| `build_matrix.py . --gate docs/reports/w45-audit-subcommand-plugins/integration/gate.json` | 0 | named followups 512, lean audits 207, contract modules 114; the gate file the validation block already named, so that block is byte-identical. This card's own [gate.json](gate.json) passes the same checks (`--out` to a private path, exit 0) |
| `python3 -m unittest discover -s scripts/tests` (after) | 0 | **Ran 364, OK** |
| `python3 scripts/build_docs_index.py --check` (after) | 0 | |

The provider gate was not run: this card touches nothing under `providers/`
(constraint 18 not engaged); `task_context.py` listed its commands only through
its unknown-path fallback on `gate.sh`. **This card changes no Rust and no
Lean**: the figures show the base is green, not that anything changed.
**Nothing is promoted.**

---

## 7. Integrator reconciliation list

1. **The measured hole first.** `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` is a
   `Safe` published over a double release (§2.1). It deserves the next
   implementation wave's brief; its corpus reach is unmeasured, and a corpus
   census for the shape (a member release before a call to a destructor of its
   base) would size it. `HELD-PREFIX-SOURCE-FORGES-PENDING` second.
2. **Retirements: none.** No present entry was measured fully discharged.
3. **Registrations: 40**, all `recorded_by: STABILITY-REEVAL-5`
   (`scripts/register.py`), and **one amendment**, the dated recurrence on
   `GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT`.
4. **Amendments to other lanes' entries, proposed not applied:**
   `W45-REVIEW-PREFIX-COLLISION-UNPINNED` ("so no verdict moves" is false;
   point at `HELD-PREFIX-SOURCE-FORGES-PENDING`);
   `W45-SUBCOMMAND-HOST-REPORT-TRUST` (widen to the aggregate verdict, subset
   reports and the missing verdict tuple, §3.2);
   `W45-PRODUCTION-WALKS-READ-WORKING-TREE` (note that the packaging test no
   longer detects the divergence, §4.2); `INTEGRATION-TIP-CITATION-UNREACHABLE`
   (four wave-41 files, §1.2); `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN` (750 →
   1,005 guards).
5. **Prose defects for DOCS-COMPACT-9** (reported, not edited):
   `docs/STATUS.md:95-104` stale latest gate; `docs/STATUS.md:44` "upstream
   double closes"; `docs/agents/gateway.md:426-429` (5l) stale since 10f;
   `docs/agents/operations.md:119` names the wrong integration-gate path;
   `docs/development/audit-checkpoints.md:134-136` overstates resume.
6. **Brief-reach** (§1.3): the eleven open firing-#4 residuals and this card's
   tooling residuals discharge only inside an implementing card's scope. The
   next wave's briefs should carry them; waves 42–45 had no close report to do so.
7. **Next firing.** `STABILITY-REEVAL` #6 is due after wave 47. Highest-value
   ground on this firing's evidence: the **interprocedural composition of
   per-body gateway rules** (5k's hole and 10d's teardown gap are both a body
   rule meeting a callee summary), and the **held/Pending plumbing**, which is
   lexical where its Lean model is typed.
