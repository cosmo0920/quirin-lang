# GAPO-RETURN-CORRECTNESS

Card GAPO-RETURN-CORRECTNESS (wave 48, round 24), base `1a399e6a1` (local
`main`). Authority: the ledger entries `GAPO-RETURN-PROBE-COUNTER-RESTORATION`
and `GAPO-RETURN-POSITION-ROUTE-GAPS`, read by GATEWAY-ANALYSIS-PATH-OPTIMIZATION
([route map](../gateway-analysis-path-optimization/ROUTE-MAP.md) sections 4
and 9, findings 1–5). Rules: [15x](../../agents/gateway.md#constraint-15x),
[15y](../../agents/gateway.md#constraint-15y),
[15z](../../agents/gateway.md#constraint-15z); rule 11o amended. Lean:
`QCore.ReturnCorrectness` (`proof/AuditReturnCorrectness.lean`). Prediction:
[PREDICTION.md](PREDICTION.md), committed before code (`59faf9fde`).

## Reproduction of the cited claims

- **Finding 1** (return probes restore `next_site` only): reproduced by
  reading. `return_call_instruction`, both B01 probes and W45-K's
  `cast_call_instruction` restored `next_site` alone; `classify_simple` also
  moves `element_lowerings` (W43 element lowerings), the list frames (rule 3's
  `list_element_binding` grows a frame's protected names and links), the
  element origins (`propose`, `note_over_formal`), and through its callees the
  argument probe and the typed-position counters. The doc comment claiming
  `next_site` as the only mutation was wrong and is corrected.
- **Finding 2** (rule 12h re-classifies without restoring): reproduced; in
  the control `a_replacing_classification_leaves_no_site_gap` the two sites
  were 1 and 2 with the restore disabled (the first classification of
  `u = r_wrap((void *) t)` minted 0 and its replacement 1 more), 0 and 1
  with it.
- **Finding 3** (the return position asks fewer statement routes):
  reproduced, and one of them was a wrong `Safe`: after `q_free((void *) t)`,
  `return r_borrow_code((void *) t);` (W4), `return 1 + r_borrow_code((void *)
  t);` (B01) and `return (long) r_borrow_code((void *) t);` (W45-K) were each
  `Safe` with the hook disabled (the cast actual read as no place). 11w and
  10u at the return position keep a refusal (rule 15z (2)).
- **Finding 4** (11o sees only the first lowering): reproduced; `gr_die(t->x
  + n)` (lowered by W44) and `gr_fatalf("%p", (void *) t)` (lowered by rule
  12h's H-U through 11w's tail) kept their continuation at the base, so a
  release before them and after them was a double release (`Unsafe`).
- **Finding 5** (`variadic_tail_statement` keeps counters; the descriptor
  guard restores nothing): reproduced by reading.

## Task A: probe state restoration (rule 15x)

`legacy_probe_state.rs` holds one snapshot, `ParserProbeState`, of every
field a classification may move, with `restore_probe_state` and a scoped
`probe(|parser| Keep(..) | Decline(..))`. The parser's 44 fields are
classified by construction:

| Class | Fields | On a declined probe |
|---|---|---|
| probe state | `cursor`, `next_site`, `element_lowerings`, `element_origins` (admitted and pending), `argument_probe`, `typed_position_uses`, `condition_widening`, `variadic_tail_widening`, `variadic_tail_readings`, `typed_place_origin` | restored |
| monotone fact | `list_frames` (a frame's protected names and links), `element_origins.over_formal` | kept: a fact about the source statement; restoring it could only remove a refusal (rule 4, W45-SR5-SOUNDNESS "wherever the binding was proposed") |
| function fact | 26 fields: the token stream, contracts, declaration readers and flags | never written after construction |
| lazy cache | `typed_path_index`, `typed_root_declarations`, `declared_no_return`, `call_graph_components`, `desugar` | value-determined |
| census | `census_function`, `census_bindings` | labels |

`probe_state` destructures the parser without `..`, so a new field does not
compile until it is classified, and `legacy_c::probe_state::tests` pins the
classification against the struct's declaration and the snapshot's field list.
The restore replaced every declined path: the W4, B01 and W45-K probes, the
guards of `classify_simple` (opaque callee, rule 11v, list rule 4 and the W43
descriptor guard, which restored nothing), rule 11w's declined widening, the
W44 probe, the element and sub-object probes, the condition call probe, and
the 54 `(next_site, element_lowerings)` restores of the fallbacks. Rule 12h's
H-U0 now classifies the uncast statement from the state the first
classification started from.

Not parser state, not restored (registered `GAPO-DECLINED-PROBE-CONSULTATIONS`):
the parity rows a declined probe wrote and its contract lookups in rule 13f's
consultation log (a dependency over-approximation, never an omission).

**Measured alone** (binary `f70039e8…`): corpus, system and slice reports
byte-identical to BEFORE, the null-property streams identical, and the parity
streams identical once Clang's per-run node addresses are normalised
([parity-task-a.txt](artifacts/parity-task-a.txt),
[runs-task-a.txt](artifacts/runs-task-a.txt)). The leak was **not a verdict
defect** on these inputs: no artifact prints a site number, and no declined
probe's element lowering reached a counted loop.

## Task B: return-position gaps (rules 15y, 15z)

| Gap (route map) | Decision | Where |
|---|---|---|
| 11o no-return exit, any lowering after the first | **closed at every position**: a CFG pass settles every node that calls a declared no-return callee and still has a successor; the discarded whole call takes rule 11o's decision (path end where Clang says `noreturn`, refusal otherwise), every other such node is refused | rule 15y, `legacy_no_return_paths.rs` |
| 12h H-U0 at `return CALL(…)` | **asked** after the W4, B01 scalar-call and W45-K classifications (from the probe's starting state); rule 12i declines a returned call with a hidden cast, since W44's re-classification is not asked H-U0 | rule 15z (1) |
| 11w tail and `va_*` hooks at the return | **out of scope**: the return keeps the tail or `va_*` refusal | rule 15z (2), census `11w-tail` / `11w-va` |
| 10u substituted call at the return | **out of scope**: a K03 refusal stays, and a W4 admission with an unbound custody-taking position is refused by the flow (`arguments do not match resource contract`) | rule 15z (2), census `10u-k03` / `10u-unbound` |

A stored no-return call cannot reach the CFG: the loader rejects a
`resource_no_return()` declaration that does not return `unit` (found while
writing the controls), so the "stored" refusal arm of rule 15y is
unreachable through contracts; nested and carried calls are refused.

Controls (`crates/qcore-c-gateway/tests/gapo_return_correctness.rs`, fixture
`tests/fixtures/contracts/gapo_return_correctness.qv`): the site gap; the two
later-lowered no-return calls ending their paths (BEFORE `Unsafe`, AFTER
`Safe`); the same calls with a non-`noreturn` declaration refused (`Unknown`
naming `no-return contract of …`); rule 11o's own statement path end
unchanged; the returned call with a cast actual of a released place (W4, B01
and W45-K: BEFORE `Safe`, AFTER non-`Safe`); three `Safe` controls with live
or caller places. Each BEFORE was observed with the card's fix disabled in
this worktree (the 15y controls on the task-A tree, where rule 15y did not
yet exist). `tests/cg21_call_exits.rs`'s operand-position control keeps its
assertion; its comment now records rule 15y's amendment.

**Measured A + B** (binary `78bbf9bd…`), parity attributed in
[attribute-task-ab.txt](artifacts/attribute-task-ab.txt):

- Reports and null streams byte-identical on all three routes; **no verdict
  moved** ([runs-task-ab.txt](artifacts/runs-task-ab.txt)).
- Rule 15y: **119 nodes** settled in the corpus, every one `rd_kafka_crash`
  (the `rd_kafka_assert` expansion) in a W44 condition plan, at 38 lines of
  29 functions, all resolved `noreturn` and made path ends; the 29 functions
  are `Unknown` before and after on their other refusals. Consequence in the
  parity stream: rule 11d's `bound-untracked-root` rows turn `path_refused`
  false at 6 lines of 3 of those functions. System and slice: none.
- Rule 15z: **91** returned calls re-read uncast (50 punned, 17 + 10 + 2
  typed, 8 byte, 4 qualifier views) in 29 functions, every one still
  admitted (27 functions `Safe` before and after, 2 `Unknown`); rule 12i's
  105 whole-call consultations of returns with such casts are no longer made
  (none had admitted), nor the 18 W44 rows they wrote in
  `sort_by_map_elem_val_toppar_list_cnt`; the gap census found 3 returns,
  all `10u-k03` in `rd_kafka_ConfigResource_add_incremental_config`
  (`Unknown`), and no `11w-tail`, `11w-va` or `10u-unbound` return.

## Prediction against outcome

| Predicted | Outcome |
|---|---|
| A: no verdict moves; parity rows naming sites may differ | no verdict moves; parity identical (no row prints a site) |
| B 15y: 0-30 nodes, 0-5 functions move | 119 nodes (more than predicted: `rd_kafka_assert`'s plans), 0 functions move |
| B 15z: 0-2 functions Safe -> non-Safe | 0: all 91 re-read calls name live or untracked places |
| census: tens of returns | 3 (lower bound by construction) |
| wall time within noise | corpus 534 / 520 / 526 s, system 87 / 77 / 79 s, slice 42 / 38 / 40 s (BEFORE / A / A+B, [seconds.txt](artifacts/seconds.txt)) |

No Safe -> non-Safe move, so no correction is named; no non-Safe -> Safe move.

## Identity

| Run | Binary | Corpus | System | Slice |
|---|---|---|---|---|
| BEFORE (base) | `7d56671b…` | 935 / 9 / 3,890, `bdf5f1c8…` (the brief's) | 93 / 4 / 351, `d337ac1d…` | 52 / 1 / 347, 4 Pending, `0a68ae1c…` |
| A | `f70039e8…` | identical | identical | identical |
| A + B | `78bbf9bd…` | identical | identical | identical |

After the rebase onto `f4ea42ac7` (ROUTE-SELECTION-ORDER-RESTATED, which
touched no gateway code), the rebased tip's binary (`01273f2a…`) reproduced
the A + B run exactly: every report and null stream identical to BEFORE, and
the parity stream identical to the pre-rebase A + B stream row for row
([runs-rebased.txt](artifacts/runs-rebased.txt),
[parity-rebased-vs-task-ab.txt](artifacts/parity-rebased-vs-task-ab.txt));
wall time 515 / 76 / 38 s.

The system and slice report digests differ from the brief's (`f596d64b…`,
`ee98ad3d…`) with equal totals: those reports embed the measuring worktree's
paths. Each run's `run.json` `binary_sha256` matches its binary.

The round-12 `mk_vhost_set` tip probe was not run: `tip-contracts.sh` is no
longer in the scratchpad or the integrator-brief backup.

## Lean

`QCore.ReturnCorrectness`: `declined_restores_probe_state`,
`declined_keeps_facts`, `declined_without_new_facts_is_identity`,
`declined_answer`, `declined_probe_moves_no_site`,
`replacement_from_before_has_no_gap`, `replacement_after_first_gaps` (15x);
`no_return_never_continues` (a declared no-return call never continues at any
position), `settle_identity`, `settle_call_cases`, `settle_cases`,
`settle_keeps_refusal`, `settled_ends_or_refuses`,
`settled_path_end_only_when_established` (15y, over `QCore.CallExits.decide`);
`uncast_at_least_as_strict`, `cast_actual_hid_a_released_place` (15z).

## Follow-ups

Retired: `GAPO-RETURN-PROBE-COUNTER-RESTORATION` (each probe saves and
restores every probe-state field; a test pins the classification and that a
declined probe leaves the state unchanged) and
`GAPO-RETURN-POSITION-ROUTE-GAPS` (each gap asked with controls or recorded
out of scope in rule 15z, with the corpus measurement above). Registered:
`GAPO-UNCAST-ACTUALS-AFTER-LATER-LOWERINGS` (H-U0 at a statement still reads
only `classify_simple`'s instruction; 7 indicative statements) and
`GAPO-DECLINED-PROBE-CONSULTATIONS` (unmeasured).

## Gate

Standing gate on the rebased tip (base `f4ea42ac7`, code steps at
`f3f512256`; [gate.json](gate.json), exits in [gate/](gate/), logs
`private/gapr/gate-g2/`): `cargo check --workspace --all-targets` 0,
submodule 0, `cargo fmt --check` 0, `clippy --locked` 0,
`run_test_binaries.py` **487 units, 500 summaries, 7,938 passed, 0 failed, 14
ignored**, exit 0; both reevaluation probes 0; `check_lean_audits.py` **316**
audits passed (`AuditReturnCorrectness` added); the Python suite exited 1
before the matrix regeneration (its one failure: the committed matrix's Lean
count 315, derived 316), then `build_matrix.py --gate gate.json` 0, the
Python suite **365 OK** (09b), the docs index 0 and its check 0. No provider
gate: `providers/` is unchanged. An earlier gate on the pre-rebase tip
(`gate-g1`) had the same shape (7,927 passed, 0 failed).

## Scripts

`scripts/`: `build.sh`, `measure.sh`, `measure-binary.sh`, `heavy.sh`,
`totals.py`, `compare.py`, `runs.py` (adapted from
QR-LEXICAL-SELECTORS-DECLARATION-NAMES), `parity.py` (full parity comparison,
Clang addresses normalised), `attribute.py`, `residual_casts.py`,
`archive.sh`, `update_ledger.py`, `gate.sh`, `write_gate_json.py`,
`finish-gate.sh`.
