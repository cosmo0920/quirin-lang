# Wave 28 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 28 close (2026-09-12): **three cards** merged one at a time under the
failure-gated chain — `EXAMPLES-AGGREGATES`, `REGION-CREDIT-JOINS` and
`CT-WIRE-LANE-2` — reconciled at integration tip `71574edc`. The wave's head is
**CT coverage**: 36/50 → **41/50 mapped** (CT07, CT08, CT09, CT21, CT23), 9
unmapped, sanitized-native 5 of 36 → **7 of 41 by real execution**, proof 0 of
41; a grown denominator is mapping, not conformance (constraint 23). CT22 is
**expressible but deliberately not mapped**: `REGION-CREDIT-JOINS` composes the
quant open onto the epoch task route after every join (three unedited checkers,
a duty-free pack, the region erasing from emission, `AbandonCredit` publishing
the parent-local unused credit), the lane publishes the **narrowed** reason in
its own record, and `CT22-CT-MAPPING` is the named follow-on — a capability is
not a mapping. `examples/records/` lands as the **twelfth** published family
under the suite's directory-scan standard. Zero reserved words, zero new
diagnostic codes (`Q302`/`Q600` condition widenings, §L21); standing constraint
**15m** records the composed region's two premises. **No arbiter ran** — no
gateway, contract, audit or provider file was touched — so the corpus rows are
carried from the wave-27 `gateway-c-location` baseline, every figure unchanged
(leak 266). Lean audits 114 → **115** (`+AuditRegionCreditJoins`). Merged gate
at the close tip: 4,392 workspace tests (8 ignored, `--no-fail-fast`), 122
provider (carried), 193 Python, both probes exit 0, fmt/clippy clean
(`docs/reports/wave28-close/gate.json`); two constraint-16 load rows tripped
under concurrent suites during integration and were re-verified green in
isolation, both facts recorded. The acceptance matrix's derived rows are
regenerated through `build_matrix.py --gate` (hand-kept sections carried, the
derived-fact tripwire green again). Nothing is promoted; full language
conformance stays **false**.

Wave 28, card `CT-WIRE-LANE-2` (2026-09-12): the second CT mapping lane, on the
wave-26 precedent, converting the wave-27 capability cards into manifest rows.
CT coverage moves **36/50 → 41/50 mapped** (unmapped 14 → 9), harness-gated
sanitized-native evidence **5 of 36 → 7 of 41**, proof **0 of 41** — by
**mapping, not conformance** (constraint 23), and with nothing semantic
changed: `git diff` over `crates/` and `compiler/src/` is empty, no diagnostic
code is allocated (the `Q-NAMESPACE-DECISION` ice box needed nothing) and no
word is reserved. Five cases map. **CT07** and **CT09** are the sequential
route's first *accepted* cases, under a new `sequential_sanitized` verdict mode
whose runner builds one binary from the case's own emitted C under
ASan/UBSan (LSan on Linux) and compares both streams byte for byte with the
route's checked observation: CT07 is `CT07-SLICE-PERMISSIONS`'s
disjoint-halves program, mapped with the boundary that card's §8 required
stated — one permission is a **read** loan and the other is the owner's
retained write authority, not two write lends
(`SLICE-PERMISSIONS-WRITE-LEND` stays open) — and CT09 is `AGGREGATES-V1`'s
three-field record program, its access proof checked and its **layout** leg an
executed sanitized byte-comparison *observation*, never a proof
(`AGGREGATE-CT09-LAYOUT` honoured; `proof` stays `unassessed`). **CT08** maps
on the Reject arm of its expected text (the overlap decided at check time,
`["Q078"]` naming the footprint; the analyzer-`unknown` arm is not claimed) and
**CT21** on `QUANT-REGION-SOURCE`'s undischarged cleanup pack (`["Q305"]`, the
discharge control admitted, the unopened control refused `["Q306"]`), retiring
the archived *"no source path emits `RegionOp::QuantOpen`"* reason. **CT23**
lands on a new `composed_epoch` route — grown as a new route rather than by
widening the check-only `epoch` route, so that route's structural-absence
statements stay true — under the `epoch_credit_split` verdict mode
`CT23-CT-MAPPING` proposed: the exact `["Q635"]` cascade of `error(close);`
abandoning a child's granted credit on the creator's own error edge, plus every
admitting control's published split compared wholesale (`restored + spent =
credit`, including `partial.qcore`'s real componentwise division) and the
`["Q636"]` mint refusal; the parked fixtures moved from
`compiler/tests/fixtures/epoch_credit/CT23/` into the CT tree with the card's
own test repointed, and `ct_corpus.rs`'s pinned tables grown deliberately.
**CT22 is deliberately not mapped**: the credit half is re-measured to accept
(`AbandonCredit(3)`, explicit) and the narrowed *"after joins"* reason is
published as this lane's own labelled row per `QUANT-REGION-SOURCE` §5
(`REGION-CREDIT-WITH-JOINS`), without coordinating with the parallel card that
owns the composition. The wave-26 lane's CT07/CT08 rows are retired by
declaration in this lane's record — the earlier record keeps its bytes
(constraint 21) — and two Python tests that pinned tree facts are updated with
no assertion weakened. Checkpoint at the card tip: 4,376 workspace tests
(8 ignored; the count is unchanged from the wave-27 close because the five new
cases run inside `ct_corpus.rs`'s existing test functions — 4,375 passed in
the single `--no-fail-fast` run and the one that did not,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` under concurrent suite
load, is a recorded constraint-16 row verified passing in isolation), 193
Python, **114 Lean audits** (unchanged — the lane adds no Lean), both
reevaluation examples exit 0, fmt/clippy clean, provider 122 carried (no
provider file touched). Nothing is promoted: the corpus baseline
stays `docs/reports/gateway-c-location/artifacts/` with leak **266**, the
contract corpus 73 modules, full language conformance **false**. Evidence:
`docs/reports/ct-wire-lane-2/README.md` and its `ct_stage_evidence.json`.

