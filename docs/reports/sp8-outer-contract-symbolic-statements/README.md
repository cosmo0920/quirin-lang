# SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch the outer contract prose card too".
User decisions 2026-11-23, answering this card's design hand-back (relayed by
the integrator): **D1 = A** (the conditions librdkafka's published
documentation states; librdkafka's public functions only), **D2 = B**
(provenance plus typed access), **D3 = B** (an interval from registration
until the destruction of every holder and copy ends); the §6 readings
accepted as recorded design choices, each kept in the faithfulness tables
with its reason.

Base `95f36b628` (branch `integrate/sp6-hypothesis-atoms`, since landed). Prediction:
[PREDICTION.md](PREDICTION.md) (committed first, `55bc33df5`). Design
hand-back: `160c77880`. Implementation: `e1d9f78d6`; merged with origin/main `28d5807fd` as `3728090f5`. The card discharges
SP5's report follow-up 1 (outer contract statements still English; never
registered) and registers and retires its own id.

## 1. Census

Pattern `quant #outer \[\[contract\]\] [A-Za-z_{}]+, *\\?"` (a statement
written as source or as a Rust string literal) outside `docs/reports/`, plus
what it cannot see: readers and asserters of the parsed statement, statements
built by `replace`/`format!`, the outer header vectors (written from `#outer`,
without `quant`), diagnostics and grammar text.

| File | Role | Base | After | What changed |
|---|---|---|---|---|
| `providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv` | `rk_plugin_library` on `rd_kafka_plugin_new` (production) | 1 string | 0 | formula (§4.1); a comment paragraph |
| `providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv` | `flb_external_plugin` on `flb_plugin_load` (production); the citation on `flb_plugin_proxy_create` carries no statement | 1 string | 0 | formula (§4.2); a comment paragraph |
| `crates/qcore-contract/tests/outer_contract.rs` | fixture `DEFINITION`, its mutation table, the blank `outer-statement` case | 2 strings + 1 blank case | 0 by pattern; refusal cases built by `replace` | formula fixture; the blank case became an ill-sorted formula case; a `QF004` case; new tests: the shipped statements are the recorded formulas, the string refused on every route, the legacy reader lifts exactly the record |
| `crates/qcore-audit/tests/outer_projection.rs` | projection fixture | 1 | 0 | formula |
| `crates/qcore-audit/src/outer_candidates.rs` | reader of `outer-candidates:HEX`; unit test statement built with `format!` and one overwrite | 1 reader, 1 `format!`, 1 overwrite | 0 | reads the formula's inventory constants (`candidate_inventories`); the test cites the registered constant; a new test binds the constant to the shipped inventory's digest |
| `crates/qcore-contract/src/outer_contract.rs` | parser (`plain_statement`, `outer-statement`), shape diagnostic, module docs | 1 reader, 3 spellings | 0 | the header reads a formula through `premise_formula::outer_header`; statement = canonical printing; `candidate_inventories`; docs |
| `crates/qcore-audit/src/outer_selection.rs` | prints `"statement"` in the projection record | 1 reader | unchanged code | now prints the canonical formula |
| `crates/qcore-contract/examples/plugin_outer_catalog_replay.rs` | `statement.contains("outer-candidates:HASH")` | 1 reader | 0 | reads the inventory constant |
| `crates/qcore-frontend/src/frontend.rs` | `QuantOuter` doc spelling; the parser passed the header through unread | 1 spelling | 0 | refuses a string header (`QF004`) and an ill-formed formula at parse |
| `crates/qcore-frontend/src/premise_formula.rs` | (new) | — | 2 (refusal doc and message) + 3 string rows of `OUTER_DECISION_VECTORS` | `outer_header`, `PROSE_OUTER`, the SP8 block, the inventory constants, the vectors |
| `crates/qcore-frontend/src/pipe.rs` | operator gate | — | — | admits an accepted outer statement's `&&` and `|` |
| `compiler/tests/premise_formula_operators.rs` | compiler route | 0 | 2 string cases (`format!`) | refusal test (`Q000` naming `QF004`) and the shipped outer modules raise no `Q038` |
| `tests/support/legacy_prose_premise.rs` | legacy reader | — | 2 (doc, pattern) | lifts the two recorded statements (§5) |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | C04.20 example `"STATEMENT"`, D2's verbatim recorded form `NAME, "text"` | 2 + 1 verbatim | 1 verbatim | example and loading text are the formula form; the user decision's verbatim spelling is kept as recorded |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | G05g `[ ",", STRING ]` | 1 production | 0 | `[ ",", premise_formula ]`, `QF004` |
| `docs/agents/rules/31.md` (rendered into `gateway.md`), `docs/agents/diagnostics.md` | — | 0 | 0 | rule 31's SP8 paragraph; the `QF004` entry |
| `providers/*/outer-contracts.lock` | `contract_sha256` of each defining loader | digest | digest | re-pinned (§9) |
| `docs/reports/plugin-premise-decomposition/outer-catalog.json` | historical outer digests the catalog example replays | digest | historical | the report owns its evidence |

