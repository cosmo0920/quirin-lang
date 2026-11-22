# SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-23: "Launch the outer contract prose card too".

Base `95f36b628` (branch `integrate/sp6-hypothesis-atoms`). Prediction:
[PREDICTION.md](PREDICTION.md) (committed first, `55bc33df5`). The card
discharges SP5's report follow-up 1 (outer contract statements, C04.20, still
English; never registered in the ledger).

**Status: design hand-back, no implementation yet.** The brief says to stop
when a conjunct can be formalised two defensible ways that differ in what is
assumed. Three can (§5: D1 "calls public APIs only under their published
conditions", D2 "forges no pointer", D3 "remain valid through the last
possible invocation"). This report holds the census at base (§1), the form
(§2), the proposed SP8 vocabulary (§3), both statements drafted as formulas
under the recommended options, each with its faithfulness table (§4), the
decisions (§5), the readings settled without a decision and why (§6) and the
implementation plan the decisions unblock (§7). No reader, contract, test,
specification or Lean file has changed.

## 1. Census at base

Pattern `quant #outer \[\[contract\]\] [A-Za-z_{}]+, *\\?"` (a statement
written as source or as a Rust string literal) outside `docs/reports/`, plus
what the pattern cannot see: readers and asserters of the parsed statement,
statement-shaped diagnostics and grammar text. The "After" column is filled
by the implementation.

| File | Role | Base | After |
|---|---|---|---|
| `providers/qcore-provider-librdkafka/contracts/librdkafka/outer.qv` | `rk_plugin_library` definition on `rd_kafka_plugin_new` (production) | 1 string | (formula) |
| `providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv` | `flb_external_plugin` definition on `flb_plugin_load` (production); the citation on `flb_plugin_proxy_create` has no statement | 1 string | (formula) |
| `crates/qcore-contract/tests/outer_contract.rs` | fixture `DEFINITION` (line 39); its mutation table writes the statement back (line 274) and a blank statement (`outer-statement`, line 256); asserts a citation has no statement (line 157) | 2 strings + 1 blank case | (formula fixture; string cases become `QF004` refusal cases) |
| `crates/qcore-audit/tests/outer_projection.rs` | projection fixture (line 279) | 1 string | (formula) |
| `crates/qcore-audit/src/outer_candidates.rs` | reader: `validate_envelopes` scans the statement text for `outer-candidates:HEX`; its unit test builds a statement with `format!` (line 345) and overwrites one (line 375, "neutral callbacks") | 1 `format!` + 1 assignment | (reads the formula's inventory constants) |
| `crates/qcore-contract/src/outer_contract.rs` | parser: `read_clause` admits `NAME, STRING` (`plain_statement`, `outer-statement`), the shape diagnostic spells `NAME, "STATEMENT"` (line 424), module docs (lines 33, 40); `OuterLoader.statement: Option<String>`, `OuterContract.statement: String` | 1 reader, 3 spellings | (formula reader, `QF004`) |
| `crates/qcore-audit/src/outer_selection.rs` | writes `"statement"` into the projection record (line 921) | 1 reader | (canonical printing) |
| `crates/qcore-contract/examples/plugin_outer_catalog_replay.rs` | checks the statement contains `outer-candidates:HASH` (line 139) | 1 reader | (reads the inventory constant) |
| `crates/qcore-frontend/src/frontend.rs` | `SourceContractClauseKind::QuantOuter` doc spells `NAME, "STATEMENT"` (line 184); the structural parser passes the clause through unread | 1 spelling | (`QF004` at parse, every route) |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | C04.20: D2's recorded form `NAME, "text"` (line 2497, verbatim user decision: kept), the example `"STATEMENT"` (line 2503), "defines the contract" (line 2516) | 2 + 1 verbatim | (formula) |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | G05g `[ ",", STRING ]`, `STRING` production, `outer-statement` | 1 production | (formula production, `QF004`) |
| `docs/agents/rules/31.md`, `docs/agents/gateway.md` | rule 31 names the loader and envelope; no statement spelling | 0 | (one sentence: the statement is a formula) |
| `proof/QCore/OuterContract.lean`, `OuterListCandidates.lean` | model no statement content | 0 | (new `QCore.PremiseFormula.Outer`) |
| `providers/*/outer-contracts.lock` | `contract_sha256` of each defining loader's text | digest | (re-pinned, same sites) |
| `docs/reports/plugin-premise-decomposition/outer-catalog.json` | historical outer digests the example replays | digest | historical (report owns its evidence) |

Totals by the pattern: 13 matches in 9 files at base (2 production
statements, 4 test strings or builders, 7 spellings in docs, diagnostics and
comments).

## 2. The form, and why

The statement position takes one C04.18.9 formula:
`quant #outer [[contract]] NAME, FORMULA;`, read by the premise reader
(`qcore_frontend::premise_formula`), canonical printing as identity, the
declaration digest covering the clause as written. Not a conjunction of
named sub-premises: an outer module admits no `resource_premise` clause
(`outer-clause-foreign`), so that would add a G05g clause kind, and the
envelope's `#premise NAME` lines already are the channel for named premises
(they say which closed-world premises the contract vouches for). Not SP6
typed atoms: those are hypothesis-graph renderings of a premise formula's
parts, not statements. One formula is what SP1 to SP5 did for every premise
statement.

Two outer specifics. (a) The formula's subject is the loaded code, not a
call: the SP8 subject term `#outer_code` is relative to the contract (as
`#call` is relative to its declaration). OpenSSL's term `#loaded` (set(fn),
modules OpenSSL loads) is unrelated to the outer keyword `#loaded` and is not
used. (b) The candidate inventory digest becomes a typed constant: a new sort
`inv` whose constants are `CANDIDATES_` followed by the 64 upper-case hex
digits of the inventory's SHA-256 (one `[A-Z][A-Z0-9_]*` token; a bare hex
digest lexes as an integer followed by a word). `outer_candidates` reads every
`inv` constant of the parsed formula, lower-cases it and checks it exactly as
it checks `outer-candidates:HEX` today: a digest naming no inventory refuses
selection, a stale inventory holds the loader, a `#registered` list line whose
inventory the formula does not cite refuses.

