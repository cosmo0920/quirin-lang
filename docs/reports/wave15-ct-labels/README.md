# CT-LABELS — the current bucket label, derived from the CT lanes' own evidence

Wave 15. Card `D02-BUCKETS-LABELS`. Base: `main` `19fbaa5`, branch
`claude/ct-labels`, isolated worktree, submodules initialised
(`third_party/libevent` at `d82464a2`).

**Nothing is promoted and nothing is mapped.** CT coverage stays **32/50** with
gated sanitized-native at 4 of 32 and proof at 0 of 32; no raw Cycle 19 unit
moves, no verdict changes, no diagnostic code is added, changed or downgraded,
the contract corpus stays 64 modules and the Lean audit count stays **64** —
this card adds no audit file (§9). Two bucket *labels* move, which is the whole
point of the card: labels move, mappings do not.

---

## 1. What was open

Card HYGIENE-14 (`docs/reports/remaining-hygiene-14/README.md` §1.5) closed
`D02-BUCKETS-STALE` by deriving the **counted set** of unmapped cases from the
manifest, and recorded the half it could not close in one sentence:

> **A count is now current by construction; a label is historical by
> construction.**

It named the two labels that were measurably out of date — CT11's
`blocked-on-task-lowering (share captures)` and CT46's
`blocked-on-unimplemented-syntax (no mutex value)` — and named the missing
mechanism exactly:

> Deriving a *current* label would need a per-case record later cards actually
> write, and the mechanism for that already exists in one direction only:
> `cases_examined_and_left_unmapped` in each `ct_stage_evidence.json` carries a
> re-verified `why_still_unmapped` prose, but no bucket field.

This card adds the bucket field, the resolution rule that consumes it, and the
refusals that keep it honest. It is the same shape of repair as its predecessor:
HYGIENE-14 made the *set* a manifest fact, and this makes the *label* a lane
fact, so a lane that re-measures a case leaves the published label correct in
the same commit that measures it.

---

## 2. The field

Each per-card `ct_stage_evidence.json` may carry one optional top-level block:

```json
"unmapped_reasons": {
  "CT46": {
    "bucket":      "blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)",
    "reason":      "The recorded Send/Sync-derivation blocker is retired: ...",
    "measured_by": "docs/reports/wave13-ct34/README.md section 'CT46 - stale, ...'"
  }
}
```

Rows are for cases the lane **examined and left unmapped**. All three fields are
required and must be non-blank; a row is refused otherwise. An optional
`supersedes` key names an earlier record whose label this row replaces (§3).

The block sits beside the existing `cases_examined_and_left_unmapped` prose
rather than replacing it: that prose is a lane's narrative, this is the machine
input, and conflating them would have forced a schema change on eight archived
files instead of an addition to three.

**Applied forward.** New CT lanes write the block for every case they examine
and leave unmapped. Archived files gain it **only** where their own README
already states the measurement, migrated verbatim with a per-file
`unmapped_reasons_provenance` note. Nothing is invented (§4, §5).

---

## 3. The resolution rule

> Each unmapped case's current label is the **newest CT lane record that
> measured it**, else **card T00's frozen archive**. Both halves are published:
> the label and its `label_source`.

"Newest" is position in `EVIDENCE_FILES`, the landing order the summarizer
already used for stage evidence. `label_source` is either the lane record's
repository-relative path or the literal `t00-archive` — deliberately not a path,
so "this label is card T00's 2026-09-07 analysis rather than a current
measurement" is legible at a glance. It is the same current-beside-archive
convention `blocker_current` introduced in wave 11, now uniform across every
case instead of hand-wired to one.

Published in two places, both generated:

* `docs/reviews/r0_3/acceptance.json` — `unmapped_bucket_counts` (now counting
  resolved labels) plus the new generated field `unmapped_label_sources`;
* the acceptance matrix's `ct_rows` — `current_bucket`, `current_bucket_source`,
  `archived_bucket` (T00's text, kept beside the current one rather than
  overwritten), and for a lane-sourced row `blocker_current`,
  `blocker_superseded_by` and `blocker_measured_by`.

The sum-equals-`ct_unmapped` property HYGIENE-14 installed is unchanged and
still refused in both derivations; the counts are now a tally of resolved
labels rather than archived ones.

