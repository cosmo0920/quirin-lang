# Explicit guarded contract decomposition

A separate `guarded_decomposition_v1` readable document layer now implements the
requested four spellings with explicit typed roles and lowering. It preserves
existing flow-v1 conjunction/positive-NOT/disjunction/terminal semantics and
lifecycle-v1 symbolic introduction/retirement. The source grammar and exact
bounds are [C04.18.5](../../QCORE_CONTRACT_SPEC_r0_3.md#c04185-explicit-guarded-decomposition-documents-version-1)
and [G05-guarded](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05-guarded-standalone-readable-guarded-decomposition).

The [actual cJSON application](applications/cjson.qgd) uses the exact requested
readability: environment and ten independent prerequisites before admission,
positive invalid membership forbidding admission, two declared cases, and
separate remaining-list/unchanged-original terminal conditions. Its nine original
representation sentences remain byte-equal to the production hypothesis graph.
The [actual plugin application](applications/plugin.qgd) retains all seven
`rd_kafka_dump0` callback-closure hypotheses, makes registered/default cases
explicit, and requires their code/context/unload lifetime conditions separately.
Together the two sidecars have31 typed symbols and10 obligations. Both exact
production modules, whole expected graphs and proposition maps are replayed;
source/model references and hashes are in the adjacent JSON files.

Guard-left `|->` lowers to **admission implies all required entry facts**.
Supported proposed admission with an Open prerequisite stays Open; a Refuted
prerequisite refutes that proposed admission. Entry facts do not execute an
operation. `!->` requires positive invalidity and positively Refuted admission;
missing/failed evidence never establishes negation. `||->` retains every case
and dependency, and `->|` evaluates every terminal separately: supporting one
case cannot hide an unresolved or refuted other terminal. Inactive is a separate
obligation status, never a safety claim.

The typed [Rust API](../../../crates/qcore-contract/src/guarded_decomposition.rs)
returns a sealed replayed model, proposed obligation statuses, and conservative
same-context recheck impact. Errors in an environment/prerequisite propagate
through admission to every declared case/postcondition. A terminal error never
establishes entry prerequisites. Full nine-coordinate Context, graph, source,
readable-document and complete proposition-map identities are bound; invalid
assignments in an unchosen alternative still refuse. No impact operation assigns
truth or updates production evidence/verdicts.

[Audited mathematical definitions](../../../proof/QCore/GuardedContractApplications.lean)
make the limits concrete. The cJSON raw early-return guard is **not** a complete
membership test: a nonnull `prev` defeats its partial rejection, so invalid
membership is an independent caller admission obligation. The plugin candidate
premise decomposes into registered/default cases, and terminal lifetime needs
its independent code/context and unload-order premises. These are conditional
model facts. [Actual source observations](source-observations.json) bind three
files byte-exactly to revision `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`;
source correspondence, trace completeness and universal prose truth remain Open.

Focused final validation passes8 Rust tests,17 Lean theorem axiom checks,
formatting and focused Clippy, all exit0. [Exact commands/exits/private logs](focused-commands.json)
retain development failures too: the added terminal-domain Lean theorem first
failed with unused simplification arguments and then an incomplete Boolean proof;
explicit Boolean rewriting repairs it without weakening the invariant. Earlier
focused7/8-test runs and successful intermediate Lean builds retain their original
scopes. No own full standing/native campaign or provider-package gate is claimed.
The integrator runs the agreed frozen joint standing/native campaign; this card
is integration-ready and full acceptance remains pending that evidence.

Production provider source, pins, graph identities, binding manifests,
interpreted clauses, classification, standing and verdicts are unchanged. These
standalone sidecars are concrete proposed contract decompositions, not inline
production adoption. Inline adoption must deliberately extend source loaders and
the sealed whole-source metadata bridge and maintain reviewed pins. No source
universal, checked production semantic/lifecycle consumer, typed truth registry,
allocation/release, automatic evidence invalidation or outer truth is discharged.

## Integrator list

Apply the exact [integration amendments](integration-amendments.json): one
increment, current status/index, and append to the existing
`PREMISE-FLOW-PRODUCTION-DISCHARGE` follow-up without retiring any original source,
prose, semantic-consumer or world/lifecycle criterion. Run and publish the shared
joint full gate/full-record native preservation evidence, then final Python after
the committed regenerated matrix. This card never advances main.
