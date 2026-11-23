# SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch both".

The card stopped once, on the draft `01864f410`. The census had found that
all 14 symbols of the frozen plugin sidecar are refused, not seven: the
seven `plugin_*` prerequisites, lifted to their typed atoms' formulas, are
not typed renderings either. On 2026-11-23 the user chose:

* **D1 A′**: an SP9 block of rendered terms and value sorts, every name
  prefixed `plg_`, with the environment implicit.
* **D2 b**: `default_lifetime_valid` states code and context liveness only,
  as a new Lean definition proved to follow from `lifetimePostcondition`.
* **D3 I2**: `unregistered` is a free warrant term, as SP7's
  `invalid_membership` is.

The user also accepted the precedent readings: `environment` is an open
assumption, `invoke` a free admission, the cases are read at a designated
invocation `#plg_ev`, and `default_case` does not assert "not registered".

Base origin/main `2c60986b0`. Prediction: [PREDICTION.md](PREDICTION.md),
committed first (`ffc10a4a5`). The card discharges
`SP7-PLUGIN-GUARDED-APPLICATION-FORMULAS`.

## 1. Census

**Pattern.** The 14 English texts of the frozen sidecar
`docs/reports/premise-guarded-decomposition/applications/plugin.json`, each
matched by its first 40 bytes (`git grep -F`, all of the tree except
`third_party/`). The table counts the seven non-prerequisite texts first,
then the seven prerequisite ones.