Totals by the pattern: 13 matches in 9 files at base (2 production
statements, 4 test strings or builders, 7 spellings); after, 5 matches in 3
files (the refusal's doc and message in `premise_formula.rs`, D2's verbatim
recorded spelling in the specification, the legacy reader's doc and
pattern). No shipped module states a string statement (2 at base, 0 after).

## 2. The form

`quant #outer [[contract]] NAME, FORMULA;`, `FORMULA` a C04.18.9 formula read
by `qcore_frontend::premise_formula::outer_header` (the parse route, so every
route) and by the outer loader; the statement is its canonical printing, and
the loader's normalised text (its digest, the pin's `contract_sha256`, the
contract identity) carries that printing. Not named sub-premises (an outer
module admits no `resource_premise` clause, and the envelope's `#premise`
lines already name the premises it vouches for); not SP6 typed atoms (those
are hypothesis-graph renderings). One formula is what SP1–SP5 did for every
premise statement.

The subject is `#outer_code`, read relative to the contract (as `#call` is
relative to a declaration); OpenSSL's `#loaded` term is unrelated and unused.
The candidate inventory digest is a typed constant of sort `obj`,
`CANDIDATES_` and the 64 upper-case hex digits of the inventory's SHA-256
(a bare hex digest lexes as an integer followed by a word), registered in
`CANDIDATE_INVENTORIES` (Rust) and `inventories` (Lean): an unregistered
constant of that shape is refused like any unknown constant, and only this
family may exceed the 64-byte constant bound. `outer_candidates` reads every
registered constant of the formula and checks it exactly as it checked
`outer-candidates:HEX`; a unit test binds the constant to the shipped file's
digest (`the_registered_inventory_constants_are_the_shipped_digests`).

## 3. Vocabulary (SP8 block)

Appended after the SP5 block in `VOCABULARY`, the specification table and the
Lean `vocab` (card SP6-GUARDED-SYMBOL-PROSE's SP7 block, built concurrently,
merges before it; no existing entry is reordered). 36 terms, no token,
operator, sort or production; one constant family. The Lean model reads the
two nullary kinds new to the SP4 mechanism (propositions, effects) through two
name-indexed components with defaults (`xPr0`, `xEf0`) and set-of-callback
arguments through `ext1St`'s two new arms; every other SP8 term uses the SP4
components. The full table with meanings is C04.18.9's SP8 block; signatures:

* subject and registrations, set(fn): `#outer_code`, `#conf_registrations`,
  `#public_setter_values`, `#meets_candidate`, `#interceptors`,
  `#on_live_conf`, `#from_on_new`, `#before_init`, `#on_share_consumer`,
  `#valid_through_last` (D3 B), `#rk_public`, `#rk_nonpublic`,
  `#cjson_init_hooks`, `#exported`; obj → set(fn): `#zero_appends`,
  `#conf_lifecycle`, `#client_lifecycle`; set(fn) → set(fn): `#copies`;
