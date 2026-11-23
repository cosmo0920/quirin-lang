# SP10-FORMAL-BINDING-DESCRIPTIONS

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch both" (the two SP7 follow-up cards).

Base origin/main `2c60986b0`. Prediction: [PREDICTION.md](PREDICTION.md)
(committed first, `28fee38c5`). The card discharges
`SP7-FORMAL-BINDING-DESCRIPTIONS`.

## 1. Census

What was counted: every English environment description (a binding
manifest's or flow model's `environment.variables` value) and every English
`FormalBinding` field (`quantifiers`, `conclusion`), in shipped data and in
code, at base and after. A value is English when it is not a content digest,
a typed term list, a canonical JSON parameter or formula, or a Lean name.
Script: `census2.py` (session scratch; the counts below are its output).

**Live manifests.** The 167 manifests that provider modules pin
(`// qcore-premise-binding-manifest-v1`) sit under `docs/reports/`, but
they are production inputs, so they are counted here.

| Where | Pinned by | Base | After |
|---|---|---|---|
| `sp2-symbolic-librdkafka-premises/bindings` (166 `rk_plugin_callback_closure`) | `client.qv` 89, `null_deref.qv` 71, `logging.qv` 3, `kafka_mem.qv`, `memory.qv`, `queue_destructor.qv` 1 each | 1,826 English values (11 per manifest), 13 distinct | 0: replaced by this report's `bindings/` |
| `sp6-symbolic-hypothesis-atoms/bindings` (1 `detach_list_representation`) | `cjson.qv` 1 | 9 English values, 9 distinct | 0 |
| quantifiers and conclusions of those 167 | | already typed JSON (`quant_binding_migration`) | unchanged |

Distinct live descriptions: 22. Ten plugin components are shared by all 166
plugin manifests; `property_view` has three variants (resource 94, null 71,
model 1); the detach environment has nine.

**Code.** Code outside `docs/reports/` that builds an environment or a
binding.

| File | Role | Base | After |
|---|---|---|---|
| `crates/quirin-source-analysis/src/cjson_local_binding.rs` | production (sealed native flow) | env: 3 English (`entry_heap`, `case_domain`, `external_boundaries`), 2 non-digest identities (`prerequisites` joined, `context` JSON); 1 English quantifier; conclusion a Lean name | env: 3 typed references, 3 digests (`native_observation_sha256`, `prerequisites_sha256`, `context_sha256`); bindings derived |
| `crates/quirin-source-analysis/src/plugin_source_witnesses.rs` | production (sealed bounded flow) | env: 6 digests, 2 slug coordinates (`scope`, `assumptions`); 2 English quantifiers, 4 English conclusions | env: 8 digests (`scope_sha256`, `assumptions_sha256`); bindings derived |
| `crates/qcore-contract/tests/production_symbolic_adoption.rs` | test (libc flow models) | 1 English (`site`); quantifiers and conclusions copied from the frozen `mapping.json` English | typed reference; bindings derived |
| `crates/qcore-contract/tests/plugin_premise_decomposition.rs` | test fixture | 1 English env value, 1 English quantifier, conclusion `"loadedArtifacts e"` | typed reference; bindings derived |
| `crates/qcore-contract/examples/support/premise_flow_fixture.rs` | example and test fixture | 1 English env value, 2 `format!` English fields | typed reference; bindings derived |
| `crates/qcore-contract/examples/plugin_binding_catalog.rs` | historical producer | 1 `format!` English `property_view`, 1 English quantifier | the view's SP10 term; bindings derived (§6) |
| `crates/qcore-contract/tests/formal_binding_descriptions.rs` (new), `crates/qcore-contract/src/formal_binding.rs` (vectors) | refusal cases | — | English only as refusal cases |

**Frozen, not migrated.** Historical manifests that no provider pins and no
Rust reads stay as their reports hold them: `plugin-premise-decomposition`
(168 manifests, 1,846 English values; quantifiers and conclusions English),
`quant-contract-hypotheses` (166, 1,826), `premise-evidence-contracts` (2
flow models, 18), the superseded live copies under
`sp2-symbolic-librdkafka-premises/bindings` and
`sp6-symbolic-hypothesis-atoms/bindings`, and
`plugin-premise-source-witnesses` (8 English quantifiers and conclusions in
recorded outputs). They are refused as they stand (`QF005`); the live ones'
predecessors are lifted by the legacy reader in tests (§5).