| File | Role | Base | After |
|---|---|---|---|
| `docs/reports/premise-guarded-decomposition/applications/plugin.json` | the frozen sidecar (bytes unchanged) | 7 + 7 | 7 + 7 (read through the legacy reader, lifted) |
| `docs/reports/sp9-plugin-guarded-application-formulas/migration.json` | this card's record | — | 7 + 7 (each text beside its formula) |
| `crates/qcore-contract/tests/guarded_decomposition.rs` | refusal case | 0 | 1 + 0 (an unrecorded English `invoke` text, refused `QF003`) |
| `compiler/tests/fixtures/contract_inherit_qcore_provider/librdkafka_memory_hand_written.qv` | frozen token fixture: string hypothesis atoms, not guarded symbols (SP6) | 0 + 7 | 0 + 7 (unchanged) |
| other files under `docs/reports/` (29: frozen `before` sources, inventories, expected graphs, measurement records, SP6's record) | the prerequisite sentences as hypothesis atoms of frozen records, not guarded symbols | 0 + 36,799 | unchanged |

**Outside `docs/reports/`.** No production module states any of the 14
texts. The one test match is a deliberate refusal case. The fixture's seven
are string hypothesis atoms, which SP6 reads through the same legacy
reader. No guarded symbol, in any file, states an English text that the
loader admits.

Files the 40-byte pattern cannot see:

* `tests/support/legacy_prose_premise.rs`, which reads the record;
* `crates/qcore-contract/tests/plugin_guarded_symbols.rs`, which builds each
  refusal from the record. Its one literal refusal text,
  `"Admission of every reached callback."`, is shorter than the pattern.

## 2. The 14 formulas

Every proposition is the canonical G01p printing
(`quant_proposition::print`) of a C04.18.9 typed-rendering formula over the
SP9 block. Each one's Lean definition is in `QCore.PluginGuardedSymbols`,
proved equal to its existing definition
(`plugin_guarded_symbols_are_the_definitions`). Disclaimers are scope
limits. Each one is shown kept structurally: by what the formula does not
assert, by the role, or by an assumption that no model derives. None is
dropped or turned into a stronger claim.

### The seven prerequisites: each atom's formula bound to the SP9 terms

**The binding (D1 A′).** Every free name `#X` of the typed atom becomes
`#plg_X`, with its first argument `#env` dropped. A function left with no
argument becomes a value: `#actual_inventory(#env)` is
`#plg_actual_inventory`. A quantifier sort `S` becomes `plg_S`. Bound
variables keep their names.

**Where it is checked.**

* Rust checks the binding syntactically on the parsed trees
  (`each_prerequisite_is_its_production_atom_bound_to_the_sp9_terms`). The
  atom checked is the SP6 record's formula, which the production
  `kafka_mem.qv` states.
* The same test checks each atom parameter's sort against its SP9 term's.
* Lean proves each bound formula equal to the atom's existing definition in
  `QCore.PluginPremiseDecomposition`.

The prerequisites' faithfulness to their sentences is QUANT-CONTRACT-HYPOTHESES
and SP6's. This card changes only the names.

| Symbol | Lean theorem | Existing definition |
|---|---|---|
| `plugin_artifact_symbol_abi` | `artifact_symbol_abi_holds` | `loadedArtifacts` (`loaderEnvironment ∧ artifactSymbolAbi` of every loaded artifact) |
| `plugin_callback_candidates` | `callback_candidates_holds` | `callbackCandidates` |
| `plugin_code_context_lifetime` | `code_context_lifetime_holds` | `loadedLifetime` |
| `plugin_interceptor_custody` | `interceptor_custody_holds` | `loadedCustody` |
| `plugin_interceptor_registration` | `interceptor_registration_holds` | `loadedRegistration` (`interceptorCandidates ∧ registrationLifecycle`) |
| `plugin_lifecycle_unload` | `lifecycle_unload_holds` | `loadedUnload` |
| `plugin_outer_bridge` | `outer_bridge_holds` | `outerBridge` (`RkProposed`, `FluentProposed`) |

Together they are `CallbackRefinement` (`prerequisites_are_the_callback_refinement`).

### The seven symbols that were English

**`environment`** (environment, E2-shaped): `#plg_trace_corresponds()`. Lean: `g.traceCorresponds`, a free proposition.

| Prose | Kept by |
|---|---|
| "Exact selected PluginPremiseDecomposition.Environment" | the term's meaning, and every SP9 term is read in one selected `Environment` (`GuardedInvocation.env`) |
| "including every loaded RK and Fluent Bit C/Go artifact, every callback event, build/ABI/inventory and full context" | `#plg_trace_corresponds`'s meaning names each of them |
| "source trace completeness remains Open" | an independent assumption: no theorem derives it, and every environment has a model in which it fails (`trace_corresponds_is_never_derived`). Its disposition is never Derived |

**`invoke`** (operation): `#plg_admits_callback_closure()`. Lean: `g.admitsCallbackClosure`, free.

| Prose | Kept by |
|---|---|
| "Admission of every reached rk_conf callback invocation under the selected rd_kafka_dump0 callback-closure premise" | the term's meaning (the gateway admits every reached invocation under `rk_plugin_callback_closure`); the operation role (C04.18.5: a proposed admission) |
| "not permission inferred from a sampled callback" | the term ranges over every reached invocation, and no model derives it (`admission_is_never_derived`); the guard `(environment ∧ …) \|-> invoke` is a necessary entry obligation only |

**`registered_case`** (case): `(#plg_member_candidate(#plg_member(#plg_ev),#plg_function(#plg_ev)) && exists (#reg: plg_event) { ((((((#plg_in_trace(#reg) && #plg_action(#reg) == #plg_register) && #plg_function(#reg) == #plg_function(#plg_ev)) && #plg_member(#reg) == #plg_member(#plg_ev)) && #plg_time(#reg) < #plg_time(#plg_ev)) && #plg_result(#reg) == 0) && #plg_public_setter(#reg) == 1) })`. Lean: `registeredCase g.env g.ev` (`registered_case_holds`).

| Prose | Formula part |
|---|---|
| "The actual exact member candidate" | `#plg_member_candidate(#plg_member(#plg_ev),#plg_function(#plg_ev))`, at the reached invocation `#plg_ev` |
| "was registered by a prior successful public registration" | `exists (#reg: plg_event)`: a `#plg_register` event of the same function and member, earlier (`<`), with result `0` and through a public setter (`== 1`) |
| "in this selected trace" | `#plg_in_trace(#reg)` |

**`default_case`** (case): `(#plg_member_candidate(#plg_member(#plg_ev),#plg_function(#plg_ev)) && #plg_static_default(#plg_member(#plg_ev),#plg_function(#plg_ev)))`. Lean: `defaultCase g.env g.ev` (`default_case_holds`).

| Prose | Formula part |
|---|---|
| "The actual exact member candidate is a stated static default in the selected build" | `#plg_member_candidate(…) && #plg_static_default(…)` |
| "this case is independent of the loaded registration case" | the formula asserts nothing about registration; both cases hold of one model (`default_case_is_independent_of_registration`) |

**`registered_lifetime_valid`** (postcondition): code and context live at `#plg_time(#plg_ev)`, and the unload order of `#plg_plugin(#plg_ev)`, which is the `plugin_lifecycle_unload` body at that plugin. Lean: `lifetimePostcondition g.env g.ev` (`registered_lifetime_valid_holds`).

| Prose | Formula part |
|---|---|
| "Every invocation in the registered case" | reached only through `registered_case ->\| registered_lifetime_valid`, read at every reached invocation `#plg_ev` |
| "has live code and opaque context" | `#plg_code_live(#plg_function(#plg_ev),#plg_time(#plg_ev)) && #plg_context_live(#plg_opaque(#plg_ev),#plg_time(#plg_ev))` |
| "and precedes matching final unload, including destruction" | `forall (#stop: plg_event) { … #plg_ref_count(…) == 0 => forall (#call: plg_event) { … (#plg_invoke \|\| #plg_destroy_callback) => #plg_time(#call) < #plg_time(#stop) } }` |
| "and copied registrations" | `forall (#copy: plg_event) { … #plg_copy_reference => 0 < #plg_ref_count(…) }` |

**`default_lifetime_valid`** (postcondition, D2 b): `(#plg_code_live(#plg_function(#plg_ev),#plg_time(#plg_ev)) && #plg_context_live(#plg_opaque(#plg_ev),#plg_time(#plg_ev)))`. Lean: the new `defaultLifetime g.env g.ev` (`default_lifetime_valid_holds`).

| Prose | Kept by |
|---|---|
| "Every invocation in the static-default case" | reached only through `default_case ->\| default_lifetime_valid`, at every reached `#plg_ev` |
| "has live code and opaque context through the last invocation" | the two liveness atoms at the invocation's position, for every reached invocation |
| "no loaded-envelope truth follows" | the formula states no unload order or outer envelope. It is implied by `lifetimePostcondition` (`lifetime_entails_default_lifetime`), but the converse does not hold (`default_lifetime_does_not_give_the_unload_order`) |

**`unregistered`** (condition, D3 I2): `#plg_refutes_registration(#plg_ev)`. Lean: `g.refutesRegistration g.ev` (`unregistered_holds`).

| Prose | Kept by |
|---|---|
| "Positive evidence" | `#plg_refutes_registration` (a warrant term inside the proposition, by user decision); the condition role's C04.18.5 rule (Supported invalidity and positively Refuted admission) is kept too |
| "that a reached callback" | `#plg_ev` is the reached invocation |
| "has neither its required successful public registration nor a stated static default" | the term entails `unregistered` (`¬ registeredCase ∧ ¬ defaultCase`): `unregistered_symbol_entails_unregistered`, `a_case_excludes_refutation` |
| "failed registration proof alone does not establish this" | `unregistered` alone does not give the term (`absence_of_registration_is_not_refutation`), and no model derives it (`registration_refutation_is_never_derived`) |

## 3. Vocabulary: the SP9 block

The SP9 block adds a function sort to the rendering
(`RenderedSort::Function`), 13 value sorts and 63 terms. They are appended
after SP7's entries in `RENDERED_VALUE_SORTS` and `RENDERED_TERMS`
(`block: "SP9"`), in the specification's C04.18.9 table, and in Lean
`sp9Sorts` and `sp9Terms`. Tests bind the Lean lists to the Rust tables
(`the_sp9_block_is_new_prefixed_and_bound_to_lean`).

* Every name is prefixed `plg_`. None is a registered term, and none repeats
  an SP6 or SP7 term.
* The unprefixed names stay unregistered and are refused (`#time`,
  `#action`, `#member`, `#in_trace`, `#env`, `#ev`, `#invoke`).
* No premise formula names an SP9 term. The premise `VOCABULARY`, grammar,
  decision vectors and Lean `vocab` are unchanged, and no `Model` field is
  added.

The terms fall into these groups:

* **Value sorts:** `plg_event`, `plg_plugin`, `plg_family`, `plg_action`,
  `plg_callback`, `plg_member`, `plg_context`, `plg_handle`,
  `plg_activation`, `plg_resource`, `plg_symbol`, `plg_digest`, `plg_abi`.
  There is no `environment` sort: the environment is implicit.
* **The 59 atom parameters other than `#env`,** each prefixed: the event
  components, actions, families and environment components. Their meanings
  are those of `PluginPremiseDecomposition.Event` and `Environment`, listed
  one by one in C04.18.9.
* **The guarded symbols' own terms:**
  * `#plg_ev` (`plg_event`): the reached invocation the cases are read at;
  * `#plg_admits_callback_closure` (`predicate<>`): the admission;
  * `#plg_trace_corresponds` (`predicate<>`): environment, E2-shaped;
  * `#plg_refutes_registration` (`predicate<plg_event>`): D3 I2.

**The Lean model.** `QCore.PluginGuardedSymbols.render` reads one
`GuardedInvocation`: an `Environment`, an `Event`, and the three free
propositions. `refutes_sound` constrains the refutation to entail
`unregistered`. A trace position is `Int.ofNat`, a Boolean flag is `1` or
`0`, and an integer argument is read through `Int.toNat`.

## 4. The legacy reader and the replay

`tests/support/legacy_prose_premise.rs` (the same file, under the same
confinement test) reads [migration.json](migration.json) `symbols`. It holds
the 14 recorded texts, their formulas, and for a prerequisite its atom's
formula.

* `lift_bindings` writes each recorded text as its formula. It checks the
  ID, role and exact text, and reads this record before SP6's: SP6 would
  repeat a prerequisite's unbound atom formula, which is not a rendering.
* An unrecorded English text stays English and is refused with `QF003`.
* The cJSON plan and sidecar are unaffected: their texts differ, and
  `guarded_contract_adoption` still lifts 17.

The plugin replay in
`guarded_decomposition.rs::actual_cjson_and_plugin_hypotheses_lower_without_truth_or_source_changes`
is positive again (`("plugin", 7)`):

* the frozen `kafka_mem.qv` source and the provenance digests are checked
  as before;
* 14 bindings are lifted;
* the document lowers with 2 guards, 1 alternative and 2 terminal
  postconditions, every status Open;
* the graphs and loaded contracts are unchanged;
* each prerequisite is the SP9 binding of its typed atom.

The refusal of an unrecorded English symbol stays a test in the same
function, and in `plugin_guarded_symbols.rs` for each of the 14 texts.

## 5. Lean

`QCore.PluginGuardedSymbols` (audit
`AuditPremiseFormulaPluginGuardedSymbols.lean`, a new file) proves:

* **The 14 symbols:** `plugin_guarded_symbols_are_the_definitions`, with
  `prerequisites_are_the_callback_refinement`.
* **The alternatives:** `candidates_give_a_case`, from the existing
  `candidate_premise_decomposes_both_alternatives`.
* **D2 b:** `lifetime_entails_default_lifetime` and
  `default_lifetime_does_not_give_the_unload_order`.
* **The independence of the cases:** `default_case_is_independent_of_registration`.
* **The assumptions:** `unregistered_symbol_entails_unregistered`,
  `a_case_excludes_refutation`, `trace_corresponds_is_never_derived`,
  `registration_refutation_is_never_derived`, `admission_is_never_derived`
  and `absence_of_registration_is_not_refutation`.

Each Lean `def` is the Rust `lean` rendering of the recorded formula
(`the_symbols_are_the_lean_definitions`). The quantifier lemmas
(`holds_forall_sort`, `holds_exists_sort`) read an SP9-sorted quantifier as
ranging over that sort's values. The proofs are placeholder-free and use
standard axioms only. `GuardedContractApplications.lean` and
`PluginPremiseDecomposition.lean` are unchanged: the sidecar's provenance
digests them.

## 6. Semantics

Guarded symbols are proposed metadata (C04.18.5, C04.18.6): no verdict,
standing, certificate or classifier reads one. No production module states
these symbols. No provider file, contract, lock, binding manifest,
hypothesis graph or frozen sidecar byte changed. `providers/contracts.py`
was not run because no provider file changed. The predicted status moves
are 0 on corpus, twelve-home, system and slice, with no digest delta
([PREDICTION.md](PREDICTION.md)).

## 7. Specification and rules

* **C04.18.9:** the SP9 block (sorts, terms, the 14 symbols, the second
  warrant-term note).
* **C04.18.6:** the SP9 block named in the formula rule, and the plugin
  sidecar's lift.
* **Rule 13h:** an SP9 amendment (`docs/agents/rules/13h.md`, regenerated
  into `gateway.md`), recording D3 I2 as a user decision.
* **The diagnostics page:** unchanged, because `QF003`'s wording is
  unchanged.

## 8. Gate

**Gate 1.** `python3 scripts/gate_scope.py --base 2c60986b0 --out
<worktree>/private/gate-1` ran on `f5bc0bea3` ([record](gate/gate-1.json)).
The scope was:

* Rust packages `qcore-audit`, `qcore-c-gateway`, `qcore-compiler`,
  `qcore-contract`, `qcore-frontend`, `qcore-verification`,
  `qcore-verifier-host`, `qcorec`, `quirin-audit-plugin-libcurl`,
  `quirin-audit-plugin-openssl` and `quirin-source-analysis`, plus five
  `quirin-audit-plugin-measure` test targets;
* Lean all, because `proof/QCore.lean` changed;
* Python always;
* no provider gate.

The gate exited 1:

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy, clippy (measure tests) | 0, 0, 0, 0 | |
| rust-suite | 101 | 633 units, 8462 passed, **1 failed**, 23 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 492 audits (this card's included) |
| python | 1 | 807 tests, **1 failed** |

Neither failure is this card's:

* **Rust.**
  `qcore-audit::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`
  is the recorded host failure (the system `curlver.h`). It was not fixed.
* **Python.**
  `test_gate_slot_starvation.Resident.test_it_is_reported_while_the_job_runs`
  failed on a resident-memory threshold under the gate's load
  (52372 KiB, expected at least 65536). Rerun alone, as the rule for
  gate-slot timing tests requires: `python3 -m unittest -v
  scripts.tests.test_gate_slot_starvation` exited 0 with 26 tests
  (`private/logs/py-starvation-alone.log`). Both results are recorded.

**After the merge of origin/main `e6d86d596`** (r62
CAST-BEFORE-STAR-READ-AS-BINARY; disjoint files: c-gateway, its own Lean
module, `proof/QCore.lean` and the matrix; merged tip `1975babc7`, no
conflict):

* `gen_lean_roots.py` and `gen_rule_docs.py` changed nothing.
* `cargo test -p qcore-contract --test plugin_guarded_symbols --test
  guarded_decomposition --test cjson_guarded_symbols --test
  guarded_contract_adoption --test cjson_typed_atoms` exited 0: 35 passed,
  3 ignored (`private/logs/t5-merge.log`).
* `lake build QCore.PremiseFormula.PluginGuardedSymbols QCore.CastBeforeStar`
  exited 0.

**Amendments.** `python3 scripts/apply_amendments.py
integration-amendments.json --check` exited 0 at the merged tip. It retires
`SP7-PLUGIN-GUARDED-APPLICATION-FORMULAS`.

**Measurement** is the integrator's step:

```text
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=baseline:<main baseline> --side tip=rev:<landed tip> --declared docs/reports/sp9-plugin-guarded-application-formulas/PREDICTION.md
```

## 9. Follow-ups

None registered. One note: the frozen sidecars' `boundary` and
`source_correspondence` fields are English descriptions of the documents.
They are not conditions a symbol states, and the files are digest-held.
They are of the same kind as SP10-FORMAL-BINDING-DESCRIPTIONS' scope.