### Both derivations, cross-checked

`scripts/ct_stage_summary.py::unmapped_labels` and
`compiler/tests/ct_corpus.rs::unmapped_labels` implement the rule
independently and are compared through the committed record, exactly as every
other generated field is. Neither can read the other's source, so the
**evidence-file list itself** is pinned through a third record:
`acceptance.json`'s `ct_evidence.derived_from` must equal the manifest followed
by that derivation's own file list, asserted on both sides (refusal R4).

---

## 4. The migration, case by case, with citations

Five rows across three archived files. Every `reason` is a verbatim copy of a
string already in that file; every `bucket` is cited below.

| Case | Migrated into | Bucket | Bucket cited from | Reason copied from | Label movement |
|---|---|---|---|---|---|
| CT11 | `remaining-wave3-ct-integration/ct_stage_evidence.json` | `blocked-on-scenario-mismatch (no local alias form)` | that card's README §6: *"two are added (`blocked-on-scenario-mismatch (no local alias form)` for CT11 …)"* — the label that card itself wrote into `acceptance.json`'s then-hand-kept counts | `cases_examined_and_left_unmapped.CT11.reason_left_unmapped`, measured in README §1.2 (`Q600 LoanOutstanding { resource: 1, outstanding: 512, borrowers: [3] }`) | **moves** from T00's `blocked-on-task-lowering (share captures)` |
| CT40 | `wave9-ctwin/ct_stage_evidence.json` | `blocked-on-gateway-expressiveness (C-twin measured)` | that card's README §4: *"CT40/CT41/CT42 are now `blocked-on-gateway-expressiveness (C-twin measured)` (3)"*, and §6 item 3 | `cases_examined_and_left_unmapped.CT40.why_unmapped` | text unchanged; source moves to the lane (§6) |
| CT41 | `wave9-ctwin/ct_stage_evidence.json` | as CT40 | as CT40 | `…CT41.why_unmapped` | as CT40 |
| CT42 | `wave9-ctwin/ct_stage_evidence.json` | as CT40 | as CT40 | `…CT42.why_unmapped` | as CT40 |
| CT46 | `wave13-ct34/ct_stage_evidence.json` | `blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)` | **composed**, and recorded as the card's one deviation (below) | `cases_examined_and_left_unmapped.CT46.why_still_unmapped`, measured in README §"CT46 — stale, and updated" | **moves** from T00's `blocked-on-unimplemented-syntax (no mutex value)` |

### The one composed label, recorded rather than hidden

Wave 13's README states no bucket *label* for CT46 — only the named obstacle
`CT46-MUTEX-CAPTURE`. The label above is therefore **composed from that name**
in the shape the sibling buckets use, and that deviation is written into the
file's own provenance note as well as here. Nothing about the *measurement* is
composed: the reason is the lane's verbatim string. The alternative — leaving
CT46 on T00's `blocked-on-unimplemented-syntax (no mutex value)` — publishes a
label that cards T03 and CT33-RANKS made false, since mutex values exist.

### The resulting tally

Sum is 18 = `ct_unmapped`, before and after.

| Bucket | n | Cases | Source |
|---|---|---|---|
| `blocked-on-unimplemented-syntax` | 6 | CT07, CT08, CT09, CT10, CT23, CT49 | `t00-archive` |
| `blocked-on-gateway-expressiveness (C-twin measured)` | 3 | CT40, CT41, CT42 | wave 9 |
| `blocked-on-W3/W4` | 2 | CT21, CT22 | `t00-archive` |
| `ambiguous + unimplemented-syntax` | 1 | CT06 | `t00-archive` |
| `blocked-on-scenario-mismatch (no local alias form)` | 1 | CT11 | **wave 3** (was `blocked-on-task-lowering (share captures)`) |
| `DECIDABLE-NOW (with a distinguishing fixture)` | 1 | CT24 | `t00-archive` |
| `ambiguous (partially unblocked)` | 1 | CT25 | `t00-archive` |
| `ambiguous` | 1 | CT44 | `t00-archive` |
| `blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)` | 1 | CT46 | **wave 13** (was `blocked-on-unimplemented-syntax (no mutex value)`) |
| `DECIDABLE-NOW (coarser reason)` | 1 | CT47 | `t00-archive` |
| **sum** | **18** | | 5 measured / 13 archive |