* cells and objects, set(obj): `#rk_internal`, `#cjson_global_hooks`,
  `#cjson_link_cells`, `#cjson_handed`, `#flb_core_owned`, `#handed`;
* effects: `#borrows_args`, `#borrows_handles`, `#no_global_custody`,
  `#balanced`, `#typed_access` (D2 B); set(obj) → eff: `#stores_none`,
  `#accesses_only`, `#provenance` (D2); set(fn) → eff: `#calls_none`,
  `#calls_only`, `#meets_published` (D1 A);
* proposition: `#allocator_hooks_premise` (cites `cjson_allocator_hooks`).

Reused: `#inert`, `#false`, `#child`, `#first`, `#last`, `#pred`, `#next`,
`#prev`, `#next_reach`, `#children`, `#nodup`.

## 4. The statements and their faithfulness tables

`H` is `CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147`
(the SHA-256 of `qcore-provider-librdkafka/outer-candidates.json`, unchanged).
`W(#a)` is the list shape of `detach_list_representation` (SP2) with its item
generalised from `#parent` to `#a` and its liveness, custody and concurrency
conjuncts dropped. The exact statements are `migration.json`'s `formula`
fields (and the two `outer.qv` files); 568 and 606 tokens (bound 1024).

```text
W(#a) = (quant all #c in #child(#a): #c in #first(#a)) && (quant all #c in #first(#a): #c in #child(#a)) && (quant all #b in #children(#a): quant all #p in #pred(#a, #b): #b in #next(#p)) && (quant all #c in #children(#a): #c in #next_reach(#c) -> #false) && ((quant all #c in #child(#a): #false) -> quant all #c in #children(#a): #false) && ((quant all #c in #children(#a): #false) -> quant all #c in #child(#a): #false) && (quant all #b in #children(#a): quant all #p in #pred(#a, #b): #p in #prev(#b)) && (quant all #f in #first(#a): quant all #l in #last(#a): #l in #prev(#f)) && #nodup(#a) && (quant all #c in #children(#a): quant all #y in #next(#c) | #prev(#c): #y in #children(#a)) && (quant all #l in #last(#a): quant all #y in #next(#l): #false) && (#a in #children(#a) -> #false)
```

### 4.1 `rk_plugin_library`

```text
RK = (quant all #f in #conf_registrations: #f in #public_setter_values && #f in #meets_candidate)
  && (quant all #f in #interceptors: #f in #zero_appends(H))
  && (quant all #f in #interceptors: #f |-> (#inert && #borrows_args && #borrows_handles && #no_global_custody && #balanced))
  && (quant all #f in #copies(#conf_registrations | #interceptors): #f in #valid_through_last)
  && (quant all #f in #interceptors: (#f in #conf_lifecycle(H) -> #f in #on_live_conf) && (#f in #client_lifecycle(H) -> #f in #from_on_new && #f in #before_init && #f in #client_lifecycle(H) \ #on_share_consumer))
  && #outer_code |-> (#calls_none(#cjson_init_hooks) && #stores_none(#cjson_global_hooks) && #stores_none(#cjson_link_cells) && #stores_none(#rk_internal) && #calls_none(#rk_nonpublic) && #meets_published(#rk_public))
  && (quant all #a in #cjson_handed: W(#a))
  && #allocator_hooks_premise
```

