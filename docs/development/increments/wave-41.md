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
