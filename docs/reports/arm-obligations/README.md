# ARM-OBLIGATIONS: carrying an admission's obligations into the row

Card ARM-OBLIGATIONS discharges `GATEWAY-RULE-API-ARM-OBLIGATIONS`
([plan](../../plans/pluggable-cards/README.md) seam B). Base `db2795100`.
[Prediction](PREDICTION.md) was committed first (`48e17a158`), then the
implementation (`1aa2555a3`), the rule 37 document (`7193c4850`), a test fix
(`2d622a091`) and the amendments (`04b178120`).

## 1. What changed

A rule package answers a position with `Admit(Admission { lowering,
obligations })` or `Refuse`. An obligation is `Premise(name)` (an assumption
the row must list) or `Open(key)` (left undischarged). Before this card the
hosts did one of two things with obligations. `rule_hosts.rs`,
`transfer_rules.rs` and `legacy_exact_formal_return.rs` refused any admission
that carried one. Four other sites used the lowering and never read the
obligations at all: rule 33's statement expansion and entry row, the route
table's statement and return fallbacks, and rule 37's exempt spans. No package
returns an obligation at those sites, so nothing was actually dropped, but the
interface allowed it.

`crates/qcore-c-gateway/src/arm_obligations.rs` is now the only place in the
gateway that reads an admission.

- `consult` wraps the API's `consult`. An admission with no obligations is its
  lowering, exactly as before. An admission with obligations is carried into
  every open **unit frame** and its lowering returned. With no frame open no
  row could carry the obligations, so the admission is refused and the
  position's existing answer stands (fail-closed).
- `consult_held` is for a host that uses the lowering later. It returns the
  lowering together with the obligations, and the host carries them wherever
  it uses the lowering.
