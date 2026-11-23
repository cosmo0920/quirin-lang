# SP6-GUARDED-SYMBOL-PROSE

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-22: "We need to convert to be symbolized atoms of
hypothesis." User 2026-11-23: "Launch SP6-GUARDED-SYMBOL-PROSE".

The card stopped once for a formalisation decision (draft `c835a92a0`). The
user then chose, on 2026-11-23:

* environment **E2**: an independent open assumption `#source_corresponds()`;
* success_case **R1**: the nine atom bodies, the membership formula and
  `#result(#item)`;
* invalid_membership **I2**: an evidence term inside the formula, chosen over
  this card's I1 recommendation;
* membership includes `#live(#parent)` and `!#parent_null()`;
* the source document is spelled with `&&` and rendered with `∧`.

Base `95f36b628` (branch `integrate/sp6-hypothesis-atoms`: SP6 merged onto
origin/main `1f2007667`). Prediction: [PREDICTION.md](PREDICTION.md)
(committed first, `a3f7ae210`). The card discharges `SP6-GUARDED-SYMBOL-PROSE`.

## 1. Census

The pattern is the string forms: `resource_guarded_symbol_v1\([a-z_0-9]+, *[a-z]+, *\\?"`
and `resource_guarded_decomposition_v1\([a-z_0-9]+, *\\?"`. They were counted
as source or as Rust string literals, outside `docs/reports/`, at base and
after. The table also lists what that pattern cannot see: forms built with
`format!` and readers of frozen texts.

| File | Role | Base (symbol, document) | After | What changed |
|---|---|---|---|---|
| `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv` | production, `cJSON_DetachItemViaPointer` | 17, 1 (8 English symbols, 9 formula-in-string, string document) | 0, 0 | 17 unquoted formulas, the document's unquoted clauses, one comment (§2, §3) |
| `crates/qcore-contract/tests/contract_views.rs` | bounds/resource relation fixture | 7, 1 (English `"Proposed …"`) | 0, 0 | formulas over SP7 terms; one mutation now shows the string refused |
| `crates/qcore-contract/tests/guarded_contract_adoption.rs` | adoption plan replay, refusals | 4, 1 | 4, 1 (refusal cases) | unquoted forms; the string mutations and reader fixtures are refusal cases; an over-long proposition has no source form (`QF003`) |
| `crates/qcore-contract/src/guarded_metadata_bridge.rs` | production `render` | 2 (`format!` with quoting `literal()`) | 0 | writes the unquoted formula and clauses; refuses a non-formula binding |
| `crates/qcore-frontend/src/guarded_decomposition.rs` | the source reader | read only a string | refuses a string (`QF003`) | formula and clause reader (§4) |
| `crates/qcore-contract/src/guarded_decomposition.rs` | lowering (standalone and inline) | admitted any proposition | refuses a non-formula proposition (`QF003`) | §4 |
| `crates/qcore-frontend/src/premise_formula.rs` | decision vectors | — | 2, 1 (refusal vectors) | `GUARDED_DECISION_VECTORS` |
| `compiler/tests/premise_formula_operators.rs` | compiler route | — | 3, 1 | refusal test (`Q000` naming `QF003`) |
| `crates/qcore-contract/tests/cjson_guarded_symbols.rs` (new) | the card's tests | — | 1 (+2 `format!`) | refusal cases on every route |
| `crates/qcore-contract/tests/guarded_decomposition.rs`, `production_symbolic_adoption.rs`, `crates/qcore-audit/tests/guarded_contract_pin.rs` | frozen plan and application replays | rendered frozen metadata with `bridge::render` | render it with the legacy `render_string_form` | §5 |
| `tests/support/legacy_prose_premise.rs` | legacy reader | 2 (`format!`) | 4 (`format!`) | lifts the recorded string texts and string documents; reproduces the retired form for frozen digests (§5) |
| `tests/support/sp2_premise_checkpoint.rs` | byte restoration | — | — | writes this card's recorded `cjson.qv` edits back first (two digests) |
| frozen inputs: `guarded-contract-adoption/adoption-plan.json` (17 English bindings, string document), `premise-guarded-decomposition/applications/cjson.json` (17), `plugin.json` (14), `scheduler.qgd` | read by tests | read as is | lifted (cjson: all 17), refused (plugin: 7 unrecorded English symbols), unchanged (`.qgd`, already symbolic) | §5 |

