# PREMISE-EVIDENCE-CONTRACTS: frozen implementation design

Exact base: c5c298db2. This card implements production same-name metadata
attachment for cJSON_DetachItemViaPointer's detach_list_representation. It does
not extend logical operators, register typed predicates, establish source truth,
or turn premise refutation into Unsafe. The previous cycle's report is frozen.

Keep the existing resource_premise NAME and exact original string unchanged.
Add nine resource_hypothesis declarations copied from cycle1's stable atoms and
one requires resource_premise_decomposition(NAME, ATOM, ...). NAME must identify
exactly one existing declared premise in this declaration; one attachment per
name. The attachment is a proposed conjunction, with no implication edge to or
from the old prose. Missing/ambiguous targets, duplicate attachments, unknown
heads/versions, dangling/cyclic/unused atoms and noncanonical children reject.

Wire: retain Graph's version/atoms/premises fields and add refinements, omitted
when empty. Each refinement is keyed by the existing premise name and carries
its generated legacy premise ID and strictly increasing nonempty atom children.
The original legacy singleton and atom, and ErasedPremise name/text/origin, stay
unchanged. Graph v1 requires no refinements and retains exact canonical bytes
and digests; Graph v2 requires a nonempty refinement map. Old readers reject v2.
New readers accept exactly v1 and v2, with strict canonical replay and no opaque
fallback. New public Rust fields are a deliberate schema major bump19.1 to20.0;
all five provider packages must be rebuilt and verified. Unattached graphs keep
v1; no blanket graph version or digest migration.

The existing compose(premise ID) evaluates the original singleton only. A
separate compose_refinement(existing name) evaluates the proposed conjunction,
checking independently held expected Graph and every Context coordinate before
using evidence. Refuted/Supported proposed conjunction metadata does not change
legacy proposition disposition, named premise vectors, static verdicts, standing
or closed-world classification. No rule may infer equivalence from erasure.

The nine formal bindings are the cycle1 Lean Props chain, distinct, empty,
firstPrev, lastNext, linkTargets, noWriter, parentOutside and predecessor over an
Environment with callActor/parent identities, finite children, ownedChild/live,
child/next/prev maps and an interference trace. Explicit atom identities,
quantifiers, antecedents, conclusions and open source/heap/trace/build/scope
binding obligations are retained in a machine-readable card map. The three
conditional theorems remain mathematical implications only. Evidence binds all
existing Instance Context coordinates; imported C/caller antecedents remain Open.

Generic ownership: qcore-schemas hypothesis surface and version, qcore-contract
hypothesis parser/validator/replay/composition and loader inheritance, audit graph
publication and gateway dependency tests where needed; grammar/contracts spec;
production cjson attachment, official source pins/inventory and corpus count.
No new source-language reserved token; contextual head only. The libc sibling
owns read-only independent evidence-binding design and no shared generic edits.

Acceptance prediction before implementation: all original DeclaredPremise
vectors and complete custody_facts records equal; all static verdicts, full null
records, full runtime records and declaration/clause standing equal on fresh
base/tip corpus/system/slice pairs. Only explicit attachment graphs/dependencies,
production source text/digests/pins and derived exact environment/configuration
hashes may change. Source/time/root changes require exact retained attribution.
No compatibility waiver, new NOT_CLOSED_WORLD exception or safety promotion.

Controls: v1 canonical golden bytes/digests; v2 total-or-reject wire/retained-source
replay; missing/altered/reordered/duplicate atoms/targets/context; inclusion and
formal renaming; exact legacy module/function premise vectors and all-row runtime
classification; actual direct and summary consultation closure with unrelated
rows and independently held contexts; refuted refinement while legacy stays
Open; all-supported refinement also leaves legacy Open. Lean matches separate
attachment/erasure/composition/context/impact semantics with audited conditional
noninterference, foreign-context refusal and exact dependency selection results.
Full standing gate, post-committed-matrix Python and conditional all-five-provider
gate are mandatory. All failures and actual commands/exits/counts/logs retained.
Heavy jobs wait for root's merged-base gate to release the shared flock.
