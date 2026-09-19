# Wave 41 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`COMPOSED-WINDOW-REACH` (2026-09-19).** The wave's soundness-adjacent card,
against `STABILITY-REEVAL-4`'s four findings about the loan-window machinery.
**The headline is negative and is reported as one: no program constraint 15h
says must refuse is admitted.** Twenty-one probe programs were measured before
anything was repaired — every position a view can be read from, a bare render
and an inline-slice render inside a composed `for` body, a nested body, a plain
and an owned-field record beside a lend and inside a body, a scalar rebound
from a byte at a lend bound — and each refuses at the code and by the mechanism
constraint 15h names, or is admitted where the constraint says it should be
(`docs/reports/composed-window-reach/probes/verdicts.log`, shipped with the
harness). What is true is narrower. `COMPOSED-VIEW-WINDOW-INERT`'s *title* is
wrong and its content is right: `std_string::view_uses_at_any_depth` does not
return the empty vector for every input — it names a view at every reading
position, and those names are what the composed window is computed from — but
its **depth contribution** is empty for every input, so **a `for` contributes
nothing to any view's window**, pinned over every `StringEvent` constructor by
`the_depth_contribution_is_empty_for_every_event`. That is safe only because no
admitted composed body can read a view, and the card measured that this rests
on **three** mechanisms rather than the one the code claimed: the body's type
(a body holds `BufferEvent`s, none of which names a view), the `Q077`
vocabulary refusal for the *constructing* forms — and, newly named,
`COMPOSED-BODY-BARE-RENDER-INCIDENTAL`, because a **bare** `print s;` is
deliberately not claimed by `StringUse::Render` and is refused instead by the
buffer route's own "names a `std.fmt` rendering" rule, which is a rendering
rule and not a window rule, and which a later card could remove without reading
this file. Measured beside it, by re-running wave 39's own M1 against each of
that card's 24 rows: **7 are anchored by the window function and 17 are not**,
and **none of the composed-route rows is** — the composed window is defence in
depth at depth and the deciding rule only at top level
(`W41-M9-composed-window-emptied` separates the two). The repair that moves
code is on the buffer side: `custody_names_at_any_depth` now classifies the
names it meets (`WindowName::Binding` / `WindowName::FieldPath`) and the four
field-path events of `AGGREGATE-OWNED-FIELD` **refuse by name** at the window
function's own new gate `window_unnameable_events` (`Q051`, a condition
extension of the band constraint 15h already names for the lend's window
shapes; no code allocated) instead of silently contributing nothing —
discharging `WINDOW-FIELD-ARM-UNREACHED` on its own second disjunct, with its
premise corrected: **two of the six arms it counted are reachable today**
(`Construct` and `Field`, measured admitted beside a lend and inside a body of
a lending entry) and are correctly empty. In Lean, `QCore.LoopWindows`'
`the_admitted_body_shapes_are_exactly_these` — the two-constructor tautology
`LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY` named — is **superseded** by
`every_admitted_body_event_is_windowed` over `LendEvent`, the window-relevant
shape of the Rust `BufferEvent`, which a mutation falsifies; and
`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION` is discharged on **two** of its three
disjuncts at once — the unrolling is instantiated at that lend-carrying
alphabet (`the_lend_alphabets_unrolling_is_loop_v1s_recursion` beside
`loop_v1s_unrolling_is_the_same_flattening`, the two sharing a right-hand
side), and the doc-comment link is replaced by a **differential test over
shared vectors**: the Rust test reads `proof/QCore/LoopWindows.lean` as text,
extracts all eight rows of its `agreement` table, rebuilds each spelling as a
`Vec<BufferEvent>` and asserts `last_uses` returns the Lean number, while the
Lean side proves `the_agreement_table_holds` — that every row's spelling is its
own program's rendering and every row's number is `lastUse`'s answer — so
neither side can be edited alone (`W41-M6` reddens the Rust row from the Lean
file; `W41-M7` fails the Lean build from the same edit). **Ten mutations, every
one non-vacuous**, every file restored sha256-equal with both baselines green
before and after. **Emission census: 535 programs, 146 admitted by both,
byte-identical, 0 differing, 0 verdicts moved in either direction** — the new
gate refuses nothing reachable from source today, which is exactly why the old
arm looked safe. Four retirements
(`COMPOSED-VIEW-WINDOW-INERT`, `WINDOW-FIELD-ARM-UNREACHED`,
`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`, `LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY`),
three registrations (`COMPOSED-BODY-BARE-RENDER-INCIDENTAL`,
`WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED`, `LOOPWINDOWS-BINDING-IS-FINITE`).
No new `Q` code, no reserved word, no corpus count, unit status or function
status moved, raw units stay unpromoted and full language conformance stays
**false**. The card was rebased mid-flight onto `96142e49`'s documentation
restructure and its ledger edits re-routed: the follow-up figure to
`docs/STATUS.md`, constraint 15h's amendment to `docs/agents/buffers-records.md`
and the `Q051` widening plus the namespace tally to `docs/agents/diagnostics.md`
— in every case **outside** the `<!-- preserved:… -->` markers, because
`docs/reports/docs-structure-compression/verify.py` pins each preserved span's
`sha256` as a byte-exact reconstruction of the pre-restructure guides. The first
draft was written inside those markers, the verifier caught it, and **the pin
was not edited** (constraint 21); `scripts/build_docs_index.py --check` caught a
stale report index in the same pass and was regenerated. Evidence:
`docs/reports/composed-window-reach/`.
