# ARM-OBLIGATIONS-UNIT-GRAIN: carrying an admission only into the rows that rest on it

Card ARM-OBLIGATIONS-UNIT-GRAIN discharges `ARM-OBLIGATIONS-UNIT-GRAIN`
([ARM-OBLIGATIONS report](../arm-obligations/README.md), section 4). Base
`3e53c34fe`. The [prediction](PREDICTION.md) was committed first
(`ef7b0ce48`), then the implementation (`051aef7f2`).

## 1. What changed

ARM-OBLIGATIONS carried every obligation a unit's analysis reached into every
row of that unit. This card gives the unit frame an attribution
(`arm_obligations::FrameCarriage`):

- **Attributed to a declaration.** Rule 37's checked-specification admissions
  are per specification (`rd_calloc`, `rd_strdup`). The settlement now hands
  the frame `BTreeMap<specification, obligations>`
  (`legacy_checked_spec::Settled::carried`), and the frame keeps them under
  the specification's name.
- **Unit grain.** Everything else is carried by every row, as before: a host's
  admission asked during the analysis (rules 33 to 36, which return no
  obligation today), a nested analysis's carriage handed up when its frame
  closes (which rows of the enclosing unit read the nested result is not
  attributed), and the unit tests' injection.

A row carries the unit-grain obligations, plus the obligations of every
declaration its derivation reaches (`FrameCarriage::row`). "Reaches" is
computed at the end of the unit's analysis by
`contract_dependencies::attribution::reaching`. It works over notes the frame
collects while the analysis runs:

- every function's work: `within` opens a scope whenever a frame is open, and
  each scope's consultation log is added under that function, nested work
  included, exactly as rule 13f's recorder adds it;
- every body a CFG build reads (`body`): the identifiers the function spells
  in that stream, with unit rewrites, indirect-call candidates and minted
  names included;
- every decided callback candidate (`contract_dependencies::relied`, which
  the four `note_relied` sites now call). It is noted under every function
  whose work is running, and it is kept even when a declining probe rolls the
  consultation log back.

A function's direct names are its own name, its body's spellings (minted,
held and withdrawn spellings read back to their declaration, as rule 13f's
`read` does), its relied-on candidates and, for a row, every callee its
solved CFG applies (`cfg_callees`). A function that has work but no recorded
body contributes every name its work looked up, unfiltered. The closure
follows every function with work in the frame. That includes inferred
summaries, standing checked specifications and declarations withdrawn here,
not just rule 13f's inferred summaries. A row carries a declaration's
obligations when the declaration is the row itself or is reached through
that closure. If the reach set for a declaration is missing, the row
carries the obligation (`is_none_or`, fail-closed).

**Why this grain and not the two the follow-up suggested.**

- *Rule 13f's recording made verdict-bearing.* The audit opens that
  recording around the published analysis only. The certificate recheck
  (`check_legacy_safety_certificate`) reruns the analysis outside it and
  compares `program summary` for equality, `rule_obligations` included. A
  carriage read from the recording would differ between the two, and every
  `Safe` unit with a standing specification would fail containment. So the
  frame keeps its own notes, in every run. The recorder is neither read nor
  changed: with a recording open, `within` and `body` write it exactly as
  before. Rule 13f's "do not read the log from any analysis decision" now
  names this one exception.
- *A differential re-run* (analyse the unit again with the obligation-bearing
  admissions refused, and drop the obligation from every row whose result is
  unchanged) was considered and rejected. With rule 37's admission refused,
  `rd_calloc` is withdrawn and W4 validates the same non-null result from the
  asserting body without naming the premise
  (`RULE37-W4-ASSERT-PREMISE-VISIBILITY`). Callers would then be identical
  and shed `asserted_allocation`, while their verdict still rests on the
  deployed-build fact, only less visibly. That is the opposite of rule 37's
  "keep the premise visible".

**Soundness: what a dropped obligation rests on.** A row loses an
attributed obligation only when its closure does not reach the declaration.
That is sound under two premises, kept separate:

