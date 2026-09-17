# DOCS-COMPACT-6 — the sixth three-file compaction pass

2026-09-17, wave 34. Base: `2ef29867` (`main`'s tip, the
`C-GATEWAY-RETURN-CALL-W4` merge). Documentation only: no `crates/`,
`compiler/`, `proof/`, `contract/` or `scripts/` source changed, no frozen
archive edited, no count moved by this card.

This pass differs from its five predecessors in one way worth stating up front.
`DOCS-COMPACT-1` through `-5` found the three files **accreting narrative**, and
compacted. `STABILITY-REEVAL` #1 (wave 34) found the ledger's **numbers in order
and its prose unstable** — and every prose defect it named is an
**understatement**: a conditional theorem presented as unconditional, a premise
list missing its two largest members, a mutation count inflated from three to
five, a discharged residual still described as unproved. Correcting an
understatement costs words. **This pass therefore grew `AGENTS.md` by 4.3% and
`CLAUDE.md` by 11.9%**, and that is the honest outcome rather than a failure to
compact; the cadence rule in `CLAUDE.md` now says so in those words. Three
regions were genuinely merged and the growth is net of them (§3).

---

## 1. Headline numbers, each verified against its source

The rule is to re-read every number **from its source and never from the file
being compacted**. Each row below names the command or artifact that produced
it.

| Number | Ledger said | Source says | Verified how |
|---|---|---|---|
| Units (185) | 2 Safe / 42 prov. Unsafe / 137 Unknown / 4 data-only | **same** | `diagnostics.json.gz` of `c-gateway-return-call-w4/artifacts/`, `Counter(u['status'])` over `units` |
| Projected functions | headline **179 / 85 / 4,570** | **186 Safe / 85 / 4,563** (total 4,834) | same artifact, `Counter(f['status'])` over every unit's `functions` |
| `leak` | 272 | **272** | same artifact, `diagnostic_counts` |
| `unsupported` | headline **50,117** | **50,106** | same artifact, `diagnostic_counts` |
| `unknown_alias` | 4,214 | **4,214** | same artifact, `diagnostic_counts` |
| `double_release` | 1 | **1** | same artifact, `diagnostic_counts` |
| `flb_input_chunk_create` | function-level Safe | **Safe** (in an `Unsafe` unit) | same artifact, by name |
| CT mapped | 45/50 | **45/50** | `acceptance_matrix.json` → `ct_conformance.totals` |
| CT sanitized-native | 9 of 45 | **9** executed | `ct_conformance.stage_histogram.native_sanitized` |
| CT proof | 3 of 45 | **3** executed | `stage_histogram.proof`, cross-read against the `PROOF_GATED_CASES` table in `compiler/tests/ct_corpus.rs` |
| Contract corpus | 76 modules | **76** | `find contract/qcore -name '*.qcore' \| wc -l` |
| Lean audits | **128** | **129** | three independent ways: `find proof -name 'Audit*.lean' \| wc -l`, the generator's own `lean audits 129`, and the gate run's `Passed 129 Lean audits` |
| Python tests | **227** | **229** | `python3 -m unittest discover -s scripts/tests` → `Ran 229 tests, OK` |
| `named_followups` | **74 entries** | **74** before this card, **80** after | counted from the matrix, never read from a file's stated number |

**Four were stale and are corrected**: the two *headline* corpus figures
(`179/4,570` and `50,117`, each contradicted by the narration in its own table
cell, which already ended at the right value), the Lean audit count and the
Python test count. The `named_followups` count was **correct at 74** and is now
80 because this card registered six entries (§5).

The corpus row's headline/narration contradiction is worth naming as a pattern:
the row is written by appending each arbiter's delta to a chain, and the
**opening figure was not re-stated** when the chain grew. The row now leads with
the current value and carries the chain behind it.

---

## 2. Constraint inventory — 47 in, 47 out, no meaning moved

Counted mechanically at HEAD and in the working tree, over the
`## Standing constraints` section:

```
BEFORE: 1 2 3 4 5 5a 5b 6 7 8 9 10 10a 10b 11 12 12a 13 14 15 15a 15b 15c 15d
        15e 15f 15g 15h 15i 15j 15k 15l 15m 15n 16 17 18 19 20 21 22 23 24 25
        26 27 28                                                    → count 47
AFTER : (identical string)                                          → count 47
```