Thirteen of eighteen labels are still card T00's 2026-09-07 analysis. That is
not progress being hidden — it is the number this card makes visible for the
first time, and it is the honest size of what a future CT lane still has to
re-verify.

---

## 5. What did **not** migrate, and why

The rule is "only where the lane's own README already states the measurement",
and applying it strictly leaves four groups out:

* **CT34's pre-mapping state is moot.** Wave 5, wave 11 and wave 13 all recorded
  a CT34 reason while it was unmapped; the wave-13 lane then mapped it. A row
  for CT34 in any of those files would be a bucket naming a mapped case, which
  is refusal R1. The case simply leaves the table, which is what wave 13's own
  record already says it should.
* **CT35/CT36/CT38/CT39** carry measured reasons in wave 3's record and were
  mapped by card T08-SRC-CT. Same refusal, same reason for omission.
* **CT06, CT24, CT25** carry a measured reason in wave 3's record but no bucket
  label anywhere in that card's output. Inventing one is exactly what this
  mechanism forbids, so they stay on `t00-archive`.
* **Wave 11's CT46 row** re-verified the *reason* (`recorded_reason_unchanged:
  true`) against card CT33-RANKS and states no bucket. Migrating a bucket into
  it would have been an invention, and would have manufactured a supersession
  pair rather than recording one. Wave 13's row therefore carries no
  `supersedes` key: a lane label always beats T00's archive by the resolution
  rule itself, so declaring that would be vacuous — and vacuous supersessions
  are refused (R2c). The conflicting-label direction is exercised by scratch
  mutation instead (§7).
* **T03, wave 5 and wave 8** records carry reasons for cases that are either
  mapped now or are re-statements of another lane's reason with no bucket of
  their own. None migrated.

---

## 6. Two findings, measured, recorded rather than repaired

### 6.1 The "frozen" T00 archive was edited in place

`docs/reports/remaining-t00/coverage.json` is described everywhere as card T00's
audit *frozen at revision ea2af0f*. It has two commits:

```
97e69c0 Map CT43 and CT45 as C twins on a new legacy_c harness route
7da4f4e Record T00 CT stage-coverage audit
```

Card CT-CTWIN (wave 9) rewrote CT40/CT41/CT42's rows in that archive with its
own measured blockers and its own new bucket, preserving the prior text under
`superseded_blocker` / `superseded_bucket` / `superseded_by`, and its README §6
item 3 says so plainly. So the archive was never fully frozen, and the *label*
half of HYGIENE-14's boundary was already being worked around — by editing
another card's audit, which is the practice this mechanism replaces.

**Consequence, and why nothing had to move:** the label those three cases now
resolve to is byte-identical to the one that edit installed, so the derived
tally does not change; only the published `label_source` does, from
`t00-archive` to the lane that actually measured it. `coverage.json` is **not**
edited by this card — it is outside this card's ownership, and reverting the
wave-9 rows would move a pin this card was not asked to move. Named for the
integrator as **`CT-LABELS-T00-REVERT`** (§11).

### 6.2 `acceptance.json`'s note had already gone stale

Its closing sentence still read *"The hand-maintained fields in this block are
the triage ones — unmapped_bucket_counts, newly_decidable_not_mapped and its
reasons"*, three fields after HYGIENE-14 moved `unmapped_bucket_counts` into
`generated_fields`. Corrected in the same edit that documents this card's rule,
with the counting rule and the 5-measured/13-archive split stated. The note is a
hand-maintained field, which is why nothing caught it; `generated_fields` itself
is pinned element-for-element in both derivations and did catch its own change.

---

## 7. Refusal matrix — 13 scratch mutations: 11 refused, 2 admitted by design

Eleven rows below. Each runs hermetically inside
`scripts/tests/test_ct_stage_summary.py`, against a scratch copy of the file set
that is deleted afterwards; row R5 is three mutations (each required field
blanked in turn), which is where 13 comes from. Six of them — R1, R2a, R2b, R3,
R4, R5 — were additionally applied to the **real tree** and run against
`cargo test -p qcore-compiler --test ct_corpus` and/or
`python3 scripts/ct_stage_summary.py . --check`, so the Rust derivation's
refusals are measured and not merely mirrored in Python; each was then reverted
with `git checkout -- <path>`. **None was committed**; the tree was verified
clean afterwards (`git status --short` empty), and the CT corpus test re-run
green.

The two admitted rows are marked as such: a refusal matrix in which everything
refuses would not show that the mechanism still lets an honest lane through.

| # | Refusal | Mutation | Refused by (measured message) |
|---|---|---|---|
| R1 | a bucket naming a **mapped** case | add a CT34 row to wave 13's file | Rust: `CT34: docs/reports/wave13-ct34/ct_stage_evidence.json records an unmapped-case bucket, but the manifest maps this case`; Python: same, `SystemExit` |
| R1b | same, reached from the manifest side | mark CT46 `mapped` without deleting its row | `SystemExit` on the same message (Python test) |
| R2a | two records, **different** buckets, no supersession | give the newest record a conflicting CT46 bucket | `CT46: two evidence records label this case differently (…wave13… says "blocked-on-mutex-capture (CT46-MUTEX-CAPTURE)", …ct34-sanitized… says "a different label") and neither declares a supersession` |
| R2b | a **stale** supersession | the same row declaring it supersedes wave 9 | `CT46: …ct34-sanitized… declares its bucket supersedes …wave9-ctwin…, but the record holding the label is …wave13-ct34…` |
| R2c | a **vacuous** supersession | a CT44 row declaring it supersedes T00's archive | `CT44: … declares its bucket supersedes …, but no earlier record labels this case` |
| R2-ok | **admitted by design**: a declared supersession | the same row declaring `supersedes: wave13` | accepted; the label and `label_source` become the newer record's |
| R2-agree | **admitted by design**: re-verification reaching the **same** label | newest record repeats wave 13's bucket, no `supersedes` | accepted — agreement is not a second claim, and refusing it would make honest re-verification harder than silence |
| R3 | a `label_source` the file set cannot support | point CT44 at a non-evidence file, CT47 at a record with no row for it | `acceptance.json labels CT44 from …README.md, which is not a CT stage-evidence record` / `… labels CT47 from …wave9-ctwin…, which records no bucket for that case` |
| R4 | the two derivations reading different files | drop the last entry of `derived_from` | Rust: `acceptance.json 'derived_from' must be the manifest plus this derivation's evidence-file list…`; Python: the matching DRIFT line |
| R5 | a row missing a required field | blank each of `bucket` / `reason` / `measured_by` in turn | `CT41: docs/reports/wave9-ctwin/ct_stage_evidence.json carries no measured_by for this case` |
| R6 | a changed lane label going unnoticed | relabel CT46's bucket | the tally moves (still summing to 18) **and** `acceptance.json 'unmapped_bucket_counts' is not what the manifest derives` |

