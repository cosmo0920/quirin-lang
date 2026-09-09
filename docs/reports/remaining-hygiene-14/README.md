# HYGIENE-14 — two bounded residuals: `D02-BUCKETS-STALE` and `BORROW-LIVENESS-MIXED`

Wave 14. Base: `main` `009bde0`. Branch `claude/hygiene-14`, isolated worktree.

Two independent halves, deliberately kept apart in the diff and in the commits:

1. **`D02-BUCKETS-STALE`** — the CT triage tally in `acceptance.json` summed to
   **22** against `ct_unmapped: 18`. It is now derived over the manifest's own
   unmapped set, with the sum-equals-unmapped property a checked assertion in
   **both** derivations, and the current numbers corrected with every delta
   named (§1).
2. **`BORROW-LIVENESS-MIXED`** — a release *joined with* a consumption left a
   settled borrowed actual `Safe`. The exact status-set condition is derived
   from the join semantics, one further union becomes refusable, the admitted
   unions are pinned with their reasons, and the classification is proved over
   the whole sixteen-status domain (§2).

**Nothing is promoted.** No raw Cycle 19 unit moves, no verdict changes, no
diagnostic code is added, changed or downgraded, the contract corpus stays 61
modules, CT coverage stays 32/50 and the Lean audit count stays **62** — this
card adds no audit file, so the acceptance matrix's derived `lean_audits` row
does **not** move (§6).

---

## 1. `D02-BUCKETS-STALE` — the tally is derived, and the labels are not

### 1.1 The finding, restated from measurement

`docs/reports/wave13-ct34/README.md` §11 recorded it: the hand-kept
`unmapped_bucket_counts` block in `docs/reviews/r0_3/acceptance.json` summed to
22 while the manifest recorded 18 unmapped cases. Re-measured here at
`009bde0`, before any change:

```
hand-kept sum: 22        ct_unmapped: 18        gap: 4
```

The gap is exactly `blocked-on-model-only-epoch-accounting (no source syntax): 4`
— CT35/CT36/CT38/CT39, mapped by wave 8 (`docs/reports/wave8-epoch-ct/`), whose
row nobody decremented. The **derived** counts in
`docs/reviews/r0_3/acceptance_matrix.json` were already correct: `build_matrix.py`
recomputed them from card T00's triage filtered to the currently-unmapped ids.
So the repair is not an arithmetic correction of a number nobody could compute;
it is the removal of a second, hand-kept copy that had no mechanism keeping it
honest.

### 1.2 Why a lane cannot be blamed, and what actually fixes it

A lane that maps a case edits the row *its own case* left. Wave 13 did exactly
that (`blocked-on-guard-profile-composition-or-unrepresented-form`, 2 → 1) and
deliberately did **not** rewrite four rows belonging to another lane's record.
Under a hand-kept tally that behaviour is correct and the drift is still
inevitable, because no row carries the case ids that justify it.

The structural repair is therefore not "recount": it is to make the *set of
counted cases* a manifest fact.

* `scripts/ct_stage_summary.py::unmapped_bucket_counts` reads the ids the
  manifest currently marks `unmapped` and looks each one up in card T00's
  archived triage (`docs/reports/remaining-t00/coverage.json`, the only per-case
  bucket record in the repository — the manifest itself has **no** blocker field
  for an unmapped case, measured).
* `unmapped_bucket_counts` moves from `hand_maintained_fields` to
  `generated_fields`, so `--write` regenerates it and `--check` reports drift.
* `compiler/tests/ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest`
  recomputes the same tally independently in Rust and refuses any disagreement,
  and the pinned `generated_fields` list — asserted element-for-element in that
  test — now names it, so the field cannot quietly become hand-maintained again.

A case that gets mapped now leaves the tally **in the same commit that maps
it**: it disappears from the manifest's unmapped set, both derivations recompute,
and the committed record is a DRIFT line until it is regenerated.

### 1.3 The checked properties, in both derivations