**None dropped, none renumbered, and the order is identical.** The six hard
invariants are **byte-identical** — `git diff -U0 AGENTS.md` produces no hunk in
that section.

Five constraints changed text. Every change **adds** a qualification or
corrects a number; none narrows or widens a rule. Before and after for each:

### 2.1 Constraint 5b — `RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED`

**Before:** "It is sound to answer `None` because `admitted_chain` refuses a
bound base while every tracked `->` place is rooted at a body-acquired name,
which `may_bind` reports bound by construction — **proved as**
`QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place`…"

**After:** the same sentence up to "by construction", then: "**That second half
is a premise, not a theorem** … the theorem is **conditional on
`TrackedBasesAreBound`**, which it takes as an explicit hypothesis; the Lean
module states in its own words that this is 'a premise, stated and not proved
here' … The theorem therefore discharges the `admitted_chain` half and
**assumes** the `may_bind` half, which is the load-bearing one."

Verified against `proof/QCore/ReturnPositionEscape.lean:84–93`, whose signature
carries `(inv : TrackedBasesAreBound tracked)`, and against the module's own
note at line 34. The rule 5b states — do not answer the second case with
`canonical_place` — is **unchanged**; what changed is the honesty of the
justification. The forward hazard the residual named (`FB-BACKLOG-CUSTODY`'s
member-typing widening is the move that can falsify the premise) is now written
into the constraint.

### 2.2 Constraint 15n — `LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED`

**Before:** "`compiler/tests/trace_alphabet_lease_boundary.rs` drives it and
**every one of its five tests** was shown non-vacuous by mutating the arm."

**After:** "…drives it, and **three of its five tests** were shown non-vacuous
by mutating the arm — not five, as this constraint claimed until wave 34 … The
measurement: silent-drop 3/5, admitting mis-encode 3/5, wrong index 2/5, union
of arm-sensitive tests **3**. The other two exercise **neighbouring routes by
construction** …" — naming both
(`dropping_the_lease_events_would_have_produced_an_admitted_order`, which builds
the dropped encoding itself, and
`the_rendered_lease_lines_are_outside_the_parsers_alphabet_too`, which exercises
`parse_observed_trace`).

The discharge claim is **not** weakened: the arm is genuinely driven, which
`STABILITY-REEVAL` #1 measured independently. Only the evidence count is
corrected, downward.

### 2.3 Constraint 16 — the measured host boundary, recorded *beside* the five

**Before:** the five `P02-FLAKE` load-sensitive rows, unchanged.

