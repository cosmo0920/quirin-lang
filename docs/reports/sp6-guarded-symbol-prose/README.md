# SP6-GUARDED-SYMBOL-PROSE (draft: census and stop for a decision)

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-22: "We need to convert to be symbolized atoms of
hypothesis." User 2026-11-23: "Launch SP6-GUARDED-SYMBOL-PROSE".

Base `95f36b628` (branch `integrate/sp6-hypothesis-atoms`). This draft holds
step 1 of the brief (the census) and the design that the stop asks about. No
code, contract, lock, Lean or record has changed yet. The card stopped
because two symbols can each be formalised in two defensible ways that assume
different things (brief: "stop and hand back with options and a
recommendation").

## 1. Census at base

Each place outside `docs/reports/` where a guarded symbol's text or the
guarded decomposition document is written as a string, or where a frozen
string form is read.

| File | Role | At base |
|---|---|---|
| `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv` | production: `cJSON_DetachItemViaPointer` | 17 `resource_guarded_symbol_v1` clauses with a string text: 8 English (`detach`, `environment`, `failure_case`, `invalid_membership`, `membership`, `original_list_unchanged`, `remaining_list_valid`, `success_case`) and 9 `detach_*` prerequisites whose string holds the typed atom's formula. 1 `resource_guarded_decomposition_v1` whose readable document is a string (its content is already symbolic: G05-guarded `∧`, `\|->`, `!->`, `\|\|->`, `->\|`) |
| `crates/qcore-frontend/src/guarded_decomposition.rs` | the reader (`parse_source_clause`) | accepts only a string text and a string document (`literal unescaped string required`) |
| `crates/qcore-contract/src/guarded_decomposition.rs` | `lower` (standalone and inline) | admits any non-empty proposition string of at most 4096 bytes, English included |
| `crates/qcore-contract/src/guarded_metadata_bridge.rs` | production `render`: the bridge-v2 deterministic insertion | writes both forms as string literals (1 symbol format, 1 document format) |
| `crates/qcore-contract/tests/contract_views.rs` | bounds/resource relation fixture | 7 English symbols (`"Proposed environment"` …) and 1 string document; 2 head mutations |
| `crates/qcore-contract/tests/guarded_contract_adoption.rs` | adoption plan replay and refusals | rebuilds the string document form (1), 1 English `unused` symbol, 6 reader-refusal fixtures (4 with strings), a 4097-byte proposition, head and version mutations |
| `crates/qcore-contract/tests/guarded_decomposition.rs` | standalone replay of the frozen applications | reads `applications/{cjson,plugin}.json` (17 and 14 English bindings) and the two `.qgd` documents |
| `crates/qcore-contract/tests/cjson_typed_atoms.rs` | SP6 | compares the 9 prerequisite strings with the atom formulas |
| `crates/qcore-audit/tests/guarded_contract_pin.rs` | the guarded pin | renders the frozen plan's metadata (17 English bindings, string document) |
| `tests/support/legacy_prose_premise.rs` | legacy reader | 2 format strings: lifts the 9 frozen prerequisite sentences to a formula held in a string |
| `crates/qcore-contract/examples/scheduler_lifetime_source.rs` | standalone `.qgd` parse | the frozen `scheduler.qgd` (symbolic document, no bindings) |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` C04.18.5, C04.18.6; `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` G05-guarded-source | the stated grammar | `"PROPOSITION"`, `"DOCUMENT"`, `RAW_STRING` |

Frozen inputs under `docs/reports/` that tests replay (these stay as they
are, and are read through the legacy reader after the ban):
`guarded-contract-adoption/adoption-plan.json` (17 English bindings and a
string document), `premise-guarded-decomposition/applications/cjson.json`
(17), `plugin.json` (14), the two `.qgd` files, and
`scheduler-lifetime-contracts/scheduler.qgd`.

No other `.qv` file states a guarded symbol. No graph or plan JSON outside
`docs/reports/` holds one.

## 2. Proposed design (pending the decision)

* **Symbol syntax.** `requires resource_guarded_symbol_v1(ID, ROLE, FORMULA,
  DEPENDENCY, ...);` with `FORMULA` an unquoted C04.18.9 typed-rendering
  formula (G01p syntax, as SP6's typed atoms use). Its free `#names` are
  registered terms and get their rendered sorts. The binding's
  `proposition` is the formula printed canonically, so the nine `detach_*`
  prerequisite propositions keep their exact bytes, and their symbol
  digests stay the same.
* **Document.** It is written unquoted, as comma-separated G05-guarded
  clauses inside the clause. The readable document (and its SHA) is rendered
  from those tokens, byte-identical to today's text. `∧` needs a lexer token,
  or `&&` in source rendered as `∧`.
* **Ban.** A string in either head is refused on every load route with
  `QF003` (the next free `QF` id: `QF001` and `QF002` are used). This covers
  the structural parser, the resource, bounds and model loaders, the
  standalone `lower` (an English proposition is refused there too) and the
  bridge `render`. The frozen texts are read only through
  `tests/support/legacy_prose_premise.rs`.
* **SP7 rendered terms** (G01p sorts, Lean model over
  `GuardedContractApplications.Cjson.Call`, plus a `result`):
  * `#item: obj`, `#item_null: predicate<>`, `#parent_null: predicate<>`.
  * The exit environment, one term per entry term the representation uses:
    `#exit_parent`, `#exit_live`, `#exit_owned`, `#exit_children`,
    `#exit_child`, `#exit_first`, `#exit_last`, `#exit_pred`, `#exit_next`,
    `#exit_prev`, `#exit_next_reach`, `#exit_nodup`, `#exit_runs`.
    `#concurrent` and `#keeps_links` do not depend on the environment.
  * `#exit_erases: predicate<obj>`: the exit child sequence is the entry
    sequence with the first occurrence of the item erased (Lean
    `List.erase`).
  * `#admits_removed: predicate<>`: the gateway admits the call under the
    declaration's Removed contract.
  * The existing `#result`.
* **Formulas that need no decision** (each equal to its Lean definition):
  * `failure_case`: `rawFailure`.
  * `invalid_membership`: `invalidMembership`.
  * `membership`: the non-null and owned-child conjuncts of `prerequisites`,
    plus "live parent".
  * `remaining_list_valid`: `remainingListValid`, which is the nine atom
    bodies over the exit terms, `#exit_erases(#item)`, and NULL exit links
    for the item.
  * `original_list_unchanged`: `originalListUnchanged`, with the child, next
    and prev links equal.
  * `detach`: `#admits_removed()`.
  * The nine prerequisites: unchanged.

## 3. The decision

See the hand-back. Options: environment (E1 model scope vs E2 independent
correspondence assumption), success_case (R1 prerequisites ∧ returns item vs
R2 raw arm ∧ returns item), invalid_membership (I1 proposition with
positivity carried by the condition role vs I2 an evidence term).
