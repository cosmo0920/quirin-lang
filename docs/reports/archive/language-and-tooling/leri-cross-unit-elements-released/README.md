# LERI-CROSS-UNIT-ELEMENTS-RELEASED

Card LERI-CROSS-UNIT-ELEMENTS-RELEASED (named follow-up recorded by
LIST-ENTRY-RELEASE-IDENTITY). Base `8fd7c8d0d`; measured tip `42043f0ef`
(later commits change records only). Rule 4c amended in place; Lean
`QCore.CrossUnitElementRelease` (new). The [prediction](PREDICTION.md) was
committed at `cf2b3069b`, before any tip measurement (fixtures, Lean and the
one-line route change had been drafted, unmeasured; the prediction says so).

## Headline

* **The clause already crosses.** Rule 4c exports a derived summary with its
  whole contract, and the caller's certificate compares the import with the
  replayed home whole (`check_imports`: `validated == inferred_imports`).
  So `ElementsReleased` is carried with the summary and applied by the
  caller's call reading exactly as in the home's unit; an import that dropped
  or added a clause is refused; every consumer that cannot apply a container
  effect refuses it (`has_container_effect`, e.g. the extern-call bridge).
  No gateway, schema or contract change was needed. This card pins it with
  fixtures in the gateway and end to end through the link-standing route
  (rule 40), and with Lean.
* **One defect on the refusal path, fixed.** On the `current` route, a
  requested home in a unit whose definition census is incomplete failed its
  export analysis, and that failure **rejected the whole route** (`current
  audit rejected (Unknown): enumeration completeness refused: …`, no row
  published). Rule 4c already says an incomplete census keeps the calls
  uncontracted. Such a unit is no longer analysed for export; the census
  refuses the import phase and the run keeps its first-phase rows
  (`crates/qcore-audit/src/current.rs`, one condition).
* **Re-measured (this card's linked preparations): no Fluent Bit caller of a
  released-element summary is reachable**, for route-level reasons that are
  not rule 43's: every core home named by the follow-up is refused by rule
  40's condition 13 on the system selection (two configurations per core
  file), its caller is not a system unit, and the slice's home `tail_file.c`
  has an incomplete definition census. Arbiter (stated map, system and slice):
  no row moved (section 4).

## 1. Setup

The host has system Clang 21.1.8 and Lean 4.35.0
(`private/toolchain/env.sh`), no published live-route inputs and no
prepared routes. `quirin-audit prepare clang-build --link` (base binary
`9574cd8f…`) prepared and linked Fluent Bit at `046accf6` (system) and at
upstream `90978ac6` (slice, not the synthetic export commit), each with a
rule 40 link-resolution artifact ([inputs](artifacts/inputs.json); logs
`private/logs/prep-{system,slice}.log`). Both executables linked and every
linked object reproduced. Absolute figures differ from published baselines;
every comparison is on the same inputs. The corpus was not prepared
(section 5).

## 2. What crosses, and what refuses

Rule 43's summary of `drain` (a formal's list drained into an element
destroyer) is `{parameters: [Borrowed], container_effects: {0: ElementsReleased}}`.
Through rule 4c's export and two-sided replay:

| Fixture (`crates/qcore-c-gateway/tests/cross_unit_elements_released.rs`) | Result |
|---|---|
| export of `drain`; of `touch` (plain borrow); of `drain_static` (static root) | clause stated; none; not exported (rule 43 withholds) |
| caller `drain(q)`; through a derived local in a condition; published then drained; through a wrapper (clause carried on) | Safe, certificate replays both bodies |
| published, drained, then unlinked | Unknown, refused on the forgotten object, no accusation |
| caller passing a local list, its caller | Unknown (`local` withholds its summary) |
| no import | Unknown, `call to uncontracted function \`drain\`` |
| import with the clause dropped, or one added | refused: `cross-unit imported contracts differ from replayed homes` |
| the extern-call bridge given the imported contract | refused (no container-effect model) |
| rule 43 off (switched child) | `drain` not exported |

`CROSS_UNIT_VECTORS` (6 rows) are run on these fixtures and equal the Lean
`vectors`.

