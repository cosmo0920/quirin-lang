# EVALUATION-PROFILES: profile records, cycle bindings, frozen/live policy

Card of the pluggable-cards plan ([seam A, "Evaluation profiles as
data"](../../plans/pluggable-cards/README.md#a-decouple-implementation-from-contracts-and-corpus-the-main-issue),
phase P1; user decision 2 of 2026-10-24). Base `710021bd7`. It changes no
analyzer code, contract, theorem or audit input selection, so it needs no
identity run; it publishes one fresh measurement of the three live profiles on
BASE as their first baseline.

## What changed

**Profile records.** `contract/evaluations/<name>/evaluation.json` holds one
evaluation profile: name, policy `live` or `frozen` at an
analyzer revision, route, audited source revision and submodules, inputs
(tracked files with digests; untracked prepared compile databases with their
digests, for the measurement tool to verify), the contract-set reference, the
baseline record and its digest, the append-only baseline history of a live
profile, and the evidence version. The baseline record
(`baseline.json`) carries the figures and, for a frozen profile, the committed
evidence files whose bytes must stay stable. The schemas, the policy and the
commands are specified once, in the docstring of
[`scripts/evaluation_profiles.py`](../../../scripts/evaluation_profiles.py).

**Why not the plan's `contract/profiles/<name>/`.** The first version placed
the records there, as the plan spells it, and the scoped gate failed: 35 Rust
tests and both reevaluation probes. `compiler/src/first_party.rs` `packages()`
loads *every* directory under `contract/profiles` as a source-set profile, and
one directory without `profile.json` makes `first_party_registry()` empty.
That is fail-closed (every provider is absent, so `Unknown`), but it changes
analyzer behaviour, and nothing in this card may do that. The records
therefore live in `contract/evaluations/`. No code reads that directory except
`evaluation_profiles.py`, and no contract corpus root or loader walks it
(`contract/corpus.json` roots are the provider `contracts/` trees). The
failing gate attempt is kept as evidence (see Gate).

| Profile | Policy | Route | Bound by |
|---|---|---|---|
| `eight-home` | live | pinned corpus, 185 units, eight-home link map | matrix `conditional_eight_home_cycle3.routes.corpus` |
| `fluent-bit-system` | live | `quirin-audit current`, system selection | matrix `…routes.system` |
| `fluent-bit-tail-pause` | live | `quirin-audit current`, tail-pause selection | matrix `…routes.slice` |
| `cycle-17-cfl` | frozen at `b26b4aa35` | unrestricted-C import probe of CFL `6e35cca` | `CYCLE_17.md`, `CYCLE_17_CFL_VALIDATION.md` |
| `cycle-18-libevent` | frozen at `bb76178d5` | pinned-Libevent source-contract audit | `CYCLE_18.md`; `CYCLE_18_LIBEVENT_VALIDATION.md` through the bindings file (below) |
| `cycle-19-fluent-bit` | frozen at `b92688d22` | pinned corpus (batch-2 link map), DETACH-LOOKUP-MEMBERSHIP | `CYCLE_19*.md`, matrix `cycle19_raw_units` |
| `cycle-19-monkey-callbacks` | frozen at `016df072c` | Monkey callback coverage, W42 | matrix `cycle19_monkey_callback_coverage` |
| `cmetrics-counter-configured` | frozen at `55145be4f` | configured cmetrics-counter audit, C02 | matrix `configured_cmetrics_audit` |
| `chunkio-core-configured` | frozen at `60eef72ac` | configured chunkio-core audit, G03 | matrix `configured_chunkio_core_audit` |

**Contract sets and pin-lock windows (user decision 9a, 2026-10-26).** Pin-lock
sets are tied to a Fluent Bit revision, and only the latest set plus the two
before it are retained. CONTRACT-SUPERSESSION implements that window; this
card only avoids contradicting it. `contract_set.lock_set` says which set a
profile uses:

- a live profile is `{"state": "latest"}`: it binds to the latest revision's
  lock set, which is the tree's provider locks;
- a frozen profile is `{"state": "retained", "fluent_bit_revision": R,
  "evidence_sha256": D}` while R's lock set is kept, or `{"state": "retired",
  "evidence_sha256": D}` once it is not. Its `references` list is empty.

`D` (`contract_evidence_digest`) is the sha256 of the sorted
`<path>\t<blob id>` lines of every `.qv`, `.qcore`, `*contracts.lock` and
`contract-load-path.json` file tracked at the frozen revision. It is read from
git objects only, so removing every contract file from the tree does not
break the frozen check (tested). The one change a frozen record may make on
its own is retirement: `retained` becomes `retired` with the same `D` and
nothing else changed. All six frozen profiles are `retired` today, because
their revisions predate the window. `cycle-17-cfl`'s revision has no contract
file at all, so its `D` is the digest of the empty listing.

Each frozen revision is the analyzer revision of the last report that measured
the claim: the revision a report names (`b92688d22`), else the commit that
published the record. Frozen figures are transcribed from that record (the
matrix row at BASE, or the cycle document); their tracked inputs are pinned by
the digest of the blob at the frozen revision, so later edits of the tree
(`profile.json`, selections) do not touch a frozen claim, and their committed
evidence files (`artifacts`) by working-tree digest.

**Cycle bindings.** Every `docs/cycles/CYCLE_*.md` now carries one line
`Evaluation profile: \`<name>\` (<policy>; [record](…)).` or
`Evaluation profile: none (<reason>).` Cycles 2–16, 7.5 and 20 claim no audit
measurement over a pinned source set and say so. One exception:
`CYCLE_18_LIBEVENT_VALIDATION.md`'s LF bytes are the `historical_validation`
digest pinned by `providers/qcore-provider-libevent/contracts/monkey/native_platform_matrix.qv`
(and two reports), so editing it would break that pin; it is bound in
`contract/evaluations/bindings.json` (`pinned_cycle_documents`)
instead. The same file classifies every key of the acceptance matrix's `corpus`
section: a profile, a split by route, or `none` with a reason (the five
contract-inventory and submodule pins). The search for pinned expectations
found no test or report script that pins corpus, system or slice totals:
Rust tests pin contract counts (CONTRACT-LOCKS' territory) and regenerate
their own small results; the corpus figures live in the matrix rows, the cycle
documents and the reports. So no Rust test changed.

**Policy, enforced.** `python3 scripts/evaluation_profiles.py check --history`,
run by the Python suite (`test_evaluation_profiles.py`), checks the tree
(records, baselines and artifacts byte-stable, live inputs equal to their
digests) and every commit that touches an evaluation-profile directory against
its first parent:

- a frozen profile's files never change, and it never becomes live or
  disappears; the only exception is a reviewed, content-addressed transition
  listed in `frozen_amendments` (none today);
- a live profile's baseline moves only in a commit that appends a
  `baseline_history` entry with the new digest, the measured revision, a
  non-empty reason and a source; history is append-only; a change of a live
  profile's tracked inputs (a re-base) is a baseline change; a live profile can
  be frozen, never deleted.

A result move that is not committed cannot be seen by a test, so the script
also gives the tool to the measurer: `summarize` turns a run's report and null
sidecar into the profile figures (route totals plus row-level verdict
digests, keyed as the AUDIT-CANONICAL-PATHS identity comparisons key rows, so
the clock-dependent pre-pass key does not enter), `compare` exits 1 when a run
moved a live profile and 2 for any frozen profile, and `record` writes the new
live baseline with its history entry.

**Amendments `profiles[]`.** The field AMENDMENTS-SCHEMA reserved is defined
here, narrowly: the live profiles whose baseline the card's measured tip
records. Every name must be a profile, none may be frozen, and with `--base`
the list must equal the live profiles whose baseline changed since the base
(`evaluation_profiles.py amendments <file> --base <rev>`). `apply_amendments.py`
still only validates the shape; calling this check from it is follow-up
`EVALUATION-PROFILES-AMENDMENTS-CHECK`.

**Docs.** One paragraph in [validation](../../agents/validation.md#corpus-arbiter-and-its-job-count)
points at the profiles; the script's docstring is the authority.

## The live baselines on BASE

Each live profile needs a first baseline. No combined measurement of `main`
existed after the CMetrics landing: AUDIT-CANONICAL-PATHS measured
`fb5412930`, and the CMetrics cards measured their own isolated tips. So
the card measured BASE once ([`scripts/baseline.sh`](scripts/baseline.sh)).
It used a `git archive` snapshot of `710021bd7`, whose
`third_party/libevent` is a symlink to the card worktree's pinned submodule
(MEASURE-SNAPSHOT-HAZARDS item 1). It built its release `quirin-audit`
(`8c0f1634…`) and ran the AUDIT-CANONICAL-PATHS driver unchanged on the
original pinned inputs: link map `98a4db76…`, llvm-21 Clang, `--jobs 3`, one
`gate_slot.py audit` slot (7 GiB) per route. Every route exited 0, and the
driver found the snapshot, link map, external inputs and binary unchanged
([baseline/](baseline/): run records without the 7.6 MB snapshot listings,
which are reduced to their digests, plus the slot receipts).

| Profile | Units | Rows Safe / Pending / Unknown / Unsafe | Null rows Safe / Unknown / Unsafe |
|---|---|---|---|
| `eight-home` | 185 | 1,013 / 0 / 3,812 / 9 (of 4,834) | 2,549 / 2,355 / 17 |
| `fluent-bit-system` | 15 | 101 / 0 / 343 / 4 (of 448) | 211 / 243 / 0 |
| `fluent-bit-tail-pause` | 16 | 57 / 4 / 342 / 1 (of 404) | 190 / 216 / 0 |

The `baseline.json` figures add the unit tallies and sha256 digests of the
sorted unit, row and null verdict tuples, so `compare` also sees a row-level
move that leaves the totals unchanged. Rerun on these reports, `compare`
exits 0 for each live profile and 2 for `cycle-19-fluent-bit`.

Against AUDIT-CANONICAL-PATHS at `fb5412930` (1,009 / 3,816 / 9;
system 99 / 345 / 4), BASE has four more corpus rows and two more system rows
Safe. The slice and all null tallies are unchanged. The commits between the
two revisions are the CMetrics initializer and map/opts landing
(`7081bf1dc`), the gate cards and AUDIT-CANONICAL-PATHS. This card did not
attribute the moved rows: the earlier reports are not kept, so the moved rows
cannot be named. The CMetrics card's own isolated measurement
(`e92d8aef3`, corpus 1,009 → 1,011) is the only attribution on record. These
figures are the profiles' first baseline, not a claim about any rule.

## Boundaries

- Freezing is a records decision, not evidence: a frozen profile's figures
  describe the analyzer at its revision. Frozen profiles are not re-measured,
  so their untracked inputs are recorded as the routes spelled them, with the
  digests read on 2026-10-25 for the inputs that still exist.
- The matrix row bound to the live profiles (`conditional_eight_home_cycle3`)
  still shows batch-5's figures; the binding names its profile, it does not
  regenerate the row (follow-up `EVALUATION-PROFILES-MATRIX-ROWS`).
- `STATUS.md`'s headline figures are prose and are not bound mechanically.
- Historical reports keep their own claims and are not rebound; new cards name
  their profiles in `profiles[]`.
- The history check needs the full object store: a shallow clone or an export
  fails it, closed, with the reason. CI does not run the Python suite.

## Gate

`python3 scripts/gate_scope.py --base 710021bd7 --out private/gate-4` on
measured tip `572489ae7` exited 0 ([gate.json](gate.json); Clang recorded
beside it). It ran in **full** mode. `gate_scope.py` cannot classify
`contract/evaluations/**`: no crate owns or reads it, and it is not prose,
Python, Lean or provider tooling. So the scoper fails closed to the whole Rust
suite (follow-up `GATE-SCOPE-EVALUATION-RECORDS`).

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule checked out |
| `cargo fmt --all -- --check` | 0 | |
| Clippy, `-D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot) | 0 | 553 units, 566 summaries; 8,671 passed, 0 failed, 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Python suite | 0 | 477 tests |

Lean did not run, because `proof/` is untouched. The provider gate did not
run, because `providers/` is untouched. Earlier attempts are in
[gate-attempts/](gate-attempts/attempts.json):

- attempt 1 was stopped when decision 9a arrived;
- attempt 2 refused an existing output directory;
- attempt 3 failed with the records under `contract/profiles/`: 35 failed
  tests in six first-party-provider units, and both probes 101.

The commit after the measured tip adds only this report's records.

## Follow-ups

Registered in [integration-amendments.json](integration-amendments.json):

- `EVALUATION-PROFILES-AMENDMENTS-CHECK`
- `EVALUATION-PROFILES-MEASURE-READER`
- `EVALUATION-PROFILES-MATRIX-ROWS`
- `GATE-SCOPE-EVALUATION-RECORDS`
- `REBASE-CORPUS`

Not registered: the plan's §3 A spells the record location
`contract/profiles/<name>/evaluation.json`. Correcting that wording to
`contract/evaluations/` is an integrator edit of the plan.