| Property | Python | Rust |
|---|---|---|
| the counts sum to `ct_unmapped` | `SystemExit` | `assert_eq!` |
| every unmapped case has a triage row (a hole is not a zero) | `SystemExit` | `panic!` naming the case |
| no two triage rows classify one case | `SystemExit` | `assert!` on `insert` |
| the committed record equals the derivation | drift line | `assert_eq!` |
| `generated_fields` is exactly the generator's own | drift line | `assert_eq!` |
| every `newly_decidable_not_mapped` id is still unmapped | drift line | `assert!` |

The duplicate-row refusal was **found while writing its test**, not designed in:
a second row does not disturb the sum, so the sum assertion cannot catch it, and
the natural map-building (a dict comprehension in Python, `collect()` in Rust)
let the *last* row silently decide the label. It is repaired in both derivations
rather than pinned as a hazard.

### 1.4 Every delta, named

Committed (hand-kept, sum 22) → derived (sum 18):

| Bucket | Was | Now | Why |
|---|---|---|---|
| `blocked-on-model-only-epoch-accounting (no source syntax)` | 4 | **absent** | CT35/CT36/CT38/CT39 were mapped by wave 8. **This is the finding.** |
| `blocked-on-unimplemented-syntax` | 3 | **6** | T00's own label for CT07, CT08, CT09, CT10, CT23, CT49. The hand-kept copy had split some of these into later relabellings. |
| `blocked-on-W3/W4` | 3 | **2** | T00 classifies CT21 and CT22 here; CT23 sits under `blocked-on-unimplemented-syntax`. |
| `blocked-on-W3/W4 (conditional half resolved)` | 1 | **absent** | a later hand relabelling with no per-case record; the case it covered is counted under T00's own label. |
| `blocked-on-guard-profile-composition-or-unrepresented-form` | 1 | **absent** | likewise; wave 13 had already decremented it 2 → 1. |
| `blocked-on-scenario-mismatch (no local alias form)` | 1 | **absent** | CT11's later, more accurate reason; CT11 is counted under T00's frozen label (see §1.5). |
| `DECIDABLE-NOW (coarser reason)` | 2 | **1** | only CT47 remains unmapped in that class. |
| `blocked-on-task-lowering (share captures)` | absent | **1** | CT11, under T00's own label. |
| `blocked-on-unimplemented-syntax (no mutex value)` | absent | **1** | CT46, under T00's own label. |
| `ambiguous + unimplemented-syntax`, `ambiguous (partially unblocked)`, `ambiguous`, `blocked-on-gateway-expressiveness (C-twin measured)`, `DECIDABLE-NOW (with a distinguishing fixture)` | 1/1/1/3/1 | unchanged | CT06, CT25, CT44, CT40/CT41/CT42, CT24. |

The four rows that vanish outright (`…epoch-accounting`, `…conditional half
resolved`, `…guard-profile-composition…`, `…scenario-mismatch…`) are the
hand-kept vocabulary: labels a later wave wrote into `acceptance.json` with no
per-case record anywhere, which is why three of them cannot be attributed to a
case here with certainty and the fourth (the epoch row) can. That
unattributability is itself part of the finding, and it is what the derivation
removes: every row below is now backed by named cases.

The complete derived tally, with its cases:

| Bucket | n | Cases |
|---|---|---|
| `blocked-on-unimplemented-syntax` | 6 | CT07, CT08, CT09, CT10, CT23, CT49 |
| `blocked-on-gateway-expressiveness (C-twin measured)` | 3 | CT40, CT41, CT42 |
| `blocked-on-W3/W4` | 2 | CT21, CT22 |
| `ambiguous + unimplemented-syntax` | 1 | CT06 |
| `blocked-on-task-lowering (share captures)` | 1 | CT11 |
| `DECIDABLE-NOW (with a distinguishing fixture)` | 1 | CT24 |
| `ambiguous (partially unblocked)` | 1 | CT25 |
| `ambiguous` | 1 | CT44 |
| `blocked-on-unimplemented-syntax (no mutex value)` | 1 | CT46 |
| `DECIDABLE-NOW (coarser reason)` | 1 | CT47 |
| **sum** | **18** | = `ct_unmapped` |

`docs/reviews/r0_3/acceptance_matrix.json` is **not** edited by this card, and
its own derived counts are byte-for-byte what this change writes into
`acceptance.json` — checked by regenerating the matrix into a scratch file and
comparing (§6).