## 3. Proposed SP8 vocabulary (appended after the SP5 and SP7 blocks)

One sort is added (`inv`); no token, operator or production. `#inert`,
`#false`, `#child`, `#first`, `#last`, `#pred`, `#next`, `#prev`,
`#next_reach`, `#children` and `#nodup` are reused. An effect's "own
instructions" excludes what a call of a function of the audited program does
(§6 R-a).

| Term | Sort | Meaning |
|---|---|---|
| `#outer_code` | set(fn) | the functions of the code the contract's loaders load (what each loader hosting the contract opens: `rd_kafka_plugin_new` a library `plugin.library.paths` names; `flb_plugin_load` a C shared object; `flb_plugin_proxy_create` a Go plugin), and every function outside the audited program a run of one of them runs, directly or in turn (its dependencies and the C library included) |
| `#conf_registrations` | set(fn) | the callbacks code of `#outer_code` stores, directly or through a call, into an rk_conf callback member (a function pointer member of `rd_kafka_conf_s`, `rk->rk_conf`'s included) |
| `#public_setter_values` | set(fn) | each value a call of a public librdkafka setter (a function librdkafka's public headers declare that stores a callback member; the `rd_kafka_conf_set_*_cb` functions among them) stores into its member |
| `#meets_candidate` | set(fn) | the callbacks that meet the candidate contract (C04.19.3, C04.19.7) declared for the member each is stored in |
| `#interceptors` | set(fn) | the interceptor methods code of `#outer_code` registers, each as the element of the `rd_kafka_conf_s.interceptors` list (or its `rk_conf` alias) it is appended to |
| `#zero_appends(i)` | inv → set(fn) | for each candidate of the inventory `i` (list member, aliases, element record, union arm, setter): the elements a call of that candidate's public setter appends to exactly that list under exactly that union arm, in a call that returns 0 |
| `#conf_lifecycle(i)` | inv → set(fn) | the elements appended under a candidate of `i` whose lifecycle is `configuration` |
| `#client_lifecycle(i)` | inv → set(fn) | the elements appended under a candidate of `i` whose lifecycle is pre-initialization, non-share |
| `#on_live_conf` | set(fn) | the interceptors installed by a call handed a live configuration (allocated, not yet destroyed) |
| `#from_on_new` | set(fn) | the interceptors installed by a call made during a run of an `on_new` interceptor |
| `#before_init` | set(fn) | the interceptors installed before the initialization of the client the call is handed completes |
| `#on_share_consumer` | set(fn) | the interceptors installed on a share consumer client |
| `#copies(S)` | set(fn) → set(fn) | the members of `S` and every registration the library copies from one of them into another configuration or client (`rd_kafka_conf_dup`, the copied `on_new` registrations) |
| `#valid_through_last` | set(fn) | (D3 option B) the registrations whose code stays loaded and whose opaque context stays valid from registration until the last point the library can invoke them: the end of the destruction of the configuration or client holding them, destruction callbacks included |
| `#borrows_args` | eff | every pointer argument of the run, and each object reachable from it, is borrowed: the run, with every call it makes, takes over, releases, retains and transfers none of them (documented borrowed output-buffer writes allowed) |
| `#borrows_handles` | eff | every scalar resource handle argument of the run (a descriptor such as `sockfd` among them) is borrowed: no close, release, duplication or transfer |
| `#no_global_custody` | eff | the run, with every call it makes, changes the custody of no object through global state |
| `#balanced` | eff | every resource the run, with every call it makes, acquires is released before the run returns |
| `#stores_none(S)` | set(obj) → eff | the run stores, by its own instructions, into no cell of `S` |
| `#calls_none(S)` | set(fn) → eff | the run calls, by its own instructions, no member of `S` |
| `#rk_internal` | set(obj) | the member cells of librdkafka's internal records (those its public headers `rdkafka.h` and `rdkafka_mock.h` do not define): callback members, `rd_kafka_op_s` members, broker links, the rule-14t members, queues (`rd_kafka_q_s`) and list links (`rd_list_t`, TAILQ links) among them |
| `#rk_public` | set(fn) | the functions librdkafka's public headers declare |
| `#rk_nonpublic` | set(fn) | librdkafka's functions its public headers do not declare |
| `#meets_published(S)` | set(fn) → eff | (D1 option A) every call the run makes to a member of `S` meets the conditions that function's published documentation states |
| `#cjson_init_hooks` | set(fn) | `cJSON_InitHooks` |
| `#cjson_global_hooks` | set(obj) | the cells of cJSON's `global_hooks` (allocate, deallocate, reallocate) |
| `#cjson_link_cells` | set(obj) | the `child`, `prev` and `next` member cells of every cJSON item |
| `#cjson_handed` | set(obj) | the cJSON items code of `#outer_code` passes to a cJSON API call, and every item reachable from one through `child` and `next` |
| `#allocator_hooks_premise` | prop | every statement of the named premise `cjson_allocator_hooks` (C04.18.9; on `cJSON_New_Item` and on `print`) holds of the hooks installed before the loaded code loads (libc's when none were) |
| `#flb_core_owned` | set(obj) | the member cells Fluent Bit's core owns of every `struct flb_*_instance` (input, output, filter, processor, custom among them) and `struct flb_config` |
| `#handed` | set(obj) | the objects the contract's loaders hand to loaded code and every object reachable from one (C04.20's reach), the objects an exported API returns to it or a callback it registered is handed, and the objects it allocates or defines itself |
| `#exported` | set(fn) | the functions the Fluent Bit binary exports (`ENABLE_EXPORTS ON`) |
| `#accesses_only(S)` | set(obj) → eff | every object of the audited program the run reads or writes by its own instructions is a member of `S` |
| `#calls_only(S)` | set(fn) → eff | every function of the audited program the run calls by its own instructions is a member of `S` |
| `#provenance(S)` | set(obj) → eff | (D2) every pointer the run dereferences, stores or passes is derived (copied, offset within its object, converted to `void *` and back) from a pointer to an object of `S` |
| `#typed_access` | eff | (D2 option B) the run reads and writes each object only at its own type or a type C permits for it (character type, the record embedding it): no conversion between unrelated record pointers |

## 4. The statements as formulas (recommended options)

`H` is `CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147`.
`W(#a)` is the list shape of the representation premise
`detach_list_representation` (SP2), its item generalised from `#parent` to
`#a`, its liveness, custody and concurrency conjuncts dropped:

```text
W(#a) = (quant all #c in #child(#a): #c in #first(#a)) && (quant all #c in #first(#a): #c in #child(#a)) && (quant all #b in #children(#a): quant all #p in #pred(#a, #b): #b in #next(#p)) && (quant all #c in #children(#a): #c in #next_reach(#c) -> #false) && ((quant all #c in #child(#a): #false) -> quant all #c in #children(#a): #false) && ((quant all #c in #children(#a): #false) -> quant all #c in #child(#a): #false) && (quant all #b in #children(#a): quant all #p in #pred(#a, #b): #p in #prev(#b)) && (quant all #f in #first(#a): quant all #l in #last(#a): #l in #prev(#f)) && #nodup(#a) && (quant all #c in #children(#a): quant all #y in #next(#c) | #prev(#c): #y in #children(#a)) && (quant all #l in #last(#a): quant all #y in #next(#l): #false) && (#a in #children(#a) -> #false)
```

Both drafts, `W` expanded and `H` written as a constant, were read by the
current syntax reader (`premise_formula::parse_tokens`, a throwaway probe
test, removed): 568 and 604 tokens (bound 1024), within the node and depth
bounds, canonical printing as shown. The sort check needs the SP8 block.

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

| # | Prose | Formula part |
|---|---|---|
| R0 | "every library named by plugin.library.paths, including code it invokes" | the subject `#outer_code` (the loader `rd_kafka_plugin_new` opens each named library; invoked code outside the audited program is a member) |
| R1 | "registers rk_conf callbacks only through public setters and only when each callback meets that member's candidate contract" | `quant all #f in #conf_registrations: #f in #public_setter_values && #f in #meets_candidate` |
| R2 | "registers interceptors only through the public setters, with success result zero and each exact list/union-arm candidate in outer-candidates:5c3f…" | `quant all #f in #interceptors: #f in #zero_appends(H)` (the public setter, the zero result, the exact list and union arm are each in `#zero_appends`'s meaning; `H` is the checked digest) |
| R3 | "every registered interceptor and all code it invokes meets that inventory's neutral custody restriction" | `quant all #f in #interceptors: #f |-> (…)`, the effects of a run with every call it makes |
| R3a | "(all pointer arguments borrowed" | `#borrows_args` |
| R3b | "no release, retention, ownership transfer" | `#inert` (of what the caller holds) and `#borrows_args` (of the arguments) |
| R3c | "or global custody effects" | `#no_global_custody` |
| R3d | "scalar resource handles borrowed too" | `#borrows_handles` |
| R3e | "and every local acquisition balanced before return)" | `#balanced` |
| R4 | "registered callback code and opaque contexts remain valid through the last possible invocation, including destruction and copied on_new registrations" | `quant all #f in #copies(#conf_registrations | #interceptors): #f in #valid_through_last` (D3; "copied on_new registrations" are `#copies`' members, "destruction" is inside `#valid_through_last`'s interval) |
| R5 | "configuration interceptors are installed on a live configuration" | `#f in #conf_lifecycle(H) -> #f in #on_live_conf` |
| R5' | "and client interceptors only from on_new before initialization completes, never on share consumers" | `#f in #client_lifecycle(H) -> #f in #from_on_new && #f in #before_init && #f in #client_lifecycle(H) \ #on_share_consumer` |
| R6 | "it calls cJSON_InitHooks nowhere" | `#outer_code |-> #calls_none(#cjson_init_hooks)` (nowhere: every member of the subject) |
| R7 | "and does not replace cJSON's global allocator hooks" | `#stores_none(#cjson_global_hooks)` (the only other way to replace them) |
| R8 | "it neither mutates cJSON child, prev or next links directly" | `#stores_none(#cjson_link_cells)` |
| R9 | "nor passes malformed cJSON lists to an API call" | `quant all #a in #cjson_handed: W(#a)` |
| R10 | "it never directly stores into librdkafka internal members (including callback fields, op fields, broker and rule-14t links, queues and list links)" | `#stores_none(#rk_internal)`; the parenthetical is an open list: each item is a member row of `#rk_internal`'s class |
| R11 | "it invokes no non-public librdkafka symbol" | `#calls_none(#rk_nonpublic)` |
| R12 | "and calls public APIs only under their published conditions" | `#meets_published(#rk_public)` (D1) |
| R13 | "any allocator hooks installed before this library loads satisfy cjson_allocator_hooks" | `#allocator_hooks_premise` (a citation of the named premise, not a restatement: its two statements differ) |

### 4.2 `flb_external_plugin`

```text
FB = #outer_code |-> (#stores_none(#flb_core_owned) && #accesses_only(#handed) && #calls_only(#exported) && #provenance(#handed) && #typed_access)
  && RK
```

`RK` is the formula of §4.1 verbatim (with this contract's `#outer_code`).
Every librdkafka and cJSON conjunct of the Fluent Bit sentence is one of
R1 to R13, and the sentence states each of them, so the relation the prose
states ("when it uses librdkafka", the librdkafka obligations) is
`FB -> RK` in every model, proved from the two formulas in Lean (§7).

| # | Prose | Formula part |
|---|---|---|
| F0 | "every Fluent Bit plugin loaded from outside the build (a C shared object through flb_plugin_load or a Go plugin through flb_plugin_proxy_create), including code it invokes" | the subject `#outer_code`; the parenthetical's two rows are the contract's two hosting loaders (a checked fact: `contracts.loaders`) |
| F1 | "stores into no core-owned flb_*_instance or flb_config member" | `#stores_none(#flb_core_owned)` (§6 R-a: its own stores) |
| F2 | "it reaches only handed objects and exported APIs" | `#accesses_only(#handed) && #calls_only(#exported)` |
| F3 | "and forges no pointer" | `#provenance(#handed) && #typed_access` (D2 option B) |
| F4 | "when it uses librdkafka it registers rk_conf callbacks only through public setters and only when each callback meets that member's candidate contract" | R1 (vacuous when it registers none) |
| F5 | "registers interceptors only through public setters" | R2 |
| F6 | "never directly stores into librdkafka internal members (including …)" | R10 |
| F7 | "invokes no non-public librdkafka symbol, and calls public APIs only under their published conditions" | R11, R12 |
| F8 | "it calls cJSON_InitHooks nowhere and does not replace cJSON's global allocator hooks" | R6, R7 |
| F9 | "neither mutates cJSON child, prev or next links directly nor passes malformed cJSON lists to an API call" | R8, R9 |
| F10 | "and any allocator hooks installed before it loads satisfy cjson_allocator_hooks" | R13 |
| F11 | "every interceptor registration uses the public setter with success result zero and the exact list/union-arm candidate in outer-candidates:5c3f…" | R2 |
| F12 | "every registered interceptor and its transitive calls meets the neutral custody restriction (…)" | R3 (R3a to R3e) |
| F13 | "registered callback code and opaque contexts remain valid through the last possible invocation, including destruction and copied on_new registrations" | R4 |
| F14 | "configuration interceptors are installed on a live configuration and client interceptors only from on_new before initialization completes, never on share consumers" | R5, R5' |

## 5. Decisions (each differs in what is assumed)

**D1. "calls public APIs only under their published conditions"** (R12, F7).
* A (recommended): the conditions librdkafka's published documentation
  states for each public function (`#meets_published(#rk_public)`). It is the
  literal "published" and what a reviewer checks a plugin against.
* B: the conditions the declarations in force in the provider crates state
  (`requires` clauses and parameter modes; `#meets_declared(#rk_public)`).
  Machine-relatable to the analysis, but a provider declaration is not a
  published condition, and a public function with no declaration in force
  would carry no condition at all (narrower than the prose there).
* C: both conjuncts (assumes the most).
* Scope sub-question: "public APIs" read as librdkafka's (recommended: the
  phrase follows "no non-public librdkafka symbol", and in the Fluent Bit
  sentence it sits under "when it uses librdkafka"; cJSON use has its own
  conjuncts), or as every library's public API the code calls (cJSON, the C
  library, Fluent Bit's exported functions; `#meets_published(S)` with a wider
  `S`).

**D2. "forges no pointer"** (F3, Fluent Bit only).
* A: provenance only (`#provenance(#handed)`): every pointer the code uses is
  derived from a handed object, an exported API's result or its own object.
* B (recommended): provenance and typed access (`#provenance(#handed) &&
  #typed_access`): also no conversion between unrelated record pointers.
  C04.20's Reach paragraph says "forged pointers and conversions between
  unrelated record pointers are what the statement excludes", and the reach
  graph (typed pointer edges only) is sound only under both.

**D3. "remain valid through the last possible invocation, including
destruction and copied on_new registrations"** (R4, F13).
* A: per run: every run of a registration or of a copy finds its code loaded
  and its opaque context valid (an effect `#code_context_valid`, written
  `quant all #f in #copies(…): #f |-> #code_context_valid`).
* B (recommended): an interval: code and context valid from registration
  until the last point the library can invoke the registration, the end of
  the destruction of every configuration or client holding it or a copy
  (`#valid_through_last`). It is the literal "through", it entails A, and it
  is the inventory's `registration_lifetime` ("valid through their last
  possible invocation, including configuration/client destruction"). A
  assumes less (it says nothing between runs, for example of a `dlclose`
  while the holder lives but before its next invocation).

## 6. Readings settled without a decision

* **R-a, "including code it invokes" excludes the audited program.** The
  subject closes over the outside code a plugin runs (its dependencies, the C
  library), and each effect is the code's own instructions. Reading the
  audited program's functions in would make R10 false of every public setter
  (each stores into an internal callback member) and F1 false of every input
  plugin (`flb_input_log_append` writes the instance's core-owned counters),
  so it is not a defensible reading of either sentence; the Fluent Bit
  sentence's "stores into" is therefore the same direct store the librdkafka
  sentence calls "directly stores".
* **Neutral custody is a property of each interceptor run as a whole**, with
  every call it makes (C04.20.1, the inventory's `review`): a per-callee
  reading of "all pointer arguments borrowed" contradicts "every local
  acquisition balanced" (the callee that releases a local acquisition).
* **"malformed cJSON lists"** is the negation of the full list shape `W`
  (SP2's representation conjuncts), never only `cjson_list_invariant`'s two
  conjuncts, which would be narrower than "malformed".
* **"handed objects"** include what an exported API returns and what a
  registered callback is handed, and the code's own objects: without them
  every plugin that reads an event chunk violates F2.
* **The cjson_allocator_hooks citation** is a proposition term naming the
  premise, so the statement cites both of its statements rather than choosing
  one to restate.

## 7. Implementation plan (unblocked by D1 to D3)

1. `qcore_frontend::premise_formula`: SP8 block appended after SP7 (sort
   `inv`, its constant family, the terms of §3 as decided), decision vectors;
   `QF004` (`PROSE_OUTER`) with an `outer_header` reader; the structural
   parser (`frontend.rs`) refuses a string header by name on every route;
   the operator gate (`pipe.rs`, `premise_formula_spans`) admits a reader-
   accepted outer header's `&&`, `|`, `->`, `\`.
2. `qcore_contract::outer_contract`: the header reads a formula (statement =
   canonical printing; the normalised text renders it canonically), the
   shape diagnostic spells `NAME, FORMULA`, `outer-statement` reports the
   formula reader's refusal, module docs.
3. `qcore_audit::outer_candidates` reads the formula's `inv` constants;
   `outer_selection` prints the canonical formula; the catalog example reads
   the constant.
4. The two `outer.qv` statements as decided; `providers/contracts.py sync`
   and `quirin-audit pin --outer` re-pinning (same sites and revisions), the
   reviewed lock diff in the same commit.
5. Tests: fixtures to formulas; the string cases become `QF004` refusal
   cases on the frontend, outer loader, resource loader and compiler routes;
   the legacy reader lifts the two recorded statements (a `migration.json`
   with both sentences and formulas), same confinement test.
6. Specification C04.18.9 (SP8 block, outer statements), C04.20, G05g and
   G05-formula, rule 31, diagnostics page.
7. Lean `proof/QCore/PremiseFormula/Outer.lean`: the SP8 vocabulary, both
   formulas well-formed, decision vectors and the `QF004` refusal bound to
   the Rust table, `fb_entails_rk : eval FB → eval RK` in every model;
   placeholder-free, no project axioms.
8. Report completion, `integration-amendments.json` (register and retire
   SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS), `apply_amendments.py --check`,
   card gate.