Out of scope, recorded: `cjson_local_binding`'s native-domain receipt holds
English `*_interpretation` strings. It is a source-check receipt, not an
environment or a binding.

The size was 22 live descriptions and 9 code sites, within the brief's
medium, so the card did not stop for a split.

## 2. Representation, and why

**An environment component is a typed environment reference**: the
canonical JSON list of G01p parameters `{"name","sort"}`, nonempty and
strictly increasing by name. Each term is a parameter of a bound typed atom
in the same sort, or a registered term in its rendered sort (the C04.18.9
vocabulary and the SP6, SP7 and SP10 blocks,
`premise_formula::rendered_parameter_sort`). A component that binds exact
bytes held elsewhere is their content digest. Nothing else is admitted.

Why this and not a typed proposition id:

* **A description is not a condition.** The ledger entry says so, and the
  manifests agree: each atom's binding already carries its condition (its
  conclusion is the atom's formula, its quantifiers the atom's parameters,
  both typed since `quant_binding_migration`). A description says what an
  environment component *consists of*. A list of typed terms says exactly
  that and asserts nothing more. A typed proposition per component would
  turn "completeness Open" into a stated obligation, which asserts more than
  the prose.
* **It is SP6's encoding.** The manifests already state typed atom
  parameters as `{"name","sort"}` JSON. The reference reuses it, so one
  reader checks both, and each term resolves mechanically against the bound
  atoms or the registered table.
* **"Open" is SP7's E2.** Every retired description that said a
  correspondence is Open now names a never-derived assumption term: SP7's
  `#source_corresponds` for the cJSON detach environment it was defined for
  (the same environment), and the new `#binding_source_corresponds` for the
  others. This is the user's environment E2 decision of 2026-11-23 applied
  to the same kind of statement.
* **Digests for identities.** The code-built environments also bound exact
  values that are not descriptions (a held Context, prerequisite names,
  bounds and assumptions coordinates). Their bytes are bound by digest:
  the binding is exactly as strong as before, the model digest still
  depends on those bytes, and the readable value stays where it lives (the
  held Context, the source constants).

**A formal binding is derived from its atom.** Its proposition is a qualified
name (a Lean definition or a recorded proposition id), and its quantifiers and conclusion are exactly the bound
typed atom's parameters and formula, each canonical JSON. This is what every
live manifest already stated; the code-built bindings now state it too.

**Pure labels.** `context`, `property_view` and `source_binding` /
`source_mapping` are not components of the Lean environment. Each became a
registered SP10 or SP7 term (§4) rather than being dropped: dropping one
would describe the environment more narrowly than the prose, so it was not
a defensible alternative, and the brief's stop condition (two defensible
handlings differing in what they assert) did not arise.

## 3. Faithfulness

One row per distinct description. "Kept by" lists where each part of the
prose is carried. Terms in **bold** are SP10 terms; the others are the bound
atoms' parameters (plugin: `QCore.PluginPremiseDecomposition`; detach: the
SP6 registered terms over `QCore.CjsonRepresentationPremises`, read through
`detachModel`).

**The plugin environment** (`plugin_selected_loader_trace_v1_{resource,null,model}_v1`, 166 manifests):