### 1.5 The boundary this does **not** close — `D02-BUCKETS-LABELS`

**A count is now current by construction; a label is historical by
construction.** The bucket *text* is card T00's analysis frozen at revision
`ea2af0f`, and for two currently-unmapped cases it is measurably out of date:

* **CT11** — T00 says `blocked-on-task-lowering (share captures)`. Card T05
  landed counted shares; the current reason, recorded in `acceptance.json`'s own
  `newly_decidable_not_mapped_reasons`, is a scenario mismatch (this profile has
  no local alias form).
* **CT46** — T00 says `blocked-on-unimplemented-syntax (no mutex value)`. Mutex
  values exist since T03 and CT33-RANKS; the measured current obstacle is
  `CT46-MUTEX-CAPTURE` (card CT46-SENDSYNC, re-verified by the wave-13 CT lane),
  which the acceptance matrix already publishes per case as `blocker_current`
  beside the frozen text.

Deriving a *current* label would need a per-case record later cards actually
write, and the mechanism for that already exists in one direction only:
`cases_examined_and_left_unmapped` in each `ct_stage_evidence.json` carries a
re-verified `why_still_unmapped` prose, but no bucket field. Adding one is a
change to what a CT lane must produce, and belongs to a CT lane rather than to a
hygiene card. Recorded as **`D02-BUCKETS-LABELS`**, not repaired. What this card
guarantees is narrower and stated plainly: the tally accounts for exactly the
cases that are unmapped, and no row can outlive the case it counts.

---

## 2. `BORROW-LIVENESS-MIXED` — the exact status-set condition

### 2.1 What was open

`docs/reports/remaining-borrow-liveness/README.md` §7:

> **`BORROW-LIVENESS-MIXED`** — an identity whose joined status is mixed is not
> decided, and the sharpest case is measured: released on one branch and
> **consumed** on the other, then borrowed, is still `Safe`.

That card's rule refused a settled borrowed actual only when its joined status
was **exactly** `RELEASED`. `AbstractState::join` unions the status bits, so any
mixed status fell through.

### 2.2 The derivation, from the join semantics

The analyzer's four bits and what each says about the pointer value on the path
that contributed it (`crates/qcore-c-gateway/src/legacy_c.rs`, unchanged):

| Bit | Meaning on that path | Does it entitle a read? |
|---|---|---|
| `LIVE` | the body acquired the identity and still holds custody | **yes** — the projection knows the object is there |
| `CONTAINED` | the identity sits inside a container the body still tracks; `cascade_resource_action` moves a contained child to a terminal state only when its **owner** is released or escapes | **yes** — containment is not destruction |
| `ESCAPED` | custody transferred out; §C04.2: `resource_consumed` "is not a proof of destruction" — and, symmetrically, not a proof of survival | **no** — the body knows nothing either way |
| `RELEASED` | a declared destructor discharged it | **no** — and this is a positive invalidation |

The join is a bitwise union over reaching paths, so a **present** bit is a may
fact and an **absent** bit is a must fact. The condition is one must fact
conjoined with one may fact:

```rust
status & (LIVE | CONTAINED) == 0   // no reaching path entitles the read
    && status & RELEASED != 0      // some reaching path destroyed the identity
    && status & !MODELLED == 0     // and no bit outside the analyzer's four
```

Over the modelled bits that set is **exactly** `{RELEASED}` and
`{RELEASED, ESCAPED}` — the original card's whole condition plus the one mixed
join it covers. Read in words: *custody ended on every reaching path, and one of
those endings was a destruction.*

### 2.3 Why the mixed join is admissible here and would not be under `Unsafe`

The published class is unchanged: the existing `UnknownAlias`, i.e. `Unknown`.
The refusal says **the projection will not certify this read**, never that the
read is a defect. That distinction is what makes `RELEASED | ESCAPED` decidable:
on the escaped path the pointee may well still be alive, so the shape is *not* a
demonstrated use-after-free, and an `Unsafe` claim over it would be exactly the
manufactured upstream allegation hard invariant 2 forbids. What the projection
can say is that on **no** path does it hold a pointer it knows to be valid, and
on one path it watched the value being destroyed. Declining to certify that is a
refusal the `Unknown` class is for.

