# Wave 34 close

[Development table of contents](../../../DEVELOPMENT.md) ·
[Increment record](../../development/increments/wave-34.md) ·
[Reconciliation record](reconciliation.json) · [Gate](gate.json)

**Three cards merged. One arbiter ran.** The reconciliation swept **25**
integrator-addressed items across those three cards and gave every one a verdict
measured against the tree. **One was unapplied**, and it is the kind the sweep
exists to catch: the arbiter asked for four residuals by name, the card that
merged after it moved `named_followups` without them, and all four reached no
tracker. They are registered here.

The gate **reproduces the `DOCS-COMPACT-6` tip figures exactly and
independently**, so `AGENTS.md`'s Merged gate row is confirmed rather than
carried and this close does not edit it. Gating also surfaced **one finding of
this close's own** — an intermittent Python row, §7.2 — which is registered
rather than mentioned.

Nothing is promoted. No unit status moved at the wave-34 arbiter, raw units stay
unpromoted, and full language conformance stays **false**.

---

## 1. What the wave contains

In merge order, on the wave-33 close (`d3225f91`) as base:

| # | Card | Merge | What it is |
|---|---|---|---|
| 1 | `STABILITY-REEVAL` #1 | `b0bc312d`, follow-up `c4b24a9b`, integrator qualification `d60d6a12` | The first firing of `CLAUDE.md`'s every-2-waves cadence. 27 residuals registered (`named_followups` 47 → 74), two committed-matrix tests, the measured full-suite figure 4,543 / 1 / 8 |
| 2 | `C-GATEWAY-RETURN-CALL-W4` | `2ef29867` | **The wave's arbiter.** The modeled call in `return` position. Its report **refutes the brief it was cut on** and says so first: definedness is not derivability, and W4 supplied 1 of its 14 leaves |
| 3 | `DOCS-COMPACT-6` | `52920868` | The sixth three-file compaction. 47 constraints in, 47 out; the files **grew** (`CLAUDE.md` +11.87%); `named_followups` 74 → 80; seven text residuals discharged |