Totals by the pattern:

* At base: 28 symbol and 3 document matches in 3 files, plus 4 built with
  `format!` in 2 files.
* After: 10 symbol and 4 document matches in 4 files, all of them refusal
  cases, plus 6 built with `format!`: the legacy reader's retired-form
  renderer and lift patterns (4) and the new test's refusal builder (2).

No shipped `.qv` file states a string guarded text: 18 at base, 0 after.
`contract_views.rs` builds one refusal case by replacement, which the
pattern does not see.

## 2. The 17 formulas

Every symbol's proposition is the canonical G01p printing
(`quant_proposition::print`) of a typed-rendering formula over registered
C04.18.9 terms and the SP6/SP7 terms. Each one's Lean definition is in
`QCore.CjsonGuardedSymbols`.

* The nine `detach_*` prerequisites keep their bytes. Each is the formula
  of its typed atom (SP6), so their symbol digests are unchanged.
* The eight that were English follow, each with its faithfulness table.
  Disclaimers are scope limits. Each one is shown kept structurally: by what
  the formula does not assert, by the role, or by an Open marker. None is
  dropped or turned into a stronger claim.

**`detach`** (operation): `#admits_removed()`. Lean: `g.admitsRemoved`, a free proposition of the call.

| Prose | Kept by |
|---|---|
| "Admission of the selected cJSON_DetachItemViaPointer call under its Removed contract" | `#admits_removed`'s meaning (the gateway admits this call at its site under `resource_owned` / `resource_removed` / `resource_return_removed`); operation role (C04.18.5: proposed admission, never an observed execution) |
| "not a claim that raw C checks complete membership" | the formula names no C check; the guard `(environment ∧ …) \|-> detach` is a necessary entry obligation only; Lean `nonnull_prev_defeats_raw_rejection` (the raw guard is no membership oracle) stands |

**`environment`** (environment, E2): `#source_corresponds()`. Lean: `g.sourceCorresponds`, a free proposition.

| Prose | Kept by |
|---|---|
| "Exact CjsonRepresentationPremises.Environment entry heap and complete interference trace, exact caller identity" | `#source_corresponds`'s meaning: the call's actual entry heap, complete interference trace and caller identity are what the terms denote. The terms are read in one exact call model (`GuardedCall`: entry and exit `Environment`, trace and caller actor) |
| "and nine-coordinate context" | the meaning names it; the evaluation API still compares all nine Context coordinates (C04.18.5) |
| "source/model correspondence remains Open" | the formula is an independent assumption. No theorem derives it, and every call has a model in which it fails (`source_corresponds_is_never_derived`). Its assignment is evidence that is never Derived, so admission genuinely requires it and its disposition stays Open |

**`failure_case`** (case): `(#parent_null() || (#item_null() || (!#child(#parent,#item) && forall (#y: obj) { !#prev(#item,#y) })))`. Lean: `rawFailure` (`failure_case_holds`).

| Prose | Formula part |
|---|---|
| "Selected raw body early return:" | the case role; the disjunction is the body's guard (`cJSON.c` early return) |
| "parent is NULL" | `#parent_null()` |
| "item is NULL" | `#item_null()` |
| "or item differs from parent.child and item.prev is NULL" | `!#child(#parent,#item) && forall (#y: obj) { !#prev(#item,#y) }` (a NULL member designates nothing) |
| "not every invalid-membership input" | the formula states only the guard, never invalid membership. A nonmember with a non-NULL prev is not a failure case (Lean `nonnull_prev_defeats_raw_rejection`) |

**`invalid_membership`** (condition, I2): `((!#parent_null() && !#item_null()) && #caller_refutes_membership(#parent,#item))`. Lean: `nullParent = false ∧ nullItem = false ∧ g.callerRefutes entry.parent item` (`invalid_membership_holds`).

| Prose | Formula part |
|---|---|
| "Positive caller-side evidence" | `#caller_refutes_membership` (by user decision, a warrant term inside the proposition); the condition role's C04.18.5 rule (Supported invalidity and positively Refuted admission) is kept too |
| "that nonnull item" | `!#item_null()` (`!#parent_null()`: "the selected parent" designates an object) |
| "is not an owned child of the selected parent" | the term entails `!#owned(#parent,#item)` (`invalid_symbol_entails_invalid_membership`: the condition implies the existing `invalidMembership`) |
| "not merely failed membership proof" | non-ownership does not give the term (`absence_of_membership_is_not_refutation`), and no call model derives it (`caller_refutation_is_never_derived`) |