| Variable | Prose | Reference | Kept by |
|---|---|---|---|
| `artifact_build_symbols_abi` | Exact expected and actual artifact bytes, selected build/configuration, symbol addresses, descriptor/function ABI records | `#actual_abi`, `#actual_artifact`, `#actual_build`, `#actual_inventory`, `#expected_abi`, `#expected_address`, `#expected_artifact`, `#expected_build`, `#expected_inventory`, `#loader_enabled`, `#required_symbol`, `#resolved_address` | artifact bytes: `actual/expected_artifact`; build/configuration: `actual/expected_build`, `actual/expected_inventory`, `loader_enabled`; symbol addresses: `required_symbol`, `resolved_address`, `expected_address`; ABI records: `actual/expected_abi` |
| `cjson` | InitHooks/global hook/link write events, well-formed lists at APIs, preexisting allocator hook validity | **`#cjson_call`**, **`#cjson_link_write`**, **`#hook_install`**, **`#hook_replace`**, **`#preexisting_hooks_valid`**, **`#proper_list`** | InitHooks/global hook events: `hook_install`, `hook_replace`; link writes: `cjson_link_write`; well-formed lists at APIs: `cjson_call` with `proper_list`; hook validity: `preexisting_hooks_valid`. No bound atom quantifies these (they enter through `rk_proposed`/`fluent_proposed`), so they are SP10 terms over the same Lean model |
| `context` | Independently held nine-coordinate property-specific Context; no custody/null context transport | **`#held_context`** | the term's meaning (`HypothesisGraph.Context`, held independently); no transport: exact Context equality in every evaluation and `foreign_property_is_not_selected` |
| `custody` | Interceptor-descendant flag, pointer/scalar borrowing, local acquisitions and matched drops before activation return, no retain/transfer/global custody | `#acquire`, `#activation`, `#global_custody`, `#in_interceptor`, `#local_resource`, `#pointer_borrowed`, `#release`, `#resource`, `#retain`, `#return_boundary`, `#return_local`, `#scalar_borrowed`, `#transfer` | flag: `in_interceptor`; borrowing: `pointer/scalar_borrowed`; acquisitions and matched drops: `acquire`, `release`, `return_local`, `resource`, `local_resource`; before activation return: `activation`, `return_boundary`; "no retain/transfer/global custody": the custody atom's formula over `retain`, `transfer`, `global_custody` (the component names them; the condition stays the atom's) |
| `frames` | Loaded-code core/internal writes, public API preconditions, handed-object access and pointer-forgery events | **`#access`**, **`#api_call`**, **`#api_precondition`**, **`#core_write`**, **`#exported_api`**, **`#forge`**, **`#forged`**, **`#handed_object`**, **`#internal_write`**, **`#is_rk_call`**, **`#public_rk_symbol`** | core/internal writes: `core_write`, `internal_write`; public API preconditions: `api_call`, `is_rk_call`, `public_rk_symbol`, `api_precondition`; handed-object access: `access`, `handed_object`, `exported_api`; pointer forgery: `forge`, `forged`. SP10 terms, as for `cjson` |
| `lifetime` | Code and opaque-context liveness at each callback including destruction; loader handle references/copies and final close order | `#close`, `#code_live`, `#context_live`, `#copy_reference`, `#destroy_callback`, `#handle`, `#handle_for`, `#invoke`, `#opaque`, `#ref_count` | liveness: `code_live`, `context_live`, `opaque`; each callback including destruction: `invoke`, `destroy_callback`; handle references/copies: `handle`, `handle_for`, `ref_count`, `copy_reference`; final close order: `close` (the order is the unload atom's formula) |
| `plugins` | Actual loaded artifact identities and RK/FluentC/FluentGo loader families, including transitive loaded calls | `#fluent_c`, `#fluent_go`, `#fluent_proposed`, `#loaded`, `#plugin`, `#rk`, `#rk_proposed` | loaded artifacts and families: `loaded`, `rk`, `fluent_c`, `fluent_go`; transitive loaded calls: `plugin` (an event's attribution, transitive calls included); the families' obligations: `rk_proposed`, `fluent_proposed`; "actual": `#binding_source_corresponds` |
| `property_view` (3) | Exact independently held {resource, null, model} source view; no cross-property Context transport | **`#resource_view`** / **`#null_view`** / **`#model_view`** | the term's meaning; no transport: as `context` |
| `registration` | Successful public setters, exact member candidates, list/union-arm/signature candidate inventory and phase/share state | `#conf_live`, `#config_registration`, `#exact_candidate`, `#from_on_new`, `#function`, `#initialized`, `#intercept`, `#member`, `#member_candidate`, `#public_setter`, `#register`, `#result`, `#share_consumer`, `#static_default` | successful public setters: `register`, `intercept`, `public_setter`, `result`; exact member candidates: `exact_candidate`, `member`, `function`, `member_candidate`, `static_default` (the candidate inventory); phase/share state: `config_registration`, `conf_live`, `from_on_new`, `initialized`, `share_consumer` |
| `source_binding` | C/model and English/formal correspondence remain Open; hashes bind references only | **`#binding_source_corresponds`** | Open: the term is never derived (`binding_source_correspondence_is_never_derived`); hashes bind references only: the resolver's reference-integrity semantics (C04.18.2) and `source_binding: "open"` |
| `trace` | Ordered bounded complete interleaved events, with plugin actor/activation/member/opaque/handle/resource identities; completeness Open | `#action`, `#activation`, `#env`, `#handle`, `#in_trace`, `#member`, `#opaque`, `#plugin`, `#resource`, `#time` | events: `in_trace`, `action`, `env`; ordered: `time`; identities: `plugin`, `activation`, `member`, `opaque`, `handle`, `resource`; "bounded complete interleaved" is what `in_trace` denotes, and that the denotation is the actual trace ("completeness Open") is `#binding_source_corresponds` in `source_binding` |

**The detach environment** (`cjson_detach_entry_and_interference_v1`, 1 manifest):

| Variable | Prose | Reference | Kept by |
|---|---|---|---|
| `callActor` | Exact detach invocation actor, source-to-Nat interpretation Open | `#concurrent`, `#runs` | `detachModel`: `runs f s` reads the trace steps whose actor differs from `callActor`, `concurrent` the other actors; interpretation Open: `#source_corresponds` |
| `child_next_prev` | Entry heap maps, all relational equations explicitly assumed | `#child`, `#next`, `#next_reach`, `#prev` | the maps: `child`, `next`, `prev`, `next_reach`; the equations: the separately bound `detach_*` atoms (each an open entry obligation), not restated |
| `children` | Finite exact owned-child sequence at call entry, distinctness assumed separately | `#children`, `#first`, `#last`, `#nodup`, `#pred` | the sequence and its projections; distinctness: the separate `detach_distinct` atom over `nodup` |
| `context` | All nine independently held … coordinates | **`#held_context`** | as the plugin `context` |
| `live` | Entry liveness predicate, source interpretation Open | `#live` | `live`; Open: `#source_corresponds` |
| `ownedChild` | Exact custody predicate at entry, source interpretation Open | `#owned` | `owned` (`detachModel`: `ownedChild`); Open: `#source_corresponds` |
| `parent` | Call actual parent identity and entry liveness, source interpretation Open | `#live`, `#parent` | identity: `parent`; entry liveness: `live`; Open: `#source_corresponds` |
| `source_mapping` | Source/model and prose/formal equivalence Open; hashes prove reference identity only | `#source_corresponds` (SP7) | the user's E2 term for this environment, never derived (`source_corresponds_is_never_derived`); hashes: reference integrity (C04.18.2) |
| `trace` | Every interleaved step during call within exact bounds, with actor and link-write fields; source completeness Open | `#concurrent`, `#keeps_links`, `#runs` | steps and actor: `runs`, `concurrent`; link-write field: `keeps_links` (`writesLinks = false`); completeness Open: `#source_corresponds` |

Every plugin atom parameter (60) and every detach atom parameter (15)
belongs to some component (`plugin_parameters_are_covered`,
`detach_parameters_are_covered`).

**The code-built environments:**

| Site | Variable | Prose | Now |
|---|---|---|---|
| `cjson_local_binding` | `entry_heap` | Complete live native allocation identities and exact child/next/prev snapshots; finite sequence membership defines local domain only | the bound atoms' `#recorded`, `#post_comparison`, `#holds` and the eight `#detach_*` snapshot predicates |
| | `case_domain` | Empty plus lengths1..4 all valid detach indices;11case keys | `#recorded_count`, `#required_count` (the exact 11-key domain is the observed domain the receipt checks, unchanged) |
| | `external_boundaries` | Native execution/compiler/allocator correspondence is observed or assumed; arbitrary custody/writers and universal atoms remain Open | **`#binding_source_corresponds`** |
| | `prerequisites`, `context` | joined prerequisite names; the held Context JSON | `prerequisites_sha256`, `context_sha256`: digests of the same bytes |
| | quantifier | Every entry/post snapshot in exactly the independently observed11case finite domain | the atom's parameters |
| `plugin_source_witnesses` | `scope`, `assumptions` | the bounds and assumptions coordinate slugs | `scope_sha256`, `assumptions_sha256` |
| | quantifiers (2), conclusions (4) | `trace : List String; selected = …`, `DuplicateNoLoaderCalls trace for this selected invocation only; …` | the atoms' parameters and formulas; "this selected invocation only" stays the evaluation's held Context and selection |
| libc flow models (test) | `site` | arbitrary reading site in the original conditional ReadingEnvironment/NestedEnvironment; source binding Open | `#env`, `#observed`, **`#binding_source_corresponds`** |
| plugin fixture (test) | `context` | All nine exact property-specific caller-held coordinates; source binding Open | **`#held_context`**, **`#binding_source_corresponds`** |
| flow fixture | `valuation` | three assigned propositions; no C/source truth | `#children`, `#live`, `#nodup`, `#parent`, **`#binding_source_corresponds`** |

## 4. The SP10 block

22 typed-rendering terms, appended after SP7's in `RENDERED_TERMS`
(`block: "SP10"`), the specification table (C04.18.9) and the Lean
`sp10Terms`, bound by `the_sp10_block_is_new_typed_and_bound_to_lean`. None
is a registered term and none is in another block. No premise formula names
one, so `VOCABULARY`, the premise grammar, the decision vectors and Lean
`vocab` are unchanged. `RenderedSort` gains `Function`, for the event flags.

| Term | Sort | Lean meaning (`denote`) |
|---|---|---|
| `#binding_source_corresponds` | predicate<> | a free proposition, never derived (E2's form) |
| `#held_context` | verdict_context | `HypothesisGraph.Context` |
| `#resource_view`, `#null_view`, `#model_view` | property_view | `PropertyView.resource`, `.null`, `.model` |
| `#hook_install`, `#hook_replace`, `#cjson_link_write`, `#cjson_call` | action | `Action.hookInstall`, `.hookReplace`, `.cjsonLinkWrite`, `.cjsonCall` |
| `#proper_list` | function<environment,event,integer> | `Event.properList` as 1/0 (`flagValue`) |
| `#preexisting_hooks_valid` | predicate<environment,plugin> | `Environment.preexistingHooksValid` |
| `#core_write`, `#internal_write`, `#api_call`, `#access`, `#forge` | action | `Action.coreWrite`, `.internalWrite`, `.apiCall`, `.access`, `.forge` |
| `#is_rk_call`, `#public_rk_symbol`, `#api_precondition`, `#handed_object`, `#exported_api`, `#forged` | function<environment,event,integer> | the `Event` flags of the same names as 1/0 |

The meanings are in `PluginPremiseDecomposition`'s own model, the one the
plugin bindings reference.

## 5. `QF005`, where it fires, and the legacy reader

**`QF005`** is the next free id: `QF001`–`QF004` are taken (checked across
the tree; card SP9 runs concurrently without a `QF` id of its own). The
checker is the new module `qcore_contract::formal_binding`
(`PROSE_BINDING`, `check_environment`, `check_binding`, `check_model`). It
fires on:

* **The manifest resolver**: `hypothesis_bindings::resolve`,
  `resolve_bounds` and `resolve_model` all reach `resolve_replayed`, which
  checks every manifest after its domain check and before its references.
* **The flow compiler**: `premise_flow::compile`, `compile_bounds` and
  `compile_model` all reach `check_model`, which checks every model after
  its domain check.

It refuses, with a message starting `QF005`: an environment value that is
English or any text that is neither a digest nor a typed term list; a term
list that is empty, unsorted, duplicated or not byte-canonical; a term that
is neither a bound atom's parameter nor a registered term in that sort; an
environment id or variable name that is not an identifier; a binding whose
proposition is not a qualified name, whose atom has no typed definition, or whose
quantifiers or conclusion are not the atom's parameters and formula. Each
refusal is fail-closed: the manifest or model does not resolve, nothing is
published from it, and no verdict reads it in any case.

Manifests are JSON, not contract source, so the parser and the compiler
route are not involved and no `Q000` path exists.

**The legacy reader.** `tests/support/legacy_prose_premise.rs` (same file,
same confinement test) gains `environment_descriptions` and
`lift_environment`: a frozen environment's recorded descriptions
([migration.json](migration.json) `descriptions`) are written as the typed
references that replaced them, and nothing else. An unrecorded description
stays English and is refused. `qcore_contract::formal_binding` does not name
the reader (the confinement test checks every file that does).

**The checkpoint restorer.** `tests/support/sp2_premise_checkpoint.rs` writes
this card's 167 marker edits back first ([migration.json](migration.json)
`files`, both module digests per file), then SP7's, SP6's, SP5's and SP2's,
so every frozen checkpoint comparison still recovers its exact bytes.

## 6. Migration and digests

`premise_formula_binding_refresh --environment-descriptions` (the SP2 tool,
extended) rewrote the 167 live manifests:

```text
premise_formula_binding_refresh --environment-descriptions docs/reports/sp10-formal-binding-descriptions/migration.json . docs/reports/sp6-symbolic-hypothesis-atoms/bindings,docs/reports/sp2-symbolic-librdkafka-premises/bindings <out> providers/qcore-provider-librdkafka/contracts/librdkafka/{client,null_deref,logging,kafka_mem,memory,cjson,queue_destructor}.qv
```

`private/logs/refresh-1.log`, exit 0: "167 open binding references
refreshed; 1835 environment descriptions rewritten". The tool admits only
that change: each old manifest is read by its content address and must be
canonical, every value must be a recorded description, the rewritten
manifest must equal the old one with the descriptions restored, every model
and audit identity must be unchanged, and the patched module must resolve
through its property view's resolver (so through `QF005`). Graph digests,
atom digests, bindings, environment ids and variable names are unchanged.

* The new manifests and [index.json](bindings/index.json) (previous and new
  manifest digest per occurrence) are in [bindings/](bindings/). The old
  ones stay in their reports.
* The detach manifest `27c07365…` became `2c620459…`. Its consumers were
  repointed: `crates/qcore-contract/tests/hypothesis_bindings.rs`,
  `crates/qcore-audit/tests/contract_dependencies_report.rs` (two sites),
  `production_symbolic_adoption.rs` (directory list, this report first) and
  `scripts/gate_scope_walkers.json` (a walker for this report's
  `bindings/`).
* The seven modules' marker lines are the only change to provider source.
  [migration.json](migration.json) `files` records both digests of each.
* `plugin_binding_catalog` (the historical producer of the first plugin
  catalog) states the view's SP10 term and derived bindings. Its design
  input is a frozen report holding English components, and its inputs state
  string atoms that the format has refused since SP6, so its check mode
  refuses; it is a record of how the first catalog was built, not a live
  tool. `quant_binding_migration` copies environments from the manifests it
  migrates and resolves the result, so an English input is now refused
  there too.

## 7. Lean

`QCore.FormalBindingEnvironments` (audit
`AuditFormalBindingEnvironments.lean`, 14 theorems, placeholder-free,
standard axioms only: `propext` in four):

* `sp10Terms`, bound to the Rust block; `denote` gives each SP10 term its
  meaning (`sp10_terms_denote`; `unregistered_term_denotes_nothing`,
  `proper_list_flag`, `forged_flag`, `frame_terms_are_model_actions`).
* `binding_source_correspondence_is_never_derived`: for every environment
  some interpretation fails the correspondence.
* `references` (the 22 recorded references), `pluginParameters` and
  `detachParameters`, bound to the record and the live manifests
  (`the_recorded_references_are_the_lean_table`); `every_reference_resolves`
  (each term an atom parameter, an SP10 term or an SP7 term),
  `plugin_parameters_are_covered`, `detach_parameters_are_covered`.
* The ban: `EnvValue`, `admitEnv`, `text_is_not_an_environment_reference`,
  `environment_admitted_iff`, `environmentVectors` bound to the Rust
  `ENVIRONMENT_DECISION_VECTORS` (`environment_vectors_agree`), and
  `text_is_not_a_binding_field`, `binding_admitted_iff`.

`lake build QCore.FormalBindingEnvironments` and `lake env lean
AuditFormalBindingEnvironments.lean` passed (logged in the gate's Lean step).

## 8. Semantics and prediction

Binding manifests and flow models are evidence-only metadata (C04.18.2,
C04.18.3): no verdict, standing, certificate or classifier reads an
environment description or a formal binding field. The graphs, atoms,
premises, erasures, interpreted clauses and declarations' contract digests
are unchanged. [PREDICTION.md](PREDICTION.md) predicts 0 status moves on
corpus, twelve-home, system and slice, with digest deltas only: the 167
manifest digests, the seven module digests (and so the inventory lock and
`contracts.json`), and the two sealed adapters' flow model and document
digests. The prediction also listed re-recorded `contract_sha256` entries;
none changed (§9), which is a smaller delta than predicted, not a move.

## 9. Locks, reviewed

`python3 providers/contracts.py sync` (`private/logs/sync-1.log`) exited 0
with no stale-lock refusal. A binding marker is a module comment outside every
declaration's contract text, so no declaration's `contract_sha256` changed
and no `quirin-audit pin` step was needed (the prediction allowed for one;
none was taken). The diff, reviewed:

* `providers/qcore-provider-librdkafka/contract-inventory.lock`: the seven
  module digests (`cjson.qv`, `client.qv`, `kafka_mem.qv`, `logging.qv`,
  `memory.qv`, `null_deref.qv`, `queue_destructor.qv`) and the 167 premise
  manifest references (old digest to new, matching
  [index.json](bindings/index.json)); 174 changed lines each way, nothing
  else. Declaration and clause counts unchanged.
* `providers/qcore-provider-librdkafka/contracts.json`: the same seven file
  digests.
* `contracts.lock`, `bounds-contracts.lock`, `outer-contracts.lock`,
  `providers/contract-load-path.json`: unchanged.

`python3 providers/contracts.py check`: `private/logs/check-1.log`.

## 10. Gate

**Gate 1**: `python3 scripts/gate_scope.py --base 2c60986b0 --out
private/gate-1` on `938c85fbc` (the full suite: `proof/QCore.lean` and
`qcore-frontend` changed). Console `private/logs/gate-1.console`, step logs
and receipts under `private/gate-1/`.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| rust-suite | 101 | 681 units, 726 summaries, 9979 passed, **1 failed**, 22 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 492 audits (this card's included) |
| python | 1 | 807 tests, **2 failures** |
| provider package build, verify (all, librdkafka) | 0, 0, 0 | |
| provider test | not run at hand-back | queued in the host's shared audit queue (ticket 338, first in line, waiting for slots `audit/0` and `audit/1`) behind a long-running measurement; the queued step was left to complete |

The failures:

* `qcore-audit::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`:
  the recorded host failure (system `curlver.h`). Not fixed.
* `test_gate_scope_census.CensusAgreesWithTheModel.test_every_directory_literal_is_declared`:
  a real omission. The new test reads three binding directories that
  `scripts/gate_scope_walkers.json` did not declare. Fixed in `437b369a6`
  (three walker entries; the test's root helper in the census-readable
  form); `python3 -m unittest test_gate_scope_census` then ran 5 tests, OK.
* `test_gate_slot_lane.Rule.test_the_lane_has_a_fixed_number_of_slots_and_its_own_arrival_order`:
  a gate-slot timing test (arrival order `p2, p1` under load). Rerun alone:
  `python3 -m unittest test_gate_slot_lane`, 37 tests, OK. Both results are
  recorded here.

Earlier scoped runs: `private/logs/t1.log` (11 qcore-contract test files;
every one passed except `production_symbolic_adoption`, whose libc flow
models named proposition ids such as
`libc.__errno_location.nested_call_admission.checked_reading_route.v1` that
the first proposition check refused as not Lean names; the check now admits
any dot-qualified name without whitespace, committed before gate 1, and the
gate's rust-suite passed the file); `private/logs/t2.log` (the card test
after the walker fix, 9 passed).

**After merging origin/main `e6d86d596`** (r62, CAST-BEFORE-STAR: gateway
code, rule 74g, a Lean module, records; disjoint from this card's files
except generated docs, merged cleanly, tip `209195bd6`):
`gen_lean_roots.py --check` 0 (549 modules), `gen_rule_docs.py --check` 0,
`apply_amendments.py … --check` 0, and `cargo test -p qcore-contract --test
formal_binding_descriptions --test hypothesis_bindings --test premise_flow
--test contract_premise` 0 (`private/logs/t3.log`, 31 passed).

**Amendments.** `python3 scripts/apply_amendments.py
integration-amendments.json --check` exits 0: it retires
`SP7-FORMAL-BINDING-DESCRIPTIONS` and registers two follow-ups. The acceptance
matrix's derived Lean-audit list is left to the landing
(`CARD-GATE-NEW-AUDIT-MATRIX-COUNT`).

## 11. Follow-ups

* `SP10-NATIVE-RECEIPT-INTERPRETATION-PROSE` (registered): the cJSON
  native-domain receipt's English `*_interpretation` strings (not a binding
  environment, so outside `QF005`).
* `SP10-RETIRE-HISTORICAL-BINDING-PRODUCERS` (registered):
  `plugin_binding_catalog` and `quant_binding_migration` now refuse their
  English historical inputs; remove or re-target them.
* Merge with card SP9 (concurrent): its rendered-term block goes before
  SP10's in `RENDERED_TERMS`; a term both blocks need must be stated once
  (the SP10 test refuses a name in two blocks).

**Measurement** is the integrator's step:

```text
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=baseline:<main baseline> --side tip=rev:<tip> --declared docs/reports/sp10-formal-binding-descriptions/PREDICTION.md
```