Every pre-existing refusal is retained and re-run: the duplicate-triage-row
refusal, the unclassified-case hole refusal, the sum property, the
mapping-moves-the-tally property, the stage-evidence supersession rules and the
`newly_decidable_not_mapped` check.

---

## 8. What did not change

* **No production Rust.** The whole `crates/` and `compiler/src/` diff is empty;
  the only Rust file touched is the test `compiler/tests/ct_corpus.rs`.
* **No manifest edit.** `docs/reevaluation/ct_manifest.json` is byte-identical:
  no mapping status, stage record or note needed to move. The card permitted a
  `note` edit if one was needed; none was — wave 13 had already rewritten the
  manifest note's CT46 paragraph to the measured obstacle, which is precisely
  why CT46's migration is a copy rather than a decision.
* **No fixture, no diagnostic code, no checker rule.**
* **CT coverage stays exactly 32/50**, with the same mapped set, the same
  `case_depth`, the same `stage_histogram`, the same `observation_counts` and
  the same `host_scope`/`tool_scope`. The only generated fields that move are
  `unmapped_bucket_counts` (two rows renamed) and the new
  `unmapped_label_sources`.

---

## 9. Lean

**Not applicable, and recorded as a judgment rather than an omission**, on the
same terms cards T00-CT-SCHEMA, T08-SRC-CT, CT-CTWIN and the wave-11 and wave-13
CT lanes recorded for the same kind of work. This card adds no checker rule, no
diagnostic code, no lowering, no model and no semantic decision procedure: it
adds a JSON field, a resolution function over records, and the refusals that
keep the two implementations of that function agreeing. The property that would
be worth proving — "the published label is the newest measurement" — is a
statement about file ordering with no semantic content, and its two
implementations are already tied to each other through a committed record that
both assert against, which is stronger evidence for *this* claim than a
transcribed model would be. `scripts/check_lean_audits.py` stays at **64
audits** and the acceptance matrix's derived `validation.lean_audits` row does
**not** move.

