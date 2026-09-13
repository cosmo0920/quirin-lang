# Wave 27 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 27 close (2026-09-12): **six pieces** merged one at a time under the
failure-gated chain, each under its own green gate — `CT23-CREDIT-ERROR-PATH`,
`DOCS-COMPACT-3` (the recurring three-file compaction *plus* a user-directed
examples style pass), `AGGREGATES-V1`, `QUANT-REGION-SOURCE`,
`CT07-SLICE-PERMISSIONS` and the arbiter `GATEWAY-C-LOCATION`. The wave's
language head is the **first source-level aggregate type**: `source.record.v1`
lands `type NAME { FIELD: i32, … }`, `let p = NAME { FIELD: OPERAND, … };` and
`let a = p.FIELD;` on `source.buffer.v1`, emitting real C structs with one
distinctly named member per field and byte-compared against a sanitized native
run, so CT09's *"distinct ordinary struct fields"* scenario is **executable** —
a lowering that gave two fields one member would print one value twice. Beside
it, `source.region.v1` gives the kernel's quantitative region its first source
path (`error region R { … } catch (E) quant open R as p { discharge p; }`), the
sealed `CleanupPack` bound as a **linear** capability consumed exactly once and
the admitted region **erasing** to byte-identical C with the hand-written
straight-line program it is; `EpochCreditRecoveryV1` checks what a flat epoch's
creator does with a grant on its **own error edges** (`restored + spent =
credit` componentwise, `restored ≤ credit`, and `restored_credits +
spent_credits = granted_credits ≤ allocation`), with abandonment and minting
both **refusals**; and `borrow_utf8(NAME, LOW, HIGH)` lends a **range** of an
owned buffer, so two disjoint permissions over one buffer are expressible and
the disjointness is *decided* (`max low < min high`, proved to be the
set-theoretic question itself rather than an approximation of it). **Three
recorded blockers were measured false, each in a different way, and that is the
wave's shape.** `STD-BUFFER-SLICE`'s blocker sentence — *"needs the ownership
IR's borrow nodes to carry a footprint fraction"* — is about the **write**
direction: a read lend leaves the write authority with the owner, the ownership
IR is untouched (`ownership.rs` does not appear in the diff) and the fraction is
what `SLICE-PERMISSIONS-WRITE-LEND` needs, so `STD-BUFFER-SLICE` is retired **in
part**. `RegionOp::QuantOpen`, the operation *variant* CT21/CT22's recorded gap
named, is **unreachable from a valid program by design** — the admitting
constructor is the `error_region::QuantOpen` *struct* on a catch clause and the
variant is retained so an opening outside a catch has an explicit `Q302`
(`REGION-QUANTOPEN-OP-VARIANT`) — so the card landed the half that matters and
said which. And the obvious aggregate spelling `record` was **rejected on
evidence, not taste**: `docs/reports/remaining-v07/probes/d02_boundary_record.qcore`
is the single line `record R { }` and two archived reports pin it to `Q000`, so
claiming the word would have falsified a frozen archive (constraint 21) — the
syntax was renamed rather than the pin edited, and the head is the
already-reserved `type` claimed contextually at `type NAME {`, censused at zero
bare occurrences. **Zero reserved words and zero new diagnostic codes across the
whole wave**: every refusal is a stated condition extension of an existing band
under the `Q-NAMESPACE-DECISION` ice box, run four times on four different bands
(`Q630`/`Q635`/`Q636`/`Q641`; `Q051`/`Q052`/`Q053`; `Q302` with `Q303`–`Q309`
reached from a second route; `Q077`/`Q078`), including by the language head
itself. `DOCS-COMPACT-3` honoured the cadence three waves on — **39 standing
constraints in, 39 out, zero dropped, zero renumbered**, 37 bodies byte-identical
and the two reworded ones argued clause by clause, `AGENTS.md` −796 bytes while
gaining lines, `README.md` refreshed to the full route surface, one drift
corrected (the entry points said `qcore-audit CONFIG.json`; the binary is
`quirin-audit`, `qcore-audit` being the package and library name) — and carried
the user-directed examples style pass beside it, *pipe the flow, parenthesize
the assembly*, five files respelled one commit each with byte identity measured
rather than predicted, nine examined and skipped with reasons, and the rule's one
open case named `EXAMPLES-STYLE-SLICE-FANOUT` for the user rather than repaired
against the brief's direction. **The wave ran one arbiter and its re-pin is
presentation-only.** `GATEWAY-C-LOCATION` makes a gateway diagnostic name the C a
person wrote — `L19357` is now `flb_input_chunk.c:2267` — by pairing the
analysed `-E -P` text with a second marked `-E` run under a **checked** line
correspondence, with the preprocessed `L`-number kept as the **internal**
coordinate by the user's ruling. No presented file spelling is ever read from a
flagless marker (proved in general as `presented_files_are_entered_files`), a
disagreeing flagless marker **withdraws** the coordinate rather than relabelling
it, and constraint 12's scope rule is untouched. The BEFORE run reproduces the
preceding baseline's `diagnostics.json` and `chunk-contracts.json` byte for byte
and the AFTER run differs in attribution presentation alone: the corpus baseline
moves to `docs/reports/gateway-c-location/artifacts/` with **185 units 2 Safe /
42 provisional Unsafe / 137 Unknown / 4 data-only**, 4,834 projected functions
150/84/4,600, leak **266**, `unknown_alias` **4,344**, `unsupported` **49,773**
and all 54,383 diagnostic rows keeping their `(function, preprocessed line,
category)` — **zero verdict moves, zero leaf-set moves, zero STOP findings**, so
constraint 7 gains no sixth move and every preceding archive stays frozen.
**Nothing is promoted**, no `Unknown` became `Safe`, every provisional `Unsafe`
stays a provisional claim and never an upstream defect claim, and full language
conformance stays **false**. CT coverage is **unchanged at 36/50 mapped**: the
wave mapped nothing and edited no manifest row, and the CT08/CT09/CT21/CT22/CT23
readiness it produced is recorded as capability in the cards' own reports and
`ct_stage_evidence.json` proposals — a capability is not a mapping (constraint
23). Standing constraints: 15h is **amended** on the slice card's own proposal
(the borrow lends a range; three asymmetries — the release arm is not range-wise,
an undecidable index conflicts with everything, two overlapping borrows are
deliberately admitted), and three new rules land in merge order, 15j (the epoch
credit law, with `Q637` deliberately not extended), 15k (a v1 record is a plain
copyable value and the field-custody answer is a measurement) and 15l (a region
erases from emission; the operation variant and the v1 path bound must not be
"repaired"), beside **12a** in the compile-task/source-transport block for the
arbiter's anti-forgery rule. Gate at the integration tip, `$HOME/.elan/bin` on
`PATH`: `cargo test --workspace --no-fail-fast` **4,376 passed, 0 failed, 8
ignored**; fmt clean; clippy clean; both probes exit 0; **114 Lean audits**
(110 → 114: `AuditSourceRecord`, `AuditQuantRegionSource`,
`AuditSlicePermissions`, `AuditSourceCoordinate`; the epoch card added 17
theorems inside an existing audit, which is why five cards produced four files);
193 Python tests OK after the matrix regeneration. Provider 122 is **carried** —
no provider source, manifest template or `qcore-schemas` version changed and
`qcore-schemas` stays 14.0.0, so constraint 18's rebuild-and-verify obligation is
not engaged. Evidence: `docs/reports/ct23-credit-error-path/README.md`,
`docs/reports/docs-compact-3/README.md`, `docs/reports/aggregates-v1/README.md`,
`docs/reports/quant-region-source/README.md`,
`docs/reports/ct07-slice-permissions/README.md`,
`docs/reports/gateway-c-location/README.md`, `docs/reports/wave27-close/gate.json`.

<!-- migrated-section:end -->
