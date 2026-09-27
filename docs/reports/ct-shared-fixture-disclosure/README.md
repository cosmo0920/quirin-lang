# CT shared fixture disclosure

> Canonical import maintenance: seven reviewed fixture-group digests changed when all their members adopted `#import`. Group membership and relations are unchanged; [exact attribution](../runtime-source-imports/integration-corrections.json) preserves old/new hashes. Original measurements below remain tied to their original revision.

Date: 2026-09-23. Card: `CT-SHARED-FIXTURE-DISCLOSURE`. Base `ffdddbbb`.

## 1. Result

All 50 CT cases are mapped, so it matters how independent those 50 are. This
card re-measured byte identity across every fixture file the CT harness reads,
and across all of `compiler/tests/fixtures/`, rather than relying on earlier
counts. There are **nine byte-identical groups that contain a CT file**:

- three already in the ledger
- two measured by `CT-LANE-5` but never tracked
- four found here for the first time

Every group is **disclosed**. None is decoupled. No fixture byte changed.

**Independent-artifact count.** The 50 mapped cases rest on **48 distinct
source-fixture programs**, measured by SHA-256 of each case's `source_fixture`.
The only shared primary artifact is still the CT01/CT12/CT14 program. The count
stays at 48 after comments and whitespace are stripped, so no further
near-duplicate hides behind a formatting difference. 48 of 50 is not materially
below 50. It is still recorded as a disclosure, checked rather than written by
hand, in two places:

- `docs/reevaluation/ct_manifest.json` → `artifact_identity`
- the `docs/STATUS.md` coverage table

No mapping status, stage, proof or verdict changed. `mapped_cases` is 50 and
`distinct_source_fixture_digests` is 48.

A mechanical check now enforces the disclosure:
`compiler/tests/ct_corpus.rs::ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure`.
It hashes every file under `compiler/tests/fixtures/` and fails on any of the
following:

- an unreviewed group
- an edit to one side of a reviewed group
- a stale allow-list entry
- a misrecorded role or relation
- drift in the independent-artifact figure

Seven mutations show it red, and baseline runs before and after are green (§5).

## 2. Measured groups

Method: `sha256sum` over every file under `compiler/tests/fixtures/ct/` (240
files). The same was done over the other fixture trees the CT harness reads:

- `apple-threads/`, which it passes as `-I` on macOS
- `task_worker_allocations/`, which it reads with `include_str!` and reaches
  through CT14's and CT37's `#include` shims

Those 244 files contain 5 groups. The scan was then widened to all 330 files
under `compiler/tests/fixtures/` and to all 7,096 tracked files in the
repository. The widened scans found four more groups containing a CT file, all
under `compiler/tests/fixtures/`. The only tracked copies outside that tree are
two archived report headers (G2). Raw outputs are in the private
`measure/ct-fixture-hash-1/` directory (`all.sha256`, `harness-read.sha256`,
`all-fixtures.sha256`, `repo-wide-groups.txt`).

