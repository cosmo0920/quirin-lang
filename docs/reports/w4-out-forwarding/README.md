# W4: validated direct Out-pointer forwarding

2026-09-07. Base `ef28d4d` (the merge of the validated Removed-custody increment).
This change infers direct output-slot forwarding through acyclic internal C
wrappers, such as `fill(void **out) { q_make(out); }`, when the callee has an
already modeled Out effect. Declared contracts remain trusted assumptions and
retain precedence. General pointer-to-pointer aliases and recursive components
remain unsupported.

## Boundary and identity checks

The previous implementation recognized an Out argument only as `&place`.
The archived baseline regression demonstrates that the simple forwarding wrapper
therefore left its caller Unknown. A bare identifier, with enclosing parentheses
permitted, can now denote the symbolic `*formal` slot. This spelling grants no
write authority: CFG transfer requires that exact original slot to be sanctioned
by the enclosing candidate, and every reachable exit must discharge its existing
Out obligation. The candidate is freshly re-analyzed before publication; the
certificate checker independently rederives the same exact contract from source.

Candidate discovery includes direct `*formal` writes and forwarded modeled Out
arguments. Stable-identity guards reject formal reassignment, local aliases used
as output authority, repeated or incomplete initialization, pointer increments,
nested assignments, embedded conditional/comma calls, and unmodeled exposure of
a sanctioned output pointer through a non-Out call. Direct borrowed argument names
are retained for this identity guard; their callee custody effect remains empty.
Borrowed complex expressions and address exposure involving a protected formal
remain Unknown.

Before parsing these expressions, a conservative inventory protects direct
indirect-use formals, exact formals used by non-borrowed resource effects,
forwarded output formals, and declared non-Borrowed formals. Thus unrelated scalar
updates need not be rejected, while `p++; q_free(p)` cannot reuse the original
resource identity. Direct resource-call local names are protected too. Protection propagates in both
directions through direct name-to-name alias assignments until stable, regardless
of statement order. Thus `p++; alias = p; q_free(alias)` and mutation of an alias
to a locally acquired resource reject. This flow-insensitive closure is only an
invalidation inventory; it proposes no summary and creates no authority.
Name-based resource places still lack lexical binding IDs, so declarations that
may shadow any formal reject, including later comma-separated declarators.
The archived intermediate failing shadow regression records why this guard is
necessary; the complete C11 gateway now also rejects its certification.

Existing all-exit validation preserves the produced resource, nullability and
original caller slot. Separate exits may establish a fresh non-null result or
literal null. An unresolved nullable abstract allocation, a joined ambiguous
handback, an untouched exit, a leaked overwritten output, or aliasing between
caller output slots never becomes Safe. This is bounded contract-guided custody
reasoning, not general C heap extraction or proof that an external contract is
true.

## Regressions and Lean

Eight additional gateway tests exercise direct and nested forwarding, parentheses,
nullable separate exits, unresolved nullable effects, caller leaks and aliased
slots, scalar versus resource mutation, declared-contract conflicts, source and
summary certificate mutation, and a table of adversarial identity/path cases.
The pre-change direct-forwarding test failed before implementation; it now passes.
The intermediate formal-shadow and mutation-before-alias counterexamples also
have archived failing runs.

`QCore.CustodySummary.forwardOutSlot` is an explicit reduced identity relation.
Five audited theorems establish preservation of the exact original sanctioned
unwritten slot and delivered value, rejection after pointer reassignment or
without authority, failure of nullable output to satisfy non-null custody, and
acceptance of a fresh non-null direct forwarding step. The importer guards are
explicit premises of this reduced model, not a proof of token parsing, lexical
scope resolution, arbitrary C executions or source-to-Lean refinement.

Recursive SCCs stay Unknown. A self-consistent custody assumption alone does not
establish a sound recursive summary: the current model has no checked termination
or ranking witness and does not retain scalar relations needed to discharge one.
This increment adds no optimistic fixed-point initialization or recursive fallback.

## Pinned evidence and standing gate

The baseline is the previously archived full 185-TU result from commit `6ffcf6e`,
whose merge is this worktree's base; it was reused rather than claimed as a newly
executed baseline. The after audit is fresh, using the persistent forced-Libevent
compile database at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/monkey-build/compile_commands.json`.
Exact inventories, diagnostic additions/removals, function and unit transitions,
source comparison, hashes, and command exit codes are archived beside this report.
Raw provisional Unsafe results remain unpromoted; diagnostic disappearance after
an Unknown guard is not evidence that an upstream defect was repaired.

Final totals: **3 Safe / 94 provisional Unsafe / 84 Unknown / 4 data-only**.
Four units downgrade from provisional Unsafe to Unknown: librdkafka `rdhttp.c`,
`rdkafka_queue.c`, `rdunittest.c`, and Monkey `mk_core/mk_event.c`.
Twenty-five function classes downgrade to Unknown (24 provisional Unsafe and
one Safe); no function or unit is promoted. The formerly Safe
`rd_kafka_messages_new` becomes Unknown conservatively because an allocation-size
expression references a formal treated as protected during candidate validation.
No typed scalar proof currently discharges that case. This is an explicit
precision limitation, not a claim of an upstream defect. Projection coverage and
function inventories are unchanged. Monkey source JSON is byte-identical.

Exact diagnostic deltas are +142 unsupported, −223 leak, −8 unknown-alias and
−18 double-release; these reflect conservative Unknown transfer, not repaired library
violations. `parity.json` contains every added and removed diagnostic and every
class transition. Contract sidecars retain exact before/after trust context; their sole difference
is the worktree prefix on source-file keys, verified explicitly in `parity.json`.

The complete standing gate passes: **567 workspace tests**, formatting, clippy
with warnings denied, both reevaluation examples, Lean build and all **19** Lean
audits. The pinned Libevent submodule was initialized before validation. Shared
quota interruptions and one intermediate clippy warning were resolved before the
successful final gate; no failing validation is counted as completion.