The predecessor card's own reasoning is preserved where it is load-bearing:
`ESCAPED` **alone** stays admitted, because no path invalidated the identity and
nothing is known to have gone wrong (`C02-P-ALIAS-CONSUME`, untouched). It is
the added `RELEASED` bit, not the consumption, that makes the join refusable —
proved as `the_released_conjunct_is_load_bearing`.

### 2.4 The admitted unions, pinned with their reasons

| Status | Decision | Reason |
|---|---|---|
| `RELEASED` | **refused** | the original card's case |
| `RELEASED \| ESCAPED` | **refused** | this card's movement (§2.2) |
| anything containing `LIVE` | admitted | a reaching path leaves the body holding live custody — the must-half both cards keep |
| anything containing `CONTAINED` | admitted | containment is not destruction; the object is alive beneath its owner on that path |
| `ESCAPED` alone | admitted | no path invalidated it; `resource_consumed` is not a proof of destruction (§C04.2) — `C02-P-ALIAS-CONSUME` |
| absent site, or `0` | admitted | the projection tracks no identity here |
| any bit outside the four | admitted | a state this rule was not written for; the mask refuses to decide |

Every row is a test in `crates/qcore-c-gateway/src/borrow_liveness.rs`, and the
whole sixteen-status table is pinned twice: the Rust test enumerates `0..=15`
and asserts the refusable set is `[RELEASED, RELEASED|ESCAPED]`, and
`QCore.BorrowLiveness.the_refusable_statuses_are_exactly_released_and_released_with_escaped`
decides the same domain and reports the same two values **by the analyzer's own
bitmap codes** (`[2, 6]`), with `the_enumerated_domain_is_every_bitmap` checking
that the enumeration really is `0..15`. Neither side can widen the condition
without the other's table moving.

### 2.5 `LEAK-ORDER` is undisturbed, by construction and by measurement

`AbstractState`, `AbstractState::join`, `alias_facts`, `NodeAliasFacts`,
`transfer`, `report_leaks` and `apply_resource_action` are **byte-identical to
the base**. The only change in `legacy_c.rs` is that four `const` declarations
became `pub(crate)` so `borrow_liveness` classifies the analyzer's own flags
instead of restating their numeric values — the values, the join and every use
site are unchanged. `C02-P-ALIAS-LEAK-ORDER` is therefore untouched, and the
corpus measurement in §3 confirms it directly.

### 2.6 Held-out evidence

`crates/qcore-c-gateway/tests/borrow_liveness_boundary.rs` — 17 acceptance tests
over card C02-P's generic `b_*` API (unrelated to ChunkIO, Fluent Bit and O04's
`g_*` family), each asserting the **complete** ordered diagnostic vector so a
rule that swallowed a pre-existing leak while adding its own refusal would fail.

| Shape | Before this card | Now |
|---|---|---|
| release on one branch, **consume** on the other, then borrow | `Safe`, no diagnostics | `Unknown`, `[unknown_alias:one]` |
| the same with a **third, live** branch | `Unsafe`, `[leak]` | `Unsafe`, `[leak]` — unchanged (new control) |
| consumed alone, then borrowed | `Safe`, none | `Safe`, none |
| release on one branch, live on the other | `Unsafe`, `[leak]` | unchanged |
| the may-alias pair on two formals | `Unknown`, `[unknown_alias:a, unknown_alias:b/a]` | unchanged |
| double release | `Unsafe`, `[double_release]` | unchanged |
| every other row of the card's family | — | unchanged |

The three-branch control is new and load-bearing: without it the widened
condition could not be distinguished from "any status carrying `RELEASED`",
which is the *may* fact the existing `Unsafe` kinds fire on and would refuse the
branch-local shape above it.

18 module tests in `borrow_liveness.rs` cover the classification table, the
unmodelled-bit refusal, both entitling bits joined with a release, and every
existing fail-closed direction.

---

## 3. Corpus — full parity against the wave-13 baseline, and a measured census

### 3.1 What was run