| # | SHA-256 | Members (under `compiler/tests/fixtures/`) | Relation | Status before this card |
|---|---|---|---|---|
| G1 | `131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c` | `ct/CT01/case.qcore`, `ct/CT12/case.qcore`, `ct/CT14/case.qcore` | shared_artifact (case) | known, ledger `CT-SHARED-FIXTURE-DISCLOSURE` |
| G2 | `b5e74449cef4fc3616194be01812c83d2cb59105938eda9ec86b11d0c26cfc63` | `ct/CT01/owned_transfer.h`, `ct/CT12/owned_transfer.h` | shared_artifact (header) | measured by `CT-LANE-5` §9, untracked |
| G3 | `bf65bbc702778effa5c82d363e08eac35b948d2abb05bcae0ec2c64c3b3d3926` | `ct/CT14/completion_before_join.h`, `ct/CT37/completion_before_join.h` | shared_artifact (header) | measured by `CT-LANE-5` §9, untracked |
| G4 | `c775f7218f3394e7d5ed809fd366587cd330af8b4d68a248048b35a94e208a7b` | `ct/CT02/case.qcore`, `ct/CT14/read_before_join.qcore` | case_as_control | known, ledger `CT-FIXTURE-CROSS-CASE-DUPLICATES` |
| G5 | `7c75577b59733994bac462404ee51173d4376336b5351d1d9e9b6a380d634be6` | `ct/CT03/case.qcore`, `ct/CT04/write_after_joins.qcore` | case_as_control | known, ledger `CT-FIXTURE-CROSS-CASE-DUPLICATES` |
| X1 | `29f2f013edbbde7f68eb8881b2c3776abf5da3dd9c05743e7c11133c13538fa9` | `ct/CT08/case.qcore`, `slice_permissions/overlapping_write.qcore` | shared_with_non_ct_fixture | **new** (provenance in prose only) |
| X2 | `9a793c465809c3dd3fff452aa31f22f185cfc6825f86e9af4cb9f9ff51810f86` | `ct/CT09/case.qcore`, `source_record/point.qcore` | shared_with_non_ct_fixture | **new** (provenance in prose only) |
| X3 | `57c3072d319d2d3b8948b5435501c0419ee632c5145312f68524cda30503dc17` | `ct/CT10/distinct_units.qcore`, `bitfield_concurrency/distinct_units.qcore` | shared_with_non_ct_fixture | **new** |
| X4 | `d91828fc546b4b10fb57736dba483373216057bc119e622782baefb4bdeaac1c` | `ct/CT10/addressable.qcore`, `bitfield_concurrency/addressable.qcore` | shared_with_non_ct_fixture | **new** |

The repository-wide scan found only one group with a tracked copy outside
`compiler/tests/fixtures/`. G2's bytes also appear in
`docs/reports/w6-owned-source-tasks/sanitizer-create-failure.h` and
`sanitizer-round-trip.h`, which are frozen archives. No other tracked file
anywhere in the repository duplicates a CT fixture.

### 2.1 A correction to the inherited hazard

The ledger entry and the wave-35 report said that editing `CT02/case.qcore` or
`CT03/case.qcore` "silently changes what CT14's and CT04's controls test". That
is not how the harness behaves. Every control is read from the controlling
case's own directory (`directory.join(&control.source)` in the task-scenario
runner, and `compiler/tests/fixtures/ct/{case_id}/{source}` in the control
runners). The fixture inventory assertion also requires the control to be in
that directory. Each member of a group is therefore an **independent copy**.
An edit to one side leaves the other side untouched.

The real hazard is different and just as silent. The edit ends an identity that
reports and scenario readings rely on. Before this card nothing failed when that
happened, except for G1: `ct12_ct14_proof_reach.rs` already asserted G1 as a
proof tripwire. This card turns every identity into a pinned fact, in both
directions.

The wave-35 table also used unlabelled **MD5** prefixes (`a842abac…`,
`1e1761bb…`, `63e938d3…`) next to a SHA-256 ledger. Both identify the same
groups. The correction note beside that report records all three points.

## 3. Per-group decision

The brief allowed either **disclose** or **decouple** for each group. Every
group is disclosed:

| Group | Decision | Rationale |
|---|---|---|
| G1 | disclose | CT01 is proof-bound: its F02 trace/digest relation is derived from these bytes. CT12 and CT14 take their distinguishing scenarios from native harness data, not from the program, and `PROOF-LEG-BATCH` already withdrew their proof rows on this identity. Decoupling would invent programs the cases do not need. |
| G2 | disclose | The header follows from G1. CT12 differs from CT01 only by `fail_create: true`. |
| G3 | disclose | A one-line shim to the shared `task_worker_allocations/completion_before_join.h`. CT37 is proof-bound. Two different programs deliberately share one native adapter. |
| G4 | disclose | A control that is exactly another mapped case's refused program is a strong control, so reusing it is deliberate practice. Decoupling means changing bytes, which destroys the property being disclosed and helps nothing. |
| G5 | disclose | The same reasoning as G4. CT03 is proof-bound through F02, so its side may not change without rebinding its proof. |
| X1–X4 | disclose | A mapping lane (`CT-WIRE-LANE-2`, `CT10-SOURCE-LAYOUT`) took the capability card's own published fixture as the case or control. The provenance is right. Only the byte identity was undisclosed. |