**After:** the same five, **byte-identical**, followed by a clearly separated
paragraph headed "**Recorded beside the five, and deliberately not among
them**", covering
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`.

The distinction is the point and is stated in the text: constraint 16's five
rows are **verified green in isolation**; this row is **red in isolation**
(measured 3/3), so it is not a flake and isolation does not clear it. Recording
it *among* the five would have changed constraint 16's meaning and its stated
count. This is discharge option (a) of
`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` — "record the row beside constraint
16's five with its measured host condition" — taken in documentation; options
(b) a throughput-derived budget and (c) an evidenced host investigation remain
open and are engineering work this card may not do. **The residual's entry is
left open** for the integrator to decide whether (a) alone retires it.

### 2.4 Constraint 23 — a stale illustrative checkpoint

**Before:** "the sanitized-native and proof counts are the ledger row's (**7 of
43 and 3 of 43** at the wave-31 close — read them there, not here)".

**After:** "(**9 of 45 and 3 of 45** at the wave-34 close — read them there, not
here; this parenthetical is a checkpoint that has been stale before, which is
why the row and not this line is the authority)".

The rule — counts live in the ledger row — is unchanged and now reinforced.

### 2.5 Constraint 28 — `GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`

**Before:** the premise list ending "…and the C layout/linker equalities are
**premises**, not proofs."

**After:** the same list, then: "The **two largest premises in the gateway**
belong on this list and were missing from it until wave 34 … **faithful C
token/type extraction** … and the **Lean-model-to-Rust correspondence** … No
extraction, refinement or simulation theorem exists in `proof/`, so invariant
6's 'Lean parity' names a correspondence that is **maintained by review, not
proved**; a gateway module's theorems are abstract rule theorems about the
model, and constraint 4's 'its soundness is an induction on execution prefixes'
is a statement about `N(m)` in the Lean model, not about `legacy_neutral.rs` as
compiled."

This adds two premises to a list of premises. It **weakens no rule and moves no
verdict** — it corrects what the file claims to have proved. The repository-map
`proof/` row carries the same disclaimer, because that row ("Lean 4 executable
core and safety theorems") is where a reader forms the stronger impression
first.

### 2.6 Compaction proper — the one place text was *merged*

The ledger's arbiter-chain paragraph stated **one rule four times**, once per
link. Before, in outline: "`C-GATEWAY-WAVE`'s BEFORE reproduced wave 30's …
byte for byte, wave 31's second arbiter's fresh BEFORE reproduced
`c-gateway-wave`'s own `diagnostics.json` and `chunk-contracts.json` byte for
byte, wave 33's BEFORE reproduces `w31-corpus-contracts-2`'s both
verdict-bearing artifacts byte for byte — an identity that card re-asserts
inside its archiving tool … — and wave 34's BEFORE reproduces
`c-gateway-return-position`'s both verdict-bearing artifacts byte for byte under
the same self-refusing tool…"

After: the rule stated **once** ("each arbiter's BEFORE run reproduces its
predecessor's verdict-bearing artifacts … byte for byte"), the tool-assertion
fact stated **once** with its wave, and the links listed newest-first as a
chain. Every named archive is retained in the frozen list; no link is lost.

---

## 3. Byte counts

| File | Before | After | Δ | % |
|---|---|---|---|---|
| `AGENTS.md` | 123,053 | 128,297 | +5,244 | +4.26% |
| `CLAUDE.md` | 13,249 | 14,821 | +1,572 | +11.87% |
| `README.md` | 11,970 | 11,970 | **0** | 0% |
| **total** | **148,272** | **155,088** | **+6,816** | **+4.60%** |

Single-digit percentages, as the rule predicts — but in the **growing**
direction, for the reason this report opens with.

Three regions were genuinely merged, and the saving is **measured, not
estimated**:

| Region | Before | After | Δ |
|---|---|---|---|
| Merged-gate row (§8) | 2,710 | 1,590 | **−1,120** |
| Corpus row (§1) | 2,829 | 2,431 | **−398** |
| Arbiter-chain paragraph (§2.6) | 1,580 | 1,377 | **−203** |
| **total** | | | **−1,721** |

So the corrections added about 6,965 bytes to `AGENTS.md` and the merges gave
back 1,721.

**A correction to this report's own first draft**, kept rather than quietly
fixed, because it is the defect this card exists to catch: that draft said the
merges "removed roughly 4,100 bytes", a figure **estimated by eye and never
measured**. Measuring it gives 1,721 — off by a factor of 2.4 in the
flattering direction. A number that is not read from its source is not a
number, whichever file it appears in.

`README.md` is **unchanged, and that is a measured finding rather than an
omission**. The compaction rule's standing worry is that "README falls behind
the ledger"; this pass checked every figure it states — 2 Safe / 42 provisional
Unsafe / 137 Unknown plus four data-only, 45 of 50 mapped, 9 sanitized native,
3 proof stages naming CT01/CT37/CT03, twelve example families — and **all of
them are current**. Nothing in it needed correcting.

---

## 4. Theorem citations — 25 resolved, 0 dangling

Every theorem-shaped identifier in the three files was resolved against
`proof/`. **25 Lean citations, 25 resolved, 0 dangling.** `CLAUDE.md` cites zero
Lean names; `README.md` cites one (`QCore.ConcurrencyPrelude`); the remaining 23
are in `AGENTS.md`. Rust identifiers of theorem-like shape (test names such as
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
`observations_match_contract`) were classified separately and are not Lean
citations.

The Lean 4.34 sweep (`d117b74f`) removed exactly 13 declarations. **None of the
13 is cited in any of the three files**, verified by direct grep — so the three
files were already clean and the residual's concern does not land there.

It lands elsewhere, and is discharged there: `QCore.BranchJoin.evaluation_is_state_free`
(removed as the literal self-equality `c.evaluate = c.evaluate := rfl`) is still
cited as a **ground** by two frozen reports,
`wave19-or-and-lowering/README.md:209` and `wave19-if-statement/README.md:376`.
Neither is edited (constraint 21); both are now **named in the superseding
report's list** in `docs/reports/lean-proof-integrity/README.md`, with the
supersession stated: their appeal to that *name* is superseded, what they say
about the condition tier is not.

**One correction to the residual's own text.** It states that "neither report
appears in the superseding report's 'Relevant historical evidence' list".
`wave19-if-statement` **was already in that list**, linked as "branching"; only
`wave19-or-and-lowering` was absent. The list named nine reports and one of the
two was among them.

---

## 5. Residual-shaped names absent from `named_followups`

Discharging the *enumerate* half of
`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`. 113 distinct
screaming-kebab names appear in the three files; 47 are card names (a directory
under `docs/reports/`), 66 are residual-shaped. Of those 66, **15 have no
mention anywhere in `named_followups`**.

`README.md` contributes **zero** such names.

**Registered as open work (5 new entries):**

| Name | Where it was named only | Why it is open |
|---|---|---|
| `WRITE-LEND-WINDOWS-RUN` | `CLAUDE.md:175` | native test ran the Linux arm only; no MSVC/Apple run claimed |
| `WRITE-LEND-TASKS-WINDOWS-RUN` | `CLAUDE.md:179` | same, for the task-route range write lease |
| `SLICE-PERMISSIONS-COMPOSED-LEND` | constraint 15h and the `Q068`–`Q079` row | the constraint's own word is "**until** a cross-model overlap rule … exists" |
| `QUIRIN-ARM64-GATE` | `CLAUDE.md:147` | carried three waves as a "candidate residual name"; no gate run on that architecture |
| `QUIRIN-ARM64-RUN` | `CLAUDE.md:148` | same; no emitted-program execution or byte comparison recorded |

The two `WRITE-LEND-*-WINDOWS-RUN` entries are the members of the platform-run
family that `STD-MODULES-O`'s roll-up did **not** mention — their siblings
(`SLICE-PERMISSIONS-WINDOWS-RUN`, `STRING-CONCAT-WINDOWS-RUN`, …) are named in
that roll-up's prose and so were at least findable.

**Recorded as classified, not registered as open (1 roll-up entry,
`CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK`):** nine names denote **rules the
constraints impose** — a refusal or a stated ground that holds today — not work
anyone owes: `REGION-JOINS-EPOCH-AXIS`, `REGION-JOINS-ERASURE-FIT`,
`REGION-JOINS-MULTI-OWNER`, `REGION-JOINS-T07`,
`SLICE-PERMISSIONS-CROSSED-WINDOWS`, `SLICE-PERMISSIONS-LOOP-WINDOW`,
`WRITE-LEND-BOUND-FROM-BYTE`, `WRITE-LEND-COMPUTED-INDEX`,
`WRITE-LEND-NESTED-ACQUIRE`. The entry records the classification so a later
sweep finds it rather than re-raising them, and states that it claims nothing
about whether any should later become work.

**Recorded as already discharged, deliberately not registered:**
`DOCS-COMPACT-5-RECONCILIATION-LIST` — `CLAUDE.md`'s own text records
`W32-RECONCILE-SWEEP` discharging it.

`named_followups` therefore moves **74 → 80**.

### Matrix discipline

`named_followups` is *carried*; `dimensions` is *derived*; two tests added by
`STABILITY-REEVAL` #1 check the **committed** file. The procedure followed:

1. a dry-run regeneration first (`--out` to a scratch path), diffed against the
   committed matrix, to learn what a bare regeneration moves. It is a **fixed
   point on every section except one**: `validation.measurement_provenance`
   would degrade from `integrator_supplied (docs/reports/wave33-close/gate.json)`
   to `carried_from_previous_matrix`;
2. the six entries appended to `named_followups`;
3. regeneration through `docs/reports/remaining-d02/build_matrix.py .`
   **with `--gate docs/reports/wave33-close/gate.json`**, whose keys match
   `GATE_KEYS` exactly, so the validation block and its provenance label are
   reproduced rather than downgraded. This card does **not** supply its own gate
   to the matrix: that is the wave close's to supply, not a documentation
   card's.

Result: `len(named_followups) == dimensions.named_followups == 80`, provenance
unchanged, byte format the generator's own (`indent=1`, `ensure_ascii=False`,
trailing newline), diff **43 insertions and 1 change** — the six entries and the
derived count, nothing else.

---

## 6. Residuals discharged, and in what words

| Residual | Where | In what words |
|---|---|---|
| `LEDGER-CT-ROW-LEASE-ARM-STALE` | `AGENTS.md` CT ledger row | the "observed and not proved … no test drives a lease-carrying reference run into it" clause is replaced by "**was at the time** observed and not proved … wave 32's `TRACE-ALPHABET-LEASE-BOUNDARY` **discharged** that residual and constraint 15n now carries the measured statement — the arm is **reachable and driven**", with the forward question kept open as `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` |
| `LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED` | constraint 15n | "**three of its five tests**", the three per-mutation counts published, the other two named as exercising neighbouring routes by construction (§2.2) |
| `LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY` | see §7 | both increment-history sites corrected in place; the four frozen sites covered by a provenance record published **beside** them |
| `GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION` | constraint 28 + repository-map `proof/` row | the two premises added by name; "maintained by review, not proved" (§2.5) |
| `RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED` | constraint 5b | "**conditional on `TrackedBasesAreBound`**", in the Lean module's own words (§2.1) |
| `DANGLING-THEOREM-CITATION-AFTER-LEAN-SWEEP` | `lean-proof-integrity/README.md` | both frozen reports named in the supersession list; three-file sweep reported 25/25 (§4) |
| `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK` | matrix | enumerate half discharged: 15 found, 5 registered, 9 classified, 1 already discharged (§5) |
| `WAVE-33-RECORD-FUSED-PARAGRAPH` | `wave-33.md:45/46` | blank line inserted; no words changed (§7) |
| The stale merged-gate row | `AGENTS.md` gate row | §8 |

**Not discharged, deliberately.** The *tooling* half of
`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK` — narrowing
`scripts/reconciliation_sweep.py:248`'s `TRACKERS` so an `AGENTS.md` mention
alone does not count as tracking — is a **code** change this card may not make.
It is left with its ground stated here. Likewise
`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` (§2.3): the documentation half of
option (a) is done, options (b) and (c) are engineering.

---

## 7. The wave-33 record repairs

**The fused paragraph.** `docs/development/increments/wave-33.md` had no blank
line between line 45 (the Lean proof-integrity paragraph's last line, ending
`[validation record](…validation.json).`) and line 46 (`Wave 33 close
(2026-09-15): …`), so two increment entries rendered as one paragraph, against
the sync rule's "one paragraph per increment". The `d3225f91` union merge
dropped it at the parent-1/parent-2 seam only. **Repaired by inserting the blank
line. No word changed** — this is a clean re-separation, which is the condition
under which the repair was in scope at all.

**The binary-count label.** `lean-proof-integrity/validation.json` settles it:
`test_binaries: 216`, `doc_test_result_summaries: 11`, `result_summaries: 227`,
and 216 + 11 = 227 exactly. Both figures are correct measurements of **different
things**, so "227 binaries" is an imprecise *label*, not a wrong number.

Six line-sites carried the imprecise label, in three files — the residual says
"five places", which **undercounts by one**; the enumeration it gives lists six
lines:

| Site | Disposition |
|---|---|
| `wave-33.md:98` | **corrected in place** → "216 test executables / 227 result summaries" |
| `wave-33.md:110` | **corrected in place** → same |
| `wave33-close/gate.json` (note) | **not edited** — frozen close artifact (constraint 21) |
| `wave33-close/README.md:169` | **not edited** — close report |
| `wave33-close/README.md:181` | **not edited** |
| `wave33-close/README.md:184` | **not edited** |

The four uncorrected sites take the residual's **second** branch, "state which
figure each one counts": `docs/reports/wave33-close/binary-count-provenance.md`
is published **beside** them, tabulating each site with its reading, exactly the
dual-publication pattern constraint 21 prescribes. `wave-33.md` is increment
history rather than a close artifact, and its own sibling residual
(`WAVE-33-RECORD-FUSED-PARAGRAPH`) asks in as many words for the 216/227 pair to
be reconciled there, so those two are corrected directly. That file previously
held **both** labels — line 37 corrected, lines 98/110 not — because the union
merge took each from a different parent.

No count moves: the wave-33 close's 4,544 / 0 / 8 is untouched.

---

## 8. Gate

`git submodule update --init --recursive` run **first** (`WORKTREE-SUBMODULE-UNGATED`),
exit 0. Logs are worktree-private under `measure/docs-compact-6/` (`CLAUDE.md`
hygiene rule — never the shared scratchpad, never under `target/`).

| Check | Result | Exit |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **4,559 passed / 0 failed / 8 ignored**, 217 test executables / 228 result summaries | **0** |
| `cargo fmt --all -- --check` | clean (empty output) | **0** |
| `cargo clippy --all-targets -- -D warnings` | clean | **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | — | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | — | **0** |
| `python3 scripts/check_lean_audits.py` | **Passed 129 Lean audits** | **0** |
| `python3 -m unittest discover -s scripts/tests` | **Ran 229 tests, OK** | **0** |

The workspace figure is derived by summing every `test result:` line, not read
off a summary: 228 summaries, 217 `Running` lines plus 11 doc-test summaries,
the same 216/9/11 shape `lean-proof-integrity/validation.json` records for
wave 33 with one executable added. Reported as **"217 test executables / 228
result summaries"**, the phrasing §7 exists to keep straight.

### 8.1 The contested host row was GREEN here, and that is a finding

**`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
passed in this run** (`measure/docs-compact-6/workspace.log:5796`, `... ok`), so
this gate is `0 failed` and exits 0.

