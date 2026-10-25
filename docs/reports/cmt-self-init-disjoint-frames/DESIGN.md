# Proposed initializer frame boundary

This is a pre-implementation design. Fresh BASE and planner evidence must precede
the prediction commit; that commit must precede Rust, Lean and canonical `.qv`
semantic changes. The current implementation still has rule 36b's default-only
custody frame.

The next bounded route preserves child pointer slots in the initialized owner.
It proves their separation from the initializer writes with source-bound Clang
record layout and `footprint_connection::core_members_disjoint`. For an embedded
initializer, checking against the whole first enclosing member gives a
conservative footprint. A field name difference supplies no proof. Different
resource IDs supply no cross-allocation separation or allocator-freshness premise.

The existing empty-frame route remains available with its original guards.
The proposed alternative admits only exact LIVE, nonnullable, precise Heap owner
and child identities, with child field links rooted at that owner. Every linked
slot must have the source/layout proof. Partial-link markers must be a subset of
the field links and remain intact. Foreign-owner graphs, containment, membership,
collections, borrows, callbacks and every other nondefault custody field refuse.
The existing indirect-access check must still add no diagnostic or state change;
otherwise execution uses the exact old fallback.

Slot provenance needs more than checking current aliases. A retyped alias can
write a field, then be cleared while its field link survives. The proposed first
source subset therefore checks pointer initializers and assignments throughout
the caller: plain same-type declaration copies, direct contracted Owned results,
or known NULL. Loaded, retyped, derived, removed and opaque pointer results decline
this optional proof. Member destinations must be direct ordinary typed places;
ambiguous declarations and record tags decline. This restriction concerns logical
resource identities under existing contracts and adds no physical allocation
freshness claim. Source readers must use iterative walks in the admitted deep-AST
band.

All current identity aliases of the owner must have compatible source types.
Only already recorded protected direct-slot pointer facts through those aliases
survive. Other recognized member pointer facts are conservatively forgotten,
including facts through foreign and derived aliases; no physical separation is
inferred for them. Unprotected owner-rooted facts with foreign or unknown targets
still refuse. Plain and derived alias bindings, resource statuses, nullable facts,
field links and partial links are preserved. No forgotten pointer fact discharges
a child obligation or invents a missing alias readback.

Required controls include actual C source with root and embedded initializers,
existing facts accessed through a compatible alias, overlapping writes, historical
retyping, missing or ambiguous layout, unsupported custody frames, partial escape,
child leaks and later lifetime violations. Independent source/certificate replay
must reject forged protection metadata. The initializer marker continues to block
ordinary and cross-unit summary export.

Alias preservation is a flow obligation, not a promise of new source admission:
the existing aggregate-store classifier rejects an owner copied into another
declared pointer. Retain that source refusal and exercise compatible-alias fact
preservation with explicit flow states. No aggregate-store rule is relaxed by
this initializer feature.

The matching Lean module and audit must cover admission, layout separation,
complete ledger/graph preservation, conservative invalidation, partial links,
refusals and finite composition, with Rust/shared-vector and source controls.
Canonical CFL initializer clauses must describe the same boundary. Their model
remains outside interpreted resource/bounds views and supplies no resource-v1
call permission. Faithful extraction, embedding, C bounds/no-undefined-behaviour
and Rust/model correspondence remain declared trust boundaries, not a C
refinement theorem.

The fresh native comparison must retain resource, NULL and runtime evidence on
the same frozen inputs. Prediction and measured movement belong in their own
records after BASE. `W48-CMT-SELF-INIT-DISJOINT-FRAMES` and the broader CMetrics
getter chain remain Open for unsupported relations, deeper/same-member paths,
cross-allocation framing and the other residuals in the brief.