End to end (`crates/qcore-audit/tests/link_standing_elements_released.rs`,
`quirin-audit current` on a two-unit checkout with a synthetic rule 40
artifact): without the artifact `subject` is Unknown (uncontracted `drain`);
with it `drain` and `subject` are Safe, `published` is Unknown without an
accusation; with `QCORE_GATEWAY_RULES_OFF=43` nothing crosses. A home with
an unbalanced conditional group (the shape of `tail_file.c`) now prints the
census refusal and keeps the route; at base the same fixture rejects the run
(verified by reverting the condition: the test fails with the route
rejection).

## 3. Lean

`proof/QCore/CrossUnitElementRelease.lean`, audited by
`proof/AuditCrossUnitElementRelease.lean` (13 theorems), importing
`QCore.CrossUnitSummary` and `QCore.ElementReleaseSummary`:
`transport_keeps_the_clause`, `a_changed_clause_is_refused`,
`the_gate_crosses_with_the_summary` (the whole-contract check);
`no_consumer_borrows_a_released_summary`,
`a_consumer_that_cannot_apply_refuses`, `an_applying_consumer_forgets`,
`the_import_reads_as_the_home`, `forgetting_holds_across_units` (rule 43's
`forgetting_covers_every_release` through an admitted import),
`a_dropped_clause_would_trust_a_released_object`;
`a_missing_import_is_refused`, `an_incomplete_census_imports_nothing`,
`a_carried_clause_crosses_no_second_unit`; `vectors_hold`. No placeholder,
no project axiom. It adds no premise: P-LERI-CALLER-PUBLISHED and rule 40's
derived facts are as they were.

## 4. Measurement

### Arbiter

`quirin-audit measure --declared PREDICTION.md --routes system,slice`
(`private/scripts/arbiter.sh`, output `measure/arbiter-1`, measure plugin
`80a30dfd…` built from this worktree), over [inputs.json](artifacts/inputs.json),
the live routes as the plugin runs them (stated eight-home map). Both sides
are checkout directories, `git archive` exports of `8fd7c8d0d` (base, binary
`57486023…`) and `42043f0ef` (tip, `ee88d563…`) with `third_party/libevent`
cloned at its pinned `d82464a2`, each built in place by the tool (the prior
card found that a `rev:` side needs user namespaces this host refuses; not
retried). `--no-slots`: the routes ran outside the host's shared gate slots,
disclosed. Exit 0, status complete, no failure.
[native-comparison-system-slice.json](artifacts/native-comparison-system-slice.json):

| Route | base | tip | unknown_alias / unsupported | moved | content changed | declared |
|---|---|---|---|---|---|---|
| system (448 rows) | 107 / 4 / 337 | 107 / 4 / 337 | 896 / 2,406 both | 0 | 4 | **match** |
| slice (404 rows) | 59 / 1 / 340 / 4 Pending | 59 / 1 / 340 / 4 | 1,075 / 2,462 both | 0 | 8 | **match** |

Every content-changed row was read: all twelve change only `review_holds`
(system `flb_output_exit` ×2, `flb_storage_create` ×2; slice
`flb_input_instance_exit` ×2, `flb_create` ×2, `flb_input` ×2,
`flb_service_set` ×2), and the change is the hold's `module` path, which
spells the side's own export directory (`private/base-src/…` against
`private/tip-src/…`). With that prefix normalised, status, diagnostics and
holds are equal on all 852 rows (`private/scripts/holds_all.py`). An
artefact of directory sides, not of the diff.

### The link-standing route (quick runs, not the arbiter)

`private/scripts/route.sh` (the measure plugin's route environment, null and
outer projections on) with `QCORE_CROSS_UNIT_LINK_RESOLUTION` set to the
preparation's artifact in place of the stated map; base binary `9574cd8f…`,
tip `b486b68b…`. [Lines](artifacts/route-lines.txt), comparisons
([compare.py](scripts/compare.py)): [slice map base/tip](artifacts/quick-slice-map-base-tip.json),
[slice tip map/resolution](artifacts/quick-slice-tip-map-resolution.json),
[system resolution base/tip](artifacts/quick-system-resolution-base-tip.json),
[system tip map/resolution](artifacts/quick-system-tip-map-resolution.json).