**`membership`** (prerequisite): `((!#parent_null() && !#item_null()) && (#live(#parent) && #owned(#parent,#item)))`. Lean: `membership` (`membership_holds`), and `prerequisites c ↔ representation c.entry ∧ membership c` (`prerequisites_iff`).

| Prose | Formula part |
|---|---|
| "Exact selected nonnull item" | `!#item_null()`, `#item` the call's item argument |
| "is contained in the live parent at entry" | `#live(#parent) && #owned(#parent,#item)` (entry terms) and `!#parent_null()` |
| "caller membership is independently required" | prerequisite role: a necessary entry obligation the caller supplies, never established by its declaration (C04.18.5) |
| "never inferred from Borrowed provenance" | the formula names no provenance term; nothing reads a mode into it |

**`original_list_unchanged`** (postcondition): the child link at each environment's parent, and the next and prev relations, are equal at entry and exit (two implications each way). Lean: `originalListUnchanged` (`original_list_unchanged_holds`).

| Prose | Kept by |
|---|---|
| "For the exact early-return case" | reached only through `failure_case ->\| original_list_unchanged` (C04.18.5: `case → postcondition`) |
| "the link heap equals the entry link heap" | `#exit_child(#exit_parent,·) ⇔ #child(#parent,·)`, `#exit_next ⇔ #next`, `#exit_prev ⇔ #prev` |
| "says nothing about arbitrary invalid lists" | the formula states only link equality, and only the early-return case reaches it |

**`remaining_list_valid`** (postcondition): the nine `detach_*` atom formulas over the exit terms, `#exit_erases(#item)`, and `forall (#y: obj) { !#exit_next(#item,#y) }` and `forall (#y: obj) { !#exit_prev(#item,#y) }`. Lean: `remainingListValid` (`remaining_list_valid_holds`, through `nine_exit_holds`: the exit atoms are the nine `CjsonRepresentationPremises` propositions of the exit environment).

| Prose | Formula part |
|---|---|
| "For the exact successful call" | reached only through `success_case ->\| remaining_list_valid` |
| "remaining owned children" | `#exit_erases(#item)`: the exit child sequence is the entry one with the item erased |
| "retain the representation" | the nine atom formulas over `#exit_*` (`representation c.after`) |
| "and removed item has NULL prev and next" | the two `forall … !#exit_next/prev(#item,…)` |

**`success_case`** (case, R1): the nine `detach_*` atom formulas, the membership formula, and `#result(#item)`. Lean: `prerequisites c ∧ result = some item` (`success_case_holds`).

| Prose | Formula part |
|---|---|
| "Selected raw body exits returning the same nonnull item" | `#result(#item)` with the membership formula's `!#item_null()` |
| "after unlinking" | names the arm. Under the prerequisites the raw guard does not fire, and the unlinked state is `remaining_list_valid`'s, reached by `->\|` |
| "under all representation and containment prerequisites" | the nine atom formulas (representation) and the membership formula (containment) |

The nine prerequisites' tables are SP6's
([report](../sp6-symbolic-hypothesis-atoms/README.md) §2). Their source text
is unchanged apart from the quotes.

## 3. Vocabulary: the SP7 block

The SP7 block adds 20 typed-rendering terms, appended to SP6's `#runs` in
`RENDERED_TERMS`. The spec table is C04.18.9's SP7 block, and the Lean list
is `sp7Terms`, bound to the Rust table by
`the_sp7_block_is_new_typed_and_bound_to_lean`. None is a registered term,
and no premise formula names one. So the premise `VOCABULARY`, grammar,
decision vectors and Lean `vocab` are unchanged, and no `Model` field is
added.