This **contradicts the expectation this card was briefed with** — that the row
would be red here and the honest figure would be `4,558 passed / 1 failed` — and
it is reported rather than smoothed over. The test totals reconcile exactly:
4,558 + 1 = 4,559, so the suite is the same suite and the single difference is
**this row's outcome**.

It also cuts against the residual's own shape. `STABILITY-REEVAL` #1 measured
the row **red 3/3 in isolation**; this card measured it **green under one full
`--no-fail-fast` workspace run**. A row that fails alone and passes under
parallel load is the **inverse** of constraint 16's five, every one of which
passes alone and fails under load. **Nothing here weakens the residual** — it
sharpens it. `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` stays open, and the
note now recorded beside constraint 16 carries **both** measurements and draws
the consequence in terms neither run alone supports: *neither a `0 failed` nor a
`1 failed` on this row settles anything*, so a gate report must state which it
saw. That is why this section states which this one saw.

No re-run was performed to obtain a pass, and none was needed: the first run
was green. Equally, no conclusion is drawn that the repair "works" — one green
observation is not a property, which is the whole content of the residual.

### 8.2 Scope of this gate

Provider packages were **not** rebuilt: no provider source, manifest template or
`qcore-schemas` major version moved, so constraint 18 is not engaged and the 122
figure is carried. Per `PROVIDER-CONFORMANCE-UNIX-ONLY` it is a **Unix** figure,
and the ledger row now says so. No corpus artifact was regenerated and no
arbiter was run: this card moves no verdict, no leaf and no count, so the
baseline stays `c-gateway-return-call-w4/artifacts/`.

---

## 9. For the integrator

1. **Left deliberately undone, both needing code**: the `TRACKERS` narrowing in
   `scripts/reconciliation_sweep.py:248` (the tooling half of
   `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`), and options (b)/(c)
   of `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED`.
2. **A judgment to confirm**: constraint 16's new paragraph is discharge option
   (a) of that residual, taken in documentation and **recorded beside** the five
   rather than among them. Whether (a) alone retires the residual is the
   integrator's call; its entry is left open.
3. **A correction to `STABILITY-REEVAL` #1's own text**, both minor and both
   in the direction of *less* defect: it says neither dangling-citation report
   appears in the superseding list, but `wave19-if-statement` **was** there
   (§4); and it says "227 binaries" stands in **five** places, where the
   enumeration it gives lists **six** line-sites (§7).
4. **Nine names classified as rules rather than work** (§5) — if any should be
   open work instead, that is a disposition only a person can take.
5. **The corpus row's failure mode**, worth one sentence at a future close: its
   headline figures went stale while its own narration stayed correct, because
   each arbiter appends a delta without re-stating the opening number. Two of
   the four stale numbers this pass found were of that kind.
