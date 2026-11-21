# SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES

Base `1cd7da6b8` (origin/main after the SP1 landing records). User decision
2026-11-20 (verbatim): "we don't want to write down the sentences on premise
clauses. We really dislike this kind of results so we must write down as
condition symbols like |-> or other types of logical symbols to represent the
prerequisites which compose for premise conditions." SP1 converted OpenSSL;
user 2026-11-21: "Launch SP2–SP4". This card converts
`quirin-provider-fluent-bit` and audits `quirin-provider-libc`.

[Prediction](PREDICTION.md) (committed first, `83fc4bdc9`): no status move on
any route; content changes only where a premise is rendered; the lock diff is
three module digests and three re-pinned entries.

## 1. Census

Counted per clause, comments excluded (`grep` of `requires resource_premise(`
after stripping `//` comments; the crates' new tests assert the same counts).

| Crate | File | Premise | Prose at base | Prose at tip | Formula at tip |
|---|---|---|---|---|---|
| quirin-provider-fluent-bit | `fluent_bit/input_chunk_name.qv` | `untracked_handback` | 1 | 0 | 1 |
| quirin-provider-fluent-bit | `fluent_bit/loop_owner.qv` | `ingestion_active` | 1 | 0 | 1 |
| quirin-provider-fluent-bit | `fluent_bit/output_singleplex.qv` | `synchronous_output_reachability` | 1 | 0 | 1 |
| quirin-provider-libc | `libc/errno.qv` | `nested_call_admission` (2 declarations) | 0 | 0 | 0 (two `resource_premise_all`) |

**Census correction.** The brief counted 5 Fluent Bit clauses (2 per name)
and 2 libc `nested_call_admission` prose clauses that "SP1 missed". The
recount agrees with SP1's ledger instead: each Fluent Bit name is stated once
(its second mention is the module comment that cites the clause), and the
libc premises on `__errno_location` and `strerror` are
`resource_premise_all(nested_call_admission, ...)` conjunctions of typed
`quant proposition` atoms (card QUANT-CONTRACT-HYPOTHESES, C04.18.7), whose
erased statement is `all(checked_reading_route,scalar_declaration,untracked_effects)`
and `all(checked_nested_site,...)`, not a sentence. errno.qv holds no string
literal outside comments. So nothing in libc is converted; the libc ledger
item is registered and discharged with this finding (§9), and the libc crate
gains the prose ban.

## 2. What changed

* **Vocabulary** (C04.18.9's table, `qcore_frontend::premise_formula::VOCABULARY`,
  Lean `vocab`): one block of 20 terms headed by this card, appended after
  `#inert`; no existing entry moved or changed. No grammar change and no new
  sort: 16 terms are sets of objects, 4 are propositions (`#faithful`,
  `#admitted`, `#returns_zero`, `#production`; the first terms of sort `prop`).
  The language has no negation, so "not in `T`" is written `#x in S \ T` with
  `#x` drawn from `S`.
* **The three premises** state formulas (§3). The modules lay each out one
  step per line; the comments beside each declaration now say what the
  formula states and keep the prose's citations.
* **Prose bans.** Neither crate has a checker binary (both are data crates,
  `src/lib.rs` only), so the nearest equivalent is each crate's own test over
  its `CONTRACTS` table: `quirin-provider-fluent-bit`'s
  `every_premise_is_a_formula` refuses a `resource_premise` whose statement is
  a string literal and asserts the three names; `quirin-provider-libc`'s
  `no_premise_is_prose` refuses a string `resource_premise` or
  `resource_hypothesis` and asserts the two `nested_call_admission`
  conjunctions. The loader still admits the string form elsewhere (tests and
  fixtures); the global ban is SP5 (§9).
* **The compiler's operator gate.** The Fluent Bit modules are in the
  contract corpus, which `compiler/tests/qcore_contract_corpus.rs` compiles
  module by module; the OpenSSL modules are not, so SP1's formulas never met
  the frontend's operator gate. Gate 1 found it: `Q038` refused every `&&`
  and `|` of the `untracked_handback` formula. The gate now admits those two
  operators inside a `requires resource_premise(NAME, FORMULA);` clause whose
  statement the premise reader accepts as a formula, and nowhere else
  (`pipe.rs`, `premise_formula_spans`, after the `quant checks` and
  `result_set` precedents; grammar G05-formula records it). A string
  statement or a refused formula gets no span. Test
  `compiler/tests/premise_formula_operators.rs` (both directions).
* **Semantics unchanged.** Premises stay undischarged metadata: no consumer
  that decides a verdict reads them, and the three declarations are re-pinned
  at the revisions and sites they were pinned at (§6).

## 3. Vocabulary additions and formulas

| Term | Sort | Meaning |
|---|---|---|
| `#result` | set(obj) | the object the call's result designates (no object for a null or scalar result) |
| `#interior(S)` | set(obj) → set(obj) | the objects the objects of `S` own, directly or in turn |
| `#chunk_name(S)` | set(obj) → set(obj) | the name (`ch->name`) of the ChunkIO chunk (`ic->chunk`) each input chunk of `S` holds |
| `#tracked` | set(obj) | the objects the resource projection holds a custody state for |
| `#claimed` | set(obj) | the objects whose custody or lifetime a clause of the declaration states (a `resource_return_none` result is not one) |
| `#bound` | set(obj) | the objects the caller binds to a name at the call's site |
| `#stored` | set(obj) | the objects the caller stores at the call's site where they outlive the statement |
| `#faithful` | prop | the declaration's result clause states exactly the custody the caller acquires at the site |
| `#admitted` | prop | the gateway admits the call under its None-result, all-Borrowed rule (O04-TRACE N6, N8) |
| `#config_of(S)` | set(obj) → set(obj) | the `struct flb_config` each input instance of `S` belongs to |
| `#ingesting` | set(obj) | the contexts whose `is_ingestion_active` is true when the call reads it |
| `#collector` | set(obj) | the collector the call finds by its id (`get_collector`); none when absent |
| `#event_registered` | set(obj) | the collectors whose own event the call registers with their loop |
| `#set_running` | set(obj) | the collectors whose `running` the call sets true |
| `#returns_zero` | prop | the call returns 0 |
| `#in_tree` | set(obj) | the output plugins Fluent Bit's own tree defines (`plugins/out_*`) |
| `#synchronous` | set(obj) | the output plugins whose flags carry `FLB_OUTPUT_SYNCHRONOUS` |
| `#proxy` | set(obj) | the output plugins a proxy registers (outside the tree, through `flb_plugin_proxy`) |
| `#production` | prop | the call is made by a running engine from its output dispatch, not by a test or another embedding |
| `#out_plugin(S)` | set(obj) → set(obj) | the output plugin (`p`) of each output instance of `S` |

The specification's table (C04.18.9) carries the full meanings; this one
abbreviates them. The canonical printings:

* `untracked_handback`:
  `quant all #r in #result: #r in #interior(#args) && #r in #chunk_name(#args) && #r in #result \ (#tracked | #claimed) && (#faithful -> #r in #result \ (#bound | #stored)) && (#r in #result \ (#bound | #stored) -> #faithful) && (#admitted -> #r in #result \ (#bound | #stored))`
* `ingestion_active`:
  `quant all #g in #config_of(#args): (#g in #config_of(#args) \ #ingesting -> #returns_zero && quant all #c in #collector: #c in #collector \ (#event_registered | #set_running)) && (#g in #ingesting -> quant all #c in #collector: #c in #event_registered && #c in #set_running) && (#returns_zero -> quant all #c in #collector: #c in #set_running -> #g in #ingesting)`
* `synchronous_output_reachability`:
  `(quant all #p in #in_tree: #p in #in_tree \ #synchronous) && (#production -> quant all #p in #out_plugin(#args): #p in #synchronous && #p in #proxy)`

`#args` is SP1's term with the same meaning (here `ic`, `in`, and
`queue`/`retry`/`task`/`out_ins`/`config`).