| Term | Sort | Lean meaning (`render`) |
|---|---|---|
| `#item` | obj | `c.item` |
| `#item_null`, `#parent_null` | predicate<> | `c.nullItem = true`, `c.nullParent = true` |
| `#exit_parent` | obj | `(detachModel c.after).parent` |
| `#exit_live`, `#exit_nodup` | predicate<obj> | `detachModel c.after`'s components |
| `#exit_owned`, `#exit_children`, `#exit_child`, `#exit_first`, `#exit_last`, `#exit_next`, `#exit_prev`, `#exit_next_reach` | predicate<obj,obj> | `detachModel c.after`'s components |
| `#exit_pred` | predicate<obj,obj,obj> | `(detachModel c.after).pred` |
| `#exit_runs` | predicate<fn,run> | `(detachModel c.after).runs` |
| `#exit_erases` | predicate<obj> | `c.after.children = c.entry.children.erase i` |
| `#admits_removed` | predicate<> | `g.admitsRemoved` (free) |
| `#source_corresponds` | predicate<> | `g.sourceCorresponds` (free, E2) |
| `#caller_refutes_membership` | predicate<obj,obj> | `g.callerRefutes p i` (free, constrained only by `refutes_sound`: it entails `¬ owned p i`) |

The existing registered terms are reused: `#result` (the call's result),
`#concurrent` and `#keeps_links`, whose meanings do not depend on the
environment. The registered terms keep their entry meaning, exactly as SP6's
typed atoms read them.

## 4. The document, the ban and where it fires

**The decomposition document is rendered from the clauses.** The source
clause states the G05-guarded clauses unquoted, separated by `,`, with `&&`
for `∧` (user decision; the shared lexer is untouched):

```text
resource_guarded_decomposition_v1(detach_guarded, (environment && detach_chain && … && membership) |-> detach, invalid_membership !-> detach, detach ||-> success_case / failure_case, success_case ->| remaining_list_valid, failure_case ->| original_list_unchanged)
```

`guarded_decomposition::readable` renders the readable document: the header,
one clause per line, `;`, `∧`. It is byte-identical to the former string and
to the frozen `applications/cjson.qgd`, so the readable SHA is unchanged
(`the_readable_document_is_rendered_from_the_clauses_unchanged`).

**`QF003`** is the next free id in SP5's `QF***`; `QF001` and `QF002` are the
only ones used, checked across the tree. It fires on:

* **The structural parser.** `frontend.rs` calls
  `guarded_decomposition::parse_source_clause` for every clause headed
  `resource_guarded_`, under any introducer. A string token anywhere in the
  clause rejects the module. Every module on every route parses first, so the
  resource, bounds and model loaders, provider packages, plugin views and
  inclusion all refuse. On the compiler route the user sees `Q000` naming
  `QF003`.
* **The lowering.** `qcore_contract::guarded_decomposition::lower`, standalone
  `replay` included, refuses a binding whose proposition is not the canonical
  printing of a guarded formula (`premise_formula::guarded_proposition`):
  English, a string or a noncanonical spelling.
* **The bridge `render`.** It writes the unquoted forms and refuses such a
  binding, so no source form exists for one.

A free name that is not a registered term in its rendered sort, an
ill-sorted application or quantifier, or a malformed clause also rejects.
These refusals carry no `QF003` prefix, because they are not string forms.
Each is fail-closed and total-or-reject.

**The legacy reader.** `tests/support/legacy_prose_premise.rs` is the same
file under the same confinement test. It reads
[migration.json](migration.json) `symbols`: the 17 recorded texts and their
formulas.

* `lift` writes each recorded string text as its formula, and each string
  document as its unquoted clauses. That is a respelling of the same symbolic
  clauses, never a new formula.
* `lift_bindings` also lifts the eight recorded English texts in frozen
  metadata.
* `render_string_form` reproduces the retired source form, so frozen digests
  are still checked.

An unrecorded string text stays a string and is refused. No production flag
admits one.

## 5. Migrated tests and fixtures

* **The adoption plan.** It is replayed as SP6 replayed it. The frozen
  digests are checked on the restored bytes; `sp2_premise_checkpoint` first
  writes back this card's recorded edits (`migration.json` `files`: 17
  symbols, the document and one comment, both digests). The plan's 17
  bindings are lifted, and the addition is lifted to the unquoted form. The
  bridge then checks on the current module.
* **The applications.** The cjson application's 17 bindings lift and lower.
  The plugin application's seven non-prerequisite symbols are English that no
  card has stated as formulas. It is now refused with `QF003`
  (`actual_cjson_and_plugin_hypotheses_lower_without_truth_or_source_changes`).
  Its formalisation is registered as a follow-up.