**The wave-35 record was opened before this close.** Commit `a300a39d` ("Open
wave 35") created `docs/development/increments/wave-35.md` and moved the index
pointer to it, and it is this close's base. That ordering is recorded plainly
rather than smoothed over: the close paragraph for wave 34 still belongs in
**`wave-34.md`**, which is where it was written, because the sync rule's unit is
the increment and not the file that happens to be current. No wave-35 file was
touched by this close.

---

## 2. The sweep

```sh
python3 scripts/reconciliation_sweep.py . --since d3225f91 \
    --check docs/reports/wave34-close/reconciliation.json
python3 scripts/reconciliation_sweep.py . --cards \
    stability-reeval-1 c-gateway-return-call-w4 docs-compact-6 --residuals
```

`--check` is green: **every swept item carries a recorded verdict.** That says
nothing about whether any item was applied, which is the point of the rule and
the reason this section exists.

### Verdict distribution

| Verdict | Count | Items |
|---|---:|---|
| `applied` | 11 | arbiter 2, 3, 5, 6, 7, 8; docs-compact-6 5; stability-reeval 2, 3, 4, 7 — of these, **arbiter 6 and 7 were landed by this close**, the other nine by the cards themselves |
| `superseded` | 3 | arbiter 4; stability-reeval 8, 10 |
| `deferred` | 5 | docs-compact-6 1, 2, 4; stability-reeval 5, 6 |
| `noted` | 6 | arbiter 1, 9; docs-compact-6 3; stability-reeval 1, 9, 11 |
| `declined` | 0 | — nothing was declined this wave |

Every one of the eleven `applied` entries names **where** it landed — file, line and
text — because a reconciliation record that says `applied` without saying where
is worth no more than silence.

Two cards the `--since` scan raised carry **no** integrator section, and each is
recorded in `no_section` with the reason, checked **both ways** so a card that
does have a section cannot be waved away:

* **`lean-proof-integrity`** — not a wave-34 card at all. A pre-existing report
  whose directory entered the window because `DOCS-COMPACT-6` appended 16 lines
  to it. It contains no occurrence of "integrator", "reconciliation" or "carry
  list"; its headings are Findings and repairs, Regression boundary, Lean 4.34
  warnings, Validation.
* **`wave33-close`** — the previous close record, caught because the window
  begins at the commit that created it. A close record addresses no integrator
  but itself; its forward-carrying sections are carried in
  `deferred_decisions` rather than lost.

---

## 3. What the sweep found unapplied

**Arbiter item 5 — the four `RETURN-CALL-*` residuals.** The card's §11 asked
for them by name. `DOCS-COMPACT-6` merged afterwards and took `named_followups`
74 → 80, but its six additions are `WRITE-LEND-WINDOWS-RUN`,
`WRITE-LEND-TASKS-WINDOWS-RUN`, `SLICE-PERMISSIONS-COMPOSED-LEND`,
`QUIRIN-ARM64-GATE`, `QUIRIN-ARM64-RUN` and
`CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK` — none of them these. The sweep's own
`--residuals` pass names all four independently, which is the same defect's other
half working as designed.

Registered by this close, each `why` written from the card's own §10 discharge
condition rather than summarised away:

| Residual | What it bounds |
|---|---|
| `RETURN-CALL-W4-REACH-BLOCKED` | W4 gave **1** of the card's 14 leaves; 366 of 643 leaves have a callee whose own body is `Unknown`, so no summary validates |
| `RETURN-CALL-SYNTHETIC-PLACE-PRESENTED` | 10 published leaves name `__qcore_returned_value`, a place in no source — presentation, not soundness |
| `RETURN-CALL-NULLABLE-WRAPPER` | 5 allocator wrappers accused by a non-null contract for legitimately returning NULL |
| `RETURN-CALL-INCOMPLETE-LINE-455` | 455 of 1,470 return leaves sit on a line where the `return` is not complete, classed shape-unknown rather than guessed |

Mechanics, because "applied" without a location is worth no more than silence:
appended by `scripts/add_followups.py` in this directory; `named_followups`
**80 → 84**; `dimensions.named_followups` *derived*, never hand-set
(constraint 25). A fifth entry — this close's own finding, §7.2 — takes the
section to **85**, and `AGENTS.md:150`'s stated count is updated **80 → 85** to
match what this close leaves.

Every append is a **generator fixed point**, verified rather than asserted:

```sh
python3 -B docs/reports/remaining-d02/build_matrix.py . \
    --gate docs/reports/wave34-close/gate.json
```

reproduces the appended file **byte-identically**, and a second consecutive run
is byte-identical again. Diffed against the pre-append matrix, the only sections
that moved are `named_followups`, `dimensions.named_followups` and — from the
regeneration carrying this close's own gate (**arbiter item 7**) — `validation`.

> The first regeneration ran against `wave33-close/gate.json` deliberately,
> while the gate was still unmeasurable: substituting a wave-34 gate whose
> numbers no command of this close had produced would have put a fabricated
> measurement in the matrix (invariant 2). Once the gate was measured, the
> matrix was regenerated against `wave34-close/gate.json`, which is what the
> arbiter asked for.

> `scripts/build_matrix.py` **does not exist**. The generator is
> `docs/reports/remaining-d02/build_matrix.py`, which is what
> `STABILITY-REEVAL` #1 used and what `AGENTS.md:151` names. Recorded so the
> next close does not go looking for it.

### The one amendment that did not land in its proposed words

**Arbiter item 3** is `superseded`, not `applied`, and the distinction is
deliberate. The proposed sentence read "**4 (0.6%)** … against 236 (70.5%) that
are merely defined here" over a **643**-leaf denominator. But 236/643 is 36.7%:
the pair 236/335 (70.5%) is the *brief's* figure over wave 33's **pre-landing**
population, and the card's own §3.3 says so explicitly while publishing **379 of
643 (58.9%)** for its own. The tree carries the card's §3.3 measurement. It is
superseded by the card's own body rather than by a later card, and **nothing was
softened**: §3.3's conclusion is that "the gap between the two columns is two
orders of magnitude either way".

---

## 4. Residuals reaching no tracker, found by hand

`--residuals` has two known blind spots, both named by `STABILITY-REEVAL` #1:
it scans only `docs/reports/` (`SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`) and counts
an `AGENTS.md` or `CLAUDE.md` mention as *tracked*
(`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`). Card
`SWEEP-BLIND-SPOTS` is repairing the tool in parallel this wave, so this close
did the check **by hand**: every residual-shaped name in the three reports, in
`wave-34.md`, and in **every line `AGENTS.md` and `CLAUDE.md` gained since
`d3225f91`**, checked against `named_followups` one at a time.

55 distinct names in those sources are absent from `named_followups`. The great
majority are **card** names (`C-GATEWAY-*`, `CT-*`, `DOCS-COMPACT-*`,
`REGION-*`, `W4-*`, `W32-*`) and correctly absent. The tool's own six candidates
classify as: four **queued-card** names — `FB-BACKLOG-ACCOUNTING`,
`FB-ENGINE-PROGRESS`, `FB-RETRY-LIFETIME`, `FB-SYSTEM-CENSUS`, which are column
1 of `STABILITY-REEVAL` #1 §5's table, headed *Queued card* — plus
`DOCS-COMPACT-1`, a past card name. That leaves two classes of genuine untracked
work:

1. **`MUTEX-SHARE-CROSS-MULTI`** — named in `docs/reports/mutex-share-cross/`
   (a **wave-32** card) and cited again by `STABILITY-REEVAL` #1 §5. Reaches no
   tracker. Wave-32 in origin, so it is reported here and left to the
   retrospective wave-32 sweep rather than registered by a wave-34 close — and
   it is direct evidence that that sweep is worth running.
2. **The platform residual family is half-tracked.** `DOCS-COMPACT-6`
   registered three (`QUIRIN-ARM64-RUN`, `WRITE-LEND-WINDOWS-RUN`,
   `WRITE-LEND-TASKS-WINDOWS-RUN`) and left the rest. Six that `CLAUDE.md` still
   describes as **open**, each with an exact discharge condition, are absent
   from `named_followups`: `HTTP-V1-WINDOWS-RUN`, `STRING-SLICE-IO-WINDOWS-RUN`,
   `SOCKET-INET-WINDOWS-RUN`, `SLICE-PERMISSIONS-WINDOWS-RUN`,
   `SOCKET-V1-APPLE-RUN`, `SOCKET-INET-APPLE-RUN`. (`EVENT-V1-APPLE-RUN` and
   `SOCKET-V1-WINDOWS-RUN` are correctly absent — `CLAUDE.md` records both
   **discharged**.) This is the clearest live instance of
   `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`: all six are named
   **only** in a constraint file, which is exactly what the tool counts as
   tracked.

Neither is registered here. Completing or reversing `DOCS-COMPACT-6`'s partial
registration is a disposition about what belongs in the matrix, so it is carried
as a decision rather than taken.

---

## 5. The corpus row's failure mode

`DOCS-COMPACT-6` asked for one sentence at a future close; this is it, and it is
written here rather than added to the ledger row, because adding prose to that
row is the accretion the compaction cadence exists to bound.

**The corpus row narrates each arbiter's *delta* without restating its opening
number, so its headline figures can go stale while every clause of its own
narration stays true.** Two of the four stale numbers `DOCS-COMPACT-6` found were
of that kind — `179 Safe / 4,570 Unknown` and `unsupported` 50,117 — each
contradicted by the narration in its own table cell, which already ended at the
right value. A row that carries both a running total and a chain of deltas has
two places to be wrong and only one of them is read.

---

## 6. Headline numbers, each verified against its source

Not trusted from `AGENTS.md`, and not trusted from any card's report either —
re-derived from the artifact or the tree:

| Ledger claim | Verified against | Result |
|---|---|---|
| 185 units: 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | `c-gateway-return-call-w4/artifacts/diagnostics.json.gz` | **matches** |
| 4,834 functions: 186 / 85 / 4,563 | same artifact, recounted | **matches** |
| leak **272**, `unknown_alias` **4,214**, `unsupported` **50,106**, `double_release` **1** | same artifact's `diagnostic_counts` | **matches exactly** |
| contract corpus **76** modules | `find contract/qcore -name '*.qcore' \| wc -l` | **76** |
| Lean audits **129** | `find proof -name 'Audit*.lean' \| wc -l` | **129** |
| CT **45/50** mapped, 5 unmapped | matrix `ct_conformance.totals` | **matches** |
| sanitized-native **9 of 45**, proof **3 of 45** | matrix `stage_histogram`: `native_sanitized.executed`, `proof.executed` | **9** and **3** |
| twelve example families | `ls -d examples/*/` | **12** |
| `named_followups` **85** | matrix, and `AGENTS.md:150` | **85 / 85**, 85 unique ids, no duplicates |
| schemas **14.0.0**; compiler / host / verification **0.4.0** | each `Cargo.toml` | **matches** |
| ten workspace members | root `Cargo.toml` | **10** (nine crates + `compiler`) |

`dimensions.named_followups` is **derived** and equals the carried section's
length, so constraint 25's clobber question is answered by construction here.

---

## 7. Gate

Measured at `a300a39d` — this close's own base — **with** its four follow-up
registrations and the `AGENTS.md` count edit in the tree, so the gate covers
what this close leaves rather than what it found.
`git submodule update --init --recursive` was run first
(`WORKTREE-SUBMODULE-UNGATED`).

| Command | Result | Exit |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **4,559 passed / 0 failed / 8 ignored**, 217 test executables / 228 result summaries | 0 |
| `cargo fmt --all -- --check` | clean | 0 |
| `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | — | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | — | 0 |
| `python3 scripts/check_lean_audits.py` | 129 audits | 0 |
| `python3 -m unittest discover -s scripts/tests` | **Ran 229 tests, OK** | 0 |

**This reproduces the `DOCS-COMPACT-6` tip figures exactly and independently**,
so `AGENTS.md`'s Merged gate row is **confirmed rather than carried**, and this
close edits it not at all.

### 7.1 The contested host row was green here, and this run says which it saw

`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
**passed** in this full `--no-fail-fast` run — the second full-suite observation
agreeing with `DOCS-COMPACT-6`'s, against `STABILITY-REEVAL` #1's deterministic
**3/3 red in isolation** on the wave-33 tip. That is the **inverse** of
constraint 16's five load-sensitive rows, which fail under load and pass in
isolation, and it is exactly why `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` is
recorded *beside* those five rather than among them.

This close **did not re-measure the row in isolation**, so it adds no evidence
about the isolated direction and settles nothing. Its disposition belongs to
card `VERIFIER-HOST-CONTROL-BUDGET` this wave. Stating which outcome was seen is
the requirement; treating either as settling the question is what the residual
forbids.

### 7.2 A Python row failed once and could not be reproduced — this close's own finding

Gating this change surfaced something no card reported. One run of
`python3 -m unittest discover -s scripts/tests` returned
**`Ran 229 tests … FAILED (failures=1)`**. **Six** subsequent runs were green —
four in isolation, two deliberately concurrent with a second copy of the same
suite — so the row is **not reliably reproducible in either direction**.

**Which row failed is unknown.** That run's per-test output was not captured,
and no reproduction attempt produced a `FAIL` line to name. The failing run was
the one executed concurrently with another Python process, and the suite does
contain at least one timing-based fixture — it prints
`Still running stalled fixture: 0.2s / 0.5s` — so a wall-clock row is the
leading hypothesis and is **explicitly not asserted**.

The finding is not that a test is broken: every other signal here is green,
including the full workspace suite and that same Python suite six times over.
The finding is that **`AGENTS.md` publishes "229 Python" as though it were a
property**, while constraint 16's five load-sensitive rows are **all Rust** and
nothing records a Python one. That is the same shape as §7.1's host row — an
observation on one machine presented as a property.

Registered as **`PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW`** (`named_followups`
84 → 85) rather than left in this README, because the standing lesson of
`W32-RECONCILE-SWEEP` is that a finding named only in a report reaches no
tracker. Its discharge condition is to re-run under controlled concurrent load
with per-test output captured **until the row is named** — or to show the
failure was an artifact of this close's environment specifically, which is
plausible: the run happened minutes after a full-disk `ENOSPC` condition cleared
and while a large build's artifacts were still settling. **Not** a discharge:
assuming it was a flake because it did not recur, which is the inference this
project's own `STABILITY-REEVAL` cadence exists to refuse.

### 7.3 The first attempt was killed, and is reported rather than hidden

An initial gate run **died at 78 of 228 result summaries with exit 143**
(SIGTERM) when this session's own ten-minute command timeout fired mid-suite.
Zero failures had occurred in what ran, and the contested row above had not yet
been reached. The suite was re-run to completion detached from that timeout.

**A partial suite is not a measurement**, so no count from that run is published
here — the figures above are entirely from the completed run. Earlier still, and
before any of this, every `cargo` invocation failed with `ENOSPC` because the
host root filesystem was 100% full (0 bytes of 621G); the gate was measured only
after that cleared. No figure in this close is carried from another run and
presented as this one's.

---

## 8. Decisions carried forward, not taken

Four from wave 33, re-registered unchanged because none has been taken, and
three from this wave. Full text in `reconciliation.json`'s
`deferred_decisions`.

| Decision | Status |
|---|---|
| `MATRIX-UPDATED-STALE-AT-WAVE-27` | Live. Verified: the matrix's `updated` field still opens "2026-09-12 (wave 27 complete…" |
| `CT10-REASON-ROW` | Carried, undecided |
| `EXAMPLES-STYLE-SLICE-FANOUT` | Carried, undecided |
| `SYNC-RULE-TOOLING-COMMIT-SCOPE` | Carried. `37bc9004` remains a live instance (`WINDOWS-COHERENCE-GATES-UNCARDED`) |
| **`DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK`** | **New.** `DOCS-COMPACT-6` grew the files above the rule's single-digit expectation and amended the rule to permit growth, arguing every defect it fixed was an *understatement*. Is compaction the right vehicle for correction work, or should corrections land as their own card? The answer sets the rule for `DOCS-COMPACT-7` |
| **`VERIFIER-HOST-ROW-DISPOSITION`** | **New, noted not taken** — owned by card `VERIFIER-HOST-CONTROL-BUDGET` this wave. This close contributes one observation and no decision: the row was **green** in its full suite (§7.1), and it was **not** re-measured in isolation |
| **`PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED`** | **New**, from §4. Either complete the registration or state the rule that keeps platform residuals out of the matrix; the present split is neither |

---

## 9. Scope of this close

* **No card was merged, pushed or re-run by this close.** It reconciles,
  registers and records.
* Changes to the tree: `docs/reviews/r0_3/acceptance_matrix.json` (four
  follow-ups plus this close's own, derived count, and a `validation` section
  carrying this close's gate), `AGENTS.md:150` (stated count 80 → 85),
  `docs/development/increments/wave-34.md` (the close paragraph), and this
  directory.
* **No frozen archive was edited** (constraint 21). The corpus baseline remains
  `docs/reports/c-gateway-return-call-w4/artifacts/`; every predecessor stays
  frozen.
* **No `Q` code allocated, no keyword reserved, no lexer rule moved.** This
  close touches no crate source.
* Provider **122 is carried, not re-measured**: no provider source, manifest
  template or `qcore-schemas` major version moved, so constraint 18's rebuild is
  not engaged. It remains a **Unix** figure (`PROVIDER-CONFORMANCE-UNIX-ONLY`).
* CT coverage is **unchanged** at 45/50 mapped, sanitized-native 9, proof 3 —
  **the wave mapped nothing**, and a grown denominator would be mapping rather
  than conformance in any case (constraint 23).
