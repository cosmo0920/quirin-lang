# Wave 33 close

Integrator reconciliation and close for wave 33. Base `main` at **`aeef9bbc`**.

Two cards merged, **one abandoned**. All three are accounted for below — a
ledger that quietly omits an abandoned card is exactly what `CLAUDE.md`'s
`STABILITY-REEVAL` method 3 hunts for.

| Deliverable | Path |
|---|---|
| Reconciliation record | `docs/reports/wave33-close/reconciliation.json` |
| Gate measurement | `docs/reports/wave33-close/gate.json` |
| This record | `docs/reports/wave33-close/README.md` |

## 1. What the wave contains

1. **`C-GATEWAY-RETURN-POSITION`** — the wave's **arbiter**
   (`docs/reports/c-gateway-return-position/`).
2. **`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`** — promoted to a card by the
   integrator after arriving **out of scope**
   (`docs/reports/verifier-host-closed-input-deadline/`). Its validation section
   and increment paragraph were **rewritten by the integrator** at `aeef9bbc`;
   that correction is part of the record and is read as such here.
3. **`C-GATEWAY-INDIRECT-CENSUS`** — cut as a measurement card, **stalled and
   abandoned**. **No report, no committed deliverable on `main`.**

## 2. The sweep

```sh
python3 scripts/reconciliation_sweep.py . --since 37bc9004 \
    --check docs/reports/wave33-close/reconciliation.json
python3 scripts/reconciliation_sweep.py . \
    --cards c-gateway-return-position verifier-host-closed-input-deadline --residuals
```

**21 integrator-addressed items across 2 cards**, all 21 from the arbiter.
`--check` is green in both the `--since` and the `--cards` form.

**The tool decided nothing.** Green means every item carries a recorded human
verdict, never that any item was applied. Every verdict below was **measured
against the tree** — a grep of the named file, a `git diff --stat` against the
base, an `ls` of the cited path, or the card's own archived census — and never
inferred from the report's own claim. Every `applied` says **where**.

### Verdict distribution

| Verdict | Count |
|---|---|
| `applied` | 6 |
| `noted` | 14 |
| `superseded` | 1 |
| `declined` | 0 |
| `deferred` (within swept items) | 0 |
| **Total** | **21** |

The `noted` count is high because this card's section is largely a
*verify-do-not-re-apply* list: the arbiter landed its own sync-rule edits, and
`noted` is the vocabulary's arm for "a statement of what the card itself already
did", which is why it demands a note saying what was checked.

### The one `superseded`

**Item 8** — *"The workspace gate is not green, and must not be made green
here."* Superseded by the wave's second card, which **repaired** the failure
rather than absorbing or weakening it. The arbiter's instruction was honoured
*in the arbiter* (its own `gate.json` records the failure as a finding), and the
repair landed separately. The arbiter's attribution was itself corrected: the
defect is in the **third, 1,000 ms** arm, not the 100 ms arm that report named.
Re-measured here — the named test passes and the suite carries 0 failed.

### Nothing was `declined`

No item proposed text this close softened or refused. Standing constraint **5b**
landed **in the card's own proposed words** (verified at `AGENTS.md` line 250,
including both measured boundaries and both "do not" sentences).

## 3. What the sweep found unapplied

**Two stale claims, one of them a live defect.**