---

## 10. Gate

| Gate | Command | Result | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | run twice, **both exit 0**; the second reports **2,562 passed / 0 failed / 8 ignored**. One transient observation under concurrent load is recorded below | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | **1 pre-existing warning, outside this card's ownership** (below, §11) | 101 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **70 passed / 0 failed** | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | **64 audits passed**, no new axiom | 0 |
| Python tests | `python3 -m unittest discover -s scripts/tests` | **146 passed** (135 at base + 11 new) | 0 |
| CT summary | `python3 scripts/ct_stage_summary.py . --check` | no DRIFT; 32 mapped / 18 unmapped | 0 |
| CT corpus | `cargo test -p qcore-compiler --test ct_corpus` | 5 passed | 0 |
| Matrix regeneration | `build_matrix.py . 0 --out <scratch>` | CT section reproduces the manifest case for case; the committed matrix was **not** written | 0 |
| Acceptance writer | `ct_stage_summary.py . --write` twice | byte-identical the second time (fixed point) | 0 |

### A transient wall-clock observation, recorded because it was seen

```
crates/qcore-verifier-host/tests/standalone_host.rs:175:5
closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request
assertion failed: start.elapsed() < Duration::from_secs(1)
```

**The workspace gate is green**: both full runs exited 0 and the completed one
records 2,562 passed / 0 failed / 8 ignored, with this test among the three
passing in `standalone_host`. The failure above was observed once, mid-run,
while other cargo work overlapped on this host, and it also passes on an
isolated re-run (`cargo test -p qcore-verifier-host --test standalone_host`:
3 passed). It is reported rather than dropped because a one-second wall-clock
bound in the same family as the project's recorded load-flaky targets (`vp04`,
P02's `RLIMIT_NPROC`, P03's confined write) is worth knowing about before a
future wave reads it as a regression. This card touches no host, transport or
timing code — its whole Rust diff is one test file — and the assertion is
**deliberately not weakened**.

One measurement caveat, stated because it affected this record: the two
workspace runs wrote to the same scratch log, so a mid-run read of that file
mixed both. The numbers above are each run's own reported summary, not a count
taken from the shared file.

### The clippy warning is pre-existing and not this card's

```
crates/qcore-c-gateway/tests/prod_contracts.rs:699:9
warning: using `contains()` instead of `iter().any()` is more efficient
```

It is the **only** clippy diagnostic in the workspace. Measured to be present at
this card's base `19fbaa5` (`git show 19fbaa5:crates/qcore-c-gateway/tests/prod_contracts.rs`
carries the same `refusals.iter().any(|text| *text == …)`) and still present on
current `main` `4b6c309` at line 712. It comes from card PROD-CONTRACTS
(`f6e68eb`, wave 14) and fires under clippy `0.1.98 (88d9e12ae1 2026-08-18)`.
The file is **not** in this card's ownership list, so it is reported rather than
edited; the fix is one mechanical line, semantics-preserving, named in §11.

---

## 11. Boundaries and integrator carry list

1. **`CT-LABELS-CLIPPY`** — the pre-existing `manual_contains` warning in
   `crates/qcore-c-gateway/tests/prod_contracts.rs:699` (line 712 on current
   `main`). One-line mechanical fix:
   `refusals.contains(&"arguments beyond the declared parameters of \`flb_log_print\` are outside the resource projection")`.
   Not this card's file; the standing clippy gate is red on `main` without it.
2. **`docs/reviews/r0_3/acceptance_matrix.json` needs regenerating.** The card
   forbids editing it, and its `ct_conformance` section will change on the next
   `build_matrix.py` run: CT11's and CT46's `current_bucket`, the new
   `current_bucket_source` / `archived_bucket` / `blocker_measured_by` per-row
   fields, the new section-level `unmapped_label_sources`, the two renamed rows
   of `unmapped_bucket_counts`, and the notes text. Verified by regenerating
   into a scratch file; the committed matrix is untouched by this card.
