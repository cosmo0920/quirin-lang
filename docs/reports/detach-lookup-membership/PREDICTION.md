# Frozen preimplementation prediction: checked array lookup membership

Base `b5a00e7dc23628f9b9317ab5e8f2fb5d7ec365d7`; card starts 2026-10-19.
This record precedes analyzer/proof implementation. Preparation may add an
exact-source fixture and a baseline-refusal reproduction, without changing the
analyzer. The prior checked-detach source is
`9bfe86a4115c4a7e3e4fb0de93c2e037ef5fcfbe`, frozen binary SHA-256
`9f10a5bbf704eb3c56af4c2e6c6c6ebe78e761cd4b7208937701c2a5295c3f06`.
Its final raw/normalized three-route artifacts are public in the prior report;
no baseline production audit is repeated.

## Actual boundary and hypothesis

At pinned Fluent Bit `66910c10`, cJSON.c1907–1923 `get_array_item` starts a local
cursor at NULL, returns NULL for NULL array, initializes the cursor from
array.child, walks cursor.next while cursor is non-NULL and index>0, decrements
only its unsigned local index, and returns that cursor. It has no call or
nonlocal store. The retained production baseline is Unknown on exactly
`unresolved resource identity current_child` at the return. `cJSON_GetArrayItem`
is Unknown on uncontracted get_array_item; `cJSON_DetachItemFromArray` is
Unknown on the nested lookup inside the modeled detach. Its negative signed
index branch returns NULL before the size_t conversion.

A checked exact traversal frame can establish a returned NULL or list member
without transferring custody. This is stronger than BorrowedFrom and weaker
than a resolved existing object identity. Identity must additionally follow
from actual caller facts identifying the reached head/successor; an unordered
set of known children is not by itself an ordered or closed physical list.
No new allocation/site may stand in for the unknown reached child. The existing
`detach_list_representation` stays an exact named DeclaredAssumption, supplies
no membership by itself, and does not become Validated or unconditional C
conformance. The lookup body, caller identity, physical representation,
remaining-list frame and removed-result transport are independent obligations.

The intended implementation is a minimal general checked read-only traversal
class in the gateway with matching audited Lean and retained-source replay.
Prefer private source-derived facts and existing contract vocabulary/schema19;
coordinate before any public schema change. A needed minimal lookup-to-call
consumer will be described to the integrator BEFORE implementation. General
whole-return/nested Removed transport remains DETACH-RESULT-WRAPPER-COMPOSITION;
object-key lookup and source closure of the physical list relation remain
explicit residuals. No declaration is added merely to obtain Safe.

## Predicted observations and controls

The planning baseline is258 single-body waits/2223candidate bodies/363hypothetical
ceiling with no-extra-body same/cross closure4/4. get_array_item has one own leaf
and three hypothetical closure rows, not three guaranteed Safe rows. The body
may become custody-Safe if the bounded return-reading obligations discharge;
callers may remain Unknown on lookup identity or separate result transport.
Do not promise any of the six detach wrappers becomes Safe.

Controls cover NULL parent, empty list, head/middle/tail, zero/in-range/missing
or huge index, the wrapper's negative signed index, repeated-lookups/aliases,
wrong parent, stale/dead/rebound parent, invalidating link writes/calls, joins,
unknown order and nonmember values. Require the exact typed source/frame and
qualifiers; no BorrowedFrom/non-NULL/list-invariant shortcut establishes the
specific item. Pure/model proofs and native fixture observations remain
separate; sanitizer/provider/native suites and full gates/audits are serialized
under the integrator's slot. Every failed attempt is retained.

Final acceptance includes the standing gate, actual Clang and full Lean,
provider checks if their input closure changes, all three fresh production
routes, five artifact families including configured Monkey absence, raw and
normalized module keys/full payloads/duplicates, every moved row, null/combined/
runtime review and fresh bottom-up planning. Update current STATUS/ledger/matrix
and exactly one own paragraph in assigned wave48part10; leave part9 and frozen
reports unchanged. The report ends with numbered For the integrator items.