REGION-CREDIT-JOINS (2026-09-12, wave 28): the composition gap two wave-27
reports named from opposite sides — `CT23-T07-SEAM` and
`REGION-CREDIT-WITH-JOINS` — closed as the minimal honest composition, making
CT22's scenario ("quant opening discards parent-local unused credit after
joins", expected Accept) expressible end to end for the first time. A
`source.epoch.tasks.v1` entry may now write, after every spawn and join of its
task scope, one `error region … catch (E) quant open … { discharge …; }` whose
body is narrowed to the parent's own quantities (`credit N;`, `effort N;`, the
decided `qcore_check(false, E);` edge — a region acquisition is refused by
name, `REGION-JOINS-DUTIES`): the sealed pack carries no duty and exactly the
parent's unused credit, which the discharge abandons as the kernel's own
explicit `AbandonCredit`, decided by the **unedited**
`qcore_core::error_region::analyze_error_regions` at its own `Q303`–`Q309`
codes (constraint 15l reached from a second route). "After joins" is derived
from the checked task IR — a spawn or join span following the region refuses at
`Q600` — and the published `abandoned_credit` is cross-checked against an
independent fold before publication. The region is blanked from the task half
exactly as the plan is, so it erases from emission: emitted C, source map and
reference observation measured byte-identical with and without it, and every
pre-existing composed module publishes `region: None` with unchanged
certificates. Zero new codes (two condition widenings stated in §L21: `Q302`'s
shape conditions from this route, `Q600`'s after-joins reconciliation), zero
new reserved words (census vacuous — every word already reserved, the shape
already parsed), and `epoch_accounting`, `epoch_credit_recovery` and
`region_task_credit` all untouched (constraint 15j honored; the declarative
route keeps its refusal). 13 new Rust tests
(`compiler/tests/region_credit_joins.rs` 9, frontend 4), eight fixtures parked
at `compiler/tests/fixtures/epoch_credit/CT22/` outside the CT corpus, and 14
audited Lean theorems in the new `QCore.RegionCreditJoins`
(`AuditRegionCreditJoins.lean`, audits 114 → **115**, no project axiom). CT
manifest untouched — CT22 mapping is the wire lane's; CT25 stays inexpressible
with its gap narrowed to `REGION-JOINS-DUTIES`. Checkpoint and boundaries:
`docs/reports/region-credit-joins/README.md`.

The published records family (EXAMPLES-AGGREGATES, wave 28): the follow-on
card `AGGREGATES-V1` named is discharged — `examples/records/` publishes the
first aggregate type as a family held to `EXAMPLES-SUITE`'s standard, growing
the published family count **11 → 12** with **no crate-source change, no
reserved word and no allocated code**. Two programs, chosen to be each other's
leak control: `point.qcore` is CT09's "distinct ordinary struct fields"
scenario as a published example — three fields, three different values, three
reads on stdout plus two on the logger's stream, record-only, so its emitted C
carries **no buffer prelude** (constraint 15k's conditional-prelude fact,
visible in a published program for the first time) and its sanitized run is a
*negative* allocation control — while `pixel_writes.qcore` puts records beside
a real resource: `Rgb` fields drive three stored bytes, a `Write` record
drives the fourth store's **index and value** both out of field reads, the
buffer owes its `drop` (removing it still refuses at `Q054` — the records
launder nothing) and the four bytes printed back are `QCor`, the loops
family's own payload. Both programs are spelled to the index's rule — every
once-read scalar is piped into its sink, constructions and stores stay
statements as the assembly half — and the page's transcripts, its emitted
struct block and its pinned `Q051` incomplete-construction refusal (span caret
included) are read out of the file by the suite rather than restated.
`compiler/tests/examples_suite.rs` reaches the new directory structurally by
its scan and gains four records-owned tests (38 → 42 in that file): the native
runs, the emitted-C claims (declared-order initialization, the three binding
prefixes' contrast, no `qcore_bytes_new` in the record-only program), the
documented refusal compared with the compiler's real diagnostic, and the four
side-table refusals (`Q052` drop, `Q052` undeclared field, `Q013` undeclared
type, `Q054` lost release) each earned by exactly the statement the table
writes. The six `AGGREGATE-SELECTOR-WORDS` route-selector words are avoided in
every published type and field name and the page says so. The family README
publishes no `quirin` command line, deliberately: `examples_cli.rs` pins the
published check/emit command-line counts at 6/6, and the loops precedent is
`cargo test` lines only. Evidence:
`docs/reports/examples-aggregates/README.md`.

Worktree submodule diagnostic (2026-09-12, post-wave-27 close, `1e1916b4`
merged at `e552e59c`): the two checkout-reading `profile_loader` tests
(`declared_pins_match_the_checkouts_and_the_retained_rust_constant`,
`f9_and_f10_missing_files_and_digest_drift_reject_the_inventory`) gain a shared
precondition requiring the Libevent submodule's own `.git` entry before any git
query runs. In a fresh worktree the submodule directory exists but is empty and
`git -C <dir> rev-parse HEAD` answers from the *superproject*, so the pin check
reported a `SourceRevisionMismatch` against whatever commit the repository was
on — a misleading invariant-5 alarm for a checkout that was merely
uninitialised. The tests still **fail** in that state (the repository's
convention; nothing skips), now naming the uninitialised submodule and the fix
command; both directions were measured (empty directory swapped in → the new
message; initialised checkout → all 18 tests pass), the pin assertion itself is
unedited, and `f7_and_f8_pin_failures_reject_the_checkout` keeps the
`SourceRevisionMismatch` arm exactly as it was. **DECIDES — discharges**
residual `DOCS-COMPACT-3-WORKTREE-SUBMODULE`
(`docs/reports/docs-compact-3/README.md` §11.1, recorded there frozen; the
discharge is recorded here per constraint 21). The same change also corrects
the two stale `CLAUDE.md` attributions of constraint 4's assume-guarantee rule
in `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` to `AGENTS.md` (residual
`DOCS-COMPACT-2-W4-DESIGN-POINTER`, likewise **discharged**). Gate at the fix's
own tip (pre-close base `27100414`): 4,349 workspace tests, 8 ignored,
`--no-fail-fast`, fmt/clippy clean, both probes exit 0, "Passed 113 Lean
audits" (the close's 114th, `AuditSourceCoordinate`, was not yet on that base).

<!-- migrated-section:end -->
