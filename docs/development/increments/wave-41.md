# Wave 41 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules are routed by
> [AGENTS.md](../../../AGENTS.md) to the [rule reference](../../agents/README.md)
> and headline status lives in [docs/STATUS.md](../../STATUS.md); each card's
> report owns its evidence.

**`OWNED-FIELD-ROUTE-REACH` (2026-09-19).** Wave 39's five owned-field rules
were measured for **reach** before anything was changed, by a shipped script
that writes 29 whole `.qcore` programs and hands each to the built `quirin
check` (`docs/reports/owned-field-route-reach/scripts/reach.py`, paired against
the same script run on a `git archive` of the card's base `463fa6e8` by
`reach_delta.py`). Four rules — construction moves in, the partial move's hole,
the declaration-order drop-through, the leak naming a path — were already
driven by real programs. The fifth, **non-copyability, was reached by nothing**:
`OwnershipOp::Copy` was constructed at four places in the tree, every one a unit
test (`STABILITY-REEVAL-4`'s `OWNERSHIP-COPY-OP-UNPRODUCED`), so the rule was
decided by hand-built operation lists and constraint 15k's "a v1 record is a
plain copyable value" was an assertion nothing could check in either direction.
The card gives the question a source spelling: `let NAME = NAME;` — **the copy
bind** — on `source.buffer.v1`, which builds `Copy` for every name the ownership
ledger holds a binding for and **no operation at all** for every other. Seven of
the 29 probes moved and the rest are unchanged: an owned buffer, an owning
record hollow or not, a moved binding, a dropped one and a write loan now refuse
at `Q054` with the inherited checker's own words where they were a `Q051` shape
refusal, and a scalar and a record of scalars are **admitted** — the card's one
new admission, its invariant-1 argument proved rather than asserted in
`QCore.OwnedFieldCopyReach` (a place the ledger holds no binding for owns no
resource, so an admitted copy moves no duty and is the identity on the ledger;
142 Lean audits, +1). The three boundaries `STABILITY-REEVAL-4` measured as
**absence rather than refusal** now refuse **by their own names**, each at the
code, span and refused set it already had: `AGGREGATE-VIEW-FIELD` and
`AGGREGATE-NESTED` at `Q051` (the latter in either declaration order, the
classification reading the module's declaration kinds), `AGGREGATE-TASK-CAPTURE`
at `Q500` in the task route's declaration walk, with the same module minus the
record pinned admitted so the refusal is attributable. **Zero words reserved,
zero tokens, zero codes**; the nineteenth run of the `Q-NAMESPACE-DECISION`
consequence, on the existing `Q051`–`Q057` band. The constraint-14 census
(535 `.qcore`/`.qc` files) **found a collision and kept it**: the claimed token
shape occurs 29 times in the tree, every occurrence `source.socket.v1`'s
`let listener = tcp;`, so the copy bind was deliberately **not** added to
`buffer_use` — the function route selection and the composed string route both
read this route's statements through — and a socket program's admission and the
composed route's `Q068` are pinned rather than argued
(`COPY-BIND-SOCKET-SHAPE-COLLISION`). 19 end-to-end tests drive whole programs
through `compile_files`; 14 mutations, and the harness earned its keep — its
**first run reported one VACUOUS** (a totality arm the loop test did not
measure), which a second test about the loop's shadowing refusal turned into a
real measurement, both logs shipped. What stays unreachable is recorded in those
words: `AggregateAtOwnerPosition` and `NotAnAggregate` are **unreachable from
any source program**, the route's own `Q051`/`Q052` gates deciding every such
shape first by design, and they are kept as the IR's fail-closed backstop with
the two things that would reach them named
(`AGGREGATE-OWNER-POSITION-UNREACHABLE`). Tracker **257 → 259**: three retired
by exact name after each discharge was read in full (`OWNERSHIP-COPY-OP-UNPRODUCED`
on its "emit Copy from a route" arm, `OWNED-FIELD-IR-REFUSALS-UNREACHED` on its
documentation arm, `OWNED-FIELD-BOUNDARIES-ARE-ABSENCE` on its "a test that
drives the refusal and asserts the boundary's own name" arm), five registered,
and four amended in place because this card made their text false —
`OWNED-FIELD-SEQUENTIAL-UNREACHED` **stands**, re-scoped from "`Q103`'s grown
vocabulary" to `ownership`'s whole operation vocabulary beyond the four variants
the sequential route can present. Nothing is promoted: no corpus count, unit
status, function status or CT row moves, raw units stay unpromoted, and full
language conformance stays **false**.

> instructions describe their recorded dates. Current agent rules and headline
> status are routed by [AGENTS.md](../../../AGENTS.md) to the
> [rule reference](../../agents/README.md) and [status page](../../STATUS.md);
> each card's report owns its evidence.

**`CT-PROOF-LEG-3` (2026-09-19).** The fifth executed `proof` row, and the
measurement that the cheap route to a sixth is spent. The card first **re-ran
the survey rather than inheriting it**: every one of the 47 mapped cases' own
fixtures compiled through `compile_files`, every task-route module executed by
`execute_task_module` and handed to `AdmittedCausalOrder::derive`, on the
default script and on five failure scripts, with the raw output shipped
(`docs/reports/ct-proof-leg-3/probes/survey.tsv`). Eleven mapped cases derive on
the default script; four were already proved; CT12's and CT14's runs are **event
for event CT01's**; CT13, CT15, CT16, CT17 and CT20 derive but their default runs
do not exhibit their scenarios, whose defining arms are refused as
`UnattributableEvent` (creation failure) or carry an `Error` outcome the Lean
alphabet has no constructor for (allocation failure, and CT16's default run).
`STABILITY-REEVAL-4`'s claim that **CT50 is the only remaining F02-shaped
candidate** therefore holds, verified rather than inherited — and this card spent
it, so the set is now empty (`CT-PROOF-F02-CANDIDATES-EXHAUSTED`, registered;
`CT-PROOF-FOURTH-CASE-UNTRACKED` retired). CT50's leg is
`proof/QCore/Ct50LifetimeIdentity.lean` with `proof/AuditCt50LifetimeIdentity.lean`
(Lean audits **141 → 142**) and the host-unconditional
`compiler/tests/ct50_proof_leg.rs`; proof **4 of 47 → 5 of 47**, which is
evidence about one case's artifact and never conformance (constraint 23). Two
things about it are worth the record. Its **closure is the degenerate case and
is proved, not assumed**: the run has one actor, so a relation demanding
agreement on every actor's projection over the same multiset admits nothing but
the run itself (`an_admitted_rendering_is_the_reference_run`), which the Rust
leg re-measures on the artifact by enumerating `admitted_linearizations` and
requiring the singleton. And its **scope is the capability half of the
scenario**: CT50 is named after a numeric *address* reuse that an alphabet
carrying generations cannot see, so what is proved is disjoint generation
lifetimes and that no fact of the run uses the old generation, while address
identity stays native observation (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`,
registered). Beside the leg, `compiler/tests/proof_leg_digests.rs` closes
`PROOF-LEG-DIGESTS-UNCROSSCHECKED` (retired): every `theorem ct<N>…digest` in
`proof/QCore/` is compared with every corresponding `compiler/tests/` constant,
total or reject in both directions, and the sweep found **six** Rust pins where
the residual's own inventory named five — CT01's is written twice. Its own
premise is registered (`PROOF-LEG-DIGEST-CROSSCHECK-TEXTUAL`). **The non-vacuity
harness found two defects in this card's own claims and both were repaired
rather than reworded**: the read-by-a-second-actor mutant fails *two* clauses so
it isolates neither (replaced by a result-claimed-by-a-second-actor mutant that
does, with the old one kept and named as non-isolating), and the payload clause
is *implied* by the read-window chain together with the read count — now the Lean
theorem `the_payload_clause_is_implied_by_the_read_window_and_the_read_count`
rather than a silent weak spot, and its mutation row is expected green with that
ground. Twenty mutations, every restore sha256-equal, baseline green before and
after (`mutations.log`). Three carried entries disposed: `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`'s
discharge text repaired to name the mechanism that now exists (the entry stays
open; this card deliberately did not apply it to another card's Lean module),
and `CT-PROOF-CONTROL-ARM-TIER` and `CT-PROOF-CREATION-FAILURE-ALPHABET`
restated as measured boundaries with wave-41 measurements — the second sharpened,
because the cases it lists need **two** alphabet extensions and the entry named
one. No `Q` code, no reserved word, no lexer change, no checker or emitter
change; the diff over `crates/` and `compiler/src/` is empty.
`named_followups` **257 → 259** (+4 −2). The card was **rebased onto the documentation
restructure** (`96142e49`) before merge, so its ledger edits land in
`docs/STATUS.md` — the CT case-bound-proof row, the registered-follow-ups row
and the paragraph naming which cases bind through F02 — and **not** in the
former `AGENTS.md` ledger, which that commit replaced with a router. No
standing constraint text moved, so no `docs/agents/*.md` preserved block is
touched.
