# CT24-CLEANUP-TASK-COMPOSITION — implementation checkpoint

Base `b3ec447b16befa5c88da172946c2a2f5628de009`. This checkpoint lands the
bounded semantic mechanism and its Lean parity. It deliberately does **not**
map CT24, edit the CT manifest/matrix/status/increment, or claim the standing
gate: the follow-on integration pass owns the case harness, adversarial review
and full gate.

## Result

`source.epoch.tasks.v1` now composes one handled cleanup with a live task in
the exact successful spawn arm. The cleanup region erases to the real
`drop NAME;` selected by the cleanup pack at the region's own source position.
The ordinary scoped-task checker then decides that release with its existing
custody ledger.

The three end-to-end fixtures live under
`compiler/tests/fixtures/epoch_credit/CT24_COMPOSITION/`, not under
`fixtures/ct/CT24` while the case is unmapped:

| Shape | Result |
|---|---|
| cleanup adopts the lent owner in `Result::Ok`, then join | `Q600 LoanOutstanding`, attributed to the cleanup statement |
| same loan, join first, then same cleanup | admitted; region evidence publishes `released = [owner]`, abandoned local credit `3` |
| live scalar child, cleanup of an independent owner, then join | admitted; cleanup releases the owner and the spawn-failure arm retains its separate release |

This supplies the distinguishing control earlier lanes could not write. A
region's placement alone is no longer a refusal: cleanup at that placement is
accepted when its owner is independent. The rejected cell changes only the
cleanup target to the owner carrying the live share.

## Bounded source surface

The earlier direct task-scope epilogue remains admitted unchanged. The only
new placement is a region that is:

1. in the entry function's task scope;
2. a direct statement of the exact builtin `Result::Ok` arm;
3. under a direct task-scope `match spawn ...` statement; and
4. still the existing single, decided, handled region body.

Arbitrary `if`, loop and nested-scope placements, the spawn-error arm, worker
regions, entry-top-level regions, multiple regions, extra region-body
statements and every previously refused cleanup-pack shape remain refused.
The reader does not duplicate loan rules. A cleanup release while lent reaches
the unchanged `TaskInstructionKind::DropBuffer` check and its existing
`LoanOutstanding` diagnostic.

`ComposedRegionEvidence` is explicitly path-conditional for this placement:
its released owners and abandoned local credit describe the successful spawn
arm where the region is reached. They make no claim that cleanup ran on the
spawn-failure arm. The Rust test reads the checked IR and pins two distinct
drops: exactly one inside the region span and one outside it for spawn-failure
recovery.

## Lean parity and boundary

`QCore.CleanupTaskComposition` composes the existing operations rather than a
second Boolean decision table:

- `ReadLoan.release` followed by `ErrorRegion.discharge` is the cleanup;
- an actual `.loaned owner borrower` state makes the release, and therefore
  the cleanup, `none` before join;
- an actual `ReadLoan.closeLoan` transition makes that same release succeed;
- a concrete two-resource state proves cleanup of independent resource `3`
  succeeds while resource `1` stays loaned and the carried child grant stays
  `4096`.

The last theorem is about the explicit Lean product. Faithful extraction of
the source/IR into that product and Lean-to-Rust correspondence remain review
premises under evidence rule 28; this report does not call them proved.
The module is registered in the existing `AuditRegionJoinsLoan.lean`, avoiding
a global audit-count change at this implementation checkpoint.

## Focused validation

All Rust builds below used
`CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_INCREMENTAL=0` and this worktree's own `target/`.

| Command | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0; `third_party/libevent` at `d82464a277d0f42703702c4dfd9af6af38595a83` |
| `cargo test -p qcore-frontend --test source_epoch_tasks --no-fail-fast` | exit 0; 25 passed |
| `cargo test -p qcore-compiler --test cleanup_task_composition --test region_joins_loan --no-fail-fast` | exit 0; 14 passed (3 + 11), including the existing native counting-allocation control |
| `python3 scripts/check_lean_audits.py` | exit 0; 143 audits, before the final strengthening from the Boolean summary to the actual operation product |
| `lake build QCore.CleanupTaskComposition` | exit 0 after strengthening |
| `lake env lean AuditRegionJoinsLoan.lean` | exit 0 after strengthening; only standard `propext` / `Quot.sound` dependencies |

The full standing gate, post-strengthening aggregate Lean audit, Python suite,
Clang recording, probes and workspace-wide Rust checks remain for the follow-on
integration pass. No gate JSON is published by this checkpoint.

## Integrator reconciliation list

1. Adversarially review the bounded placement and add the CT24 corpus harness
   only after the semantic review. The harness path should be
   `compiler/tests/fixtures/ct/CT24/` when the manifest row becomes mapped;
   until then the implementation fixtures remain in `epoch_credit/`.
2. Regenerate the derived acceptance matrix through its generator when mapping
   CT24, update both hand-kept acceptance reasons, and apply constraint 25's
   measured key-set change. Do not hand-edit derived fields.
3. Proposed replacement for the CT24-relevant placement sentence in standing
   constraint 15m:

   > A composed handled region is either the creator's direct epilogue after
   > every creation and join, or one direct statement in the exact
   > `Result::Ok` arm of a direct `match spawn` in that creator task scope. In
   > the second form, its straight-line erasure places every cleanup release at
   > the region's own source position; the unedited scoped-task ledger checks
   > that release before any later join. Arbitrary branches and loops, the
   > spawn-error arm, worker regions and every other nested placement remain
   > refused. Region strict credit is creator-local and neither settles nor
   > abandons a child's epoch grant; published region evidence in a success arm
   > is conditional on that arm, while spawn-failure recovery stays the task
   > route's own.

4. Proposed source-profile/spec statement beside the existing composed-region
   grammar:

   > `source.epoch.tasks.v1` admits one handled cleanup directly in a spawned
   > task's exact success arm. Its cleanup pack is executed at that statement:
   > destroying an owner whose read share remains with the unjoined child is
   > refused at `Q600` by the task custody ledger; the same cleanup after join,
   > and cleanup of an independent owner before join, are admitted. No new
   > diagnostic code or keyword is introduced.

5. Run and publish the full standing gate with a run-local private log
   directory and an actual `clang --version` recording. Re-run the aggregate
   Lean audit after this checkpoint's final proof strengthening.

## Remaining boundaries

This checkpoint does not add cleanup to workers, arbitrary control flow,
multiple regions, an unhandled error edge, or an implicit scope-exit cleanup.
It does not prove faithful source extraction, Rust/Lean refinement, native
scheduler coverage, or CT conformance. It supplies a bounded executed cleanup
task composition and leaves mapping and promotion to the evidence lane.
