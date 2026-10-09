# CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS

Wave 48, round 21. Authority: the ledger entries `CG16-BOUNDS-CONTRACT-TRANSPORT`
(the CHUNKIO-META-LEN-BOUNDS-CONTRACTS amendment's five open items) and
`CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS`. Gateway rules
[14f](../../agents/gateway.md#constraint-14f)–[14j](../../agents/gateway.md#constraint-14j);
contract specification C09.1 (status paragraph "Body evidence, refutation
phase and tips"), C06.2, C06.4, C06.5; grammar G05b. Lean:
`QCore.BoundsBodyStanding` (18 theorems, `proof/AuditBoundsBodyStanding.lean`)
and `QCore.BoundsTransport` (14 theorems, `proof/AuditBoundsTransport.lean`).
Base: local main `84ec54d1d`. Prediction frozen in [PREDICTION.md](PREDICTION.md)
(`18f61b181`) before implementation code.

**Published property and scope.** CG16's `c.bounds.index-extent.v1` (rules
11s, 11e, 12l, 12r), with body evidence for its declarations. Custody is
unchanged. No verdict reads anything this card adds.

## 1. Task A: the rest of the transport

| Item | Outcome | Rule | Controls |
|---|---|---|---|
| Header-defined bodies on the routes (the one that matters most) | **Done, as evidence.** Phase one already analysed every declared body a unit defines, header `static inline` definitions included, to withdraw a refuted declaration in-unit; the result was discarded. It is now kept per unit and folded per declaration into `body_evidence` of the bounds selection record, with a bounds Safe roll-up (C06.5's shape, rules 13g–13k). | 14f, 14g | `crates/qcore-c-gateway/tests/bounds_body_evidence.rs` (4), `crates/qcore-audit/tests/bounds_transport.rs::a_header_body_is_evidence_for_its_declaration_and_not_a_row`, core vectors `crates/qcore-core/tests/bounds_body_standing.rs` |
| Cross-unit refutation phase on the routes | **Done.** The table is the union of every unit's refuting bodies (from the evidence, computed under the declarations as loaded); phase two runs when the custody or the bounds table is not empty, with the bounds table in `BoundsContracts::refuted`. | 14h | `a_body_refuting_its_declaration_withdraws_it_in_every_unit` (another unit's caller Safe → Unknown, never Unsafe), `the_empty_table_changes_nothing` |
| Tip tracking for bounds pins | **Done.** `pin --bounds --track-tip/--untrack-tip`; the record's `tip-in-force` standing, the tip record, the notice, `assumptions.tip_tracked` and the console notice, as C06.2 gives resource declarations. | 14i | `a_bounds_tip_entry_is_a_named_assumption_in_force_and_held_otherwise` (in force, other sites → `source-changed`, a deleted ref → `tip-unresolved`), `tip_vectors_match_lean` |
| `.qo` checkpoint transport | **Not done; refused instead of dropped.** Before the card a checkpointed `current` run with `QCORE_BOUNDS_PROPERTY` computed bounds rows into fragments no section carries and the composer never wrote: the requested rows were silently absent. It now refuses. | 14j | `checkpoint_cli.rs::a_checkpointed_run_refuses_the_bounds_selection` |
| G05a inclusion and inheritance | **Not done; kept refused.** The bounds loader refuses `#include` and `contract inherit`; the probe leaves such a module out of every view; a load path naming one in a bounds view refuses the selection. | 14j | `an_inheriting_bounds_module_never_reaches_a_bounds_view`, `bounds_contract.rs::malformed_declarations_reject_the_whole_module` (inheritance case added) |

**Header bodies: why evidence and not rows.** C09.1 and rule 12r define the
routes' denominator as each unit's main-source definitions, and the spec does
not admit header rows; this card does not change that. A header definition is
analysed once per including unit (each unit's own preprocessed body), so the
declaration is the right subject; a row would need a custody row to join
(there is none), and would multiply one function by its including units.

**Coverage, conservatively.** A `Safe`, unrefuted body covers its quant checks
and preconditions. A frame clause is covered only by a positive, syntactic
frame check (rule 14g): the refutation check (rule 11e(4)) does not follow
copies, so "not refuted" is not evidence of a frame. A derived relation is
covered by nothing even when every `return` returns the declared base and
offset, because its length is a fact about the caller's object. Return
extents, `bounds_preserves_extents()` and global extents are covered by
nothing.

**Why the two items are not done.** Checkpoints: a `.qo` file would need
bounds rows and body evidence in a section, the bounds environment's digest
and the bounds refutation table's digest in its unit identity (a reuse under
another environment or table must recompute), and composition of the bounds
selection record; a partial version would publish rows from a stale
environment. Inclusion and inheritance: the resource loader's agreement gate
compares loaded records; a bounds gate must compare quant predicates and
counts under the positional renaming, and the bounds environment has no
retained-evidence replay (resource environments do, in
`check_source_assumptions`). No bounds module needs either today (0 modules,
0 checkpointed bounds runs). Both stay on `CG16-BOUNDS-CONTRACT-TRANSPORT`
with their discharge controls named.

## 2. Task B: the chunkio neighbours

`lib/chunkio/include/chunkio/cio_file_st.h` is byte-identical at `66910c10`
and `046accf6` (sha256 `02eb3909…`). New module
[`chunkio/file_st_bounds.qv`](../../../providers/qcore-provider-chunkio/contracts/chunkio/file_st_bounds.qv),
in the crate's bounds view, pinned with `pin --bounds` at both revisions
([pin log](artifacts/pin.log), [check log](artifacts/pin-check.log): 5
recorded at each, exit 0):

| Accessor | Body | Declared | Body evidence (unit test on the pinned body) |
|---|---|---|---|
| `cio_file_st_get_hash` (l.70) | `return map + 2;` | `bounds_frame_reads(map)`, `bounds_return_derived(map, 2, 4, bytes)` | partly validated: the frame confirmed; the relation covered by nothing (its length is the caller's); the body's formation `map + 2` has no extent |
| `cio_file_st_infer_content_len` (l.104) | scalar arithmetic, one `get_meta_len(map)` | `quant checks map, Idx #i: 22 <= #i && #i < 24`, `bounds_frame_reads(map)` | validated (the escape to `get_meta_len` decided inside; the hand-off at a read position) |
| `cio_file_st_set_content_len` (l.116) | writes `map[10..13]` through `(uint8_t *) &map[10]` | `quant checks map, Idx #i: 10 <= #i && #i < 14`, `bounds_frame_writes(map)` | partly validated: the byte view (rule 14g) decides the four writes inside; the frame walk cannot follow the view |
| `cio_file_st_get_meta` (l.89) | `return map + 24;` | **not declared**: its length is the metadata length read from `map[22..23]` | — |
| `cio_file_st_get_content` (l.95) | `map + 24 + get_meta_len(map)` | **not declared**: an offset read from the object | — |
| `cio_file_st_get_content_len` (l.129) | the byte view of `map[10..13]`, `map[24 + meta_len]` under `size > content_offset` | **not declared**: `bounds_extent(map, size, bytes)` and an index read from the object | — |

**`CG16-BOUNDS-SYMBOLIC-RANGES` is not closed narrowly** for "a length read
from fixed bytes of a covered extent". The consumer keeps facts on locals and
parameters, never on object contents; a length read from `map[22..23]` would
need content facts that every write through an alias, a frame position or a
call withdraws, and no corpus caller has an extent to compare it with. The
missing vocabulary (a content-read length term with content facts and their
invalidation) is named on that entry.

## 3. Prediction against outcome

Binaries: BEFORE `e025473f…` built at the base `84ec54d1d` (a first BEFORE,
`7a842bbe…`, picked up this card's first edits during its build and was
discarded: the string `bounds-frame` is in it), AFTER `79b693b2…` built at
`a7ed36763` (the final code; later commits change tests and records only),
both under `private/cg16-bounds-transport-and-chunkio-neighbours/bin/` and
checked against each run's `run.json`. Scripts: [`scripts/`](scripts/);
comparisons in [`artifacts/`](artifacts/). **The routes read the load path from
the checkout at run time**, so BEFORE (base binary) already loaded this card's
three new declarations; a third run, BASE (the base binary over the base
contracts, [`measure-base-contracts.sh`](scripts/measure-base-contracts.sh)),
isolates them.

| Route | BEFORE | AFTER | Prediction |
|---|---|---|---|
| Corpus arbiter (custody) | 881 / 9 / 3,944, `ab8f5f14…` | identical | identical: held |
| System route | `fe93e739…` | identical | identical: held |
| Tail-pause slice | 45 / 1 / 354, 4 Pending | identical | identical: held |
| Typed-path parity (corpus / system / slice) | 683,320 / 139,962 / 50,407 rows | equal multisets | identical: held |
| Custody report with `QCORE_BOUNDS_PROPERTY` | `ab8f5f14…` | identical | identical: held |
| Bounds selection | 5 declarations, 2 modules, 5 in force, 0 held | identical | held |
| Bounds rows | `363f09c2…`, 162 Safe / 4,759 Unknown | `79a90176…`, identical verdicts, reasons and accesses | no verdict move: held |
| Combined | 111 Safe / 4,801 Unknown / 9 Unsafe | identical | held |
| Wall time (corpus / system / slice / bounds corpus) | 442.0 / 59.4 / 26.6 / 414.5 s | 441.6 / 58.9 / 26.2 / 429.8 s | within noise: held (one sample each, under other cards' load) |

[Identity](artifacts/identity-routes.json) (20 files, identical after erasing the
run's directory name and preparation durations);
[bounds rows](artifacts/bounds-compare.json) (0 verdict moves, 0 other
changes); [evidence summary](artifacts/evidence-summary.json).

* **The only row change** BEFORE → AFTER is the new `contract_dependencies` of
  11 rows (chunkio functions calling a declared accessor: `get_meta_len` 6,
  `get_hash` with `get_meta_len` 2, `set_content_len` 2, `set_meta_len` 1).
* **Body evidence.** All five declarations are examined in the same **6**
  units (`chunkio.c`, `cio_chunk.c`, `cio_file.c`, `cio_file_unix.c`,
  `cio_meta.c`, `cio_scan.c`), validated in every one: `get_meta_len`,
  `set_meta_len` and `infer_content_len` **validated** (every clause covered;
  bodies `Safe`), `get_hash` and `set_content_len` **partly validated**
  (uncovered: the relation `bounds_return_derived(map, 2, 4, bytes)`, whose
  body row is Unknown on its formation `map + 2`; the write frame, written
  through a byte view). Prediction: 7 units (wrong: the prediction's search matched a comment of
  `src/flb_input_chunk.c`, which deliberately does not include
  `<chunkio/cio_file.h>`); the three validated as predicted; `get_hash` predicted a declared
  assumption and is partly validated (its read frame is confirmed).
* **Safe roll-up.** All 162 bounds Safe rows rest on no declaration: no Safe
  row calls a declared accessor, as predicted.
* **Calls on `cf->map`.** 14 corpus calls reach the five accessors: 12
  call obligations (`get_meta_len` 8, `set_content_len` 3, `set_meta_len` 1),
  every one `bounds-missing-extent` on `cf->map`, and 2 `get_hash` calls,
  whose declaration states no precondition and so no obligation. Prediction
  said 14 with `set_content_len`: held, though 2 are not obligations.
* **Refutation table**: empty; no second phase ran. **No bounds Unsafe**.

**The contracts alone (BASE → BEFORE, both the base binary;
[comparison](artifacts/bounds-compare-base.json)).** BASE loads the base's two
declarations (rows `2f1d8395…`, 162 Safe / 4,759 Unknown). Adding
`file_st_bounds.qv` changes every row's environment digest (`79bd0d18…` →
`bd174836…`) and the reasons of **4 rows, no verdict**: in
`cio_file_format_check` and `cio_file_scan_dump` the `cio_file_st_get_hash`
obligation `states no bounds contract` disappears (the declaration states no
precondition; its result is the declared 4-byte view), and in `cio_file_write`
and `write_init_header` the 3 `cio_file_st_set_content_len` obligations now
read `requires quant checks map, Idx #i: 10 <= #i && #i < 14`, still
`bounds-missing-extent` on `cf->map`. So BASE → AFTER moves no verdict either;
the code adds only body evidence and the 11 rows' dependency field.

## 4. Lean

`QCore.BoundsBodyStanding`: coverage (`a_refuting_body_covers_nothing`,
`nothing_covers_these`, `body_evidence_needs_a_safe_body`,
`frame_evidence_needs_the_frame_check`, `a_safe_body_alone_never_covers_a_frame`),
units (`a_held_unit_keeps_no_evidence`, `a_refuting_unit_fails`), standing
(`a_held_declaration_is_held`, `no_examined_body_is_a_declared_assumption`,
`validated_needs_every_unit_and_clause`, `a_failure_wins`,
`partly_names_its_uncovered_clauses`), roll-up
(`only_validated_needs_every_dependency_validated`,
`an_unknown_dependency_is_an_assumption`) and the shared vectors.
`QCore.BoundsTransport`: refutation (`the_empty_table_changes_nothing`,
`its_own_body_keeps_it`, `a_refuted_declaration_is_withdrawn`,
`a_withdrawal_never_accuses`, `a_caller_of_a_refuted_declaration_is_never_safe`,
`mem_bounds_table_iff` from `QCore.RefutedContractWithdrawal`,
`the_table_is_stable_across_phases`), tips (`a_tip_is_in_force_iff`,
`an_out_of_force_tip_is_held` through `QCore.BoundsPinHold.standing`,
`a_tip_assumption_is_listed_iff_in_force`, `tip_vectors_hold`), checkpoints
(`a_composition_would_omit_the_requested_rows`,
`a_bounds_run_is_never_composed_from_checkpoints`). Not modelled: the C
bodies, the frame walk and Git (trust boundary 28; the Rust controls above).

## 5. Follow-ups

* Amended `CG16-BOUNDS-CONTRACT-TRANSPORT`: header bodies (as evidence),
  the cross-unit refutation phase and tip tracking landed; `.qo` checkpoint
  transport and G05a remain, each now refused instead of dropped (rule 14j),
  with their discharge controls named.
* Retired `CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS`: every accessor read at both
  revisions and either declared (3) or its missing vocabulary named (3, on
  `CG16-BOUNDS-SYMBOLIC-RANGES`), payoff measured (0 moves).
* Amended `CG16-BOUNDS-SYMBOLIC-RANGES` (the content-read length term and
  content facts, 3 accessors) and `CG16-BOUNDS-POINTER-EXTENTS` (14 calls on
  `cf->map`).
* Registered `BOUNDS-BODY-EVIDENCE-UNCOVERED-CLAUSES` (2 partly validated
  declarations: a relation's length and a frame through a byte view).
* Follow-ups 740 → 740.

## 6. Gate

Standing gate `g2` ([`scripts/gate.sh`](scripts/gate.sh), every step under the
shared lock, logs `private/cg16-bounds-transport-and-chunkio-neighbours/gate-g2/`,
exits copied to [`gate/`](gate/), summary in [`gate.json`](gate.json)), every code
step at `eb8268df9`:

| Step | Command | Exit | Count |
|---|---|---|---|
| 00 | `cargo check --workspace --all-targets` | 0 | |
| 01 | submodule update `--init --recursive` | 0 | |
| 02 | `cargo fmt --all -- --check` | 0 | |
| 03 | `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| 04 | `python3 scripts/run_test_binaries.py` | 0 | 466 units, 479 summaries, 7,773 passed, 0 failed, 12 ignored (`tests-gate-g2/`) |
| 05 | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| 06 | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| 07 | `python3 scripts/check_lean_audits.py` | 0 | 305 audits (303 → 305) |
| 08, 09 | Python suite, docs index check (before the matrix regeneration) | 1, 0 (08: the committed matrix carried `lean_audits` 303, the expected stale derived value) | |
| 09a, 09b, 10b, 10c | `build_matrix.py . --gate gate.json`, the Python suite, `build_docs_index.py`, `--check` | 0, 0 (365 OK), 0, 0 | |

Provider gate `g2` ([`scripts/provider-gate.sh`](scripts/provider-gate.sh), at
`eb8268df9`): `cargo check --locked` 0, `cargo test --locked` 0 (129 passed),
fmt 0, clippy 0, `contracts.py check` 0, `package.py build` 0, `package.py
verify` 0 for all five packages, `python3 -m unittest discover -s
providers/tests` 0 (25 tests).

The first gates (`g1`, code at `706a47dac`) failed one step each, both a pin
this card's new tracked bounds module moved and both updated deliberately:
standing step 04 (7,772 passed, 1 failed:
`bounds_contract::the_reserved_spellings_are_unused_by_tracked_contracts`,
tracked bounds modules 1 → 2) and provider p8
(`test_the_real_workspace_manifest_is_current`, chunkio's bounds view 1 → 2
modules); their other steps were 0 (Lean 305). `cargo clean --profile dev` ran
after the test step; the release targets under `private/` were deleted after
the measurements.
