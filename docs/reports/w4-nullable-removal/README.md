# Nullable removal and conservative custody transfers

2026-09-07, integration base `49806a7`. W4 now infers direct nullable Removed
wrappers after re-analysis, and repairs ten independently reproduced false-Safe
patterns in the generic C custody transfer engine. These counterexamples pass
through real Clang imports and the public certificate rechecker. They are
synthetic C examples under explicit external resource contracts, not claims
about upstream library defects.

## Inference

The ordinary live-entry search still takes precedence. A modeled removal call
with exact formal child/owner arguments proposes the contained-entry candidate.
That candidate must validate before each child is independently weakened to
nullable. Every accepted weakening reruns the whole CFG with all earlier
weakenings still enabled, checking their combinations. A failed weakening keeps
the validated stronger precondition. This bounded search supports nullable
removal calls, guarded nonnull calls, reordered forwarding and mixed nullable/
nonnull children; it does not discover heap links or arbitrary aliases.

A null child has no resource custody. Null refinement removes only its incoming
containment edges. A nonnull child must retain its original identity, become
live/detached at every exit, and leave the original nonnull owner live. Other
children remain accounted for. Local reassignment cannot rename caller-owned
sites. Skipped removal, reattachment, wrong owners, release of original custody,
unknown effects and hidden returned payloads cannot validate a Removed summary.
Declared contracts retain precedence and conflicts; certificates independently
rederive source-bound summaries with InferredValidated trust. Schema/wire modes
and versions are unchanged.

## Transfer corrections

The review reproduced these previously accepted patterns:

1. Conditional removal followed by unconditional removal erased an already
   detached alternative from a merged CONTAINED|LIVE state.
2. A child stored in alternative owners could be detached through one convenient
   edge of the union, ignoring the other owner.
3. Removing a selected alias detached every possible child.
4. Releasing a selected alias discharged every possible allocation.
5. Storing a selected alias transferred every possible allocation.
6. Reallocation replaced all possible old allocations, hiding the unselected leak.
7. Aggregate cleanup erased a directly live alternative of a conditionally
   contained child.
8. Already released custody could be escaped.
9. Already escaped custody could be escaped again.
10. Reallocation erased a previously released alternative from mixed live state.

Every before probe reported Safe and its independent recheck accepted. The
probe provenance distinguishes the first pre-increment build from later
reproductions during partial repairs; these are not all attributed to one
unchanged intermediate binary. Preserved source, before/after logs and final
artifact hashes accompany this report.

A strong update now requires a known singleton selected resource (or null only
where permitted), the exact required disposition, and sufficient owner identity.
Detachment requires one exact containing owner. Aggregate release/escape
prevalidates every descendant's exact containment and unique owner before
changing custody. Reallocation validates its original resource before parameter
effects or result creation. Explicit nullable-old input contracts still admit
null and maybe-null input; this is separate from nullable allocation results. Terminal escape and mixed live preconditions reject.
Singleton aliases continue to work. General correlation across merged branches
remains unsupported; safe programs requiring that relation may remain Unknown.

Rejected effects preserve abstract custody and record affected unresolved sites,
including possible descendants. The uncertainty is retained across joins and
repeated allocation sites. A residual caused by skipping an unresolved effect
is not itself a proven leak; unrelated tracked allocations can still provide
independent leak evidence. Uncertain sites cannot validate a custody summary.
This avoids both optimistic discharge and an artificial Unsafe result caused
only by the newly rejected action.

## Verification boundary

The 19-test public integration suite covers direct null/maybe-null/nonnull callers,
guards, forwarding, sibling preservation, exact inferred modes, certificate
mode/source tampering, declared precedence, valid nullable-old reallocation, caller leaks, invalid exits, all
ten transfer failures and positive singleton controls. Gateway tests separately
exercise the transfer state and inference validation.

`QCore.NullableRemoval` models explicit null alternatives, stable identities,
nonempty exact disposition/owner/target gates, conservative direct and aggregate
updates, and unresolved-effect leak classification. Its 48 new theorems are audited
without project axioms. Arbitrary C alias extraction, source reachability and
Rust/Clang/Lean correspondence remain explicit boundaries.

The combined gate passes 716 workspace tests, nine provider tests, 43 Python
tests, formatting, Clippy, both probes, Lean build and 20 axiom audit modules.
The first combined Rust run exposed an over-rejection of legitimate nullable-old
reallocation; its log is retained separately. The corrected final run passes
the original expectation plus null/maybe-null/nonnull public controls.
The full 185-unit verdict/report comparison and counts are in
`validation.json` and the audit record. Raw units remain unpromoted. This work
advances W4; it does not complete the generalization plan, v0.3 conformance,
provider extraction or the remaining 43 CT mappings.

## Final pinned-corpus comparison

The complete audit passes with **2 Safe /42 provisional Unsafe /137 Unknown /
4 data-only**. Five former provisional Unsafe units become Unknown:
`cmetrics/cmt_cat.c`, `cmetrics/cmt_decode_statsd.c`, `librdkafka/rdkafka_lz4.c`,
`librdkafka/rdkafka_mock_handlers.c` and `monkey-libevent/mk_server/mk_fifo.c`.
Their former leak diagnostics depended on unresolved effects. This withdraws
unsupported defect claims; it does not prove those units safe. In total 22 units
have diagnostic changes, recorded field-by-field and as a complete unified diff.

Source, chunk-contract and source-effect outputs are byte-identical. The process
provider retains 3 checked function contracts and 93 Unknown functions. No package
rebuild or raw-unit promotion occurred. All ten final stable-artifact probes
reject their previous Safe result: 6 Unknown and 4 Unsafe. The fixed public suite
and final full gate validate the same source identities recorded here.