3. **`CT-LABELS-T00-REVERT`** (new, §6.1) — card CT-CTWIN edited card T00's
   archived `coverage.json` in place for CT40/CT41/CT42. The derived label no
   longer depends on that edit, so reverting those three rows to their
   pre-wave-9 text is now a safe, tally-neutral hygiene move — but it is a pin
   change in a file this card does not own, and is left to whoever owns it.
4. **Thirteen of eighteen labels are still `t00-archive`** — CT06, CT07, CT08,
   CT09, CT10, CT21, CT22, CT23, CT24, CT25, CT44, CT47, CT49. Each is card
   T00's 2026-09-07 analysis, not re-verified against the current
   implementation. The mechanism to correct any of them now exists and costs a
   lane three JSON fields; this card deliberately re-verified none of them,
   because measuring a blocker is a CT lane's work and inventing one is what the
   mechanism forbids.
5. **`measured_by` is prose, not a checked citation.** The refusals check that
   the field is present and non-blank, that its case is unmapped, and that no
   two records disagree silently. Nothing checks that the README line it names
   says what the row says; that remains an author's claim, as the surrounding
   `derivation_rule` and `purpose` fields already are.
6. **`CT-LABELS-FLAKE`** (new, informational — the gate is green) —
   `standalone_host.rs`'s
   `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
   carries a one-second wall-clock bound and was seen to fail once under
   concurrent load on this host, in the same way `vp04`, P02's `RLIMIT_NPROC`
   and P03's confined write already do. It passes in both completed workspace
   runs and on an isolated re-run. Untouched by this card and not weakened;
   worth adding to the recorded load-flaky list so a future wave does not read
   it as a regression.
7. **Provider-test count.** This run measures 70 passing provider tests where
   wave 14 recorded 102. Nothing under `providers/` or `crates/qcore-schemas/`
   is in this card's diff and the packages were not rebuilt here (no schema or
   package version moves), so the difference is environmental rather than
   attributable; the integrator's own rebuild-and-verify is the number to trust.
8. **The rule orders by landing position, not by date.** `EVIDENCE_FILES` is a
   hand-kept list in both derivations, pinned to each other through
   `derived_from`. A lane that adds its record in the wrong position would order
   labels wrongly without tripping any refusal — the same property the stage
   evidence already relies on.

---

## 12. Files changed

| Path | Change |
|---|---|
| `scripts/ct_stage_summary.py` | `unmapped_labels`, `unmapped_label_sources`, `check_label_sources`, `_lane_labels`, `_triage_rows`, `ARCHIVE_SOURCE`; `unmapped_bucket_counts` now counts resolved labels; `unmapped_label_sources` joins `GENERATED_FIELDS`; `derived_from` pinned in `check_acceptance` |
| `scripts/tests/test_ct_stage_summary.py` | eleven new tests (resolution, source support, R1, R1b, R2a/b/c, R2-ok, R2-agree, R3, R5, R6, `derived_from`) and two scratch-root helpers |
| `compiler/tests/ct_corpus.rs` | the independent Rust `unmapped_labels`, its `EVIDENCE_FILES`/`ARCHIVE_SOURCE` constants and three refusals; `unmapped_label_sources` added to the pinned generated-field set and the compared fields; the `derived_from` cross-pin |
| `docs/reports/remaining-d02/build_matrix.py` | the hand-wired `SUPERSEDING_REASONS` reader replaced by the shared resolution; `current_bucket_source`, `archived_bucket`, `blocker_measured_by` per row; section-level `unmapped_label_sources`; notes text |
| `docs/reviews/r0_3/acceptance.json` | regenerated by `--write` (two bucket rows renamed, `unmapped_label_sources` added, `generated_fields` extended) plus the corrected hand-maintained note (§6.2) |
| `docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json` | CT11's migrated row and its provenance note |
| `docs/reports/wave9-ctwin/ct_stage_evidence.json` | CT40/CT41/CT42's migrated rows and their provenance note, including the §6.1 finding |
| `docs/reports/wave13-ct34/ct_stage_evidence.json` | CT46's migrated row and its provenance note, including the composed-label deviation |
| `docs/reports/wave15-ct-labels/README.md` | this record |
