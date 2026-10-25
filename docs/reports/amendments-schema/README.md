# AMENDMENTS-SCHEMA: a checked schema for integration amendments

Card of refactoring round 1, [pluggable-cards plan](../../plans/pluggable-cards/README.md)
seam D, phase P1. Base `710021bd7` (main = origin/main). Changes no analyzer,
contract, Lean or verdict: the diff is Python, one operations paragraph and
this report.

## What changed

- `scripts/apply_amendments.py` is the single authority for the schema
  `quirin-amendments/1`; its docstring is the specification, and
  [operations](../../agents/operations.md#integration-and-reconciliation) routes
  to it. The repository keeps no JSON-Schema files and has no `jsonschema`
  dependency. Script-side record formats are checked by the scripts that read
  them (`reconciliation_sweep.py`, `build_matrix.py`), so this does the same.
  `qcore-schemas` holds Rust wire types, which these records are not.
- Top-level keys are exactly `schema`, `card`, `base`, `measured_tip`,
  `increment_paragraph`, `status_paragraph` (paragraph or `null`) and `ledger`,
  plus the reserved `profiles` and `contribution`. Ledger operations and their
  fields:

  | operation | fields | effect on `named_followups` |
  |---|---|---|
  | `register` | id, title, text, source, optional recorded_by | appends `{id, title, why: text, source[, recorded_by]}`; refused if the id exists |
  | `amend`, `append` | id, text, source | `why += " " + text`; `source += ", " + source` unless listed; refused if the id is absent or the text already present |
  | `retire` | id, why, source | removes the entry; refused if absent; `why` (the DISCHARGE) stays in the card's records |

  `amend` and `append` write identical bytes, as the hand landings did
  (722a1d0e9, 3353e1c63). Ops apply in order, so a register followed by an
  append to the same id works, and a second retire is refused.
- `profiles` (distinct names, `[a-z0-9][a-z0-9-]*`) and `contribution` (null or
  an object) are shape-checked and **not applied**. The tool prints that,
  because their producers (EVALUATION-PROFILES, CONTRIBUTION-LEDGERS) have not
  landed (follow-up `AMENDMENTS-RESERVED-FIELDS-UNAPPLIED`).
- Application: the increment paragraph is prepended below the checkpoint
  notice (the `> ` block) of the record that
  `docs/development/increments/README.md` names as current. That covers a record
  with no paragraph yet, the past pitfall, and stays above any
  `<!-- migrated-section -->` block. The status paragraph is prepended below
  `docs/STATUS.md`'s title, where the last two landings put theirs. The matrix
  is re-serialised as `build_matrix.py` writes it (`indent=1,
  ensure_ascii=False`, trailing newline). Every edit is computed in memory and
  written only after every check passes. A second application is refused
  because the paragraph or text is already present. `build_matrix.py` and
  `build_docs_index.py` are printed, never run.
- Fail-closed refusals (exit 1, "nothing was written"): unknown top-level key,
  missing key, wrong `schema`, non-hex revision, multi-paragraph or empty text,
  unknown operation, missing or extra field, register of an existing id,
  amend/append/retire of an absent id, malformed reserved field, a current
  record the index does not name or that lacks a checkpoint notice.
- `--check` (validation and applicability, no writes) and `--dry-run` (plus a
  unified diff).

## Legacy files

Main holds 17 pre-schema `integration-amendments.json` files in six shapes.
`--legacy` reads each shape through an explicit adapter chosen by its keys, so
an unrecognised shape is refused rather than guessed:

| shape (key) | files |
|---|---|
| `ledger` list (rules 33–35, refactoring round) | audit-canonical-paths, detach-result-wrapper-composition, gate-elastic, generated-roots, w48-bu-cmt-map-metric-get, w48-bu-external-callee-contracts, w48-bu-single-leaf-bodies, w48-cmt-list-self-init, w48-sched-cleanup-production |
| `followup_amendments` list with `action` (rule 32) | logger-effect-contracts, w48-rk-log-caller-frame |
| one `followup_amendment` object (`operation`, `mode` or a bare `append` key) | guarded-contract-adoption, premise-guarded-decomposition, scheduler-lifetime-contracts |
| `named_followup_amendments` with prose actions, `status`/`increment` objects | w48-bottom-up-batch4 |
| `residual_plan.retain` + `append_paragraph` | w48-bottom-up-batch5 |
| `tracking` with null paragraphs | w48-bu-rk-result-constructors (refused) |

Legacy operation words map `add`→`register` and take the first word of a prose
action (`amend existing open record, ...`→`amend`). The normaliser reports every
derived value. A missing card is taken from the report directory. A missing
base or measured tip is `null`, which is accepted only in legacy mode;
`measured_source`/`measured_joint_source` stand for `measured_tip`. Surrounding
whitespace in text is stripped, as the hand landings show (a7e5ed4cc). An empty
status becomes `null`. Fields the schema does not carry (`field`, `status`, an
amend's `title`) are dropped and listed, and informational keys are listed as
not applied. An amend/append without a source becomes `source: null`, legacy
only: the entry's source is left alone, which is what the integrator did
(a7e5ed4cc's residual-plan append). w48-bu-rk-result-constructors is refused as
proposal-only. Its paragraphs are null and its `tracking` entries are
conditional instructions ("after passing gates ... append only measured ..."),
not text, so normalising it would invent an amendment. The test pins exactly
that one refusal. No historical file is rewritten.

## Parity (`replay.py`)

`python3 docs/reports/amendments-schema/replay.py <commit> <amendments> [--legacy]`
reads the parent's increment index, current record, STATUS.md and matrix from git
objects into a fresh directory and applies the amendments read at `<commit>`.
It then compares each file with `<commit>`'s version.

- **c3d8ad3d4** (AUDIT-CANONICAL-PATHS, parent merge e8e281742; retire 1,
  register 3, status paragraph): `wave-48-part12.md`, `docs/STATUS.md` and
  `acceptance_matrix.json` are **byte-identical** to the commit, the whole
  matrix included (that commit did not touch `dimensions`; 710021bd7
  regenerated them afterwards). The diff is empty. This holds for the legacy
  file and for the same content with `"schema": "quirin-amendments/1"` added.
  `scripts/tests/test_apply_amendments.py` checks both.
- Earlier hand landings, as extra evidence (private outputs under
  `/media/Data/GitHub/quirin-lang/private/amendments-schema/replay-*.json`):

  | commit | card | increment record | named_followups | STATUS.md | other matrix keys |
  |---|---|---|---|---|---|
  | 66aa16683 | GATE-ELASTIC | identical | identical | identical | none (whole file identical) |
  | 4753bfe62 | GENERATED-ROOTS | identical | identical | unchanged by both | dimensions, validation (regenerated in that commit) |
  | 722a1d0e9 | W48-BU-SINGLE-LEAF-BODIES | identical | identical | differs | dimensions, validation |
  | 3353e1c63 | W48-SCHED-CLEANUP-PRODUCTION | identical | identical | differs | dimensions, validation |
  | a7e5ed4cc | W48-RK-LOG-CALLER-FRAME | identical | identical | differs | dimensions, validation |

  The three STATUS.md differences are placement: those landings put the
  paragraph below or inside the then-current halt paragraph, and edited it.
  The tool always prepends below the title. That is the convention since
  GATE-ELASTIC, and an integrator who wants another position moves it by hand.

## Scope and boundaries

- The parity test reads `c3d8ad3d4` from git objects. A shallow clone or an
  export fails it with that reason instead of skipping. That is the same
  fail-closed stance as `tracked_files.py`.
- The tool applies only what a card owns: the increment paragraph, the status
  paragraph and `named_followups`. The report index, matrix dimensions and
  validation stay with their generators. Merging, gates and reconciliation
  sweeps are unchanged.
- Not touched: AUDIT-COMPOSITION's and CONTRACT-LOCKS' files, every Rust crate
  and `proof/`.

## Gate

`python3 scripts/gate_scope.py --base 710021bd7 --out private/gate` at
`5d6a890d4` ([gate.json](gate.json)): plan `mode: scoped`, 0 affected crates,
no Rust suite, no Lean, no provider gate, Python suite always. Steps:
submodules exit 0 (1 submodule), `cargo fmt --all -- --check` exit 0, `python3
-m unittest discover -s scripts/tests` exit 0 (460 tests, 0 skipped). Overall
exit 0. Console log:
`/media/Data/GitHub/quirin-lang/private/amendments-schema/gate-console-2.log`.

Attempt 1, at `b0e27b9ce`, exited 1, retained in
`/media/Data/GitHub/quirin-lang/private/amendments-schema/gate-console.log`.
`test_tracked_files` rejected the new test's `rglob` snapshot of its temporary
fixture directory (constraint 29's walk census). `e8b32f7af` snapshots the
fixture files by name instead.
