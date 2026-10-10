# NULL-TRIAGE-UNRESOLVED-PATHS

Card **NULL-TRIAGE-UNRESOLVED-PATHS** (wave 48, round 23, 2026-10-11;
card-high). Base: local `main` at `c77efedbe`, rebased onto `d7b0882ac`
(NULL-MEMBER-INVARIANT-SCOPE landed; its `nonnull_` names adopted, section 4
re-measured on the rebased tip). Prediction:
[PREDICTION.md](PREDICTION.md), frozen at `aa4e15a84` before any
implementation code (its one later edit is the clause's rename, below).
Gateway rules [15q](../../agents/gateway.md#constraint-15q)–[15u](../../agents/gateway.md#constraint-15u);
contract specification C09.3 ("Exact and inexact paths"); grammar G05e
(`nonnull_result_range`); Lean `QCore.NullTriagePaths`
(`proof/AuditNullTriagePaths.lean`).

**Rule ids.** The brief assigned `15c`–`15g`, but the router already uses
`15a`–`15p` (source-routes, io, effects, buffers, concurrency topics). On
the integrator's correction this card uses `15q`–`15u`; every id is unique
across `docs/agents/*.md` (the duplicate-id check returns nothing).

**Clause name.** USER DECISION 2026-10-11, recorded verbatim in C09.3:
"null_result_range -> this contract element's name a bit of strange so
nonnull_result_range would be good." The element is `nonnull_result_range`;
the earlier spelling is refused like any unknown clause (a control in
`null_triage_unresolved_paths.rs`), and the cfl declaration was re-pinned at
both revisions after the rename. The landed `null_member_nonnull` and
`null_result_nonnull` keep their names here (NULL-MEMBER-INVARIANT-SCOPE owns
their renaming).

## 1. What changed

| Piece | Where | Rule |
| --- | --- | --- |
| A call's integer result is a value with a **bound** (C type, a declared range in force, the unit's validated summary of the callee), never free: the bound decides, anything else is inexact | `null_deref/witness.rs` (`AtomInfo::call`, `result_bound`), `null_deref/range.rs` | 15q |
| `ensures normal nonnull_result_range(LO, HI);` (bounds v1, null property only), checked against every analysed body (rule 13g's fold, `result_range_declarations`), withdrawn on a refuting body | `qcore-contract/src/bounds_contract.rs` (`BoundsClause::NullResultRange`), `range.rs`, `qcore-audit/src/null_fold.rs` | 15q |
| `cfl_list_size` in [0, 2147483647], body read at both revisions, pinned with `pin --bounds` (the cfl crate's first bounds view) | `providers/quirin-provider-cfl/contracts/cfl/null_deref.qv`, `bounds-contracts.lock`, `contract-load-path.json` | 15q |
| Two-phase search: the exact phase is rule 14r's; a second phase explores inexact arms (call result, correlated value or test, unfollowed value, ordered pointer comparison, such a switch) on both sides and draws only infeasibility | `witness.rs` (`phase`, `inexact_take`), `qcore_core::null_paths` (`explore`, `conclude`) | 15r |
| Exact relational facts: `x ± k` without wrap, `a + b` inside its type, two independent entry values through the difference (sum) interval; enumeration types read with their compatible type's range | `witness.rs` (`S::Off`, `S::Sum`), `null_deref/ast.rs` (`enum_ranges`), `null_paths::relation_arms` | 15s |
| A proved-infeasible candidate is removed (`path-search:infeasible` plus a proof per consulted declaration; assumption `path-search:`) | `null_deref.rs` (`infeasibility_proofs`), `qcore_core::null_deref::Proof::Infeasible`, `null_paths::removes` | 15t |
| A cached memory value dies at an assignment of a followed variable it spells | `witness.rs` (`forget_mentions`) | 15u |

Two soundness defects of the existing search were found while reading it and
are corrected here, besides the priority-0 one: a comparison of two values
took both arms whatever their ranges (rule 15s), and a cached `p->n` kept its
value across `p = q` (rule 15u). An enumeration type was read as `int`,
which an enumeration with no negative enumerator is not (Clang gives it
`unsigned int`); an enumeration whose range the unit does not state is now
unfollowed.

## 2. Priority 0: the false finding

`copy_label_values` (`cmt_cat.c:93`) was a provisional Unsafe whose path
needed `cfl_list_size() < 0` (`s == 0` false, `s > 0` false). With rule 15q
the comparison is inexact unless the bound decides it; with the declared range
`[0, 2147483647]` (validated by all 45 bodies the corpus analyses: the
summary of `ret = 0; … ret++` is `[0, INT_MAX]`) the `s < 0` arm is excluded.
**Correction (Unsafe → Unknown):** the finding is gone; the candidate is
**unresolved**, not infeasible as predicted: the second phase reaches the
`cfl_list_foreach` loop's 64-iteration budget on the `s > 0` path
(rule 11x's budgets are unchanged), so no removal is drawn.

## 3. Findings and the unresolved classes

[triage-corpus.json](artifacts/triage-corpus.json) (every searched
candidate), [triage-diff-corpus.json](artifacts/triage-diff-corpus.json)
(BEFORE → AFTER per candidate), [findings.json](artifacts/findings.json)
(every finding with path and excerpts from 66910c10).

| Corpus, 139 strong-source candidates | BEFORE `b1` | AFTER `a3` |
| --- | ---: | ---: |
| finding, `null-on-allocation-failure` | 30 | **24** |
| finding, `null-on-normal-path` | 3 | **5** |
| infeasible | 2 (kept as candidates) | **9 (removed)** |
| unresolved | 104 | **101** |

**Findings lost (8), each a correction: the path needed a call's result to
take a value no contract, type or summary establishes** (rule 15q):
`copy_label_values` (section 2), `mk_print` (`isatty`; normal path),
`mk_rconf_section_entry_add` (`mk_list_is_empty`), `mk_string_split_line`
(`strlen`), `mk_server_listen_init` (`mk_socket_server`),
`mk_utils_url_decode` ×2 (`mk_string_char_search`), `mk_vhost_read`
(`stat`). Read by hand: each of the seven others is a plausible allocation or
lookup failure, but its path also needs the callee to return a particular
value (`mk_list_is_empty`'s summary `[-1, 0]` bounds it without saying the
list can be non-empty there), which nothing checked establishes; they are
candidates again, not refuted.

**Findings added (4), all `null-on-normal-path` in
`rd_kafka_toppar_op_serve`** (`rdkafka_partition.c:2197/2202/2206/2210`):
`rktp` stays NULL when `rko->rko_rktp` is NULL (the function's own test
admits it), and the `switch ((int)rko->rko_type)` reaches the
`FETCH_START`/`FETCH_STOP`/`SEEK`/`PAUSE` cases, whose callees dereference
`rktp` (inferred clauses). Before the card the switch was over an
enumeration type the search did not follow; now its range is the
enumeration's compatible type (rule 15s). Read by hand: real-looking under
the entry premise; whether such ops always carry a partition is an invariant
of librdkafka's op queue that nothing declares. Provisional, never an
upstream defect.

**Removed (9)**, every path explored without reaching the dereference NULL:
`rd_kafka_mock_pid_new` and `mk_start` (infeasible already, kept before),
`rd_kafka_commit` (`repq` is created and destroyed under the same `!async`),
`rd_kafka_toppar_lag_handle_Offset` and `rd_kafka_toppar_handle_Offset` ×2
(`rktpar` is dereferenced only when `err == 0`, which a failed lookup
excludes), `rd_kafka_toppar_offset_request`, `rd_kafka_toppar_broker_migrate`,
`rd_kafka_mock_validate_records` (the last four through inexact arms only).
Read by hand: rd_kafka_commit and both handle_Offset functions agree with the
source.

**Unresolved (101) by class** (the class of the condition that stopped the
search; when the second phase reached the dereference NULL only inexactly,
its reason):

| Class | BEFORE | AFTER (inexact reach) | Extension |
| --- | ---: | ---: | --- |
| condition over a call's result (new: priority 0) | 0 | 57 (16) | 15q; ranges or achievability contracts needed |
| value not followed | 47 | 3 (1) | 15s (enumerations, offsets, relations); most of the 47 now stop earlier, at the call result in the same condition (`rd_slice_read`, `rd_slice_abs_offset`) |
| correlated condition | 32 | 30 (14) | 15r explores them; none proved: no frame or summary states what a call does not write |
| loop iteration bound | 14 | 5 | budget unchanged (rule 11x) |
| switch over an unfollowed value | 4 | 0 | 15s: 4 findings |
| case label in a nested statement | 3 | 3 | none |
| ordered pointer comparison | 3 | 3 | 15r explores; CG16's same-object reading not wired here |
| correlated pointer test | 1 | 0 | 15r: removed (`rd_kafka_toppar_broker_migrate`) |

## 4. Measurement

**Custody**: byte-identical on all three routes (corpus 915 / 9 / 3,910
`ded14632…`; system 91 / 4 / 353 `65e8024f…`; slice 50 / 1 / 349 / 4 Pending
`0ee2a3aa…`); the parity streams are multiset-identical (corpus 690,246 rows,
system 142,512, slice 51,342; [parity-corpus.json](artifacts/parity-corpus.json),
[-system](artifacts/parity-system.json), [-slice](artifacts/parity-slice.json)).

**Null property** (BEFORE `b1`, binary `9da12dbe…` at main `d7b0882ac` → AFTER
`a3`, binary `3ff018d6…` at `3b2cdb636`; [compare-corpus.json](artifacts/compare-corpus.json)):

| | corpus | system | slice |
| --- | --- | --- | --- |
| Safe unconditional | 1,113 → 1,113 | 84 → 84 | 71 → 71 |
| Safe conditional | 1,433 → **1,435** | 125 → 125 | 103 → 103 |
| Unknown | 2,353 → **2,355** | 245 → 245 | 232 → 232 |
| Unsafe (provisional) | 22 → **18** | 0 | 0 |
| candidates | 12,648 → **12,643** | unchanged | unchanged |
| findings | 33 → **29** (18 functions) | 0 | 0 |

Corpus row moves: Unsafe → Unknown **5** (`copy_label_values`, `mk_print`,
`mk_rconf_section_entry_add`, `mk_string_split_line`, `mk_utils_url_decode`;
named corrections above; `mk_server_listen_init` and `mk_vhost_read` keep
other findings); Unknown → Unsafe **1** (`rd_kafka_toppar_op_serve`); Unknown
→ Safe (conditional) **2** (`rd_kafka_mock_pid_new`, `rd_kafka_commit`, by
removal). **Safe → non-Safe: 0.** The other seven removals sit in rows with
further candidates. `cfl_list_size`'s range: `validated` (45 bodies examined,
0 refuted). System and slice: no searched candidate changes status (slice: 2
unresolved, now at `flb_log_check`'s result).

**Wall time** (inside the lock, null and parity selected; `b1` → `a3`):
corpus 508.8 s → 563.0 s (+10.6 %); system 75.4 → 91.8 s (+22 %); slice
34.0 → 62.9 s (+85 %). The second phase reruns the body per path up to the
budgets (1,024 paths, 40,000 steps): the slice's two candidates, both in
`do_start`, exhaust the 1,024-path budget. The host carried other load (load
average 7–14) during the AFTER runs, so these are upper bounds, not a
controlled comparison. The first measurement before the rebase (`b0` at
`c77efedbe` → `a2`, binary `e8141daa…`) gave the same triage, row moves and
custody identity (corpus 504.4 → 562.7 s, system 74.7 → 92.8 s, slice
33.8 → 45.7 s).

## 5. Prediction against outcome

| Predicted | Measured |
| --- | --- |
| Custody byte-identical | yes, all three |
| Findings 27–32 | **29** |
| Allocation-failure 26–30 | **24** (below: six rested on call results) |
| Normal-path 1–2 | **5** (above: four from enumeration switches, not predicted) |
| Null Unsafe rows 18–21 | 18 |
| Infeasible 10–50, removed | **9** (below), removed |
| Unresolved 50–95 | **101** (above: the call-result class absorbs most) |
| Value not followed 15–45 | **3** (below) |
| Correlated 5–30 | 30 |
| Loop bound 12–20 | **5** (below: the exact phase stops earlier) |
| `copy_label_values` infeasible with the range | **unresolved** (loop budget) |
| Null Safe 2,536–2,560 (+0 to +24 on the base) | 2,538 at `c77efedbe` (+2); 2,546 → 2,548 on the rebased base (+2) |
| Wall +0–10 % | **+11.6 %** corpus (`b0` → `a2`), **+10.6 %** (`b1` → `a3`) (above) |

## 6. Lean

`QCore.NullTriagePaths`, registered in `proof/QCore.lean`, audited by
`proof/AuditNullTriagePaths.lean` (15 theorems; `propext`,
`Classical.choice` and `Quot.sound` at most; no placeholder, no project
axiom): no free widening (`a_call_result_never_contributes`,
`a_witness_holds_whatever_the_callees_return`: a validated witness holds for
every valuation of the call results), the bound
(`a_bound_excludes_its_outside_arm`), exhaustive exploration
(`exhaustive_search_is_sound`: a search that decides branches only soundly
and explores both arms of every other one, reaching no leaf, means no
valuation the facts admit reaches one), relations (`difference_attains`,
`sum_attains`, `lt_arm_is_attained`, `an_unmet_arm_has_no_value`), the body
check (`counter_stays_above`), the conclusion
(`a_finding_needs_an_exact_path`, `infeasible_needs_a_complete_empty_search`,
`an_inexact_arm_is_never_exact`). Vector lists bound to the Rust
(`crates/qcore-core/tests/null_paths.rs`, parsed from the Lean text):
`exploreVectors`, `concludeVectors`, `armsVectors`. `QCore.NullDeref`'s
`Proof` gains `infeasible`. Trust boundary 28: that the search's facts are
sound for C (decided conditions, cached memory forgotten at calls, stores and
variable assignments) is a review premise, pinned by the controls.

## 7. Controls

`crates/qcore-c-gateway/tests/null_triage_unresolved_paths.rs` (10): a call
result is never free (`copy_label_values`' shape is unresolved); a declared
range bounds it and removes the candidate (row `Safe`, proofs and dependency
named); a refuting body takes the range out of force (and the body's own
summary then decides: a real finding); a validating body and an in-unit
summary bound the result; an unsigned result is never negative; a relation
takes an arm only when the intervals admit it (the corrected widening); an
offset is followed exactly; a variable assignment forgets the memory it
spells; inexact conditions (ordered pointer comparison, re-read memory, a
switch over a floating conversion) are explored for infeasibility only; the
loader reads the clause and refuses a pointer result, an empty range, a
non-canonical bound and the old spelling. `crates/qcore-core/tests/null_paths.rs`
(5).

### Deliberate test and pin updates

* `null_candidate_triage.rs`: the repeated-condition control's row is now
  `Safe` (the infeasible candidate is removed; rule 15t).
* `cfl/null_deref.qv`: `qcore_contract_corpus.rs` (one pinned row; corpus
  130 → 131 modules), `contract_loader.rs` (modules 130 → 131, resource-v1
  rejected declarations 246 → 247), `bounds_contract.rs` (tracked bounds
  modules 5 → 6), `RESERVED_SPELLINGS` 14 → 15.

## 8. Gate

See [gate.json](gate.json) and [gate/](gate/) (filled by the gate run).

## 9. Follow-ups

* **Amended:** `NULL-TRIAGE-UNRESOLVED-PATHS` (re-measured: 101 unresolved;
  the correlated class, 30, needs what a call provably does not write, which
  nothing states yet; loops 5, nested labels 3, ordered comparisons 3,
  unfollowed 3).
* **Registered:** `NULL-TRIAGE-CALL-RESULT-ACHIEVABILITY` (57 candidates whose
  search stops at a call's result, 16 of them reaching the dereference NULL
  only through it; 22 at `rd_slice_read`; seven former findings among them).

## Scripts

`scripts/`: `build.sh`, `measure.sh`, `measure-binary.sh`, `heavy.sh`,
`totals.py`, `triage.py`, `triage_diff.py`, `compare.py`, `parity.py`,
`artifacts.py`, `pin.sh`, `lean.sh`, `candidates.py`, `bodies.py`.
