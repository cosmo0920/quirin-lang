# LANDING-PUBLISH-COMPLETION

Completes MEASURE-TOOL-LANDING-PUBLISH (pluggable-cards plan §3 F "Published
baselines", §4 step 5). Base `486b05a71`, merged with `f774f54de`. It changes slot accounting and landing
orchestration, not analyzer code, contracts, routes or Lean, so it needs no
identity run. The measurements below are observations of the existing routes.

## 1. Two-word slot budget (GATE-SLOT-CURRENT-BUDGET-SPELLING)

`scripts/gate_slot.py` `apply_budget` inserted `--memory-budget-mib=N` after
`quirin-audit current`, but `current_cli.rs` reads only `--memory-budget-mib
N`. It now inserts the two-word form. It still reads an existing flag in
either spelling, refuses one that differs from the slot budget, and leaves an
equal one as given. `test_a_stub_current_route_accepts_the_inserted_budget`
runs a stub `quirin-audit current` under a real `audit` slot. The stub parses
arguments the way `current_cli.rs` does and rejects `--memory-budget-mib=N` as
an unknown argument. The earlier unit test now pins the two-word argv.

## 2. Corpus admission budget (CORPUS-PEAK-ABOVE-AUDIT-SLOT-BUDGET)

Admission to an `audit` slot used to count only the planning budget
(`floor(MemAvailable / budget)`). Now it counts the larger of that budget and
the route's measured budget. `ROUTE_ADMISSION_MIB` gives the pinned corpus
route (`run_pinned_audit.py` or `quirin-audit pinned` anywhere in the argv,
behind the measure plugin's `unshare` wrapper too) 11 GiB (11,264 MiB). Its
workers stay planned at the slot budget: 3 at the 7 GiB default
(`QCORE_AUDIT_JOBS`). User decision 5 still holds, with 7 GiB as the default
and 5 or 4 GiB selectable. The corpus is the one documented measured
exception, and its admission never falls below the slot budget. The receipt
records `admission_mib` and `route_class`. It also records the child's peak
resident set `max_rss_kib`, which is `wait4`'s `ru_maxrss`: the largest single
process among the child and its waited-for descendants, here the
`quirin-audit` process. `max_rss_within_admission` compares that peak with
the admission budget. A slot holder's note carries its `admission_mib`.

**Measured after the change**
([receipt](corpus-peak.slot.json), [comparison](corpus-peak.native-comparison.json)):
`quirin-audit measure --routes corpus --side prev=baseline:b0b63da7 --side
tip=rev:a42edeae4`, run under this tree's `gate_slot.py`.

| | Value |
|---|---|
| Admission budget | 11,264 MiB (`route_class` `pinned-corpus`), planning budget 7,168 MiB, 3 workers |
| Slots at admission | slot 0 of 4 (MemAvailable 51,886,604 KiB; at 7 GiB it would have been 7) |
| Peak RSS (`max_rss_kib`) | 9,824,888 KiB = 10,060,685,312 B (9,594.6 MiB), `max_rss_within_admission: true` |
| Headroom | 1,669 MiB (14.8 % of the admission budget) |
| Wall | 18:08:08 to 19:41:32 UTC (5,604 s), exit 0 |

The peak is 0.7 % below CFG-RELEASE's 10,129,997,824 B, which was one
observation on a shared host. Both observations are under 11 GiB. The corpus
figures reproduce the `eight-home` baseline on both sides, with no moved rows.

Limits that remain. Admission still reads `MemAvailable` at the moment of the
request, and a running audit that has not yet reached its peak is not
reserved. That heuristic is unchanged (GATE-ELASTIC). The route class is
recognised from the argv, so a corpus run started through a configuration file
is admitted at the planning budget.

## 3. Profile policy at publication

`land.py publish` now applies the evaluation-profile policy between the
measurement and `measure publish`. It reads the `prev -> tip` comparison of
`native-comparison.json`, and `scripts/evaluation_profiles.py` remains the
policy's only authority (`load_profiles`, `compare`, `check_history`). It
refuses, and keeps the measurement, when any of these holds:

- a live profile was not measured, or a route has no figures on both sides;
- `compare` of the tip's figures against the baseline committed in the landed
  tree exits 1 (moved) or 2 (frozen profile). The refusal names the moved
  figures. A move therefore completes a landing only when the landing commits
  the new baseline with an appended `baseline_history` entry and reason;
- the figures moved since the previous published landing although the
  committed baseline did not;
- the history policy fails (history is append-only; frozen profiles never
  move).

Tests with a fake profile API cover each refusal. One command-level test runs
a stub host and shows that a moved profile refuses publication, measures once
and never publishes, and keeps the measurement. Another runs the real API on
this tree: the committed live baselines' own figures pass, and one changed
figure refuses. The earlier bootstrap-then-reuse test now applies the policy
too.

## 4. First landed publication

**Pending: the integrator runs it after the landing push.** It runs `land.py
publish` (the landed revision's own `land.py`) on the pushed landing SHA with
`--previous b0b63da7dcdd7c3d289259caf4067b72ad45c757`. That verifies reuse of
the bootstrap baseline and compares all three live routes against the live
baselines committed at `710021bd7`. Fill in, per profile, either *verified
unchanged* with the figures or the moved rows with their attribution. Use the
result to discharge GATEWAY-RULE-API-SLICE-IDENTITY (`fluent-bit-tail-pause`
unchanged) or amend it.

Already observed: this card's tree (analyzer = `486b05a71`) reproduces the
`eight-home` corpus figures against `baseline:b0b63da7`, which is the run in
section 2. The corpus artifacts are not byte-identical across the two sides.
`corpus-null.jsonl` and the `resource-diagnostics` reports, components and
source effects all differ in their environment digest fields (`assumptions`,
`build_identity`, `environment_sha256`, `resource_contracts_sha256`, `id`), and
the component union is 370 against 185 rows. This is
MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY, so the corpus is compared by rows:
0 moved, 0 content changes, 0 unmatched, and equal figures (rows 4,834: Safe
1,013 / Unknown 3,812 / Unsafe 9; units 185; null rows 4,921: Safe 2,549 /
Unknown 2,355 / Unsafe 17). The four other artifacts are byte-identical.

## 5. Docs

- [operations](../../agents/operations.md#integration-and-reconciliation):
  one sentence on the integration step, stating that `land.py publish`
  refuses a moved live profile unless the landing commits its new baseline
  with a history reason.
- [validation, gate slots](../../agents/validation.md#gate-slots): the receipt
  fields, the admission budget and the corpus exception. That section is the
  slot authority.
- [measure plugin](../../development/audit-subcommand-plugins.md#the-measure-plugin):
  the publication paragraph names the policy.
- `crates/quirin-audit-plugin-measure/src/sides.rs` keeps a comment saying
  that `gate_slot.py` "would insert `--memory-budget-mib=N`". The plugin's
  behaviour is unaffected, because it passes the two-word form itself. The
  comment was left unchanged so the scoped gate would not widen from one crate
  to seven (`qcore-compiler` names the crate directory). See the follow-ons.

## Gate

Scoped card gate `python3 scripts/gate_scope.py --base 486b05a71 --out
private/gate`, run on `a42edeae4` (items 1–3; before the evidence commits and
the merge of `f774f54de`), [gate.json](gate/gate.json). Console log:
`/media/Data/GitHub/quirin-lang/private/landing-publish-completion/gate-1.console.log`.
Exit 1. The selection was `quirin-audit-plugin-measure` and the Python suite.
Steps: submodules 0; fmt 0; Clippy 0; Rust suite 0 (6 units, 43 passed, 0
failed); Python suite 1 (554 tests, 2 failures). Both failures are on the base
and outside this card's files:

- `test_build_matrix_sections ... recomputed_rather_than_carried`: the
  committed matrix holds `qcore_schemas` 21.0.0 / 375 Lean audits against the
  derived 22.0.0 / 376. These are the integrator's records.
- `test_tracked_files ... reviewed_one`: the reviewed tree-walk table differs
  from the Rust tests' walks (`checkpoint_cli.rs`, `configured_cli.rs`).

Neither test reads any file this card changes. After the merge,
`python3 -m unittest scripts.tests.test_gate_slot scripts.tests.test_land`
exits 0 (39 tests). `python3 scripts/apply_amendments.py
docs/reports/landing-publish-completion/integration-amendments.json --check`
exits 0. The scoped gate has not been rerun on the final commit. The landing's
full gate is the arbiter.

## Follow-ons

- The `sides.rs` comment on the inserted spelling (section 5) is now stale.
  Fix it with the next change to the measure plugin.
- Admission reserves no memory for running audits that have not reached their
  peak (section 2). That limit is unchanged from GATE-ELASTIC.
- After the first landed publication: retire MEASURE-TOOL-LANDING-PUBLISH
  once a later card reuses the landed baseline, and amend or retire
  GATEWAY-RULE-API-SLICE-IDENTITY from the tail-pause result.
  MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY stays open; this card only
  observed it (section 4).
