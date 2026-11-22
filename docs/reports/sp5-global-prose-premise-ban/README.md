# SP5-GLOBAL-PROSE-PREMISE-BAN

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." SP1-SP4 converted every provider premise; user 2026-11-22:
"Launch SP5". Base origin/main `1d69b7b81`. Prediction:
[PREDICTION.md](PREDICTION.md) (committed first, `a9a2d443b`).

The contract format now refuses `resource_premise(NAME, "STATEMENT")` in
every vocabulary and on every load route, by name (`QF001`). The card
discharges `SP5-GLOBAL-PROSE-PREMISE-BAN` and `SP2-SHIPPED-PROVIDER-PROSE-BAN`.

## 1. Census

Pattern `resource_premise\([a-z_0-9]+, *\\?"` (the string form, as source or
as a Rust string literal), outside `docs/reports/`, by file, at base and
after; plus what the pattern cannot see (multi-line comments, statements
built with `format!`, readers of frozen texts). "Refusal" means the text
exists to show the string form is refused.

| File | Role | Base | After | What changed |
|---|---|---|---|---|
| `crates/qcore-frontend/src/premise_formula.rs` | the one reader (`premise_clause`) | admits a plain string (`PremiseStatement::Prose`) | 3 (refusal message, `CLAUSE_DECISION_VECTORS`) | string refused, `QF001`; `PremiseStatement` removed, the reader returns `(name, PremiseFormula)` |
| `crates/qcore-contract/src/local_condition.rs` | C04.21's reserved entry-snapshot statement (production constant) | English sentence | 0 | the formula `quant all #m in #condition_reads: #m in #entry_stable` (SP5 block) |
| `crates/qcore-audit/src/null_member_scope_fold.rs` | generated contract text (unit tests) | 2 | 0 | formula |
| `crates/qcore-frontend/tests/premise_formula.rs` | reader tests | 4 | 4 | refusal tests (`QF001`) |
| `crates/qcore-frontend/tests/premise_formula_librdkafka.rs` | reader (`PremiseStatement`) | type only | 0 | the reader's new result type |
| `crates/qcore-contract/tests/contract_premise.rs` | loader tests | 20 | 6 | formulas (SP2's `served_queue`); 6 refusal cases; confinement and record tests added |
| `crates/qcore-contract/tests/hypothesis.rs` | graph tests; reads `premise-decomposition-cycle1/proposed-cjson.qv` | 5 + reader | 0 | formulas; the proposal read through the legacy reader |
| `crates/qcore-contract/tests/local_conditions.rs` | S3 scoped premises | 5 | 0 | formulas |
| `crates/qcore-contract/tests/caller_frame.rs` | caller-frame spare premise | 1 (+2 pattern false positives) | 0 (+2) | formula |
| `crates/qcore-contract/tests/contract_lifecycle.rs`, `premise_flow.rs`, `quant_proposition.rs`, `plugin_premise_decomposition.rs`, `detach_same_item_clause.rs`, `nonnull_member_qualifier.rs` | fixtures | 2, 2, 1, 2, 1, 1 | 0 | formulas |
| `crates/qcore-contract/tests/guarded_contract_adoption.rs`, `guarded_decomposition.rs`, `production_symbolic_adoption.rs` | load SP2's byte restoration of `cjson.qv` (and the plugin checkpoint) | reader | reader | frozen digests checked on the restored bytes; replay on the current module with the plan and graphs lifted (§3) |
| `crates/qcore-c-gateway/tests/null_member_premise.rs` | bounds premise | 6 | 2 | formula; 2 refusal cases |
| `crates/qcore-c-gateway/tests/loop_lifetime_s3.rs` | S3 | 4 + 2 `format!`-built | 0 | formulas, the snapshot formula |
| `crates/qcore-c-gateway/tests/unit/tracked_self_init_frames.rs`, `contract_clause_standing.rs` | fixtures | 2, 1 | 0 | formulas |
| `crates/qcore-audit/tests/contract_dependencies_report.rs` | fixtures; reads `proposed-cjson.qv` and the restoration | 2 + reader | 0 | formulas; legacy reader |
| `compiler/tests/premise_formula_operators.rs` | operator gate | 1 (accepted) | 2 | a refusal test (`Q000` naming `QF001`) |
| `compiler/tests/contract_inherit_qcore_provider.rs` | token parity with a frozen fixture | 1 + `restore` | 0 | formula; fixture read through the legacy reader; load-time ban test added |
| `compiler/tests/fixtures/contract_inherit_qcore_provider/librdkafka_memory_hand_written.qv` | frozen fixture | 1 | 1 | unchanged bytes, read only through the legacy reader |
| `compiler/tests/qr_lexical_selector_census.rs`, `qr_print_route_census.rs` | parse every tracked source, `docs/reports/**/*.qv` included | reader | reader | sources lifted before parsing, so the frozen proposal stays in the census |
| `tests/support/sp2_premise_checkpoint.rs` | writes prose back (`restore`) | reader | reader | also writes back SP5's comment edits (two digests per file); its output is text, loaded only through the legacy reader |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | C04.18 example, C04.21 example and snapshot, rk_plugin mention | 4 | 0 | formulas |
| `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv`, `client.qv` | comments | 2, 2 | 0 | `FORMULA` |
| `providers/quirin-provider-fluent-bit/contracts/fluent_bit/loop_owner.qv`, `output_singleplex.qv`, `input_chunk_name.qv` | comments | 1, 1, 1 (multi-line) | 0 | `FORMULA` |
| `providers/quirin-provider-libc/contracts/libc/errno.qv` | comments (wrongly naming a `resource_premise`) | 2 | 0 | names the `resource_premise_all` conjunction it is |
| `providers/quirin-provider-openssl/src/documented.rs` | doc comments | 6 | 0 | `FORMULA` |
| `providers/quirin-provider-openssl/src/check.rs` (`prose_premise`), `qcore-provider-libevent/src/main.rs`, `qcore-provider-librdkafka/src/main.rs`, `quirin-provider-fluent-bit/src/lib.rs`, `quirin-provider-libc/src/lib.rs` | per-crate bans | 1, 1, 0 (textual), 1, 1 | same | kept as belt-and-braces (textual scans; the provider workspace does not link the core reader) |
| `crates/qcore-contract/examples/premise_formula_binding_refresh.rs` | SP2's one-off refresh tool; reads its BEFORE roots | reads prose | refuses prose | a historical tool; its BEFORE roots no longer load (recorded, not changed) |

Total by the pattern: 88 matches in 31 files at base; after, 35 in 12 files,
every one a refusal case, the frozen fixture, the reader's own refusal text
and vectors, or a per-crate ban. No shipped `.qv` states one (0 before and
after).

**Other premise heads.** `resource_premise_all(NAME, ID, ...)`,
`resource_premise_decomposition(NAME, ID, ...)` and
`resource_premise_flow_v1(NAME, EXPR)` take identifiers and flow expressions
only (G05); none carries a string anywhere in the tree. `resource_hypothesis(ID,
"STATEMENT")` atoms do carry English: the nine `detach_*` atoms of
`cJSON_DetachItemViaPointer` (`cjson.qv`), seven in the frozen compiler
fixture, four in `crates/qcore-contract/examples/support/*.qv`, about 17 in
qcore-contract tests and three generated by
`quirin-source-analysis::cjson_local_binding`. They are
SP2-DETACH-HYPOTHESES-TYPED-ATOMS's, not converted here. Outer contract
definitions (`quant #outer [[contract]] NAME, "STATEMENT";`, C04.20) also carry
a string; they are not premise clauses and are out of scope (follow-up §9).

## 2. The ban and where it fires

`qcore_frontend::premise_formula::premise_clause` returns
`(name, PremiseFormula)`; a statement that is, or holds, a string literal
returns the refusal `QF001: premise `NAME` states a string; ...`
(`PROSE_PREMISE`, `prose_premise`). Every route reads a premise through it:

* the structural parser (`frontend.rs`, `premise_clause`), which every
  module passes through: resource v1, bounds v1, model contracts, provider
  packages and plugin views all parse first, and the inclusion elaborator
  parses each module once its inclusion forms are set aside (`QI001` then
  carries `QF001`);
* the contract loader (`contract_loader::premise_clause`, `module_premises`),
  the bounds loader (`bounds_contract`), the local-condition reader
  (`local_condition::named_prerequisite`) and the operator gate
  (`pipe::premise_formula_spans`).

On the compiler route the user sees the `Q000` parse refusal whose message
starts with `QF001`.

**Diagnostic id.** The `Q000`-`Q099` route space is exhausted and new `Q` codes
are iceboxed (`Q-NAMESPACE-DECISION`). Existing lettered namespaces: `QP***`
(provider host) and `QI***` (contract inclusion); no `QF` id existed anywhere
in the tree. The card opens `QF***`, owned by the premise reader, on the
`QI***` precedent (no `Q` code allocated, the namespace decision not settled):
`docs/agents/diagnostics.md#premise-formula-namespace-qf` and the §L21 table
of `docs/QCORE_SPEC_r0_3.md`.

**SP2-SHIPPED-PROVIDER-PROSE-BAN.** Its discharge asked for a load-time
refusal for crates that opt in. The ban is unconditional instead: every crate
on every load path, shipped or user, opting in or not.
`compiler/tests/contract_inherit_qcore_provider.rs`
`a_string_premise_in_any_crate_on_the_load_path_is_refused_at_load` rewrites,
in every crate on the real load path that states a premise
(`qcore-provider-libevent`, `qcore-provider-librdkafka`,
`quirin-provider-fluent-bit`, required), the first premise's statement as a
string and shows elaboration through the load path and direct parsing both
refuse with `QF001`, the unmodified module parsing. The per-crate tests
(`every_premise_is_a_formula`, `no_premise_is_prose`) and OpenSSL's checker
(`prose_premise`) stay as belt-and-braces: the provider workspace does not
link the core reader, so they are the crate's own check.

## 3. The legacy reader

`tests/support/legacy_prose_premise.rs`, included by `#[path]` from test
targets only. `lift` writes each `resource_premise(NAME, "PROSE")` recorded by
SP2-SYMBOLIC-LIBRDKAFKA-PREMISES (`migration.json`, 16 statements) as its
formula and changes nothing else; an unrecorded string stays a string and the
loader refuses it. `lift_graph` does the same for a frozen hypothesis graph's
legacy atoms and erasures; `lifted_adoption_plan` checks the frozen guarded
adoption plan's two source digests against SP2's exact byte restoration, then
returns the plan with its graph lifted and its digests the current module's.
Every frozen text the tests read states only recorded statements (checked by
a script over every tracked `.qv` under `docs/reports/`, the compiler and test
fixtures: only comment placeholders fall outside the record).

**Unreachable from production.** The file belongs to no crate's source, and
`crates/qcore-contract/tests/contract_premise.rs`
`the_legacy_prose_reader_is_confined_to_test_code` checks that every tracked
Rust file naming it lies under a `tests` directory. Nothing in it admits a
string: it rewrites text before an unchanged production reader sees it. No
flag, feature or entry point admits a string statement.
`every_recorded_statement_lifts_to_a_canonical_formula` checks each recorded
formula is its own canonical printing and each recorded prose is refused.

**What the frozen replays check now.** The guarded adoption plan, the
decomposition applications and the symbolic-adoption inventory froze the
module before SP2. Their bytes are still recovered exactly and checked against
every frozen digest (SP2's restoration, now also writing back SP5's comment
edits with two digests per file, `comments.json`); the production replay runs
on the module as it stands with the frozen plan and graphs lifted. What is no
longer possible, by design, is loading the pre-SP2 bytes themselves.

## 4. Migrated tests and fixtures

Formulas reuse registered terms; no test-only term was added.

* `served_queue` fixtures state SP2's production formula
  `#op_callbacks |-> #takes(#held_on(#served)) == {}`.
* The bounds `no_plugin` fixtures state `#interleaved |-> #no_writes(#args)`
  (SP4 terms).
* Generic test premises (`p`, `assigned`, `premise`, `source_law`,
  `rk_spare`, `worker`, `entry_protocol`, `instance_config`, `detach_guard`,
  `rk_plugin_callback_closure` in a fixture) state `#concurrent |-> #inert`; a
  second, distinct one (`same` B, the "another protocol" variant) states
  `#interleaved |-> #inert`; `second` states `#false`.
* The model-contract `no_plugin` states `quant all #f in #loaded_registered: #false`.
* The rule-37 constructor specification states the formula SP2 recorded for
  its sentence.
* Tests that exist for the string form's behaviour now check its refusal:
  `a_premise_clause_is_a_formula_and_a_string_is_refused`,
  `a_string_statement_rejects_the_module_by_name`,
  `a_string_premise_is_refused_by_name_at_parse`, and the string cases kept in
  the malformed lists.

None of these test premises is read by a verdict, so none is a semantic
choice.

**The reserved entry-snapshot premise.** C04.21's recognizer matches the
exact name and statement `s3_condition_snapshot_stable` / "Every member read
by this function's condition bindings preserves its entry value until each
guarded use." That statement could no longer be written, so it is now the
formula `quant all #m in #condition_reads: #m in #entry_stable`, over two
terms appended in an SP5 block (`#condition_reads`, `#entry_stable`, both
`set(obj)`; specification table, Rust `VOCABULARY`, Lean `vocab`, read by
name through `ext0`). The two terms say exactly what the sentence's two parts
say (the cells the condition bindings read; holding their entry value at each
guarded use), so nothing is assumed beyond the sentence. No production
contract states the snapshot premise (the tree's only uses are tests), so no
verdict can move.

## 5. Specification, grammar, rules

* C04.18: the example states SP2's formula; a "string form is retired"
  paragraph (`QF001`, every route, the legacy reader); loading rule and
  "every vocabulary" paragraph say formula. C04.18.9: SP5 block. C04.21: the
  example and the reserved snapshot premise as formulas.
* G05: `named_premise` takes `premise_formula` only; the prose paragraph and
  G05-formula's opening say the string form is refused; the hypothesis-graph
  paragraph keeps the plain-string restrictions for `resource_hypothesis`.
* Rule 13h amended (summary and an SP5 paragraph); rule 14x's mention says
  formula; `contract/README.md`'s table row; §L21 `QF***` row; diagnostics
  page `QF` section. `gen_rule_docs.py --check`: 254 rule files match.

## 6. Lean

`QCore.PremiseFormula`, section "Clause statements": `Stmt` (`.str s` or
`.formula t`), `accept`, and

* `string_is_not_a_premise` (every string), `accept_formula`,
  `formula_premise_iff_wf`, `accepted_is_wf`, `accepted_is_a_formula`,
  `accepted_denotes` (an accepted premise denotes a proposition in every
  model);
* `clauseVectors` / `clause_vectors_agree`, bound to the Rust
  `CLAUSE_DECISION_VECTORS` by `the_clause_vectors_are_the_lean_table`
  (strings render as `.str`, the first literal; formulas as `.formula`);
* the SP5 vocabulary rows and four decision vectors (`decision_vectors_agree`
  re-proved over the extended table).

Registered in `proof/AuditPremiseFormula.lean`; placeholder-free, no project
axioms (`check_lean_audits.py`, §8).

## 7. Semantics

No production premise changed (every shipped premise was already a formula),
no declaration text changed (comments only), so no pin moved and no
declaration left force. Lock diff (`providers/contracts.py sync`): file
digests only, in `qcore-provider-librdkafka/contract-inventory.lock` and
`contracts.json` (`cjson.qv`, `client.qv`), `quirin-provider-fluent-bit/contract-inventory.lock`
(three modules) and `quirin-provider-libc/contract-inventory.lock`
(`errno.qv`); declaration and clause counts unchanged; `contracts.lock` and
`providers/contract-load-path.json` unchanged. Predicted: 0 status moves on
corpus, twelve-home, system and slice; byte-identical or contract-source
digests only. The card did not run the routes; the measurement is the
integrator's (`quirin-audit measure --declared PREDICTION.md`).

## 8. Gate

GATE

## 9. Follow-ups

* Outer contract definitions `quant #outer [[contract]] NAME, "STATEMENT";`
  (C04.20) state their envelope in English; under the user decision they may
  want the same symbolic form. Not a premise clause; not registered here
  (recorded for the integrator).
* `crates/qcore-contract/examples/premise_formula_binding_refresh.rs` (SP2's
  one-off tool) can no longer read prose BEFORE roots; a rerun would need the
  legacy reader. Recorded only.