## 4. Faithfulness: prose to formula

**`untracked_handback`** (`flb_input_chunk_get_name`):

| Prose | Formula |
|---|---|
| "the result" | `quant all #r in #result` (the object it designates) |
| "aliases into ic" | `#r in #interior(#args)` |
| "(it is ic->chunk->name)" | `#r in #chunk_name(#args)` |
| "and this projection does not track it" | `#r in #result \ (#tracked \| …)` |
| "resource_return_none is faithful" | `#faithful` |
| "only where the result is never bound or stored" | `#faithful -> #r in #result \ (#bound \| #stored)` and, reading R-U1, `#r in #result \ (#bound \| #stored) -> #faithful` |
| "which the gateway's admission of a call with a None result and only Borrowed parameters checks per site" | `#admitted -> #r in #result \ (#bound \| #stored)`; the rule's scope is `#admitted`'s meaning |
| "(card O04-TRACE premises N6 and N8)" | `#admitted`'s meaning names them; the module comment cites them |
| "the alias itself is outside every claim here" | `#r in #result \ (… \| #claimed)` |
| "(O00 B03/R13, O04-TRACE-HANDBACK)" | provenance: the module comment cites both; no assumption |

**`ingestion_active`** (`flb_input_collector_resume`):

| Prose | Formula |
|---|---|
| "C3:" | provenance (the module's "C3's ingestion premise") |
| "config->is_ingestion_active" | `#g` over `#config_of(#args)`, `#ingesting` |
| "while … is false" | `#g in #config_of(#args) \ #ingesting ->` |
| "the call returns 0" | `#returns_zero` |
| "without resuming the collector" | `quant all #c in #collector: #c in #collector \ (#event_registered \| #set_running)` (neither part of a resume) |
| "(src/flb_input.c:3221 at 66910c10, :3241 at 046accf6 and 34155955)" | provenance: now in the module comment |
| "otherwise" | `#g in #ingesting ->` |
| "it re-registers the collector's own event" | `#c in #event_registered` |
| "and sets running" | `&& #c in #set_running` |
| "a return of 0 means the collector runs only while ingestion is active" | `#returns_zero -> quant all #c in #collector: #c in #set_running -> #g in #ingesting` (reading R-I1); entailed by the first conjunct (§5) |

**`synchronous_output_reachability`** (`flb_output_task_singleplex_enqueue`):

| Prose | Formula |
|---|---|
| "in-tree output plugins" | `quant all #p in #in_tree` |
| "never set FLB_OUTPUT_SYNCHRONOUS" | `#p in #in_tree \ #synchronous` |
| "so this function runs in production" | `#production ->` |
| "only for an output" | `quant all #p in #out_plugin(#args)` (the plugin of `out_ins`) |
| "that does" | `#p in #synchronous` |
| "(a proxy plugin)" | `&& #p in #proxy` (reading R-S1) |
| "so" (the inference) | entailed: in production the call's plugin is not in-tree (§5) |
| "any verdict on it holds under that premise" | the clause itself: a named premise is published with every row resting on its declaration (C04.18, C06.5); no conjunct |

**Readings recorded** where a phrase admits more than one parse:

* **R-U1** ("faithful only where ..."). Literally a necessary condition. It is
  read as "exactly where" (both directions): the module comment's "Two
  separate things keep that from being a fail-open" and "which the gateway ...
  checks per site" say callers' verdicts rest on the mode being faithful
  where the gateway admits the call, which needs the sufficient direction.
  The formula holds both, so it is never narrower than either reading; the
  necessity-only alternative would be the first implication alone.
* **R-U2** (null result). The quantifier ranges over the object the result
  designates; a null result designates none, and a null handback carries no
  custody, so the clauses about the alias and faithfulness are vacuous there.
* **R-I1** ("the collector runs"). Read as the call setting it running
  (`#set_running`): a collector already running before the call would keep
  running whatever the flag, so reading "runs" as "is running after the call"
  would assume more than the body reading states.
* **R-I2** (presupposition). "returns 0 ... while false" is kept unconditional
  (`#returns_zero` is not under the collector quantifier), as written: a
  per-found-collector reading would assume less where no collector has the
  id.
* **R-S1** ("(a proxy plugin)"). Stated as a conjunct, so the formula assumes
  it whether the parenthetical identifies the outputs that set the flag or
  glosses them by example.

## 5. Lean

`QCore.PremiseFormula` gains one block (`FluentBitTerms`, the Model field
`fb`, the 20 `vocab` rows, their `prim0`/`prim1` arms and 12 decision
vectors). Its three vocabulary-kind proofs (`prim0_kind`, `prim1_kind`,
`prim2_kind`) now split the vocabulary membership with
`repeat' (obtain ⟨rfl, hs⟩ | hm := hm)` over the disjunction ending in `False`,
instead of a pattern of fixed length, so a card appending terms changes no
proof. `QCore.PremiseFormula.FluentBit`, audited by
`proof/AuditPremiseFormulaFluentBit.lean`:

* `fluent_bit_formulas_wf`: the three formulas and the ingestion premise
  without its last conjunct are well-formed (`decide`);
* `eval_untracked`, `eval_ingestion`, `eval_ingestion_core`,
  `eval_synchronous`: what each states, unfolded;
* `untracked_admitted_faithful`: where the gateway admits the call, the
  result clause is faithful for the result's object;
* `ingestion_core_entails_premise`: the ingestion premise's first two
  conjuncts entail the whole, so "a return of 0 means ..." follows;
* `synchronous_output_not_in_tree`: in production the call's output plugin is
  not an in-tree one, the prose's "so".

The base theorems (`check_denotes`, `decision_vectors_agree`, the OpenSSL
results) hold over the extended vocabulary and table. Standard axioms only
(`propext`, `Classical.choice`, `Quot.sound`). The Rust test
`the_fluent_bit_formulas_are_the_lean_definitions` binds the three Lean trees
to the clauses read from the modules; `the_vocabulary_is_the_lean_table` and
`the_decision_vectors_are_the_lean_table` bind the block (their row reader
now skips a `--` comment line, the block's header).

## 6. Locks, reviewed

Order: `contracts.py sync` (exit 1, `private/logs/sync-1.log`: "stale lock"
for exactly the three declarations, the expected refusal; it wrote the
inventory lock), then `quirin-audit pin` (the tip's debug binary) at the three
prepared trees each declaration was pinned at:

| Tree (HEAD) | Functions | Log |
|---|---|---|
| system checkout, `046accf6` | all three | `private/logs/pin-1-046accf6.log`, exit 0 |
| corpus checkout, `66910c10` | all three | `private/logs/pin-2-66910c10.log`, exit 0 |
| tail-pause checkout, `34155955` | `flb_input_collector_resume` | `private/logs/pin-3-34155955.log`, exit 0 |

then `contracts.py sync` (exit 0, `sync-2.log`) and `contracts.py check`
(exit 0, `contracts-check-1.log`). Against base:

* `contract-inventory.lock`: exactly the `sha256` of the three modules.
* `contracts.lock`: exactly the three entries' `contract_sha256` and
  `pinned_at`. Their identities (reviewed revisions and every site digest)
  are byte-identical to base; the other 83 entries, the order and every
  top-level key are unchanged; nothing is added or removed.
* `bounds-contracts.lock`, `outer-contracts.lock`, the libc locks and
  `providers/contract-load-path.json` are unchanged (the Fluent Bit crate
  publishes no module).

## 7. Measurement

All four pinned routes load the Fluent Bit crate's resource view (corpus,
twelve-home through the corpus inputs, system and slice), so the predicted
content change is real there: a row resting on `flb_input_chunk_get_name`,
`flb_input_collector_resume` or `flb_output_task_singleplex_enqueue` lists the
same premise name with the canonical formula as its statement, and any digest
over that text (reading-site environment texts, the contract bundle) moves
with it. No status move is predicted (`measure-prediction` in the
prediction). The measurement is the integrator's `quirin-audit measure
--declared docs/reports/sp3-symbolic-fluent-bit-libc-premises/PREDICTION.md`.

## 8. Gate

**Gate 1** (`python3 scripts/gate_scope.py --base 1cd7da6b8 --out
private/gate-1` on `19cf85020`, mode full; [record](gate/gate-1.json), log
`private/logs/gate-1.log`) exited 1. Rust: 624 units, 8335 passed, 2 failed:
the recorded host failure `qcore-audit::test::libcurl_user_crate_versions`
(system `curlver.h`, not fixed here) and
`qcore-compiler::test::qcore_contract_corpus`, this card's: the operator gate
refused the formula's `&&` and `|` (§2). Lean 469 audits, Python 779 tests,
provider build, verify, tests (166 passed, 1 ignored) and clippy exit 0.
`provider-fmt` exited 1 on `compiler/tests/premise_formula_operators.rs`,
the fix's test, which I wrote into the tree while the gate's later steps ran;
gate 1 is kept as the attempt that found the defect, not as evidence for the
tip.

**Gate 2, the gate of record** (`python3 scripts/gate_scope.py --base
1cd7da6b8 --out private/gate-2` on `670f0e6e3`, mode full, clean tree;
[record](gate/gate-2.json), log `private/logs/gate-2.log`) exited 1, from the
known host failure only:

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| rust-suite | 101 | 625 units, 8338 passed, 1 failed (`libcurl_user_crate_versions`, the recorded host failure), 20 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 469 audits |
| python | 0 | 779 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 166 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The matrix's Lean-audit count is not touched here; the integrator's matrix
rebuild carries the new audit.

Targeted runs before the gates (logs under `private/logs/`): the
`premise_formula` test (8 passed), `qcore-frontend` (305 passed), the
`qcore-contract` premise and hypothesis tests (37 passed), both provider
crates' tests (7 passed), the compiler's `premise_formula_operators`,
`qcore_contract_corpus`, `pipe_operator` and `result_set_operators` (52
passed), `lake build QCore.PremiseFormula.FluentBit` and the new audit
(standard axioms only), `contracts.py check` and `apply_amendments --check`,
each exit 0.

## 8a. Merge of the SP2 landing (`17b0e77b2`)

On the integrator's instruction the card merged `17b0e77b2` (origin/main
`70e10930f` plus SP2-SYMBOLIC-LIBRDKAFKA-PREMISES) and ran fast checks only,
no full gate. Resolution:

* **Vocabulary, decision vectors, spec table, Lean `vocab`:** SP2's block
  first, this card's appended after it, both whole (no name collides). The
  spec's Fluent Bit rows are their own table after SP2's.
* **`pipe.rs`:** both cards added `premise_formula_spans`; one is kept, this
  card's, because it is strictly tighter (SP2's exempted every clause headed
  `resource_premise(`; this one only a clause the premise reader accepts as a
  formula). The two compiler tests are one file covering both cards' cases.
* **Lean:** SP2's length-independent `prim*_kind` proofs are kept; `Model`
  has SP2's defaulted components and this card's `fb`, now defaulted to
  `FluentBitTerms.none U` the same way. The merged decision-vector table
  exceeds the default recursion depth of `decide`, so
  `decision_vectors_agree` runs under `set_option maxRecDepth 8192` (same
  proof, standard axioms).
* **Rule 13h, spec prose, G05-formula:** SP2's amendment then this card's;
  the negation sentence now names both spellings (`#x in S \ T` here, SP2's
  `P -> #false`).

Fast checks on the merged tree, each exit 0 (logs `private/logs/merge-*`):
`cargo test -p qcore-frontend --lib --test premise_formula --test
premise_formula_librdkafka` (61, 8, 2 passed); `cargo test -p qcore-compiler
--test premise_formula_operators --test qcore_contract_corpus` (2, 6); the
Fluent Bit and libc provider tests (4, 3); `lake build QCore.PremiseFormula
QCore.PremiseFormula.FluentBit QCore.PremiseFormula.Librdkafka` (after the
`maxRecDepth` line; the first attempt failed on it); the base, librdkafka and
Fluent Bit audits (40 declarations, standard axioms only);
`contracts.py check`; `cargo fmt --check` for both workspaces; clippy on
`qcore-frontend` and `qcore-compiler`; `apply_amendments --check` with base
`17b0e77b2`.

## 9. Ledger and follow-ups

[`integration-amendments.json`](integration-amendments.json):

* retires `SP3-SYMBOLIC-FLUENT-BIT-PREMISES` (discharged here);
* registers and retires `SP3-LIBC-NESTED-CALL-ADMISSION-PREMISES`, the
  brief's libc item, with the census correction of §1;
* registers `SP5-GLOBAL-PROSE-PREMISE-BAN` (not on main at base): the loader
  refuses a string premise everywhere once SP2-SP4 land, with tests and
  fixtures migrated. If SP2 or SP4 lands first with the same registration,
  the integrator drops this one.

Not registered, noted: the language has no propositional negation, so
"not in `T`" is `#x in S \ T`; a `!` operator would read more directly but
is a grammar change, out of this card's scope.

**Files shared with SP2 and SP4** (merge mechanically, keep every card's
block): `crates/qcore-frontend/src/premise_formula.rs` (the `VOCABULARY` and
`DECISION_VECTORS` tails), `crates/qcore-frontend/tests/premise_formula.rs`,
`proof/QCore/PremiseFormula.lean` (the `vocab` and `decisionVectors` tails,
`Model`'s last field, the last arms of `prim0`/`prim1`, the counterexample
model in `ex_data_does_not_entail_unstated`, and the three `prim*_kind`
proofs this card made length-independent), `docs/QCORE_CONTRACT_SPEC_r0_3.md`
(C04.18 and the C04.18.9 table), `crates/qcore-frontend/src/pipe.rs`
(`premise_formula_spans`: SP2's librdkafka modules are in the contract corpus
too and would meet the same `Q038` refusal, so keep one exemption, not two)
and G05-formula's note in `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`,
`docs/agents/rules/13h.md` with the
generated `docs/agents/gateway.md`, and the generated `proof/QCore.lean`
(rerun `scripts/gen_lean_roots.py`). A card whose block adds a field to
`Model` must also add it to the counterexample model.

## Integrator measurement (2026-11-21)

Landing `8952bdb8d` on base `17b0e77b2`, measured with `--declared PREDICTION.md`:
corpus, twelve-home, system and slice all match the declared prediction with
0 moves. Comparison: `integration/native-comparison.json.gz` (uncompressed
sha256 `ca683f4b6cc97494b5c9f69e6b6063529807a67612a2a4b09253481026043e2a`). Landing gate: `integration/gate.json` (Rust 9818
passed, 1 failed: the known host libcurl failure; Lean 472 audits; Python
779; python exit 1 on the timing-sensitive test_gate_slot_starvation Resident.test_it_is_reported_while_the_job_runs only (rerun alone 5 times: 4 passed, 1 failed; gate-slot timing, untouched by this card)).

Landing race: merged main `afd8f202d` (SP2 and r47-r52) in `f4b0dee35`; the
targeted check named in `integration/gate.json` passed.
