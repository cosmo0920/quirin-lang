# Checked same-item detach: frozen prediction

Source baseline: `e8b9ff07defe4c6375b1547ce2016c68a8ded98c`.
User decision: implement a checked transfer of the same item from its parent
into the result on success; a NULL result transfers nothing. This prediction
precedes implementation. Later rebase and integration evidence is appended in
the report, never substituted here.

The pinned `cJSON_DetachItemViaPointer` at
`lib/librdkafka-2.15.0/src/cJSON.c:2243–2277` returns NULL before writing when
parent or item is NULL, or when item is neither the first child nor has a
predecessor. Otherwise it rewires predecessor/successor/parent links, clears
item's two links and returns exactly item. This is no allocation. Its guard
does not prove membership: a foreign item with a predecessor can pass. That
is a precondition example, not a validated upstream defect.

Three direct wrappers call it (`cJSON_DetachItemFromArray`,
`cJSON_DetachItemFromObject`, `cJSON_DetachItemFromObjectCaseSensitive`); three
DeleteItem wrappers call those. The historical six-body reach is hypothetical,
not a prediction that six rows become Safe. Existing `resource_removed`
checks exact abstract containment and restores the named item's custody, but
cannot name that same item as an owning return. Existing borrowed-from
results establish neither child-list membership nor ownership.

Implementation must preserve the distinction between exact item identity,
checked owner membership, list representation/frame validity, and NULL branch.
Unknown membership, joined owners, unrelated aliases and unsupported physical
list effects must refuse. A list invariant alone cannot establish membership.
A source-body declaration will be added only if its representation and transfer
obligations can be checked; no fresh-owned return declaration will substitute
for the missing relation. No corpus gain is promised before this boundary is
established and freshly measured.

The final immediate baseline will be the completed S3 integration. Reuse its
three route artifacts with explicit provenance rather than performing a
redundant expensive baseline now. Measure final corpus 66910c10, configured
system 046accf6 and configured slice 34155955 with the pinned link map, all
five corpus artifacts, configured artifact absence disclosures and separate
static/null/combined/runtime review. No runtime Pending-to-Safe claim follows
from this static class.
