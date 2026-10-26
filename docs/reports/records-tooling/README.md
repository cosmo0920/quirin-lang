# RECORDS-TOOLING: profiles check, evaluation records scope, landing check

Card of refactoring round 1: small tooling follow-ups from the
[amendments schema](../amendments-schema/README.md) and
[evaluation profiles](../evaluation-profiles/README.md) cards. Base `8a015cd83`
(main = origin/main); measured tip `468fde228`. The diff is Python, its tests,
one operations paragraph and this report. It changes no analyzer, contract,
Lean or verdict, and no live evaluation baseline (`profiles: []`, checked).

## What changed

- **`profiles[]` check** (`EVALUATION-PROFILES-AMENDMENTS-CHECK`).
  `scripts/apply_amendments.py` calls `evaluation_profiles.check_amendments`
  on every run: `--check`, `--dry-run` and a write. It compares against the
  document's `base`, or `--profiles-base REV` when given. The check is the one
  `scripts/evaluation_profiles.py` owns: every name is a profile, none is
  frozen, and the list equals the set of live profiles whose baseline changed
  between that revision and the working tree. An absent list counts as empty,
  so a card that moves a baseline without listing it is refused.
  - Fail-closed cases: a revision missing from the object store; a legacy
    document with no `base` and no `--profiles-base`; a non-empty list in a tree
    without `contract/evaluations/`. Each refusal happens before any write.
  - `--profiles-base` is for a merge into a newer main. Comparing against the
    card's own base would then count other cards' baseline moves as this card's.
  - The schema docstring, the single authority, no longer calls `profiles`
    reserved. `contribution` stays reserved. The field writes nothing, because
    the baselines are in the card's own commits. No schema bump is needed: a
    document that was valid keeps its meaning, unless its list names a
    non-profile or disagrees with the moved baselines. EVALUATION-PROFILES' own
    file still passes against the current tree:
    `['eight-home', 'fluent-bit-system', 'fluent-bit-tail-pause']`.
  - The sibling import works under `python3 -I`, which is tested.
- **Evaluation records in the scoped gate** (`GATE-SCOPE-EVALUATION-RECORDS`).
  `scripts/gate_scope.py` classifies `contract/evaluations/**` as `records`:
  data the Python suite checks (`test_evaluation_profiles.py` validates the
  committed profiles). A records-only diff now selects no Rust package and
  still runs the Python suite.
  - Census: no tracked `.rs` file (`git ls-files '*.rs'`) names these records
    by `evaluations`, `evaluation.json` or `bindings.json`. The two hits are
    unrelated: `evaluations` in a doc comment of
    `qcore-c-gateway/src/legacy_c.rs`, and a scratch `bindings.json` in
    `qcore-audit/src/observed_wait.rs`.
    The repository-wide `tracked_files` walkers filter to Quirin source
    extensions or scan named roots (`contract/corpus.json` roots,
    `compiler/tests/fixtures`). The scanner's own model agrees, and
    `RealRepository.test_no_crate_reads_the_evaluation_records` pins that.
  - A crate found reading a record still claims it, which is tested.
    `contract/orphan.qv`, `contract/evaluations.json` and
    `contract/evaluationsX/…` still fail closed to the full suite.
- **Landing check** (optional item 5). The new `scripts/land.py` is the single
  authority for the post-gate sequence:
  1. write the gate record;
  2. `build_matrix.py . --gate <gate.json>`;
  3. `build_docs_index.py`;
  4. `land.py check`;
  5. commit;
  6. rerun the Python suite.

  `land.py check` exits 1 if `dimensions.named_followups`,
  `dimensions.requirements` or `docs/reports/INDEX.md` is stale, and names the
  command that regenerates it. That is the recurring landing failure.
  `apply_amendments.py` prints the sequence after a write. The command it
  printed before, `build_matrix.py --gate`, omitted the generator's required
  `ROOT` argument; the old test pinned that spelling and now pins the corrected
  one.
- **Card-brief shape** (`CARD-BRIEF-AMENDMENTS-SHAPE`). Item 5 of the card-brief
  section in `docs/agents/operations.md` now names `quirin-amendments/1`, and
  makes `python3 scripts/apply_amendments.py <file> --check` exiting 0 a
  checklist item. The integration bullet mentions `--profiles-base` and
  `land.py check`. No other brief template exists under `docs/`. The plan's P3
  row points at this section.

## Not done: the private common brief

The task asked for an in-place edit of the untracked
`private/integrator-briefs/refactor-r1-common.md` in the main checkout. The
card's worktree isolation refused the edit, and the card did not work around
it. The integrator should replace its "Done means" bullet with:

```text
- Done means: implementation, tests, identity run where required, full gate,
  docs/reports/<card>/README.md, and integration-amendments.json in schema
  `quirin-amendments/1` (the docstring of scripts/apply_amendments.py is the
  schema's single authority: `"schema": "quirin-amendments/1"`, per-operation
  field sets, `profiles[]` for any live evaluation baseline the card moves),
  with `python3 scripts/apply_amendments.py docs/reports/<card>/integration-amendments.json --check`
  exiting 0 on the card's tree (one increment paragraph, a short status
  paragraph only if a current-status fact changes, else null, ledger edits with
  DISCHARGE lines for anything left open).
```

`CARD-BRIEF-AMENDMENTS-SHAPE` is therefore amended, not retired. It retires
when the first card briefed from the updated template lands a file that
validates without `--legacy`.

## Ledger

- Retired: `EVALUATION-PROFILES-AMENDMENTS-CHECK` and
  `GATE-SCOPE-EVALUATION-RECORDS`.
- Amended: `CARD-BRIEF-AMENDMENTS-SHAPE` (as above) and
  `AMENDMENTS-RESERVED-FIELDS-UNAPPLIED`. The `profiles` half of the latter is
  discharged; `contribution` stays reserved for CONTRIBUTION-LEDGERS.

`python3 -I scripts/apply_amendments.py docs/reports/records-tooling/integration-amendments.json --check`
exits 0 with `profiles: [] checked against 8a015cd83…`.

## Gate

The scoped card gate ran on the measured tip `468fde228`:
`python3 scripts/gate_scope.py --base 8a015cd83 --out private/gate`. It exited
0 with 7 steps passed. Its record is [gate.json](gate.json). The console log is
in the main checkout at `private/records-tooling/gate-console.log`, and the step
logs are in the card worktree under `private/gate/`.

The run was in full mode, because the diff changes `scripts/gate_scope.py`
itself.

| step | exit | counts |
|---|---|---|
| submodules | 0 | 1 submodule, none moved |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`gate_slot.py rust-suite` → `run_test_binaries.py`) | 0 | 555 units, 8688 passed, 0 failed, 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Python suite | 0 | 513 tests, 0 skipped |

The Rust suite step took 7487.6 s, but the suite itself ran for about 11
minutes (slot receipt: 01:33:17 to 01:44:40 UTC). The rest of the time it waited
for the `rust-suite` slot, which also holds every `audit` slot, behind other
cards' audit runs. Lean was not run, because no `proof/` file changed. The
provider gate was not run, because no `providers/` file changed.

This report and the amendments file were added after the gate, in prose and
records only. `docs/reports/INDEX.md` is left to the integrator, so
`python3 scripts/land.py check` reports the index as stale until
`build_docs_index.py` runs.
