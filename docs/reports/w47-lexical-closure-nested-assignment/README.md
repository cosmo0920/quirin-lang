# W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT

Discharges the follow-up of the same name that W47-TYPED-ROOT-DECLARATION-READER
registered. A soundness card. Base `66be515f` (local `main` = `origin/main`,
"Merge PROVIDER-PY-DUAL-STEM-FIXTURE-STALE onto the round-1 integration"). The
prediction was frozen at `99b103e4` before the analyzer change
([PREDICTION.md](PREDICTION.md)). Gateway rule
[10o](../../agents/gateway.md#constraint-10o); Lean model
`QCore.LexicalClosureNestedAssignment`.

## The gap

`borrowed_actual::untracked_bases` (the custody closure behind every
`never_bound` answer) read one assignment per `;{}`-delimited token run, its
first top-level `=`. It never read an assignment nested in a condition or loop
header (`if (!(p = f()))`, `while ((p = next(q)))`), a `for` clause (the run
starts at `for`, so the `=` is at depth one), a comma or `?:` operand, a later
declarator (`T *a = f(), *b = a;`), a parenthesised destination (`(p) = …`,
the macro-expanded spelling of `TAILQ_FOREACH`'s `(var) = …`), a brace
initialiser's value (the `{` ends the run), or a parenthesised `&(p)`,
`(p)++`, `(p) op= …`. Their destinations were called never bound although the
body binds them. At the base the controls show the effect: a read
`b->item->count` after `b = a; c_drop(a)` bound in any of those forms was
admitted through `b` as an untracked root, and a pointer return
`return b->item;` after `if (!(b = c_make(1)))`, `for (b = c_make(1); …)` or
a macro-expanded assignment was admitted with a `NoOp` exit
([base outcomes](artifacts/controls/base-outcomes.txt)). Every such function
at the pin was already non-Safe through another leaf (rule 10l's 33 admitted
roots, all non-Safe), so the gap moved leaves, not verdicts; the premise was
nonetheless false.

## Mechanism (rule 10o)

1. **The superset reading** (`lexical_nested_assignment.rs`; hook in
   `borrowed_actual::untracked_bases`). Every simple `=` of the body is an
   assignment, with the name before it (or a parenthesised sole name, or an
   array declarator whose value is a brace initialiser) as destination and
   the rest of its group as value; a `{` straight after the `=` is read to
   its `}`. The old statement reading is kept unchanged beside it, so the
   risky set only grows; the same opacity test (a call, an `&`, a refused
   name; an admitted field read is not opaque) decides both. A statement
   expression's value is still read as its opening `(`, as rule 5l's
   `container_of` reading requires. The fixed point now runs at least
   `assignments + 1` rounds (the old cap of 64 could stop unsettled on a long
   copy chain) and refuses the function if it does not settle. This serves
   every consumer of `never_bound` — C02-P borrowed actuals and opaque field
   reads, O04-B08 store values, rule 5j returns, rule 5l head reads and
   guard, W44 call actuals, P2/P3 — all of which must be re-derivable from the
   certificate's own tokens, so the AST cannot serve them.
2. **The join at the typed positions** (`legacy_typed_roots.rs`; P2's
   `typed_path_plan` hook in `legacy_c.rs`). In every function the lexical
   declaration reader reads, a root is never bound only when the extended
   lexical closure says so **and** the rule-10l AST closure does not bind its
   spelling (a file-scope object included), wherever the AST reader answers
   the function; a pointer root (rule 5b) also needs neither the lexical nor
   the AST's `may_bind`. Where the AST reader refuses the function, the
   lexical answer stands; where the lexical reader answers nothing, rule 10l
   is unchanged (its served answer already joins the lexical closure, now
   extended).
3. **The same reader in three other premises**: `may_bind`
   (`typed_field_access::bound_names`, which already caught a nested `x =` by
   adjacency, now also parenthesised writes), O04-B08's store-base premises 3
   and 5 (a second, parenthesised or nested binding of a base, and a nested,
   later-declarator or parenthesised alias `x = BASE`), and the
   protected-formal alias closure (the aliases its run-first reading
   skipped; a run's first `=` keeps its old decision, so `*routes = result`
   stays a store, as the first AFTER run showed it must).

**Choice of the join against extending the scanner alone.** The brief
preferred the join; the card does both, because they reach different
consumers. The token reader is the only fix available to the seven non-typed
consumers (certificate re-derivation holds only the token stream). At the typed
positions the join adds Clang's own parse as a fail-closed net over whatever a
token reader can misread. Measured, the net caught nothing of the card's class
once the scanner was extended (the re-emitted disputed set below): everything
the join still adds is an AST class that rules 5l and 5i read as unbound
under their stated premises, and it withdrew six sound `Safe` functions. The
card keeps the join as asked; narrowing or teaching it is
`W47-JOIN-AST-CONSERVATIVE-CLASSES`, with that evidence.

## Measurement

Arbiter: `scripts/run_pinned_audit.py OUT COMMIT --inputs-root
/home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 5` with
`QCORE_TYPED_PATH_PARITY`, and `w46-contract-gap-expansion/scripts/run_system.py
OUT --jobs 5` at `046accf6`. Private runs: `private/nested/` of this worktree.

**BEFORE** (binary `8f49af40…`, [artifacts/before/](artifacts/before/)): both
exit 0. Corpus `resource-diagnostics.json`, chunk contracts and Monkey source
byte-identical to the round-1 baseline
(`w47-callee-closure-r2/integration/artifacts/corpus/`), sorted parity and
contexts identical (`scripts/identity.py`); the contracts and source-effects
sidecars differ only by the worktree root and the `resource_contracts:`
digests keyed on it (`scripts/sidecar_leaves.py`: no leaf differs after the
root alias except digest-keyed ones). System diagnostics differ in 45 leaves,
all `compile_configuration` digests of the contract sources and inventory
(cwd-bound); sorted parity identical. `baseline_identity.py` raises on these
file-keyed manifests, as the brief says, so this comparison replaces it.

**AFTER** at `f0271d8d` (binary `44fb3090…`,
[archives](artifacts/after/)). Two earlier AFTER runs are superseded: at
`c8fe40fd` the protected-formal hook also re-read each run's first `=` (adding
`*routes = result` as an alias: 178 changed functions against 159 after the correction, no status move), corrected at
`718d0fcc`; at `718d0fcc` the re-emitted disputed set still named one
initialiser the scanner skipped (`cert_map[N] = { &…, … }` in
`rd_kafka_conf_set_ssl_cert`), read from `f0271d8d`. Status figures are the
same in all three.

| Measure | BEFORE | AFTER |
|---|---:|---:|
| Corpus functions Safe / provisional Unsafe / Unknown | 510 / 10 / 4,314 | **506 / 10 / 4,318** |
| Corpus units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | unchanged |
| Leak / unknown_alias | 36 / 7,443 | 36 / **7,602** |
| COND / RET / IND / UNC / OTHER leaves | 6,331 / 284 / 15,037 / 8,883 / 5,674 | **6,374 / 290 / 15,084 / 8,885 / 5,680** |
| Unsupported total | 36,209 | **36,313** |
| System configuration rows Safe / Unsafe / Unknown | 63 / 4 / 381 | **60 / 4 / 384** |
| System unique functions | 33 / 2 / 199 | **31 / 2 / 201** |
| System unknown_alias / unsupported | 985 / 2,908 | 998 / 2,914 |

### Named corrections (every Safe → non-Safe move)

All six are withdrawals by the join, not unsound `Safe`s: each root is bound
only by an AST class the lexical closure reads under an established rule.

| Function | Root | AST binding | Why the Safe was sound |
|---|---|---|---|
| `mk_fifo_queue_get` (corpus) | `q` | statement expression (`mk_list_entry`) | rule 5l(2): a `container_of` element over an active cursor is list memory the body does not track |
| `mk_plugin_cap` (corpus) | `plugin` | same | same |
| `rd_kafka_Uuid_str` (corpus) | `bytes` | sub-object store `bytes[i] = …` | the admitted store's own root: a scalar store into a local `char` array (rule 5i) |
| `unit_test_telemetry_decoded_int64` (corpus) | `unit_test_data` | sub-object store (file-scope struct) | the admitted scalar member store itself; missed by the prediction, whose records list declared names only |
| `flb_engine_get_chunk_backpressure_percent` (system, 2 configurations) | `ins` | statement expression | rule 5l(2) |
| `sb_find_segregated_backlog_by_output_plugin_instance` (system) | `backlog` | statement expression | rule 5l(2) |

### Attribution

[`scripts/attribute.py`](scripts/attribute.py) classifies every changed leaf
([corpus](artifacts/after/corpus-comparison/),
[system](artifacts/after/system-comparison/)); none is left unexplained.

- **Corpus, 162 functions, 271 leaves** (267 added, 4 removed):
  171 at typed rows whose root the superset reading now binds (125
  `unknown_alias`, the chain needing the root's identity; 30 COND, 11 IND,
  2 OTHER, 1 RET refusals), in 109 functions — `TAILQ_FOREACH` elements,
  `for`-clause cursors, nested copies; 6 at non-typed consumers of a newly
  bound base (C02-P, W44); 80 at typed rows whose root only the AST binds
  (the join; 34 `unknown_alias`, 7 COND, 35 IND, 3 OTHER, 1 removed), 5 RET
  where the AST's `may_bind` names a static array's initialiser
  (`rd_kafka_*_name`), 4 elsewhere in join functions (`ret` in
  `rd_kafka_Uuid_str` after its refused store; `label_v` in `format_metric`),
  and 5 summary cascades (`mk_fifo_queue_get`'s callers, and
  `flb_input_chunk_drop_task_route`'s call of the `static inline`
  `flb_task_get_route_data`, whose own rows the join refuses). Both
  directions of every `unknown_alias` change are at a root rule 10o binds.
- **System, 19 function rows, 31 leaves**: 9 `unknown_alias` at typed rows the
  join refuses; 22 summary cascades from the `flb_task_get_route_*` /
  `set_route_*` inline bodies (`handle_output_event`,
  `record_retry_failure_metrics`, `flb_input_chunk_drop_task_route`,
  `flb_output_task_flush_error`).
- **Leaks and violations:** none added (leak 36, provisional Unsafe 10,
  system counted_user 4 and Unsafe 2 unchanged), so there is nothing to
  classify as real, artifact or premise gap.

## Prediction against outcome

1. BEFORE reproduces the baseline — **holds** (verdict-bearing identity;
   sidecar and system digests cwd-bound).
2. The three corpus and two system withdrawals — **hold**, named above.
   **Missed:** `unit_test_telemetry_decoded_int64`: the join reads spellings,
   so file-scope roots join too, and the 10l exposure records list declared
   names only.
3. Corpus totals 507 / 10 / 4,317 — **missed by one** (506 / 10 / 4,318, the
   fourth withdrawal above); the tolerance was for (A)'s non-typed consumers,
   which moved no status. Units, leak and Unsafe unchanged — **hold**.
4. Leaves confined to join and newly-bound roots — **holds**; `unknown_alias`
   rose by 159, removals were 3.
5. System 31 / 2 / 201 — **holds**.
6. Disputed set — **holds** (below).
7. Controls — **hold**.

## The disputed set, re-emitted

[`scripts/disputed.py`](scripts/disputed.py) pairs each admitted typed row with
its own analysis's `root-declarations` row
([corpus](artifacts/after/disputed-corpus.json),
[system](artifacts/after/disputed-system.json)):

- **Served never bound while the AST binds: 0** of 24,717 corpus and 3,502
  system admitted typed rows (by construction wherever the AST answers).
- **The lexical closure alone** (what the non-typed consumers read) still
  calls never bound, at admitted-row roots the AST binds: 127 statement
  expressions (32 function records: `container_of` elements, rule 5l), 22
  sub-object stores (4 records, rule 5i), and 24 rows of one copy of a
  `container_of` element (`format_attributes: v = p->val`); system: 498
  statement-expression rows. **Zero** of the nested, `for`-clause,
  later-declarator, parenthesised or initialiser classes (rule 10l's 119 +
  295 + 120 + 177 records).

## Other readers of the same shape (step 3)

| Reader | Shares the reading | Direction | Disposition |
|---|---|---|---|
| never-bound closure (`untracked_bases`) and its nine consumers | yes | fail-open | fixed (1) |
| P2/P3 typed premises | yes, through the closure | fail-open | fixed (1) and (2) |
| `may_bind` (`bound_names`) | adjacency: nested `=` seen, parenthesised not | fail-open | fixed (3) |
| O04-B08 premises 3 and 5 | run-first `=` / adjacency | fail-open | fixed (3) |
| protected-formal alias closure | run-first `=` | fail-open | fixed (3) |
| declared-local check (`FunctionTypes`) | reads declarations, not bindings | fail-closed: an unread declaration answers *no*; the AST answers there (3,044 AST-only records, rule 10l) | none |
| W43 traversal recognisers, index `=` check | whole statements / `for` clauses; `contains("=")` at any depth | positive recognisers | none |
| rule 5j return family `untracked` | reads `never_bound` | — | fixed by (1) |
| continued-access publication reader (`publication.rs`) | run-first `=` | fail-closed (a missed store publishes nothing) | registered `W47-PUBLICATION-NESTED-STORE-READER`: 3,649 nested member-store records, 0 in the system slice |
| other adjacency write readers (10d stable formals, 5l element names, 5k P1, …) | parenthesised destination | fail-open in principle | registered `W47-PARENTHESISED-WRITE-READERS`: 803 records, 72 spellings, **0 formals**; 0 in the system slice |

## Lean

`proof/QCore/LexicalClosureNestedAssignment.lean`, registered in
`proof/AuditLexicalClosureNestedAssignment.lean` (32 theorems). It imports
`QCore.TypedRootDeclarations` and proves: `Bound` is monotone in the events it
reads, so the superset reading keeps every binding of the old view
(`the_superset_reading_keeps_every_old_binding`) and a set seeded and settled
for every event is one for the old view; the gap, witnessed
(`the_old_set_is_settled_for_the_old_view`, `the_old_set_misses_a_binding`:
the old set is settled for what it saw while a nested copy is bound);
soundness of the full reading; the join over-approximates each closure
(`a_joined_root_has_no_binding`: if either set is seeded and settled for the
body, a joined root has no binding), and where the AST refuses the lexical
answer stands; the served answers keep rule 10l and only strengthen it
(`the_join_implies_rule_10l`, `the_joined_pointer_root_implies_rule_10l`,
`an_unreadable_function_keeps_rule_10l`), keep both `may_bind` halves, and an
AST-bound root is never served in a readable function; P3's return and chain
theorems and P2's mixed-path refusal hold under the join
(`a_return_through_an_ast_bound_root_refuses_everywhere`,
`a_pointer_actual_through_an_ast_bound_root_refuses`,
`a_mixed_path_through_a_joined_bound_root_refuses`); non-vacuity witnesses.
The Rust-to-Lean correspondence (that the token reader extracts every
assignment event and the loops return seeded, settled sets) is a review
premise (constraint 28).

## Tests

`crates/qcore-c-gateway/tests/lexical_closure_nested_assignment.rs` (4 tests,
the fixtures run at the base first; [base](artifacts/controls/base-outcomes.txt)
and [after](artifacts/controls/after-outcomes.txt) outcomes): each nested form
(condition, negation, loop header, comma, later declarator, `?:` arm,
macro-expanded parenthesised destination, call actual) makes the read through
its root carry `unknown_alias:b`; the three pointer returns keep the return
refusal; a compound assignment in a condition and a flow-tracked
parenthesised copy are unchanged; untouched roots still certify. Unit tests:
`borrowed_actual` (every nested form binds, a nested admitted field read binds
nothing) and `lexical_nested_assignment` (5: every form, the old run value,
comparisons and member stores, parenthesised writes, the census).

## Gate

[gate.json](gate.json), exits in [gate/](gate/),
[`scripts/gate.sh`](scripts/gate.sh). Code steps at `4e744d6c` (later commits
change docs only): submodule 0; `run_test_binaries.py` 0 — **342 units, 355
summaries; 6,151 passed, 0 failed, 11 ignored**
(`private/test-binaries/20260928T033852Z-1438367/summary.json`);
`cargo clean --profile dev` 0; fmt 0; clippy 0; census-feature gateway clippy 0
and library tests 0 (630); both reevaluation examples 0;
`check_lean_audits.py` 0 (**218** audits); Clang 21.1.8 recorded. After the
matrix and index regeneration (`0e617e77`): `unittest discover -s
scripts/tests` 0 (**365**) and `build_docs_index.py --check` 0. No
`providers/` path is touched, so the provider gate does not apply.

## Follow-ups

- Retired: `W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT`.
- Registered ([followups.json](followups.json)):
  `W47-JOIN-AST-CONSERVATIVE-CLASSES`, `W47-PARENTHESISED-WRITE-READERS`,
  `W47-PUBLICATION-NESTED-STORE-READER`. Ledger 536 → 538.
- For the parallel widening cards: the disputed set is closed for the
  nested classes; a new `Safe` resting on a statement-expression or
  sub-object-store root is refused at typed positions by the join.

## Files likely to conflict

`crates/qcore-c-gateway/src/{lib.rs,legacy_c.rs,legacy_typed_roots.rs,borrowed_actual.rs,typed_root_declarations.rs,typed_field_access.rs,aggregate_store.rs}`
(thin hooks), `proof/QCore.lean`, `docs/agents/{gateway.md,README.md}`,
`docs/STATUS.md`, `docs/plans/unsupported-coverage/README.md`,
`docs/development/increments/wave-47.md`,
`docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