1. **Rule 13f's completeness claim**: every lookup of a declaration by a
   function's work is a lookup by name, noted in that function's log, and a
   lookup the body does not spell belongs to a whole-unit pass, not to the
   function. Here the claim is used in a weaker form, since the body's
   spellings are a superset of rule 13f's filtered lookups. Unit-level passes
   (outside every function's work) are not attributed to a row, as in rule
   13f. After the settlement, the unit-level steps of
   `analyze_translation_unit_under_holds` read no specification for a row:
   evidence collection, dialect refusals by token,
   `checked_spec::diagnostic` (the specification's own row) and the
   cmetrics embedded-storage graph (tokens only).
2. **The closure is computed correctly**, pinned by
   `the_reaching_closure_follows_every_source_of_a_consultation`.

Boundaries that are unchanged. The attribution is thread-local, like the
frame. The spawned workers in the gateway parse JSON, run the bounds and null
readers (separate properties, not custody rows) or drive Clang I/O, so a
custody row's lookups all happen on the analysis thread. A cross-unit home is
still exported only when its row carries nothing. Now that applies per row:
a home row that does not reach the specification is exported, which restores
the pre-ARM-OBLIGATIONS export for it.

**Open obligations.** No package returns one, so no row lost an open
obligation. No `Unknown` became `Safe` on any route. The live list of rows
that could have moved is empty (section 4).

## 2. Tests

`tests/unit/arm_obligations.rs` (16 tests: 4 new, and the 12 existing ones
read the attributed frame):

- `a_declarations_admission_reaches_exactly_the_rows_that_reach_it`: a whole
  unit (`alloc`, `wrap` calls `alloc`, `top` calls `wrap`, `other`), with a
  premise attributed to `alloc`:
  - `alloc`, `wrap` and `top` (the transitive callee case) carry it, and
    `other` does not;
  - an open obligation makes exactly those three rows, and the unit,
    `Unknown`, while `other` stays `Safe`;
  - a unit-grain obligation beside it reaches all four;
  - the rows are identical with the audit's recording open.
- `a_rows_set_is_a_subset_of_the_units_and_unit_entries_reach_every_row`
  (the row set is a subset; unknown attribution gives the unit grain).
- `a_nested_frame_hands_what_it_carried_to_the_enclosing_one`: a nested
  frame's attributed and unattributed obligations reach the enclosing frame
  at unit grain, while the enclosing frame's own declaration stays attributed.
- `the_reaching_closure_follows_every_source_of_a_consultation`: a transitive
  callee, a relied-on callback candidate the body does not spell, a minted
  spelling, a row's applied callee and a function with work but no body all
  reach. An unspelled lookup does not (rule 13f's filter). A relied note
  survives a rollback.
- `the_attribution_is_noted_without_a_recording`.
- The source scan (`no_host_reads_an_admission_without_carrying_its_obligations`)
  is unchanged, and every admission still goes through
  `arm_obligations::consult`/`consult_held`.

`tests/rule37_switch.rs`: the real provider specifications with their
asserting bodies, plus `make` (calls `build`) and `answer` (calls neither).
`build`, `make`, `rd_calloc` and `rd_strdup` carry `asserted_allocation`, and
`answer` carries nothing. The unit is `Safe`, so the certificate recheck,
run outside any recording, reproduced every row's carriage.

## 3. Lean

`QCore.ArmObligations` gains 11 theorems (28 in all, every one in
`AuditArmObligations.lean`, no placeholder, no project axiom):

- `Anchor`, `FrameCarriage`, `unitGrain`, `rowGrain`, `closeInto`;
  `Reaches` and `Used` (the reflexive-transitive closure of the derivation's
  consultations).
- `the_row_grain_refines_the_unit_grain`: each row's set is a subset of the
  unit's.
- `the_row_grain_holds_every_obligation_its_derivation_used`, with
  `a_transitive_callee_use_is_carried`. These assume the record is complete
  for the consultations (`complete`, premise 1) and the reach set is closed
  over the record (`closed`, premise 2).
- `an_unattributed_obligation_reaches_every_row`,
  `an_unknown_attribution_is_the_unit_grain`,
  `a_nested_frame_reaches_every_enclosing_row`.
- `an_unreached_declaration_is_not_carried` (the precision the card adds).
- `a_safe_row_rests_on_no_open_obligation`: a `Safe` row rests on no open
  obligation, neither an unattributed one nor one of a declaration it used.

## 4. Measurement

The release host and plugin were built from `051aef7f2`
(`quirin-audit-measure` `ac755de6…`). Every run used `--profile eight-home
--profile fluent-bit-system --profile fluent-bit-tail-pause --inputs-base
/media/Data/GitHub/quirin-lang --declared PREDICTION.md`, with
`SOURCE_DATE_EPOCH` at the default.

- **Run 1** (`--side base=baseline:98b1bb785 --side tip=rev:051aef7f2`):
  exit 1, `failed`. The baseline was refused on every profile
  (`inputs.route_digest` differs). Its route definitions predate
  `3e53c34fe`, which moved the link map to
  `contract/evaluations/eight-home/link-map.json` and added
  `required_inputs`. So the brief's allowance for `baseline:98b1bb785` does
  not hold mechanically. The TIP side completed and is kept: corpus
  18:04–20:29Z, system and slice exit 0, every route stable.
- **Run 2** (`--side base=rev:3e53c34fe --side tip=run:<run 1's TIP side>`):
  exit 0, status `complete`, no failure. BASE was built and run
  20:30–22:30Z. Record: [`native-comparison.json`](native-comparison.json),
  sha256 `fcd8b64b5350…`. Private output:
  `private/measure/run-1/` and `private/measure/run-2/` in the card's
  worktree; consoles `private/measure/run-{1,2}.log`.

**Declared check:** `match` on all three routes. No moved row, nothing
unpredicted and nothing missing. Every profile reproduces its baseline on
both sides.

| Route | Rows | Safe / Pending / Unknown / Unsafe (both sides) | Moved | Content-changed | Environment-only |
|---|---:|---|---:|---:|---:|
| corpus | 4,834 | 1,013 / 0 / 3,811 / 10 | 0 | 1,904 | 2,930 |
| system | 448 | 101 / 0 / 343 / 4 | 0 | 0 | 448 |
| slice | 404 | 57 / 4 / 342 / 1 | 0 | 0 | 404 |

"Environment-only" is the build commit and the side directories, which
differ because the sides were built at two revisions and measured in two
runs (no shared `--build-commit`). The corpus composition has 185
components per side and 370 in the union, with no conflict named
(MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY).

**What moved: rule 37's premise left 1,904 rows.**
[`identity/narrowed.txt`](identity/narrowed.txt) and the row list
[`identity/rows.tsv`](identity/rows.tsv) (per unit:
[`identity/units.tsv`](identity/units.tsv)) give the split. Of the 2,693
rows that carried `asserted_allocation` at BASE:

| | Rows | Safe | Unknown | Unsafe | Units |
|---|---:|---:|---:|---:|---:|
| keep it | 789 | 34 | 752 | 3 | 54 |
| drop it | 1,904 | 394 | 1,508 | 2 | 54 |

- No row gained a carriage, and no row outside those 54 units changed.
- Every row whose rule 13f `contract_dependencies` names `rd_calloc` or
  `rd_strdup` (271) keeps the premise, so the row grain contains rule 13f's
  set, as section 1 argues.
- The other 518 kept rows reach a specification through an in-unit function
  rule 13f does not follow (one with a declaration rather than an inferred
  summary). For example, `ut_max` in `rdhdrhistogram.c` reaches it through
  the histogram constructor.
- **The prediction's verdict block held; its row estimate did not.** It
  predicted 250 to 600 kept rows, and 789 kept it. The kept set is wider than
  rule 13f's 271 rows by the closure through non-inferred functions. The
  prediction file is kept as committed.

**Rows that could have moved toward `Safe`: none.** Every dropped obligation
was a premise, and `premises_alone_keep_the_verdict` holds. The measurement
shows no status or diagnostic change on any row.

**Identity, checked.** The harness is
[`scripts/identity.sh`](scripts/identity.sh); the outputs are in
[`identity/`](identity/) and the card worktree's `private/identity-2/`.

1. [`scripts/narrowed_carriage.py`](scripts/narrowed_carriage.py) (exit 0)
   checks every BASE/TIP row pair:
   - a kept carriage is identical, with the same digest;
   - every BASE carriage is exactly rule 37's premise;
   - every dropped row's BASE `assumptions` is exactly ARM-OBLIGATIONS' fold
     of TIP's value (`sha256({"rule_premises": […], "sources": <TIP>})`);
   - no row gains a carriage or moves.

   It then undoes the narrowing on BASE: the fields go, the digests are
   restored, and the contract sidecar's 3,316 quotations of dropped rows are
   rewritten per artifact (2,522 quotations of kept rows are left). No
   artifact is both kept and dropped.
2. RULE36-PILOT's `corpus_substitution.py` on the result:
   - the 180 build identities are one-to-one;
   - no row moved or changed status, diagnostics or dependencies;
   - every corpus artifact is equal under the substitution, or with the
     remaining digests blanked and lists read as multisets.

   It exits 1 for one reason only. `corpus-null.jsonl` has 617 distinct
   digests at BASE and 671 at TIP. A null row's context digest follows its
   custody row's assumptions, and in each of the 54 units the kept and
   dropped rows shared one digest at BASE and have two at TIP.
3. [`scripts/null_contexts.py`](scripts/null_contexts.py) (exit 0) confines
   that: of 4,921 null rows, 3,017 are unchanged. The 1,904 changed ones are
   exactly the dropped custody rows' null rows, and they differ only in
   `hypothesis_context.assumptions`, with 54 BASE digests mapping one-to-one
   to 54 TIP digests.
4. RULE35-PILOT's strict `substitution_ids.py`, on copies with each side's
   directory and build commit respelled
   ([`scripts/respell_sides.py`](scripts/respell_sides.py)):
   - corpus (every artifact but the null rows): exit 0, every artifact equal
     under a one-to-one substitution or byte-identical.
   - system and slice: every artifact is byte-identical except
     `resource-diagnostics.json` and `.components.json`, where 15 and 16
     list groups stay ambiguous. [`scripts/leaf_census.py`](scripts/leaf_census.py)
     shows these are the same shape with only source byte offsets
     (`start`/`end` 35 each), `environment_sha256` (6) and reading-site `id`s
     (24) differing in system, and 2 `id`s in slice. Those leaves are
     computed over preprocessed text that spells each side's directory, so
     they differ by run, not by analysis. The measure tool classifies all
     852 rows as environment-only.

## 5. Gate

`python3 scripts/gate_scope.py --base 3e53c34fe --out <worktree>/private/gate-1`
on `051aef7f2` ran in full mode (the diff reaches `qcore-c-gateway`). Record:
[`gate.json`](gate.json). Console:
`/media/Data/GitHub/quirin-lang/private/arm-obligations-unit-grain/gate-1.log`.
Step logs: `private/gate-1/` in the card's worktree. Exit 0:

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 584 units, 617 summaries; 8,945 passed, 0 failed, 15 ignored (2,768 s) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`gate_lean.py --module QCore.ArmObligations --audit AuditArmObligations.lean`) | 0 | 1 audit |
| Python suite | 0 | 568 tests |

The commits after `051aef7f2` (this report, the records, `identity/`,
`scripts/`, the amendments) reach no crate and no Lean module.
`gate_scope.py --plan` on the final commit selects the Python suite only.

## 6. Files likely to conflict

- `crates/qcore-c-gateway/src/arm_obligations.rs`: `Frame::close` now
  returns `(FrameCarriage, Attribution)`, `Frame::carry` became
  `carry_declarations`, and `carry_open` carries into the innermost frame
  only. RULE32A-PILOT's caller-frame hold host must still go through
  `arm_obligations::consult`. Its admissions are unit grain.
- `crates/qcore-c-gateway/src/contract_dependencies.rs`: `within`, `body`,
  `Scope`, the new `relied` and `attribution` module.
- `legacy_c.rs` (the carriage loop at the end of
  `analyze_translation_unit_under_holds`), `legacy_checked_spec.rs`,
  `legacy_callback_route.rs`, `legacy_retained_by.rs`,
  `loop_lifetime_source.rs` (`note_relied` → `contract_dependencies::relied`).
- Rule documents 13f and 37, and the generated `docs/agents/gateway.md`.
- `proof/QCore/ArmObligations.lean`, `proof/AuditArmObligations.lean`.

## 7. Follow-ups

- `ARM-OBLIGATIONS-HOST-ATTRIBUTION` (registered): a host's admission and a
  nested frame's hand-up are still carried at unit grain. No host returns an
  obligation today.
- `ARM-OBLIGATIONS-UNIT-GRAIN-WALL-TIME` (registered): TIP's corpus route
  took 2 h 25 min and BASE's 1 h 53 min. The two were measured once each, in
  different runs and under different host load, so the difference is
  unattributed.
- `RULE37-W4-ASSERT-PREMISE-VISIBILITY` (appended): the W4 residual is why
  the attribution follows consultations and not a rerun.
- `ARM-OBLIGATIONS-CROSS-UNIT-AND-THREADS` is unchanged.
- For the integrator: `baseline:98b1bb785` cannot serve as a base for any
  revision from `3e53c34fe` on, because its route definitions (the link
  map's path) differ. `baseline:3e53c34fe` or later must be published first.