**No fixture comment was added in any group.** Every group includes a
proof-bound side (G1, G3, G5), or a comment would either break the identity it
is meant to disclose or change executed bytes of a mapped case for no
evidential gain. The disclosure lives where the harness reads it: the
manifest's checked `artifact_identity.shared_fixture_groups`, each entry with
its digest, members, roles, relation, decision and note.

**Correction notes** sit beside the frozen reports (constraint 21), which are
unchanged. Each note is a file named `shared-fixture-correction.md` in one of
these report directories:

- `w6-ct-harness` (G1, G2, G4)
- `w6-owned-source-tasks` (G1, G2)
- `w6-completed-child-custody` (G1, G3, G4)
- `remaining-wave3-ct-integration` (G5)
- `ct-wire-lane-2` (X1, X2)
- `ct10-source-layout` (X3, X4)
- `ct07-slice-permissions` (X1)
- `aggregates-v1` (X2)
- `ct-ctwin-lane` (the §2.1 corrections)
- `stability-reeval-1` (discharge of §3.1)

Commit history also corrects one provenance assumption. The CT01/CT12/CT14
identity was created by `w6-owned-source-tasks` (`59f5c097`), which rewrote
CT12's program and added CT01's with the same bytes. `w6-ct-harness`'s
original CT12 program is not the current one.

## 4. The check

The manifest gains a required top-level `artifact_identity` object, parsed with
`deny_unknown_fields`, so deleting it fails parsing. It carries:

- `report`
- `scan_root` (`compiler/tests/fixtures`)
- `mapped_cases`
- `distinct_source_fixture_digests`
- `shared_fixture_groups`

`ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure` checks it in four
steps:

1. **Independent-artifact figure.** The test recomputes `(mapped cases, distinct
   SHA-256 over mapped source_fixture files)` from the files and compares it
   with the manifest.
2. **Measured groups.** The test takes the SHA-256 of every file under
   `scan_root`, recursively, skipping nothing. It keeps the groups that have
   two or more members and at least one member under `ct/`.
3. **Roles derived, not trusted.** A member is classed in this order:
   - `non_ct_fixture` if it is outside `ct/`
   - `case` if it is a mapped case's `source_fixture`
   - `header` if it is a `.h` file
   - `control` otherwise

   The recorded role must equal the derived one. The relation must match the
   roles:
   - `shared_artifact`: one CT role, either case or header, and no non-CT member
   - `case_as_control`: at least one case and at least one control, and no
     non-CT member
   - `shared_with_non_ct_fixture`: at least one non-CT member

   The decision must be `disclose`. A decoupled pair is no longer a group. A
   digest or member listed twice is refused.
4. **Total or reject, both ways.** Every measured group must be listed with
   exactly its members, and every listed group must be measured with exactly
   its members. The failure message names both sets and says what each
   direction requires.

Windows note: CI checks out with `core.autocrlf false` (`ci.yml`), so raw-byte
digests agree across hosts. The test walks the tree with `/`-joined relative
paths, not OS separators.

## 5. Mutations

The harness is `scripts/mutate.py`. Per-mutation logs and exits are under
`mutations/`. Each input is restored by rewriting its original bytes, and the
round trip is asserted by SHA-256. The created file is removed and its absence
asserted. Measured on this tree:

| Run | Mutation | Exit | Expected failure text |
|---|---|---|---|
| m0 | none (baseline) | 0 | — |
| m1 | append `\n` to `CT04/write_after_joins.qcore` (control side of G5) | 101 | found |
| m2 | append `\n` to `CT02/case.qcore` (case side of G4) | 101 | found |
| m3 | new `compose/ct05_copy_probe.qcore` holding CT05's bytes (outside `ct/`, where the CT inventory assertion cannot see it) | 101 | found |
| m4 | `distinct_source_fixture_digests` 48 → 49 | 101 | found |
| m5 | `CT14/read_before_join.qcore` role `control` → `case` | 101 | found |
| m6 | G4 relation `case_as_control` → `shared_artifact` | 101 | found |
| m7 | G5 removed from the allow-list | 101 | found |
| m8 | none (baseline after) | 0 | — |