| # | Prose | Formula part | Reason / reading |
|---|---|---|---|
| R0 | "every library named by plugin.library.paths, including code it invokes" | the subject `#outer_code` | settled reading R-a: invoked code outside the audited program is a member, the audited program's functions are not (otherwise R10 is false of every public setter); each effect is the code's own instructions |
| R1 | "registers rk_conf callbacks only through public setters and only when each callback meets that member's candidate contract" | `quant all #f in #conf_registrations: #f in #public_setter_values && #f in #meets_candidate` | |
| R2 | "registers interceptors only through the public setters, with success result zero and each exact list/union-arm candidate in outer-candidates:5c3f…" | `quant all #f in #interceptors: #f in #zero_appends(H)` | setter, zero result, exact list and arm are `#zero_appends`'s meaning; `H` is the checked digest |
| R3 | "every registered interceptor and all code it invokes meets that inventory's neutral custody restriction" | `quant all #f in #interceptors: #f \|-> (…)` | settled reading: a property of the run with every call it makes (C04.20.1); a per-callee reading contradicts R3e |
| R3a | "(all pointer arguments borrowed" | `#borrows_args` | |
| R3b | "no release, retention, ownership transfer" | `#inert` and `#borrows_args` | |
| R3c | "or global custody effects" | `#no_global_custody` | |
| R3d | "scalar resource handles borrowed too" | `#borrows_handles` | |
| R3e | "and every local acquisition balanced before return)" | `#balanced` | |
| R4 | "registered callback code and opaque contexts remain valid through the last possible invocation, including destruction and copied on_new registrations" | `quant all #f in #copies(#conf_registrations \| #interceptors): #f in #valid_through_last` | **D3 = B**: an interval ending with the destruction of every holder and copy; entails validity at each run |
| R5 | "configuration interceptors are installed on a live configuration" | `#f in #conf_lifecycle(H) -> #f in #on_live_conf` | |
| R5' | "and client interceptors only from on_new before initialization completes, never on share consumers" | `#f in #client_lifecycle(H) -> #f in #from_on_new && #f in #before_init && #f in #client_lifecycle(H) \ #on_share_consumer` | |
| R6 | "it calls cJSON_InitHooks nowhere" | `#outer_code \|-> #calls_none(#cjson_init_hooks)` | "nowhere": every member of the subject |
| R7 | "and does not replace cJSON's global allocator hooks" | `#stores_none(#cjson_global_hooks)` | the other way to replace them |
| R8 | "it neither mutates cJSON child, prev or next links directly" | `#stores_none(#cjson_link_cells)` | |
| R9 | "nor passes malformed cJSON lists to an API call" | `quant all #a in #cjson_handed: W(#a)` | settled reading: the full list shape, never only `cjson_list_invariant`'s two conjuncts (narrower than "malformed") |
| R10 | "it never directly stores into librdkafka internal members (including callback fields, op fields, broker and rule-14t links, queues and list links)" | `#stores_none(#rk_internal)` | open list: the class `#rk_internal`, each item a member row of its meaning |
| R11 | "it invokes no non-public librdkafka symbol" | `#calls_none(#rk_nonpublic)` | |
| R12 | "and calls public APIs only under their published conditions" | `#meets_published(#rk_public)` | **D1 = A**: published documentation; librdkafka's public functions only |
| R13 | "any allocator hooks installed before this library loads satisfy cjson_allocator_hooks" | `#allocator_hooks_premise` | settled reading: a citation of both of the premise's statements, not a restatement of one |

### 4.2 `flb_external_plugin`

```text
FB = #outer_code |-> (#stores_none(#flb_core_owned) && #accesses_only(#handed) && #calls_only(#exported) && #provenance(#handed) && #typed_access) && (RK)
```