**(a) A live drift, repaired.** Item 13's already-landed matrix edit is now
stale in one **derived** field: `dimensions.named_followups` stood at **45**
against a `named_followups` of **47**. The integrator's own `aeef9bbc` hand edit
added `CARD-SCOPE-SELF-MERGE` and `WORKTREE-SUBMODULE-UNGATED` without moving
the count beside them — which is precisely the matrix's own registered residual
**`D02-DIMENSIONS-FOLLOWUP-COUNT`** ("*a hand edit to a carried matrix section
can leave the derived count beside it stale*") firing a **second** time.

Repaired **by regeneration, never by hand** (constraint 25): `dimensions` is
`owner: derived` ("counted from the emitted rows"), so

```sh
python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave33-close/gate.json
```

recomputes it while carrying `named_followups` verbatim (`owner:
hand_maintained`). The resulting diff is **4 lines changed, 4 lines only** — no
reserialization, the wave-32 integrator error not repeated:

| Field | Before | After |
|---|---|---|
| `dimensions.named_followups` | 45 | **47** |
| `validation.workspace_tests` | 4,534 | **4,544** |
| `validation.measurement_provenance` | `wave32-close/gate.json` | `wave33-close/gate.json` |
| `validation.note` | wave-32 note | wave-33 note |

`AGENTS.md` line 145 already read `named_followups, 47 entries` and needed no
change. The 227 Python tests were re-run **after** the regeneration and stay
green.

> **Correction to this close's own brief.** The brief named the generator
> `scripts/build_matrix.py`. **No such file exists.** The generator is
> `docs/reports/remaining-d02/build_matrix.py`, which is also what the matrix's
> own `section_ownership.generator` names.

**(b) A stale account, nothing owed.** Item 21 says `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`
is "named in `AGENTS.md`". It is **not** — the `aeef9bbc` rewrite of the merged-gate
row replaced the failure text with the repair text, keeping the report *path*
and dropping the SCREAMING-KEBAB *name*. The **tracking property still holds**
(the residual is in `named_followups`), so nothing is owed; only the card's
account of *where* it is tracked is out of date. Recorded rather than corrected
in a merged report.

## 4. Residuals

```
0 residual-shaped names in no tracker.
```

Wave 33's five known residuals — the three `RETURN-POSITION-*` entries,
`CARD-SCOPE-SELF-MERGE` and `WORKTREE-SUBMODULE-UNGATED` — all reach
`named_followups`, now **47** entries, with `AGENTS.md`'s stated count matching.
Each of the eight names in play was also checked individually against the three
trackers rather than taken from the tool's aggregate.

## 5. The abandoned card

`C-GATEWAY-INDIRECT-CENSUS` is recorded in `reconciliation.json` under
`abandoned_cards`, **not** under `no_section`.

**Why not `no_section`:** the sweep cannot represent it. The tool exits with
`no such card report: docs/reports/c-gateway-indirect-census/README.md` when the
name is passed to `--cards`, so the name cannot appear in the record's `cards`
list — and `check()` rejects a `no_section` entry for a card not in `cards`.
`no_section` would have been the honest sweep answer had the tool been able to
hold it; **that it cannot is itself a small gap in the wave-32 mechanism**, and
it is surfaced here rather than worked around silently.

**Salvaged partial:** of the 18,555 `indirect memory access is outside the
resource projection` leaves, the 284 functions blocked by that reason **alone**
hold only **833** of them.

**Boundary, stated rather than implied:** this figure is **not** reproduced at
this close and **nothing on `main` can reproduce it** — there is no committed
artifact. The WIP tool sits unmerged on branch
`worktree-agent-a5b3a29c1b3667c11` with a classifier **known wrong for
macro-expanded accesses**. It is an **unaudited observation**, must not be cited
as a measurement, and decides and promotes nothing.

## 6. Gate, measured at the close tip

`git submodule update --init --recursive` was run **first** — this is wave 33's
own residual `WORKTREE-SUBMODULE-UNGATED`, which had already cost the wave two
runs. Submodule checked out at `d82464a277d0f42703702c4dfd9af6af38595a83`. **No
submodule-conditioned failure occurred.**

| Check | Result | Exit |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **4,544 passed / 0 failed / 8 ignored**, **227 test binaries** | 0 |
| `cargo fmt --all -- --check` | clean (empty output) | 0 |
| `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | — | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | — | **0** |
| `python3 scripts/check_lean_audits.py` | **128** Lean audits passed | 0 |
| `python3 -m unittest discover -s scripts/tests` | **Ran 227 tests, OK** | 0 |

Logs are worktree-private under `measure/wave33-close/` (`CLAUDE.md` hygiene
rule — never the shared scratchpad, never under `target/`).

**This reproduces the integrator's `aeef9bbc` measurement exactly** — 4,544 / 0 /
8 over 227 binaries, both probes 0, 128 Lean, 227 Python, fmt and clippy clean.
No figure differs.

**No constraint-16 load-sensitive row tripped.** All 227 binaries were green in
a single `--no-fail-fast` invocation, so no isolation re-verification was
required and there is no second fact to record.

**Provider 122 is carried, not re-measured**, and says so in `gate.json`: no
provider source, manifest template or `qcore-schemas` major version moved, so
constraint 18's rebuild-and-verify is not engaged.

## 7. Headline numbers, each verified against its source

Not trusted from the ledger — re-derived:

| Measure | Value | Source measured |
|---|---|---|
| `unsupported` | **50,117** (50,150 → 50,117, −33) | `c-gateway-return-position/logs/census.txt` |
| leak / `unknown_alias` / `double_release` | **272** / **4,208** / **1**, all +0 | same census |
| Unit histogram | 2 / 42 / 137 / 4, **0 moves** | same census |
| Function histogram | 146/85/4,603 → **179 / 85 / 4,570**, 33 promotions, 0 withdrawals | same census |
| 511-function split | 33 leaf-free (6.5%) / **0 exchanged** / 478 unmoved | same census |
| Lean audits | **128** | `ls proof/Audit*.lean \| wc -l`, and the gate run |
| Contract corpus | **76** | generator's `contract modules 76`; `contract/qcore` untouched since base |
| CT mapped | **45 / 50** | `ct_manifest.json` |
| CT sanitized-native | **9 of 45** (CT03, 07, 09, 17, 22, 25, 26, 34, 49) | `ct_manifest.json`, stage status `executed` |
| CT proof | **3 of 45** (CT01, CT03, CT37) | `ct_manifest.json` |
| `named_followups` | **47**, `AGENTS.md` matching | matrix + `AGENTS.md` line 145 |

A note on one apparent discrepancy, checked rather than waved past:
`scripts/ct_stage_summary.py` prints `native_sanitized: 8`. That is the **depth**
histogram — the *deepest* stage each case reaches — under which CT03 is counted
at `proof` instead. The count of cases whose `native_sanitized` stage status is
`executed` is **9**. Not a drift.

**A grown leaf-free count is not conformance.** The card's own census says so in
as many words: *leaf-free is NOT Safe; `Safe` additionally requires every
obligation discharged and full analysed coverage at the certificate gate.* Unit
statuses did not move and **raw units remain unpromoted**. Full language
conformance stays **false**.

## 8. Decisions surfaced, not taken

Recorded in `reconciliation.json` under `deferred_decisions` with grounds. They
are the integrator's and the user's. **No user message reached this close**, so
no user direction on any of them exists to act on.

1. **`MATRIX-UPDATED-STALE-AT-WAVE-27`** — the matrix's `updated` field still
   reads wave 27, six waves behind; the open decision `W32-RECONCILE-SWEEP`
   surfaced, still unresolved. **Verified not moved by accident:** `updated` is
   `owner: integrator_supplied` but `build_matrix.py` carries it
   (`carried("updated")`) and it is **not** among `GATE_KEYS`, so this close's
   regeneration could not and did not touch it — confirmed in the four-line
   diff. *Ground:* a framing field whose content is a judgment about what the
   checkpoint note is for; guessing would put a fabricated summary into a
   carried section.
2. **`CT10-REASON-ROW`** — carried unchanged from `W32-RECONCILE-SWEEP`'s one
   `deferred` verdict. *Ground:* explicitly a mapping lane's work, and
   constraints 23 and 25 make it unsafe for a close — the
   `newly_decidable_not_mapped_reasons` map is **derived** from the judgment's
   own key set, so a close writing a reason row would be mapping by the back
   door. Wave 33 ran no CT lane and mapped nothing.
3. **`EXAMPLES-STYLE-SLICE-FANOUT`** — the standing **user** decision, open
   since `DOCS-COMPACT-3` §9. *Ground:* repairing it would **un-pipe** a line,
   the opposite of the direction that pass was asked to move lines in, and it
   turns on a real open question about the rule (is a *slice* of a value a
   second reader, or the assembly half?). Wave 33 touched no example program.
4. **`SYNC-RULE-TOOLING-COMMIT-SCOPE`** — whether the sync rule binds a
   **tooling** commit publishing under `docs/reports/` without moving a semantic
   surface or a headline number. Live in this wave: this close's own
   deliverables are such a publication. *Ground:* an interpretation of the rule
   this close is judged by; resolving it here would be authoring rule text for
   my own benefit (invariant 1's shape).

## 9. For the next wave, noted and not acted on

* **`STABILITY-REEVAL` #1 is now due.** `CLAUDE.md` sets the first at "after
  wave 33". This wave hands it unusually good raw material: the arbiter itself
  published **two vacuity residuals found by mutation** —
  `RETURN-POSITION-ESCAPE-UNREACHABLE` (an arm **no test distinguishes**,
  standing on its theorem and explicitly *not* on the green suite) and
  `RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS` (proved and unit-tested,
  **reaching no function of the pinned corpus**). Those are method #1's
  quarry — vacuous greenness — already named by the card that created them, and
  method #2's shape besides: a boundary **proved** on one side and **observed**
  on the other. `CARD-SCOPE-SELF-MERGE` is method #3's quarry in the same wave:
  a semantic edit that reached `main` with no card, no report and no review.
* **`DOCS-COMPACT-6`** falls around wave 34 per `CLAUDE.md`'s cadence.
* The `no_section` gap in §5 — the sweep cannot represent an abandoned card
  that produced no report — is a small, concrete improvement to
  `scripts/reconciliation_sweep.py` if the integrator wants it.

## 10. Scope of this close

Ledger and report files only. **No `crates/` or `compiler/` source, no frozen
archive (constraint 21), and no pinned expectation was changed.** Work was done
in an isolated worktree on its own branch; **nothing was merged and nothing was
pushed** — the integrator merges.

Files changed: `AGENTS.md` (merged-gate row provenance),
`docs/development/increments/wave-33.md` (the close paragraph, newest first),
`docs/reviews/r0_3/acceptance_matrix.json` (regenerated, 4 lines), and this
directory.
