# CT24-CLEANUP-TASK-COMPOSITION — completed implementation and CT mapping

Base `b3ec447b16befa5c88da172946c2a2f5628de009`; implementation checkpoint
`19711e13fddd75948522bc11e8a6a551ed6318c7`. This completion pass hardens the
bounded semantic mechanism, strengthens its Lean parity, maps CT24 through the
actual corpus harness, and publishes the case's own stage evidence. Global
status, increment and standing-rule reconciliation remain integrator-owned and
are listed exactly below.

## Result

`source.epoch.tasks.v1` now composes one handled cleanup with a live task in
the exact successful spawn arm. The cleanup region erases to the real
`drop NAME;` selected by the cleanup pack at the region's own source position.
The ordinary scoped-task checker then decides that release with its existing
custody ledger.

The three implementation fixtures remain under
`compiler/tests/fixtures/epoch_credit/CT24_COMPOSITION/`. The mapped corpus
artifact and its controls live under `compiler/tests/fixtures/ct/CT24/`:

| Shape | Result |
|---|---|
| cleanup adopts the lent owner in `Result::Ok`, then join | `Q600 LoanOutstanding`, attributed to the cleanup statement |
| same loan, join first, then same cleanup | admitted; region evidence publishes `released = [owner]`, abandoned local credit `3` |
| live scalar child, cleanup of an independent owner, then join | admitted; cleanup releases the owner and the spawn-failure arm retains its separate release |

This supplies the distinguishing control earlier lanes could not write. A
region's placement alone is no longer a refusal: cleanup at that placement is
accepted when its owner is independent. The rejected cell changes only the
cleanup target to the owner carrying the live share.

CT24 is therefore mapped as a `composed_epoch` rejection with exact cascade
`[Q600]` and the `LoanOutstanding` detail pinned. Its own artifact reaches
parse, elaboration, task lowering and task-ledger checking. The task half
refuses before `lower_composed_epoch`, so epoch lowering, epoch-bound checking,
emission and native execution are not credited. The audited Lean module is not
an artifact-bound proof runner, so `proof` remains `unassessed`.

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

The exact spawn form is recognized from the parsed `SourceTaskExpressionKind::Spawn`
whose statement span equals the result match, not from a raw source-text prefix.
A focused control puts a newline and comment between `spawn` and its scope and
is admitted as the same structured expression.

The completion review found and repaired one checkpoint defect: the first
implementation had removed the after-all-task-facts check for direct
task-scope regions as well as for the new success-arm form. `ComposedRegion`
now carries an explicit placement. `TaskEpilogue` retains the prior checked-IR
span rule; only `SpawnSuccessArm` may place cleanup before its matching join.
The existing direct pre-spawn fixture remains refused at Q600.

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

The after-join theorem now requires `pack.actions = [resource]` and proves the
discharge's `cleanupTrace = [resource]`. The checkpoint's earlier `isSome`
conclusion allowed an arbitrary unrelated pack to discharge after the loan
closed; the strengthened statement binds the closed loan identity to the
cleanup pack identity while retaining the explicit correspondence boundary.

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
| `cargo test -p qcore-frontend --test source_epoch_tasks --no-fail-fast` | exit 0; 25 passed after placement hardening |
| `cargo test -p qcore-compiler --test cleanup_task_composition --test region_joins_loan --no-fail-fast` | exit 0; 14 passed (3 + 11), including the preserved direct-scope pre-join refusal and existing native counting-allocation control |
| `lake build QCore.CleanupTaskComposition` | exit 0 after identity binding |
| `lake env lean AuditRegionJoinsLoan.lean` | exit 0; only standard `propext` / `Quot.sound` dependencies |
| `python3 scripts/check_lean_audits.py` | exit 0; 143 audits after the final proof strengthening |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| both reevaluation probes | exit 0; zero failed expectations |
| `python3 -m unittest discover -s scripts/tests` after regeneration | exit 0; 305 passed |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0; 48 mapped / 2 unmapped |
| `clang --version` | exit 0; this card's recording is in `gate/clang-version.txt` |

The first sandboxed corpus attempt reached the existing sanitized heap runner
and exited 101 because LeakSanitizer's process inspection was sandbox-blocked.
It is a failed attempt, not evidence. The later unrestricted workspace command
ran to its final doctest summary and its log contains 242 strict result
summaries totaling 4,758 passed, zero failed and 10 ignored. The shell wrapper
then assigned zsh's read-only `status` variable before it could capture Cargo's
exit, so that observation has an **unknown Cargo exit** and is not published as
a standing-gate pass. A duplicate rerun was stopped at the integrator's request;
the merged tree will receive the definitive combined workspace gate.

The complete observation is preserved at
`private/ct24-gate-20260919/workspace-rerun.log`; the wrapper error is the
session exit text `zsh:1: read-only variable: status`. Other final logs and
their explicit exit files are in the same private directory. The matrix was
regenerated without `--gate`: its CT mapping and stage rows are current, while
its standing-gate measurements remain explicitly carried until the integrator
regenerates it with the combined run's gate JSON.

The corpus inventory now executes the mapped CT24 row and all eight controls:
same cleanup after join and independent-owner cleanup before join admit;
missing join refuses Q600; nested `if`, nested loop and the spawn-error arm
refuse Q302; double discharge refuses Q304; and an unsupported region body
refuses Q302. These controls prevent both a vacuous placement refusal and a
generic unsupported-body refusal from satisfying the case.

## Integrator reconciliation list

1. Apply the global status and newest increment changes for mapped coverage
   `47/50 -> 48/50`, unmapped `3 -> 2`, gated sanitized-native unchanged at
   `10`, proof unchanged at `5`, and Lean audits `142 -> 143` only where the
   merged tree and full gate confirm those bases. This card itself adds no
   audit file; its module is registered in `AuditRegionJoinsLoan.lean`.
2. The hand-kept `newly_decidable_not_mapped` list and reasons map now lose
   CT24, and the matrix generator removes CT24 from the derived reason map.
   Regenerate the matrix once more with the definitive integration gate JSON
   so its carried validation block becomes the merged tree's measurements. Do
   not reuse the stale 142-audit predecessor gate.
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

5. The diagnostic ledger needs no new code allocation. Record that CT24 reaches
   the existing Q600 `LoanOutstanding` condition through a real erased cleanup
   release; Q302/Q304 remain the bounded placement/body and linear-pack
   controls. Update the CT harness prose/counts from its generated evidence,
   not by copying the case count into a standing rule.

## Remaining boundaries

This checkpoint does not add cleanup to workers, arbitrary control flow,
multiple regions, an unhandled error edge, or an implicit scope-exit cleanup.
It does not prove faithful source extraction, Rust/Lean refinement, native
scheduler coverage, or CT conformance. It supplies a bounded executed cleanup
task composition and maps CT24 as one source-level rejection with the stage
and control boundaries above. No status or conformance promotion follows from
that mapping.