* **`contract_views.rs`.** Its fixture states formulas over SP7 terms.
* **The string-form tests** now test refusals: `guarded_contract_adoption.rs`,
  `contract_views.rs`, `cjson_guarded_symbols.rs` and the compiler's
  `a_string_guarded_symbol_or_document_is_refused_by_name_at_parse`.

## 6. Lean

`QCore.CjsonGuardedSymbols` (audit `AuditPremiseFormulaCjsonGuardedSymbols.lean`)
defines the call model `GuardedCall` and its rendering `render`. It proves:

* The nine prerequisites at entry (`chain_holds` … `predecessor_holds`, each
  reduced to SP6's `*_atom_iff`).
* The same nine over the exit terms (`exit_*_holds`, `nine_exit_holds`).
* The eight new symbols: `detach_holds`, `environment_holds`,
  `failure_case_holds`, `membership_holds`, `invalid_membership_holds`,
  `original_list_unchanged_holds`, `remaining_list_valid_holds`,
  `success_case_holds`, with `prerequisites_iff`. All are collected in
  `guarded_symbols_are_the_definitions`.
* The two assumptions: `invalid_symbol_entails_invalid_membership`,
  `membership_excludes_refutation`, `source_corresponds_is_never_derived`,
  `caller_refutation_is_never_derived`,
  `absence_of_membership_is_not_refutation`.
* The ban: `string_is_not_a_guarded_proposition`, `guarded_admitted_iff_typed`,
  and `guarded_vectors_agree`, bound to the Rust `GUARDED_DECISION_VECTORS`.

Each Lean `def` is the Rust reader's `lean` rendering of the formula as
`cjson.qv` states it (`the_symbols_are_the_lean_definitions`). The proofs are
placeholder-free and use standard axioms only.

## 7. Semantics

Guarded symbols are proposed metadata (C04.18.5, C04.18.6): no verdict,
standing, certificate or classifier reads one. The hypothesis graph, the
binding manifest, the interpreted contract, premises and erasure of
`cJSON_DetachItemViaPointer` are unchanged; the bridge compares the loaded
contracts and premises of the plan's before and after texts. The predicted
status moves are 0 on corpus, twelve-home, system and slice
([PREDICTION.md](PREDICTION.md)).

## 8. Locks, reviewed

The steps ran in this order:

1. `providers/contracts.py sync` (`private/logs/sync-1.log`) exited 1 with
   the expected refusal "stale lock: `cJSON_DetachItemViaPointer`'s contract
   text changed since it was pinned".
2. `quirin-audit pin --source-root <Fluent Bit at REV> --crate
   qcore-provider-librdkafka --function cJSON_DetachItemViaPointer` ran at
   `66910c10` and then at `046accf6`, so the lock set stays tied to
   `046accf6` as before. The source was a `--shared --no-checkout` clone of
   the local Fluent Bit repository under the worktree's ignored `private/`,
   detached at each revision. Logs: `private/pin-1/pin-*.log`, both exit 0.
   The first run reported "contract text changed; 1 earlier identit(ies)
   voided" and a new identity at `66910c10`; the second reported `046accf6`
   "recorded beside" it.
3. `contracts.py sync` ran again (`private/logs/sync-2.log`) and exited 0.

The lock diff:

* `contracts.lock`: only the `cJSON_DetachItemViaPointer` entry's
  `contract_sha256` (`7b1a99ac…` → `b76d6aa7…`) and `pinned_at`. Its
  identity keeps the same two sites and the same two revisions.
* `contract-inventory.lock` and `contracts.json`: only
  `librdkafka/cjson.qv`'s digest (`0fcd9833…` → `24ced049…`). Declaration
  and clause counts are unchanged.
* `bounds-contracts.lock`, `outer-contracts.lock`, the binding manifest and
  `providers/contract-load-path.json`: unchanged.

## 9. Gate

**Gate 1** ran `python3 scripts/gate_scope.py --base 95f36b628 --out
private/gate-1` on `c26d04f41` ([record](gate/gate-1.json); console outside
the worktree). The full suite was selected because `proof/QCore.lean`
changed. The gate exited 1:

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| rust-suite | 101 | 678 units, 9907 passed, **2 failed**, 22 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 486 audits (this card's included) |
| python | 0 | 807 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 174 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The two failures:

* `qcore-compiler::qcore_contract_corpus` was a real defect. The compiler
  route's `Q038` operator gate (`pipe.rs`) refused the `&&` and `||` of
  `cjson.qv`'s guarded formulas. Fixed in `691900e66`: a guarded clause the
  reader accepts gets an exemption span, like a premise formula's, and a
  refused one gets none.
* `qcore-audit::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`
  is the recorded host failure (system `curlver.h`). It was not fixed.

**After the fix and the merge of origin/main `28d5807fd`** (SP6 landed, r57,
r58; merged tip `de880bfc0`; no conflict):

* `python3 scripts/gen_lean_roots.py` and `python3 scripts/gen_rule_docs.py`:
  no change.
* `python3 providers/contracts.py check`: exit 0
  (`private/logs/check-merge.log`).
* `cargo test --no-fail-fast -p qcore-frontend -p qcore-compiler -p
  qcore-contract -p qcore-audit`, through `gate_slot.py rust-suite`
  (`private/logs/t3.log`): exit 101, 279 summaries, 3368 passed, 1 failed (the
  same host failure only), 5 ignored. `qcore_contract_corpus` passed 6 of 6
  and `cjson_guarded_symbols` 8 (1 ignored, the printer).
* `check_lean_audits.py` through `gate_slot.py lean`
  (`private/logs/lean-2.log`): exit 0, 487 audits.

Earlier scoped runs before gate 1: `private/logs/t1.log` (the five guarded
and adoption test files, exit 0) and `t2.log` (exit 0).

**Amendments.** `python3 scripts/apply_amendments.py
integration-amendments.json --check` exited 0 against the base `95f36b628`.
At the merged tip it refuses: "retire SP6-GUARDED-SYMBOL-PROSE: no
registered follow-up has this id". The cause is on main. Its matrix lost
SP6's ledger edits in the integrator merge `3c79b276c` ("Merge origin/main
into integrate/sp6-hypothesis-atoms"): `SP2-DETACH-HYPOTHESES-TYPED-ATOMS`
is registered again and `SP6-GUARDED-SYMBOL-PROSE` is absent. SP6's
increment paragraph survived. Once SP6's two ledger edits are restored, this
card's retire applies.

The acceptance matrix's derived Lean-audit list is left to the landing. The
card gate accepts the one added audit (`CARD-GATE-NEW-AUDIT-MATRIX-COUNT`).

**Measurement** is the integrator's step:

```text
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=baseline:<main baseline> --side tip=rev:de880bfc0 --declared docs/reports/sp6-guarded-symbol-prose/PREDICTION.md
```

## 10. Follow-ups

* `SP7-PLUGIN-GUARDED-APPLICATION-FORMULAS` (registered). The frozen plugin
  guarded sidecar (`premise-guarded-decomposition/applications/plugin.json`)
  has seven English symbols: `environment`, `invoke`, `registered_case`,
  `default_case`, `registered_lifetime_valid`, `default_lifetime_valid` and
  `unregistered`. It is now refused with `QF003` rather than lowered. Stating
  them over `PluginPremiseDecomposition` terms would restore its positive
  replay.
* `SP7-FORMAL-BINDING-DESCRIPTIONS` (registered; recorded by SP6, not
  registered there). Binding manifests' `environment.variables` descriptions
  and the `FormalBinding` records built in code (`cjson_local_binding`,
  `plugin_source_witnesses`) still describe their environments in English.
  They are not conditions a declaration states, and converting them is not
  small (the manifests are digest-held), so this card did not.

## Integrator measurement (2026-11-21)

Landing `35b5f48a2` on base `9193b196a`, measured with `--declared PREDICTION.md`:
corpus, twelve-home, system and slice all match the declared prediction with
0 moves. Comparison: `integration/native-comparison.json.gz` (uncompressed
sha256 `8fe42108dd43d0cff143a5d28d7e89ce7764fa14fc76098f0426640474f318de`). Landing gate: `integration/gate.json` (Rust 9932
passed, 1 failed: the known host libcurl failure; Lean 488 audits; Python
807Combined landing with the other card (one gate, one measurement of the combined tip 35b5f48a2 against SP6-GUARDED-SYMBOL-PROSE's PREDICTION.md, declared result match; SP8's PREDICTION.md also predicts 0 status moves on corpus, twelve-home, system and slice, which the same measurement shows). Base 9193b196a includes the restoration of SP6's two ledger edits lost in the r57 race merge.).
