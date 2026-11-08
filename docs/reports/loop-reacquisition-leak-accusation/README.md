# LOOP-REACQUISITION-LEAK-ACCUSATION: a leak joined across a refused re-acquisition

Round 26 implementation card (card-high). USER DECISION (2026-11-07,
verbatim): "Register four as implementation cards"; this is one of the four
registered by [CURRENT-UNSAFE-TRIAGE](../current-unsafe-triage/README.md).
Base `82ec0e80b` (integrate/r6); measurement side
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`. New gateway
[rule 42](../../agents/gateway.md#constraint-42) and Lean module
`QCore.LoopReacquisitionLeak`. Every Unsafe here stays provisional; nothing is
presented or filed as an upstream defect.

Commits: `0fb3c63ee` per-generation census (measurement only, behind the
`indirect-census` feature), `f2bd5c6e3` census evidence and
[PREDICTION.md](PREDICTION.md), `5de709b79` the rule (the analyzer the
measurement builds), then this report and the :1712 scope update (one
fixture and its Lean shape row, PREDICTION §5; no analyzer change).

## 1. The census

`QCORE_LEAK_WITNESS` now writes, beside each reported leak's
current-generation witness, the previous and older generations' witnesses,
with the generation, whether the settled analysis refuses the site's
re-acquisition and how many re-acquisitions the path passes. It ran on the
two rows of the user's `current` run (046accf6) whose leaks sit on an
acquisition line carrying `loop re-acquisition: an earlier generation that
may be live is still held` (`scripts/candidates.py`; the corpus baseline has
the same two, system and slice none), through `quirin-audit current` with a
two-unit selection ([evidence/selection.json](evidence/selection.json),
[scripts/run_census.sh](scripts/run_census.sh); binary
`3a7b31c2…` built at `0fb3c63ee`; audit slot, exit 0). It reproduces the
user's leaves exactly (snappy 7, scram 1, both units Unsafe;
[evidence/before/rows.json](evidence/before/rows.json)). Per reported exit,
the generations' status at the settled exit (`2` released, `4` escaped, `0`
absent, `u` unresolved effect) and whether a witness path reaches it
([evidence/before/table.json](evidence/before/table.json),
`scripts/census_table.py`; three invocations of snappy, four of scram):

| exit | current | previous / older | verdict |
|---|---|---|---|
| snappy :1648 (pass-2 length re-check) | live u, path | live u, path | **not a join**: the current generation's own path (acquire :1705, back edge, re-check, exit) passes no re-acquisition |
| snappy :1660 (pass-2 `uncompressed_length`) | live u, path | live u, path | **not a join** (same) |
| snappy :1680 (`rd_free(outbuf); return NULL`) | released | live u, path | **join** |
| snappy :1694 (`if (outbuf) rd_free(outbuf); return NULL`) | released | live u, path | **join** |
| snappy :1701 (`pass == 1 && uof <= 0`) | live u, path | live u, path | **not a join**: the current generation reaches the `pass == 1` arm again over the back edge |
| snappy :1712 (`unlikely(!outbuf)` arm) | absent (the null arm is refined at this base) | live u, path | **join** |
| snappy :1721 (`return outbuf`) | escaped in 2 invocations; live u in the third (the return is refused there) | live u, path | **join** (the current generation never reaches the exit reportable; see §4) |
| scram :249 (`return safe`) | live u, path, bound to `d` only | live u, path | **not a join** (CHAINED-ASSIGNMENT-INNER-DESTINATION) |

So the census **confirms** the loop-join attribution for snappy :1680, :1694,
:1712 and :1721 and **refutes** it for :1648, :1660 and :1701 (current
generation, path-insensitive pass guard: LEAK-PATH-SCALAR-CORRELATION and
LOOP-GENERATION-COUNTER-CORRELATION) and for scram :249. The rule is scoped
to the sites it confirms. Two findings outside the rule:

* **:1712 is a join, not a refused null test.** At this base the
  `unlikely(!outbuf)` failure arm removes the current generation (status 0);
  the leaf is the earlier generations', which the pass guard keeps live. The
  card's brief left :1712 to BUILTIN-EXPECT-NULL-TEST; the coordinator then
  moved it here (that card found its premise stale: rule 5h already reads
  `__builtin_expect(E, K)` as `E`), which matches this census, so all seven
  snappy leaves are this card's: four withdrawn, three kept.
  LEAK-UNDECIDED-NULL-TEST's snappy half is amended accordingly.
* **The settled exits reproduce none of the eight leaves.** At every reporting
  exit every generation, current included, is live with an unresolved effect
  (`settled: false` on all 75 witnesses). All eight are published from
  intermediate iterates, before the re-acquisition refusal's unresolved
  effect reaches the exit: C02-P-ALIAS-O's `-LEAK-ORDER` residual, which
  already names `rd_kafka_sasl_safe_string`. Reporting from settled states
  would withdraw all eight, but that residual records why it was declined
  (it withdraws true leaves elsewhere by the union join); amended with the
  snappy leaves, not repaired here.

## 2. The rule (gateway rule 42)

An earlier generation (previous or older) exists only because the site's
node ran again while the site was mentioned, so every witness of its leak
passes that re-acquisition with the earlier generation live: it joins two
generations of one acquisition. `report_leaks` now inserts a current
generation's `Leak` at once, as before, and **defers** an earlier
generation's (`legacy_loop_reacquisition_leak.rs`, hooks in
`legacy_transfer.rs`). After the work list a deferred report is
**withdrawn** exactly when the settled incoming state refuses the site's
re-acquisition (rule 11a's `ShiftRefused`, read by
`refused_reacquisitions`; the decision is monotone, so the settled state
decides as the node's last iterate did) **and** the function reported the
refusal at the site's acquisition line; every other deferred report is
inserted. A leak key is published when any identity other than a withdrawn
one reported it on any iterate, so more reports never withdraw more and the
order does not matter. Withdrawn keys go to the `QCORE_TYPED_PATH_PARITY`
stream (`position: "loop-reacquisition-leak"`).

* **Withdraws**: a leak only earlier generations of a refused, reported
  re-acquisition report (snappy :1680, :1694, :1712, :1721).
* **Keeps**: any current generation's report (a true leak in the iteration
  that acquired it; snappy :1648, :1660, :1701; scram :249); an earlier
  generation's report where the re-acquisition is not refused (rule 11a's
  own loop leaks); anything where the refusal is not reported.
* **Fail-closed**: a withdrawal needs the refusal in the same function, so the
  function is `Unknown` or, with another accusation, provisional `Unsafe`;
  never `Safe`. The rule only withdraws.

## 3. Fixtures and Lean

`crates/qcore-c-gateway/tests/loop_reacquisition_leak.rs` (real Clang
gateway, synthetic `q_*` surfaces; "before" measured on the same tree with
the decision forced to `false`, equal to `0fb3c63ee`):

| fixture | before | after |
|---|---|---|
| `snappy-shape` (one acquisition only pass 1 runs) | Unsafe, leaks 9, 17, 25, 28 | Unsafe, leak 9 (the current generation's pass-2 re-check), refusal kept |
| `refused-earlier-generation` | Unsafe, leaks 8, 13 | **Unknown**, no leak, refusal kept |
| `refused-earlier-generation-expect` (the same, failure arm `__builtin_expect(!p, 0)` as at snappy :1706/:1712) | Unsafe, leaks 8, 13 | **Unknown**, no leak, refusal kept |
| `refused-current-generation-leak` (a true leak inside the refused loop) | Unsafe, leaks 8, 10, 15 | **Unsafe**, leak 10 (validates) |
| `unrefused-loop-leak` (rule 11a's `for (;;)` leak, no refusal) | Unsafe | Unsafe, unchanged |
| `no-reacquisition-loop-leak` | Unsafe | Unsafe, unchanged |
| `no-reacquisition-loop-safe` | Safe | Safe, unchanged |

Unit tests (`tests/unit/legacy_loop_reacquisition_leak.rs`): every input of
the decision, and `WITHDRAW_VECTORS` equal to the Lean table. Lean
`proof/QCore/LoopReacquisitionLeak.lean` (audit
`proof/AuditLoopReacquisitionLeak.lean`, 15 theorems, standard axioms only):
`a_current_report_is_published`, `an_unrefused_site_is_published`,
`an_unreported_refusal_withdraws_nothing`, `the_rule_only_withdraws`,
`withdrawn_iff_every_reporter_is_withdrawn`, `more_reporters_never_withdraw`,
`a_withdrawal_names_a_reported_refusal`,
`an_earlier_generation_of_a_refused_site_is_withdrawn`,
`a_withdrawal_is_never_safe`, `a_withdrawn_key_has_its_refusal`,
`a_fully_withdrawn_function_is_unknown`, `the_rule_moves_no_verdict_to_safe`,
`withdraw_vectors_hold`, `the_vectors_name_site_seven`, `shape_vectors_hold`
(the fixtures' verdicts, bound by `lean_shape_vectors_match_the_rust_table`).
Not proved: that `report_leaks` hands each identity's generation to the
decision and that the refused set is read from the settled states (trust
boundary 28, bound by the vectors and fixtures).

## 4. Prediction and measurement

[PREDICTION.md](PREDICTION.md) (committed before the rule): no status move on
corpus, system or slice; snappy loses :1680, :1694, :1712, :1721 and stays
Unsafe; scram unchanged; the current run's totals unchanged.

Card-local check on the current route, same two-unit selection, rule binary
`6c4861e9…` built at the rule's tree (with the census feature): exit 0; one
row differs from the before run
([evidence/after/row-diff.json](evidence/after/row-diff.json)):
`rd_kafka_snappy_java_uncompress` stays provisional Unsafe and loses exactly
the four predicted leaves; nothing added; the other 73 rows of the two units
and both unit statuses are byte-identical in status and diagnostics. In
every invocation of snappy, :1721 was reported only by earlier generations,
although the census's may-path keeps the current generation live (but
unresolved) to that exit in one invocation: the rule reads which identity
reported, the census which may reach.

**snappy.c does not leave Unsafe from this rule**, as acceptance 3 asked:
the census refutes the attribution for its three remaining leaves. No Safe
moved. The corpus/system/slice and full `current` measurements are the
integrator's (`quirin-audit measure --declared PREDICTION.md`).

## 5. Gate

`python3 scripts/gate_scope.py --base 82ec0e80b --out <worktree>/private/gate-N`
(mode full: the diff reaches `qcore-c-gateway`; provider gate not
assigned). Logs are worktree-private, consoles outside the worktree.

| attempt | head | steps | Rust suite | Lean | Python |
|---|---|---|---|---|---|
| `private/gate-1` | `d49b56390` | 8 / 8 exit 0 (submodules, fmt, clippy, rust-suite, both reevaluation probes, lean, python) | 616 units, 9,282 passed / 0 failed / 16 ignored / 135 filtered, 657 summaries | 415 audits | 685 tests, 0 skipped |
| `private/gate-2` | `b297b667a` (final code and test tip) | 8 / 8 exit 0 | 616 units, 9,282 passed / 0 failed / 16 ignored / 135 filtered, 657 summaries | 415 audits | 685 tests, 0 skipped |

The `:1712` fixture is a row of an existing table test, so the count does
not move between the attempts. The timing-sensitive
`test_gate_slot_starvation.py` case passed in both. Outside the gate:
`cargo clippy -p qcore-c-gateway --features indirect-census --lib -- -D
warnings` exit 0; `apply_amendments.py --check` exit 0.

## 6. Follow-ups

* No new follow-up id. Amended: LOOP-GENERATION-COUNTER-CORRELATION (its four
  snappy leaves are now reported under the refusal; three pass-guard leaves
  remain), LEAK-PATH-SCALAR-CORRELATION (snappy's three leaves are the
  current generation's), LEAK-UNDECIDED-NULL-TEST (:1712 is a generation
  join at this base, withdrawn; BUILTIN-EXPECT-NULL-TEST is not amended
  here, its own records cover it, and its amendment of
  LOOP-REACQUISITION-LEAK-ACCUSATION must apply before this card's retire),
  CHAINED-ASSIGNMENT-INNER-DESTINATION (scram :249 is the current
  generation's, unaffected), C02-P-ALIAS-O (`-LEAK-ORDER`: the eight leaves
  are intermediate-iterate reports).
* Observed, not caused here: `cargo clippy -p qcore-c-gateway --features
  indirect-census --all-targets -- -D warnings` fails on base code
  (`clippy::duplicate_mod`, `tests/support/tracked_files.rs` loaded by
  `lib.rs` and `tests/unit/arm_obligations.rs`); the library alone
  (`--lib`) passes with the feature.

## Integrator measurement (2026-11-08)

Declared measurement, base `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`, tip `rev:5de709b79`, run from the
main checkout (`private/measure/loop-reacq-20261107T193437Z`): exit 0, all three routes
complete, no unmatched rows, declared result **match**.

- corpus: no row moved; 1 rows changed content only.
- system: no row moved; 0 rows changed content only.
- slice: no row moved; 0 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`08f2513a99a9122f2625288f19bfabb487ab688904af7edf62eaa53c48cdfeaf`).

`rd_kafka_snappy_java_uncompress` loses exactly the four confirmed leaves (:1680, :1694, :1712, :1721) and stays provisional Unsafe on the current-generation witnesses; it is not presented as an upstream defect.
