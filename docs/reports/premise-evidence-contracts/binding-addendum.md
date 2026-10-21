# Content-addressed bindings: preimplementation refinement

The graph-v2 design stays unchanged. No structured JSON is encoded into opaque
atom strings, and their prose gains no machine predicate semantics. A separate
version1 canonical binding manifest binds the exact graph digest, existing
premise name, every attached atom ID/digest, exact model/audit artifact
paths+SHA256, formal proposition names and explicit environment/quantifier/
prerequisite/conclusion records. Source binding remains Open in every record.

The retained production module pins its digest in a module comment:
`// qcore-premise-binding-manifest-v1 DECLARATION PREMISE SHA256`
The pin is machine-readable retained metadata, not a language predicate or
additional named premise. Comments are retained in source_assumptions and
provider package asset hashes. A comment is outside graph serialization, so a
manifest can bind the full graph digest without a content-hash cycle. This
marker is exact and total-or-reject when presented to the binding reader; no
opportunistic path discovery or hidden report lookup is allowed.

A generic explicit resolver takes independently held expected graphs, retained
source bytes, content-addressed manifest bytes and independently held model/
audit artifact bytes. It freshly replays sources, checks each pin/manifest SHA,
canonical version/schema, exact graph and atom identity, exact attached-child
domain, and all referenced artifact hashes. Missing inputs, mismatch, duplicate
pins, unknown versions/fields, unsafe/ambiguous references and source-binding
promotion refuse. Returned records warrant reference integrity only; matching
hashes never prove the C/source/environment antecedents or mint Derived truth.
Ordinary contract loading/graph replay does not silently search for manifests.

Audit publication includes declared binding requirements next to the graph.
An explicit checked-binding publication API resolves supplied inputs and
publishes the checked records next to independently held actual graphs. The
card's replay command supplies manifest and model/audit files explicitly and
records every argv/hash/result. A generic explicit resolver is the bounded
interface; a broader CLI/package asset routing system is not claimed here.
Required supplied evidence is part of this card's scope. Production contract
assets carry the exact manifest pin, and the report ships the addressed manifest
as an explicit input. No unspecified filesystem lookup is sufficient.

Controls independently freeze the expected graph and manifest, alter every
identity/reference/domain and prerequisite/environment record, remove artifacts,
reject foreign/stale Context evidence, and demonstrate direct+summary evidence
impact while unrelated and other-context records remain unchanged. Publication
and erasure theorems keep declared legacy facts and truth separate from these
checked reference records and proposed conjunction dispositions.
