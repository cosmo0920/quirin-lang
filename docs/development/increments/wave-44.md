# Wave 44 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-43.md) holds the wave-43 paragraphs and stays unchanged.

**2026-09-25 — W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: a destination is the whole left-hand side.**
This is a soundness repair. `direct_assignment_destination` named the first
place-spelling suffix of a left-hand side, so `recs[i].name = x_dup(s);
x_free(name);` bound the local `name` and certified `Safe`. A destination is now
named only when the suffix is the whole left-hand side or follows a declarator
prefix (identifiers, `*`, `,`). Every other such left-hand side is a store into
a sub-object, and the lexical gate refuses it.

The reach census of the base found 10,979 distinct suffix-named records:

* 7,853 declarations, whose names are unchanged;
* 3,126 sub-object stores, of which 1,450 had been admitted under a misnamed
  destination, in 173 functions.

Only two Safe functions relied on it, `LZ4_write16` and `LZ4_write32`, and
both move to Unknown. `unpack_basic_type` moves from provisional Unsafe to
Unknown when its two false leaks, accusations about the misnamed `opts`, are
withdrawn. Leak 271 → 269, `unknown_alias` −1,116, no unit moves, and every
delta is attributed. The forecast missed only nine caller leaves of the two
functions that lost their W4 summaries.

The descriptor guard's missed index read is fixed by the same reader. The
protected-formal closure keeps the old over-approximating reader. Lean
`QCore.AssignmentDestinationSuffix` (17 theorems, including "the repair never
promotes") is audited by `AuditAssignmentDestinationSuffix.lean`. The card
registers three residuals; its own ledger name lives on the call-argument
branch, for the integrator to retire. See
[the report](../../reports/w44-assignment-destination-suffix-place/README.md).

**2026-09-25 — W44-INDIRECT-CHAIN-ACCESS: typed record-pointer member chains.**
W43's typed statement route and typed condition read now accept a member chain
`r->l1->…->f`, `(*r).l1->…->f` or `s.l1->…->f` as an access. A chain has two
to four selections, every link exactly `struct POINTEE *`, and a scalar final
member. The chain rides on `IndirectScalarAccess` (a `chains` field, no new
variant), and each pointer it reads through carries an obligation:

* a root that is never bound and unbound or non-null is untracked;
* a root that is one live identity is tracked;
* a link of an untracked owner is untracked;
* a link of a tracked owner must be the member place an admitted store bound
  to one live linked child. Otherwise it records `UnknownAlias`.

The census of the instrumentation commit reproduces BEFORE byte for byte, and
W43's classifiers reproduce both ledger sizes at base (5,565 statement and 1,248
condition leaves). The census also shows that 3,564 of those 6,813 leaves carry
no member chain. The rule's recogniser admits 74 leaves. The measured outcome
is exactly as forecast:

* 24 leaves are removed;
* 50 are exchanged for 49 root refusals;
* 0 are downstream;
* 0 function or unit moves;
* leak 271 unchanged;
* 0 violation candidates.

Lean `QCore.IndirectChainAccess` (36 theorems) is audited by
`AuditIndirectChainAccess.lean`. Six gateway pins move deliberately. The card
retires `W43-INDIRECT-CHAIN-ACCESS` and `W43-INDEX-TRAVERSAL-CONDITION-CHAIN`
and registers nine measured residuals; the largest real one is nested
aggregates (1,287 leaves). See
[the card report](../../reports/w44-indirect-chain-access/README.md).
