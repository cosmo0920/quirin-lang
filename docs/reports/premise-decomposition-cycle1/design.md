# Premise decomposition cycle 1: cJSON physical representation

Base `2d8f6d34257db14031a2578ce14f941bee25fc19`. Frozen before implementation.

Select the existing `detach_list_representation` premise of
`cJSON_DetachItemViaPointer` in the provider-owned `librdkafka/cjson.qv`.
Its concrete physical representation and writer exclusion obligations make
it a bounded first family; allocator-hook and list-insertion premises remain
untouched. The tracked-input census beside this design records selection.

Retain the exact legacy `resource_premise` clause because the detach consumer
requires that exact semantic identity. Add a distinct evidence-only conjunction
`detach_representation_atoms` beside it. Grammar v1 cannot give a structured
conjunction the same name as a legacy premise or preserve custom erasure prose.
The new conjunction is therefore a parallel decomposition proposal, not a
replacement, equivalence proof, extra consumer or standing promotion.

The nine declaration-local opaque atoms, in canonical ID order, are:

| Atom | Obligation in the old proposition |
| --- | --- |
| detach_chain | live parent represents exactly its owned children by a finite acyclic child/next chain |
| detach_distinct | child identities in that chain are pairwise distinct |
| detach_empty | child is NULL precisely when that chain is empty |
| detach_first_prev | first child's prev is the last child |
| detach_last_next | last child's next is NULL |
| detach_link_targets | every non-NULL next/prev target is a live child of the same parent |
| detach_no_writer | no concurrent writer mutates these links during the call |
| detach_parent_outside | parent is not a child |
| detach_predecessor | each nonfirst child's prev is its predecessor |

Every atom retains the removed-result declaration qualifier. These remain
opaque/open with declared warrant in exact per-row contexts. Source reading
is a locator for the obligations, never evidence that arbitrary caller heaps
satisfy them. Atom separation does not claim logical independence. Equivalence
between prose and conjunction remains an open human/formal review obligation.

Acceptance: production retained-text replay and include/inherit controls;
recomputed expected graph rejects removed/changed atoms; actual dependency
publication carries every selected atom; existing graph Lean theorem families
cover canonical conjunction, open-atom refusal, context/warrant separation and
verdict erasure. No semantic rule changes are planned. Update deliberate clause
count/pin/inventory through official tools. Standing and conditional provider
and package gates pass, with commands/exits/counts/logs retained.

Prediction: zero static custody/null/combined verdict, diagnostic, reason or
standing movement on six fresh base/tip corpus/system/tail-pause audits;
complete runtime projections retain their statuses. Expected metadata changes
are the selected declaration digest/pins, module inventory/load-path hashes,
added conjunction premise and graph publication/dependencies. Preserve old
premise text exactly. Any additional runtime movement must be attributed and
resolved rather than silently accepted. No outer envelope expansion.

Open follow-ons: equivalence review; typed semantic consumer only under a
separately reviewed registry/rule card; remaining allocator hooks, insertion
invariant and sibling-chain release premises; generic successor lookup and
nested Removed transport remain their existing independent cards.
