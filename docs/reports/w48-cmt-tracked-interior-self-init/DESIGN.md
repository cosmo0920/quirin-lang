# Checked effect transport into tracked storage

CG11 already checks literal self-address stores, and rule 36a transports a
checked leaf only inside an otherwise empty private caller frame. This feature
adds a separate, source-bound effect marker at a direct unit-local call; it
retains the leaf's self-write fields and the exact caller target. It does not
widen CG11's general tracked-store rule or make an initializer Borrowed/None.

A leaf retains rule 36a's complete void/single-original-struct-pointer profile:
only blocks, base NULL guards, unvalued returns and literal self-stores, with at
least one store and no calls. Its complete CFG must independently validate
through CG11, with exact source/store mapping and empty exits. Escaping function
designators, missing declarations/types/spans, other effects, recursion and
existing declared/guarded interfaces decline.

The actual is either an exact struct-pointer root or the address of a record
member reached by one root arrow followed only by embedded record selections.
Clang declaration identity, compatible pointer type, complete ordinary member
facts and exact statement occurrence/source tokens are required. Loaded member
pointers, arrays, casts, union/bit-field/volatile/atomic views, incomplete layouts
and ambiguous or foreign root declarations decline. The embedding depth is
bounded. Source walks must be measured in the existing deep-reader band.

At the call, the root must be one known, non-null, identity pointer to exactly
LIVE owned storage, with no nullable-site uncertainty. A borrowed/name-only
formal does not provide that custody. The bounded frame may contain pointer
bindings, resource statuses and nullable-site facts; every other state field
must remain its default. This explicitly refuses child, containment, partial
link, lookup, replacement, membership, collection, retention, borrow, reference,
callback, constructed-state, invalidation and unresolved-effect frames rather
than discarding their obligations. Future state fields participate in this
check automatically. All resource statuses and nullable facts are preserved.

The ordinary root/chain and may-alias read checks still apply. Every root
spelling whose binding can target the same owner, including non-owning derived
aliases, participates in member-fact invalidation. An old member binding that
is unknown or targets a different identity refuses. Admitted target-free or
same-owner self facts are forgotten through those spellings; plain aliases and
the resource ledger stay intact. No owning child link or new resource alias is
created. An embedded self edge is bounded by its containing object's lifetime;
the existing parent release still invalidates aliases into that storage.

Failure applies the exact pre-existing Unknown fallback. The new instruction
is appended to preserve serialized tags, retains its write set and cannot
establish an ordinary custody summary or checked specification. Structurally
unreachable markers recover the prior lowering. Indirect callbacks and cross-
unit neutral summaries gain no effect privilege; the caller's whole CFG and
all reachable exits remain the authority for its result. Existing modeled
calls and rule 36a retain precedence, and declined probes restore all reading
and consultation state.

This is initializer effect transport for fresh tracked storage, with known
membership/child frames refused. General owned-list operations and the rest of
the CMetrics getter ownership chain remain separate work. Rust/Lean
correspondence and faithful Clang extraction remain premise 28; no C refinement
or allocation-bounds proof is claimed. The report must own complete fresh
BASE/TIP attribution and the standing gate, with failed attempts retained.