- `analyze_translation_unit_under_holds` opens a `Frame`. A nested analysis's
  frame passes what it carried to the enclosing frame when it closes or is
  dropped. Once the unit's rows are built, `apply` carries the frame into
  every row (`LegacyFunctionSummary::rule_obligations`):
  - each premise goes into the row's assumptions;
  - each open obligation adds an `Unsupported` diagnostic ("gateway rule R
    admits this unit's analysis only with the obligation `K` left open"), so
    the row is `Unknown` (`Unsafe` stays `Unsafe`) and stays in the
    denominator, and so does the unit.
- The audit publishes the row field `rule_obligations` (`{premises, open}`,
  each `{rule, name}`). It folds the premises into the verdict contract's
  `hypothesis_context.assumptions`: `sha256({"sources": <old>,
  "rule_premises": [...]})` (`report::assumptions_with_premises`). A row that
  carries nothing keeps its bytes and its digest.
- A cross-unit home whose row carries an obligation is not exported
  (`legacy_cross_unit::export_homes`), because an imported contract has no
  premise slot and the caller's rows would shed it.

**Rule 37, the first consumer.** The `asserted-allocation-premise` arm at
`legacy_checked_spec::settle` is consulted through `consult_held`.
`result_constructor_rule::settle_premises` returns the withdrawn
specifications and, for every admitted one, its admission's obligations. A
premise the declaration does not state still withdraws, as before. An open
obligation, which rule 37's package never returns, is no longer a withdrawal:
the specification stands and the rows carry the open obligation. That is more
refusing for the unit's rows, which all become `Unknown`, and the export guard
covers callers in other units. `Settled.carried` holds the obligations of the
specifications that stand when the settlement ends. A specification withdrawn
in any round (no body, recursive, not established, premise not admitted)
contributes nothing, because no row rests on its admission. The unit
analysis carries `Settled.carried` into its frame. With rule 37 on, a unit
that defines `rd_calloc`/`rd_strdup` in scope (`tests/rule37_switch.rs`'s
provider fixture, and 54 librdkafka corpus units, §4) publishes
`asserted_allocation` on every row, `build`, `rd_calloc` and `rd_strdup`
included, with every verdict unchanged. With 37 off, nothing stands and no
row carries it.

**Hosts.** `rule_hosts.rs` (rule 35's lexical gate and typed reader) and
`transfer_rules.rs` (rule 34) carry the same way. Asked outside a unit's
analysis they refuse, as before. The transfer host still refuses an admission
whose phase count differs from the call's effects; that check now runs before
the obligations are carried, so a refused admission carries nothing. Rule 34's
unswitchable guard is unchanged.

**Grain.** The unit is the carriage's grain. An obligation reached anywhere in
a unit's analysis is carried by every row of that unit. That is an assumption
too many, never one too few, and it is registered as
`ARM-OBLIGATIONS-UNIT-GRAIN`. The frame is thread-local: an admission asked on
a thread the analysis spawns sees no frame and is refused. Open obligations
publish as `Unknown`; no route maps an open key to `Pending`. Both are
registered as `ARM-OBLIGATIONS-CROSS-UNIT-AND-THREADS`.

## 2. Tests

- `tests/unit/arm_obligations.rs` (12 tests):
  - Every subset of `{premise p, premise q, open k}` is carried exactly, and
    refused when no frame is open.
  - Nested and dropped frames hand their obligations on.
  - The consultation carries the winner's obligations or ends the table.
  - On a row, a premise keeps the verdict, an open obligation turns `Safe`
    into `Unknown` and leaves `Unsafe` as it is, and the empty carriage is
    the identity.
  - `APPLY_VECTORS` equals Lean's `applyVectors`, and the Rust `apply`
    reproduces them on real diagnostics, `Pending` included.
  - A whole unit (`analyze_legacy_c_translation_unit`, a test-only injection
    into its frame) puts a premise on every row with the summary otherwise
    equal, and an open obligation makes every row and the unit `Unknown`.
  - Dropped obligation, mutation-style:
    `no_host_reads_an_admission_without_carrying_its_obligations` scans the
    tracked gateway sources (code, not comments) for the API's `consult` (by
    path, by braced, aliased or glob import, or through `answer::`),
    `into_admission(`, `.admission.lowering`, `.admission.obligations` and an
    `Admission` pattern. Nine mutant snippets are asserted to be caught
    before the tree is scanned.
- `rule_hosts.rs`: the refusal test now checks carried-or-refused.
- `result_constructor_rule.rs`: an admitted specification keeps its premise;
  a withdrawn one keeps none.
- `tests/rule37_switch.rs`: the premise appears on the provider fixture's
  rows with 37 on and not with 37 off.
- `qcore-audit` `report::tests`: the field and the assumptions digest change
  only for a row that carries a premise.

## 3. Lean

`QCore.ArmObligations` (`proof/QCore/ArmObligations.lean`, 17 theorems, all
in `AuditArmObligations.lean`):

- `the_lowering_never_travels_without_its_obligations`
- `the_carriage_preserves_every_obligation` and `the_carriage_never_erases`
- `an_uncarriable_obligation_is_refused`
- `a_nested_frame_hands_on_its_obligations` and
  `the_row_carries_every_obligation`
- `an_open_obligation_is_never_safe`, `the_carriage_keeps_rows_well_formed`
  and `a_safe_row_carries_no_open_obligation` (`Safe` only when no carried
  obligation is open)
- `the_carriage_never_improves`
- `premises_alone_keep_the_verdict` and `the_empty_carriage_is_the_identity`
  (why the identity run is the expected outcome)
- `the_apply_vectors_are_apply`

No placeholders and no project axioms (`check_lean_audits.py`: 397 audits).
Two premises are kept separate: the Rust/model correspondence, pinned by the
vectors, and the claim that every consultation goes through the carriage,
pinned by the source scan.

## 4. Measurement

There was one `quirin-audit measure` run (release host and plugin built from
`1aa2555a3`). Arguments: `--profile eight-home --profile fluent-bit-system
--profile fluent-bit-tail-pause --side base=rev:db2795100 --side
tip=rev:1aa2555a3 --declared PREDICTION.md --build-commit
arm-obligations-identity`, with `SOURCE_DATE_EPOCH` at the default
`1767225600`.

- Ran 2026-10-31T08:51Z to 14:00Z; exit 0, status `complete`, no failures.
- Record: [`native-comparison.json`](native-comparison.json), sha256
  `d981bde275f9…`.
- Private output:
  `/media/Data/GitHub/quirin-lang/private/arm-obligations/measure-1/` (the
  snapshots and work directories are left out of that copy), console
  `/media/Data/GitHub/quirin-lang/private/arm-obligations/measure-1.log`.

**Declared check:** `match` on all three routes. No moved row, nothing
unpredicted and nothing missing. Every profile reproduces its baseline on
both sides.

| Route | Rows | Safe / Pending / Unknown / Unsafe (both sides) | Moved | Content-changed | Artifacts |
|---|---:|---|---:|---:|---|
| corpus | 4,834 | 1,013 / 0 / 3,811 / 10 | 0 | 2,693 (`rule_obligations` only) | equal after the card's rows are undone, under a checked one-to-one substitution |
| system | 448 | 101 / 0 / 343 / 4 | 0 | 0 | byte-identical |
| slice | 404 | 57 / 4 / 342 / 1 | 0 | 0 | byte-identical |

The shared `--build-commit` makes system and slice byte-identical. The tool
marks the corpus comparison `incomplete` for one reason only, the named
`composition-conflict`: 185 components on each side, 365 in the union. The
corpus route digests each side's working directory into the unit build
identity (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY). RULE36-PILOT's round-2
record has exactly the same 185/185/365.

**No verdict moved. 2,693 corpus rows gained an assumption.** Every one of
them is in librdkafka and carries exactly `{"premises": [{"rule": "37",
"name": "asserted_allocation"}], "open": []}`. They are every function row of
54 units: 428 `Safe`, 2,260 `Unknown`, 5 `Unsafe`, each with its status,
diagnostics and contract dependencies unchanged. In each of those units
`rd_calloc`, `rd_strdup` or both stand as checked specifications.
[`identity/carrying-units.txt`](identity/carrying-units.txt) cross-checks the
unit-level body evidence:

- the 54 carrying units are exactly the units whose `contract_body_evidence`
  examined one of the two bodies (24 both, 22 `rd_calloc` only, 8
  `rd_strdup` only);
- every row of those units carries the premise, which is the unit grain;
- none of the other 131 units carries it.

So the bodies are in those units' derivation scope, and the specifications
admitted with the premise are what their rows rest on. This is the carriage
the card exists to make visible, and it moves no verdict
(`premises_alone_keep_the_verdict`).

**The prediction's verdict block held; its explanation did not.** It said no
live row would gain a premise, reasoning that every row consulting
`rd_calloc`/`rd_strdup` lists them under `withdrawn`. That reading was wrong.
A row's `withdrawn` set (`contract_dependencies::row_dependencies`) takes a
name from the unit record's withdrawals, and that record is the union of
every analysis the recording holds. The same units record `rd_calloc` both
as `C06.3-no-body` and as validated by `W4`, so `withdrawn` on a row does not
mean the specification was absent from the published analysis. The
prediction file is kept as committed.

**Identity, checked.** The harness is
[`scripts/identity.sh`](scripts/identity.sh); outputs are in
[`identity/`](identity/) and
`/media/Data/GitHub/quirin-lang/private/arm-obligations/identity-2/`.

1. [`scripts/strip_carriage.py`](scripts/strip_carriage.py) undoes the
   card's additions on the TIP corpus side, checking each one (exit 0):
   - every carried field is exactly rule 37's premise, in both row files;
   - every carrying row's `assumptions` is exactly
     `sha256({"rule_premises": […], "sources": <BASE's value>})`;
   - the one folded digest maps to one BASE digest, and its 5,838 quotations
     in the contract sidecar's dependency section are restored.
2. RULE36-PILOT's `corpus_substitution.py` then runs on BASE and the
   stripped TIP (exit 0):
   - the 180 build identities are one-to-one;
   - no row moved or changed status, diagnostics or dependencies;
   - every artifact is equal under the substitution, or equal with the
     remaining digests blanked and lists read as multisets.
3. RULE35-PILOT's strict `substitution_ids.py` (exit 0): every corpus
   artifact is equal under a one-to-one token substitution or
   byte-identical.

The 2,693 differing leaf positions of `corpus-null.jsonl` are the null rows'
context digests. Each one is derived from its custody row's `assumptions`,
one per carrying row, and the strict check maps them one to one.

## 5. Gate

`python3 scripts/gate_scope.py --base db2795100 --out <worktree>/private/gate-2`
on `04b178120` (the final code, test fix and amendments). Full mode, because
the diff reaches `qcore-c-gateway`. Record: [`gate.json`](gate.json). Console:
`/media/Data/GitHub/quirin-lang/private/arm-obligations/gate-2.log`. Step
logs: `/media/Data/GitHub/quirin-lang/private/arm-obligations/gate-2-logs/`.
Exit 1, for one Python test:

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, whole workspace) | 0 | 584 units, 617 summaries; 8,941 passed, 0 failed, 15 ignored (5,073 s) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` | 0 | 397 audits |
| Python suite | 1 | 568 tests, 1 failure |

The failure is `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`.
The committed matrix records `lean_audits` 396 and the tree has 397
(`AuditArmObligations.lean`). The matrix is the integrator's record and is
regenerated at landing (CARD-GATE-NEW-AUDIT-MATRIX-COUNT).

An earlier run on `7193c4850` (`gate-1`, logs
`/media/Data/GitHub/quirin-lang/private/arm-obligations/gate-1-logs/`) had
the same counts and one more Python failure. The rule 29 tripwire
`test_every_remaining_tree_walk_in_the_tests_is_a_reviewed_one` flagged the
bypass scan's `read_dir` walk. `2d622a091` replaced it with
`tests/support/tracked_files.rs`.

## 6. Files likely to conflict

- `crates/qcore-c-gateway/src/rule_hosts.rs`, `transfer_rules.rs` and
  `result_constructor_rule.rs`: RULE32A-PILOT's hold host may add a
  consultation. It must go through `crate::arm_obligations::consult`, or the
  bypass scan fails.
- `legacy_route_table.rs`: only the dispatchers changed, not the generated
  region. Rerun `scripts/gen_gateway_registry.py` on a conflict.
- `legacy_selection.rs`, `legacy_exact_formal_return.rs`.
- `legacy_c.rs` (`LegacyFunctionSummary` gains `rule_obligations`; a new
  struct literal needs it), `legacy_checked_spec.rs`, `legacy_cross_unit.rs`.
- `crates/qcore-audit/src/report.rs`, `projection.rs`.
- Generated: `docs/agents/gateway.md` (rule 37's text) and `proof/QCore.lean`.

## 7. Follow-ups

- `ARM-OBLIGATIONS-UNIT-GRAIN`: narrow the carriage to the rows that rest on
  the admission.
- `ARM-OBLIGATIONS-CROSS-UNIT-AND-THREADS`: carry through cross-unit imports
  and spawned workers instead of refusing, and decide whether an open key may
  publish `Pending`.
- `RULE37-W4-ASSERT-PREMISE-VISIBILITY` and `AUDIT-ROW-ASSUMPTIONS-UNRECORDED`
  are appended, not discharged.