| Route | stated map | rule 40, base | rule 40, tip |
|---|---|---|---|
| slice (artifact `21c7c6d8…`) | 59 / 1 / 340 / 4 Pending; import phase refused (census) | **route rejected**, no rows | 59 / 1 / 340 / 4; import phase refused (census); rows equal to the map's |
| system (artifact `adb20950…`) | 107 / 4 / 337 | 107 / 4 / 337 | 107 / 4 / 337; no row differs from base or from the map |

Rule 40 derives 62 slice homes (62 more refused under a two-configuration
key) and 2 system homes (103 refused under one); the system run exports none
of the 2 (neither home body is exportable), so it holds no import.

Why no released-element caller is reached (census at base, every named row
read):

* `flb_engine_destroy_tasks`, `flb_storage_input_destroy`, `flb_output_exit`,
  `flb_sched_timer_cleanup`, `flb_task_retry_clean`, `flb_sched_destroy`:
  linked homes, refused by condition 13. The system selection keeps both
  configurations of each core file (static, linked; shared, not linked), so
  `fluent_bit/src/<f>.c` names two route configurations. Their caller
  `flb_input_instance_destroy` (`src/flb_input.c`) is not a system unit; in
  the slice, where it is, the homes fail condition 12 (not analysed).
* `flb_tail_file_remove_all`: its home `plugins/in_tail/tail_file.c` has one
  configuration, but its unit reads `enumeration completeness refused: a
  conditional group's arms leave different file-scope brace depths`, which
  refuses the slice's whole import phase on every run. Its row is Safe. Its
  callers would not move status anyway: `in_tail_exit` also reads an
  indirect access, unresolved `data`/`ctx` and uncontracted
  `flb_tail_fs_exit`; `flb_tail_config_destroy` five other uncontracted calls.

So the moves this follow-up anticipated wait on three route-level items
(section 6); the clause itself needs nothing more.

## 5. Corpus

Not measured. `quirin-audit pinned` refuses a link-resolution artifact
(rule 40), never calls the changed function (its export phase is
`pinned_route.rs`'s), and no gateway, schema, contract or provider code
changed, so no corpus row can move; preparing the pinned corpus inputs on
this host would measure nothing this diff reaches.

## 6. Follow-ups

[integration-amendments.json](integration-amendments.json) retires
`LERI-CROSS-UNIT-ELEMENTS-RELEASED`, registers
`LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS` (condition 13 over linked
configurations), `TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE` and
`SELECTION-ROUTES-SPLIT-CALLER-AND-HOME` (no measured route holds both the
drains and their callers; the measure plugin has no link-resolution side),
and amends `LINK-PREPARE-PINNED-ROUTE-INPUTS`. No evaluation-profile baseline
changes (`profiles: []`). The increment paragraph is the file's
`increment_paragraph`, which `apply_amendments.py` prepends at landing (it
refuses a record that already holds it, so the record is not edited here);
`--check` exits 0 on this tree.

## 7. Gate

`python3 scripts/gate_scope.py --base 8fd7c8d0d` (through
`private/toolchain/env.sh`) on `8e770549e` (code equal to the measured
`42043f0ef`; later commits are this record and the gate file): **exit 0**,
mode scoped, 11 steps ([gate.json](gate.json), logs
`private/gate-scope/20261113T071948Z-593668/`, console
`private/logs/gate-1.console.log`; Clang recording
[gate/clang-version.txt](gate/clang-version.txt), taken in this worktree).

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy (4 invocations, `-D warnings`) | 0 / 0 / 0 | 1 submodule |
| Rust suite (`qcore-audit`, `qcore-compiler`, `qcorec` whole; 6 test targets) | 0 | 203 units, 205 summaries; 2,641 passed, 0 failed, 4 ignored |
| both reevaluation probes | 0 / 0 | |
| Lean (`check_lean_audits.py`, all: a proof root changed) | 0 | 430 audits (this card adds `AuditCrossUnitElementRelease.lean`) |
| Python suite | 0 | 761 tests |

No provider change, so no provider gate.