`RK` is §4.1's statement verbatim, parenthesized, so the Fluent Bit statement
is `fbOwnFormula && rkFormula` (a Rust test checks the right conjunct's tree
equals the librdkafka statement's) and `QCore.PremiseFormula.Outer.fb_entails_rk`
proves FB → RK in every model.

| # | Prose | Formula part | Reason / reading |
|---|---|---|---|
| F0 | "every Fluent Bit plugin loaded from outside the build (a C shared object through flb_plugin_load or a Go plugin through flb_plugin_proxy_create), including code it invokes" | the subject `#outer_code` | the parenthetical's two rows are the contract's two hosting loaders (checked: `contracts.loaders`); R-a as R0 |
| F1 | "stores into no core-owned flb_*_instance or flb_config member" | `#stores_none(#flb_core_owned)` | R-a: the plugin's own stores (otherwise every input plugin breaks it through `flb_input_log_append`'s counters), the same direct store the librdkafka sentence calls "directly" |
| F2 | "it reaches only handed objects and exported APIs" | `#accesses_only(#handed) && #calls_only(#exported)` | settled reading: handed objects include exported API results, registered callbacks' arguments and the code's own objects |
| F3 | "and forges no pointer" | `#provenance(#handed) && #typed_access` | **D2 = B**: provenance and typed access (C04.20's Reach paragraph) |
| F4 | "when it uses librdkafka it registers rk_conf callbacks only through public setters and only when each callback meets that member's candidate contract" | R1 | vacuous when it registers none |
| F5 | "registers interceptors only through public setters" | R2 | |
| F6 | "never directly stores into librdkafka internal members (including …)" | R10 | |
| F7 | "invokes no non-public librdkafka symbol, and calls public APIs only under their published conditions" | R11, R12 | D1 = A |
| F8 | "it calls cJSON_InitHooks nowhere and does not replace cJSON's global allocator hooks" | R6, R7 | |
| F9 | "neither mutates cJSON child, prev or next links directly nor passes malformed cJSON lists to an API call" | R8, R9 | |
| F10 | "and any allocator hooks installed before it loads satisfy cjson_allocator_hooks" | R13 | |
| F11 | "every interceptor registration uses the public setter with success result zero and the exact list/union-arm candidate in outer-candidates:5c3f…" | R2 | |
| F12 | "every registered interceptor and its transitive calls meets the neutral custody restriction (…)" | R3 (R3a–R3e) | |
| F13 | "registered callback code and opaque contexts remain valid through the last possible invocation, including destruction and copied on_new registrations" | R4 | D3 = B |
| F14 | "configuration interceptors are installed on a live configuration and client interceptors only from on_new before initialization completes, never on share consumers" | R5, R5' | |

## 5. The diagnostic and the legacy reader

`QF004` (`premise_formula::PROSE_OUTER`; `QF003` is card
SP6-GUARDED-SYMBOL-PROSE's): an outer definition whose statement is or holds
a string literal, whatever the name's spelling. Where it fires: the
structural parser (`frontend.rs` → `outer_header`), so the outer loader, the
resource and bounds loaders (both refuse the module at parse), the compiler
route (`Q000` whose message starts with `QF004`) and every other route; the
outer loader's own header reader returns the same refusal. A formula the sort
judgment refuses also rejects at parse; the shape diagnostic now spells `NAME,
FORMULA`; `outer-statement` names a formula refusal the loader's reader
returns.

`tests/support/legacy_prose_premise.rs` (test code only; the confinement test
`the_legacy_prose_reader_is_confined_to_test_code` unchanged) gains
`outer_statements()` and lifts each recorded `quant #outer [[contract]] NAME,
"PROSE"` to `NAME, FORMULA` in `lift`, from
[migration.json](migration.json) (the two sentences and formulas); an
unrecorded sentence stays a string and is refused
(`the_legacy_reader_lifts_exactly_the_recorded_statements`). No frozen text a
test replays states an outer string statement today (the only one under
`docs/reports/` is `outer-contract-design/README.md`, prose no test reads).

## 6. Tests migrated and added

* `qcore-contract/tests/outer_contract.rs`: formula fixture; 17 malformed
  cases (an ill-sorted formula and the `QF004` string added, the blank string
  case replaced); `the_shipped_statements_are_the_recorded_formulas`,
  `a_string_statement_is_refused_on_every_route` (parse, outer, resource,
  bounds, each recorded sentence), `the_legacy_reader_lifts_exactly_the_recorded_statements`.
* `qcore-audit`: `outer_projection.rs` fixture; `outer_candidates` unit tests
  cite the constant, and `the_registered_inventory_constants_are_the_shipped_digests`.
* `qcore-frontend/tests/premise_formula.rs`: the vocabulary and decision
  vectors bind the SP8 block automatically; `the_outer_vectors_are_the_lean_table`,
  `the_inventory_constants_are_the_lean_table`,
  `the_outer_statements_are_the_lean_definitions`.
* `compiler/tests/premise_formula_operators.rs`:
  `an_outer_statement_keeps_its_union_and_conjunction`,
  `a_string_outer_statement_is_refused_by_name_at_parse`.

## 7. Specification, grammar, rules

C04.18.9 (the SP8 block: table, constants, decisions, settled readings, Lean),
C04.20 ("What it states", "Loading", a new "Statements as formulas"
paragraph with both formulas, the example), C04.20.1 (the citation is the
typed constant), G05g (`[ ",", premise_formula ]`, `QF004`, the operator
gate), rule 31 (an SP8 paragraph, rendered by `gen_rule_docs.py` into
`gateway.md`), the diagnostics page (`QF004`), module docs of
`outer_contract.rs`, `premise_formula.rs`, `pipe.rs`, `frontend.rs`,
`outer_candidates.rs`, and both `outer.qv` comments.

## 8. Lean

`proof/QCore/PremiseFormula/Outer.lean` (new), audited by
`proof/AuditPremiseFormulaOuter.lean` (new):

* `outer_formulas_wf`: `fbOwnFormula`, `rkFormula` and `fbFormula` are
  well-formed (so each denotes a proposition in every model, `wf_denotes`);
* `fb_entails_rk`: `(eval M [] fbFormula).holds → (eval M [] rkFormula).holds`
  for every model `M`, the relation the Fluent Bit sentence stated, proved
  from the formulas; `fb_states_own` its other half;
* `string_is_not_an_outer_statement`; `outer_vectors_agree` (bound to the
  Rust `OUTER_DECISION_VECTORS`, eight rows: three formulas accepted, three
  strings refused, an ill-sorted formula and an unregistered inventory
  constant refused).

`proof/QCore/PremiseFormula.lean`: the SP8 `vocab` block; `inventories`; the
constant judgment (`classes` → `cls`, `inventories` → `obj`); `Model` gains
`xPr0` and `xEf0` (defaults, so every earlier model still elaborates);
`ext0` and `ext1St` gain the arms the SP8 signatures need; `check_denotes`'s
constant case re-proved; ten SP8 decision vectors (`decision_vectors_agree`
re-proved); `prim0_kind`, `prim1_kind`, `prim2_kind` and Libevent's
`eval_tls_params` get `maxRecDepth 8192` for the longer vocabulary.
Placeholder-free; no project axioms (the audit's `#print axioms`).

## 9. Locks, reviewed

Order: `providers/contracts.py sync` (`private/logs/sync-1.log`, exit 0
with the expected refusals "stale lock: `rd_kafka_plugin_new`'s contract text
changed" and the same for `flb_plugin_load`); `quirin-audit pin --outer
--source-root <Fluent Bit> --crate C --function F` for each loader at
`046accf6` and then `66910c10` (a `--shared --no-checkout` clone of the local
Fluent Bit checkout under the worktree's ignored `private/`, `HEAD` set to
each revision; `private/pin-1/pin-{rk,fb}-{046accf6,66910c10}.log`, all exit
0); `contracts.py sync` again (`sync-2.log`, exit 0); `pin
--verify-contracts` exit 0 (`private/pin-1/verify.log`). Diff:

* `qcore-provider-librdkafka/outer-contracts.lock`: `rd_kafka_plugin_new`'s
  `contract_sha256` `f51206e1…` → `82dd2dd2…` and `pinned_at`; the same
  sites and the same two revisions.
* `quirin-provider-fluent-bit/outer-contracts.lock`: `flb_plugin_load`'s
  `contract_sha256` `2f961735…` → `51a1dc44…` and `pinned_at`; the same sites
  and revisions; `flb_plugin_proxy_create` unchanged.
* `contract-inventory.lock` of both crates and librdkafka's `contracts.json`:
  the `outer.qv` file digest only (librdkafka `88516636…` → `f3ae5ffa…`,
  Fluent Bit `b91ee5f6…` → `52fc0da6…`); declaration and clause counts
  unchanged.
* `contracts.lock`, `bounds-contracts.lock`, `outer-candidates.json`,
  `providers/contract-load-path.json`: unchanged.

The contract identities `rk_plugin_library@…` and `flb_external_plugin@…`
move with the loader text digests, as predicted; a selected runtime
projection names the new digests and moves no row.

## 10. Files touched

Shared files (the integrator merges them; SP6-GUARDED-SYMBOL-PROSE edits some
of the same): `crates/qcore-frontend/src/premise_formula.rs` (SP8 block
appended after SP5, `outer_header`, `QF004`, vectors),
`proof/QCore/PremiseFormula.lean` (SP8 `vocab` block appended, SP8 decision
vectors appended, `inventories`, two Model components, the constant arm),
`proof/QCore.lean` (one import), `docs/QCORE_CONTRACT_SPEC_r0_3.md` (the SP8
block follows the SP6 block), `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`,
`docs/agents/diagnostics.md` (`QF004` after `QF002`),
`docs/agents/rules/31.md` and the generated `gateway.md`,
`crates/qcore-frontend/tests/premise_formula.rs`,
`tests/support/legacy_prose_premise.rs`. Others:
`crates/qcore-frontend/src/{frontend,pipe}.rs`,
`crates/qcore-contract/src/outer_contract.rs`,
`crates/qcore-contract/{tests/outer_contract.rs,examples/plugin_outer_catalog_replay.rs}`,
`crates/qcore-audit/{src/outer_candidates.rs,tests/outer_projection.rs}`,
`compiler/tests/premise_formula_operators.rs`,
`proof/QCore/PremiseFormula/{Outer,Libevent}.lean`,
`proof/AuditPremiseFormulaOuter.lean`, the two `outer.qv`, the locks above,
and this report.

## 11. Gate

**Gate 1** (`python3 scripts/gate_scope.py --base 95f36b628 --out
private/gate-1` on `e1d9f78d6`; record [gate/gate-1.json](gate/gate-1.json),
console `private/logs/gate-1.log`). Scope: rust suite (24 packages), Lean
all (`proof/QCore.lean` changed), provider gate, Python. Exit 1 only for
the known host failure:

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| clippy (`--all-targets -D warnings`) | 0 | |
| rust-suite | 101 | 677 units, 9908 passed, **1 failed**, 21 ignored |
| reevaluation probe / follow-up | 0 / 0 | |
| lean (`check_lean_audits.py`) | 0 | 486 audits |
| python | 0 | 807 tests |
| provider package build / verify-all / verify | 0 / 0 / 0 | |
| provider test | 0 | 49 summaries, 174 passed, 1 ignored |
| provider fmt / clippy | 0 / 0 | |

The one failure is
`qcore-audit::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`,
the known host failure (curlver.h on this host), recorded and not fixed per
the brief.

**After merging origin/main** (`28d5807fd`: SP6's landing, r57
RK-TOPIC-PARTITION-LIST-CONSTRUCTORS, r58 NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP)
into the card as `3728090f5` (clean merge; disjoint from this card except
the auto-merged `librdkafka` `contract-inventory.lock`/`contracts.json`,
`proof/QCore.lean` and the generated `gateway.md`; the merge brought no
outer statement and no string premise): `providers/contracts.py sync` exit
0 with no change (`private/logs/sync-3.log`); `gen_rule_docs.py --check`
exit 0 (256 rule files); `check_lean_audits.py` exit 0, 487 audits
(`private/logs/lean-merged.log`); targeted Rust: `qcore-frontend`
`premise_formula`, `qcore-contract` `outer_contract` and `contract_premise`,
`qcore-compiler` `premise_formula_operators` and `qcore_contract_corpus`
exit 0, 5 targets, 45 passed (`private/logs/targeted-1.log`);
`qcore-audit` `outer_projection` (6 passed) and `outer_candidates` unit
tests (7 passed) exit 0 (`private/logs/targeted-2.log`). Per the landing
race policy the full gate is the integrator's at landing.

`python3 scripts/apply_amendments.py
docs/reports/sp8-outer-contract-symbolic-statements/integration-amendments.json
--check`: exit 0 (registers and retires SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS; would write the increment page and the acceptance matrix).

## 12. Follow-ups

None registered. The acceptance matrix's derived Lean-audit list gains
`AuditPremiseFormulaOuter.lean` at landing (the integrator's record).