The canonical pinned route on a freshly prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`), per
`docs/reports/corpus-repin/provenance.json`:

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 --clang clang --clangxx clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-h14
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-h14/qcore.chunkio \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey compile_commands.json> \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Three runs, all exit 0 with zero gateway failures: **BEFORE** (base `009bde0`),
**AFTER** (this card's tree) and one **instrumented census** run (§3.3).

### 3.2 The result

**AFTER is byte-identical to BEFORE on all five artifacts**, measured directly:

| Artifact | SHA-256 (BEFORE = AFTER) | Bytes |
|---|---|---|
| `resource-diagnostics.json` | `70994e38bac7e45a…` | 11,147,459 |
| `resource-diagnostics.chunk-contracts.json` | `c4f438e4ed3dab3c…` | 4,186,178 |
| `monkey-source.json` | `06bbb40ffef3e58e…` | 1,665,474 |
| `resource-diagnostics.contracts.json` | `7b6f2b761890fe3c…` | 13,759 |
| `resource-diagnostics.source-effects.json` | `ed4d08ed7b296bf8…` | 118,502 |

BEFORE against the current baseline
[`docs/reports/wave13-corpus/artifacts/`](../wave13-corpus/artifacts):

| Artifact | BEFORE vs the wave-13 baseline |
|---|---|
| `diagnostics.json` | **byte-identical** (`70994e38bac7e45a…`) |
| `chunk-contracts.json` | **byte-identical** (`c4f438e4ed3dab3c…`) |
| `contracts.json` | differs — **path-literal provenance only**, verified by diff: the six embedded contract source paths carry this worktree's prefix. Nothing else. |
| `source.json` | differs — the same class: the compile command, directory, file and two `__FILE__` expansions (`char[147]` → `char[189]`) carry this worktree's prefix |
| `source-effects.json` | differs — **package identity only**: `package_digest`, `build_digest` and the derived configuration digests of the provider packages rebuilt here |

Both differing classes are the ones waves 9–13 already record, and both are
**identical between BEFORE and AFTER**, so neither is attributable to this card.
Unit statuses are unchanged: **2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only**. Because AFTER equals BEFORE and BEFORE reproduces the wave-13
baseline on both analysis artifacts, **this card does not move the pin**;
`docs/reports/wave13-corpus/artifacts/` remains the current 185-unit baseline
and the AFTER set is not re-archived — it would be a byte-for-byte copy.
`C02-P-ALIAS-LEAK-ORDER` is undisturbed, which the unchanged `diagnostics.json`
digest confirms directly.

### 3.3 The census — the firing count is zero, and the blocking conjunct is named

A null result is worth only its explanation, so the reach was **measured on this
tree, with the widened rule in place**. A third run of the same pinned route
emitted one scratch stderr line per `Borrowed` position the rule considered,
recording whether the actual was bound and whether its denotation settled. The
instrumentation was removed before this card's commits, no production route
emits anything, and its own run produced `resource-diagnostics.json` **identical
to BEFORE/AFTER** (`70994e38bac7e45a…`), so it is proved inert.

| Figure, over all 185 units | Count |
|---|---|
| `Borrowed` positions the rule considered | **267** |
| of those, **unbound** — every scalar actual, and every name this projection never tracks | **233** |
| of those, **bound** to something | **34** |
| of the 34, **settling** on a single non-null identity | **0** |
| positions at which the status test is evaluated at all | **0** |
| refusals published by the widened condition | **0** |
| refusals published by the old condition | **0** |

**The blocking conjunct is requirement 1 (settling), not the status condition
this card widened**: no borrowed actual anywhere in this corpus reaches a state
where the status is even looked at, so `{RELEASED}` and `{RELEASED, ESCAPED}`
are equally unreachable here. That reproduces card BORROW-LIVENESS's archived
census (`artifacts/census.txt`: 267 / 233 / 34 / 0) exactly, now re-measured
after the widening rather than carried.

So, stated honestly: **this card closes a demonstrated hole in the rule (§2.6)
and costs the corpus nothing measurable.** It is not evidence that the mixed
shape is rare in C; it is evidence about these 185 units under this projection's
current, small committed-binding surface (`C02-P-ALIAS-USES`). Widening what the
projection *binds* is what would move this rule's reach; widening the rule again
would not.

---

## 4. Lean

`proof/QCore/BorrowLiveness.lean`, audited from the existing
`proof/AuditBorrowLiveness.lean`. **No new audit file**, so
`scripts/check_lean_audits.py` stays at **62 audits** and the acceptance
matrix's derived `lean_audits` row does not move.

The model's `liveness` now reads `custodyDefinitelyEnded`, defined over the same
two conjuncts as the Rust decision. Three theorems of card BORROW-LIVENESS were
**retired by name** because the widening made them false or imprecise, rather
than being quietly restated:

| Retired | Replacement | Why |
|---|---|---|
| `a_release_joined_with_an_escape_is_never_refused` | `a_release_joined_with_an_escape_is_refused` | states the opposite of what the rule now decides |
| `a_refusal_implies_every_incoming_path_released` | `a_refusal_implies_no_incoming_path_entitles_a_read` | its escaped-bit conjuncts are no longer implied |
| `a_refusal_names_a_settled_released_identity` | `a_refusal_names_a_settled_identity_whose_custody_ended` | its final conjunct named the old condition |

Added: `a_release_joined_with_an_escape_is_refused`,
`the_refusable_statuses_are_exactly_released_and_released_with_escaped`,
`the_enumerated_domain_is_every_bitmap`,
`a_release_joined_with_a_containment_is_never_refused`,
`an_entitling_incoming_path_prevents_the_refusal`,
`the_released_conjunct_is_load_bearing`,
`the_entitling_conjunct_is_load_bearing`. Every existing theorem that stayed
true kept its name and its statement, including
`an_escaped_identity_is_never_refused`, `a_refusal_satisfies_the_existing_bar`
and `the_existing_violation_bar_is_strictly_weaker` (the must/may asymmetry
against the `Unsafe` bar survives the widening: `custodyDefinitelyEnded` still
implies `released`).

**One conjunct of the Rust decision has no counterpart in the model**, and it is
recorded rather than omitted: the model's `Status` has exactly four bits by
construction, so "a bit outside the analyzer's four is not decided" is
unstatable there. It is tested in Rust
(`an_unmodelled_status_bit_is_not_decided`).

Everything here models the **decision**. That the Rust reader extracts the
bindings, the release events and the contract modes faithfully from C source
remains a premise, exactly as in `QCore.MayAlias` and `QCore.BorrowedActual`.
No new axiom: every theorem depends only on `propext` and `Quot.sound`.

---

## 5. Open boundaries

* **`D02-BUCKETS-LABELS`** (new, §1.5) — the tally's *labels* are T00's frozen
  analysis; two currently-unmapped cases (CT11, CT46) carry a measurably stale
  one, published beside a current reason elsewhere. Deriving a current label
  needs a per-case bucket field in the CT lanes' own evidence records.
* **`BORROW-LIVENESS-PRESTATE`, `BORROW-LIVENESS-PLACES`,
  `BORROW-LIVENESS-OUT`, `BORROW-LIVENESS-ASSUMPTION`** — unchanged from card
  BORROW-LIVENESS §7. This card touched only the status condition.
* **`C02-P-ALIAS-CONSUME`** — unchanged, and explicitly not closed through the
  back door: an escaped identity **alone** is still admitted, in the Rust and in
  the Lean, and the non-vacuity of the `released` conjunct is proved.
* **`C02-P-ALIAS-LEAK-ORDER`** — unchanged and undisturbed (§2.5).
* **`C02-P-ALIAS-USES`** — unchanged, and it is why this rule's corpus reach is
  what §3 measures.
* **Reachability is not established.** No claim is made that any refused body
  contains a defect; the published class is `Unknown` for exactly that reason.
* **The declared premises are inherited unchanged and unproved** — declaration
  completeness and C effective types, as C02, C02-P and C02-P-ALIAS record them.
* **`acceptance_matrix.json`, `ct_manifest.json`, `AGENTS.md`, `CLAUDE.md`,
  `README.md` and `DEVELOPMENT.md` were deliberately not touched**; the
  integrator consolidates those. `acceptance.json` is edited only for half 1's
  corrections and assertions, as the card allows.

---

## 6. Gate

Branch `claude/hygiene-14`, isolated worktree on base `009bde0`, submodules
initialised (`third_party/libevent` at `d82464a2`).

| Gate | Command | Result | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,303 passed / 0 failed / 7 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **94 passed / 0 failed** | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | **62 audits passed**, no new axiom | 0 |
| Python tests | `python3 -m unittest discover -s scripts/tests` | **131 passed** (125 + 6 new) | 0 |
| CT summary | `python3 scripts/ct_stage_summary.py . --check` | no DRIFT; 32 mapped / 18 unmapped | 0 |
| Pinned 185-unit corpus | canonical pinned route, §3 | AFTER byte-identical to BEFORE on all five artifacts | 0 |

No load-flaky target failed in these runs; `vp04`, P02 `RLIMIT_NPROC` and P03's
confined-write conformance all passed in the single full-workspace run recorded
above, so no isolated re-run was needed.

**For the integrator, in one line:** this card adds **no** Lean audit file, so
`scripts/check_lean_audits.py` stays at 62 and the acceptance matrix's derived
`validation.lean_audits` row does **not** move. The matrix does not need
regenerating for this card at all: its `ct_conformance.unmapped_bucket_counts`
was already the derived tally and is byte-for-byte what `acceptance.json` now
carries (verified by regenerating the matrix into a scratch file and comparing
that field — the committed matrix was not touched).

### 6.1 Parallel cards in the same crate — recorded, not resolved

`PROD-CONTRACTS` runs in parallel in `crates/qcore-c-gateway`. Recorded overlap,
**not resolved here** (cross-card reconciliation is the integrator's):

* `legacy_c.rs` — this card's whole diff there is **five lines**: four `const`
  declarations gain `pub(crate)` (values, join and use sites unchanged) and one
  enum-variant name at the single call site in
  `require_live_borrowed_actuals`. No new instruction, state field, CFG shape or
  contract-loading path, so a textual conflict is confined to the constant block
  at the top of the abstract-state section and to that one call.
* `borrow_liveness.rs`, `borrow_liveness_boundary.rs`,
  `QCore/BorrowLiveness.lean` and `AuditBorrowLiveness.lean` are this card's
  alone.
* If `PROD-CONTRACTS` adds or changes a *contract mode* rather than a status
  bit, the two are independent by construction: this rule selects positions by
  `ResourceParameterMode::Borrowed` and reads only the status bitmap.
* Half 1 touches no crate file except `compiler/tests/ct_corpus.rs`, which no
  gateway card should be in.

---

## 7. Files changed

| Path | Change |
|---|---|
| `scripts/ct_stage_summary.py` | `unmapped_bucket_counts`, `unmapped_ids`, `check_newly_decidable`; `unmapped_bucket_counts` joins `GENERATED_FIELDS` |
| `scripts/tests/test_ct_stage_summary.py` | six new tests (sum, coverage, mapping-moves-the-tally, unclassified hole, duplicate row, newly-decidable) |
| `compiler/tests/ct_corpus.rs` | the independent Rust derivation, its two total-or-reject refusals, and the `newly_decidable_not_mapped` check |
| `docs/reports/remaining-d02/build_matrix.py` | calls the shared derivation instead of duplicating it |
| `docs/reviews/r0_3/acceptance.json` | corrected tally, field moved to `generated_fields`, note rewritten |
| `crates/qcore-c-gateway/src/borrow_liveness.rs` | `custody_definitely_ended`, the classification table, the renamed decision, 18 module tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | four `const`s become `pub(crate)`; one enum-variant rename at the call site. No other line |
| `crates/qcore-c-gateway/tests/borrow_liveness_boundary.rs` | the mixed-join movement and the three-branch control |
| `proof/QCore/BorrowLiveness.lean`, `proof/AuditBorrowLiveness.lean` | the widened condition, three retirements, seven new theorems |
| `docs/reports/remaining-hygiene-14/` | this record, `artifacts/census.txt` and `artifacts/run-hashes.json` |