Harness exit 0: every mutation was red with its expected message, and both
baselines were green.

## 6. Ledger

- **Retired:** `CT-SHARED-FIXTURE-DISCLOSURE`. The discharge it asked for ("a
  content digest beside `source_fixture` so the manifest cannot present one
  program as three", or an artifact-vs-case count) is delivered in checked
  form.
- **Retired:** `CT-FIXTURE-CROSS-CASE-DUPLICATES`. It was the same work, and
  it named this card's G4 and G5. Both pairs are disclosed in their owning
  reports, and case-as-control reuse is recorded as deliberate practice (§3).
  Both names were registered before removal.
- **Registered:** `CT-FIXTURE-IDENTITY-BYTE-SCOPE`, the check's stated boundary.
  It does not see:
  - a CT program duplicated as an embedded Rust string constant. The one
    recorded instance, CT17 against `error_payload_tasks.rs`'s `PAYLOAD`, was
    measured: it is no longer identical.
  - sharing by `#include` through non-identical shims
  - archived copies under `docs/reports/`
  - equality up to comments or whitespace. A probe found none, but nothing pins
    that.

`named_followups` goes from 276 to 275, and `dimensions.named_followups` is
275. Not touched: `CT-FIXTURE-EXTERNAL-READERS`, which concerns
`include_str!` readers of CT fixtures outside the harness and is a different
hazard. Also not touched: `CT-STAGE-TALLY-LITERAL-UNDISCLOSED`. This card moves
no tally literal.

**Pinned expectations changed deliberately:**

- the manifest schema, which gains a required `artifact_identity`
- `docs/STATUS.md`, which gains a CT independent-artifacts row and a follow-up
  count of 275. The prose count also read 282, which was already stale at 276
  on the base commit.

No contract-count, submodule, proof-digest or CT stage pin moved.

## 7. Validation

The standing per-change gate ran on 2026-09-23 over this card's tree, on base
`ffdddbbb` in its own worktree, with Clang 21.1.8 present. `clang --version` was
run in this worktree and recorded in `clang-version.txt` and `clang-version.exit`
next to this report. Each step's exit was written to a file, never piped.
Private logs are in the worktree's ignored `measure/gate-1/`. Two other cards
ran gates on the same host at the same time, with a one-minute load average of
about 10 at both the start and the end of the workspace run.

| Step | Exit | Result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | libevent `d82464a2` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,086 passed / 0 failed / 11 ignored**, 273 summaries (`workspace.log`) |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | 8 pass, 0 failed expectations |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | 11 pass, 0 failed expectations |
| `python3 scripts/check_lean_audits.py` | 0 | **165** Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | **313** tests (`python-3.log`, run after the last edit) |
| `python3 scripts/build_docs_index.py --check` | 0 | index matches |

**Failed attempts, kept:**

- The focused identity test first exited 101 (`measure/run-1/focused.log`).
  The report file it requires did not exist yet, so the test was right.
- `cargo fmt --check` exited 1 before the first commit because of one tuple
  layout in the new test. That layout was then formatted.
- The first Python run exited 1 with 1 failure out of 313
  (`CommittedMatrixIsSelfConsistent.test_the_committed_matrix_keeps_the_generators_byte_format`).
  My ledger edit had dropped the matrix's trailing newline. The newline was
  restored, the byte format was re-verified, and the rerun exited 0.

**Changes from the previous gate:**

- **Rust:** one new test. 5,085 passed moves to 5,086. That 5,085 is the
  `W42-CHUNK-LIFECYCLE-CERT` gate, which `MAIN-GUARD-REMOVAL` re-measured
  unchanged.
- **Lean:** no change. This card has no Lean component.
- **Python:** no change. The check is Rust-only.

**Why no Lean change.** Operations requires a Lean-backed module for a card
that changes semantics. This card changes no semantics. It is a disclosure
backed by a harness check, and no rule, checker or verdict is modelled. No
provider path changed, so the provider package steps do not apply.
Load-sensitive rows under rule 16 did not fail in this run.
