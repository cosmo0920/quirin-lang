# CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION

Card CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION (USER DECISION 2026-11-25
"CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION -> launch this"). Base `110a69c8e`.
New gateway rule [89](../../../../agents/gateway.md#constraint-89);
`QCore.CallNotDeclarator`.

This card discharges the follow-up CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION.
OUT-SLOT-CONDITIONAL-DELIVERY registered it on its own branch
(`3d1937748`), which has not landed, so the amendments file does not retire
it. The integrator retires it once OUT-SLOT's registration has landed.

## 1. The misreading

`classify_simple_base` (`legacy_classify.rs`) decides whether a statement
is declaration-like with four token tests:
- a keyword specifier first;
- `IDENT IDENT`;
- `IDENT *`;
- `IDENT ( *`, read as the function-pointer declarator `T (*NAME)…`.

The shadow guard then refuses a declaration-like statement that names a
formal outside an initializer, with "local declaration may shadow a formal
resource identity". The fourth test also matches a call whose first actual
is `*NAME`: `q_observe(*out);`, or librdkafka's
`rd_list_add(*toppars_add_list, rd_kafka_toppar_keep(rktp));`
(`rd_kafka_broker_share_session_add_remove_toppar`). Tokens cannot tell these
two apart, because a typedef name and a function name are both identifiers.

## 2. The change

Where the fourth test holds, the hook `Parser::paren_star_declarator`
(`legacy_call_not_declarator.rs`) asks the function's Clang AST for the
statement's own node. It uses `StatementIndex::statement_node`, which matches
by line and exact tokens and tells several matches apart by occurrence, as
rule 38h does.
- A `CallExpr` drops the fourth test: a call expression declares nothing.
- A `DeclStmt`, any other kind, or no unique node keeps it.
- The other three tests are never consulted.
- No token guess is made: a known callee name with no Clang answer keeps
  today's reading.

The admitted call then takes the classifier's normal path unchanged:
- the contract reading;
- the borrowed-actual and access gates;
- every guard after `classify_simple_base`.

`declaration_like` also feeds rule 38h's `typed_declaration_gate`. That gate
admits only on a `DeclStmt`, so a statement whose node is a `CallExpr` gets
the same answer from it with `declaration_like` true or false.

**Hook point, exact** (for merging with JUMPED-SHADOW-FORMAL-READINGS):
- `legacy_classify.rs`, `classify_simple_base`, the `let declaration_like =`
  expression just above the shadow loop.
- The third alternative of the `is_identifier(first)` arm used to be
  `(*second == "(" && declaration.get(2) == Some(&"*"))`.
- It is now the same conjunction plus `&& self.paren_star_declarator(tokens)`,
  with the comment `// Rule 89: unless Clang reads a call.`
- Nothing else in that file changes.
- `legacy_c.rs` gains one `#[path] mod call_not_declarator;` next to
  `typed_declaration_gate`.

## 3. Probes (base `110a69c8e` and tip, end to end through `analyze_unrestricted_c11`)

The base column is a run of the same shapes on a `git archive` of `110a69c8e`
(`private/cwd/probe-base.sh`, log `private/cwd/probe-base.log`). Fixture contracts as in `tests/support/clang_gap.rs` (synthetic, trusted test
assumptions). Every row is pinned in
`crates/qcore-c-gateway/tests/call_with_deref_actual.rs`.

| shape | base | tip |
|---|---|---|
| `q_observe(*out);` | Unknown, SHADOW | **Safe** |
| `cg_pair(*out, x);` | Unknown, SHADOW | **Safe** |
| `if (x) q_observe(*out);` | Unknown, SHADOW | **Safe** |
| `(void)q_observe(*out);` | Unknown, IND | Unknown, IND (no `IDENT ( *` prefix) |
| `r = r_borrow_code(*out);` | Safe | Safe |
| `q_observe(**out);` | Unknown, SHADOW | Unknown, IND |
| `q_observe((*out));` | Safe | Safe |
| `q_store(*list, item);` (add_remove_toppar, minimised) | Unknown, SHADOW | Unknown, IND |
| `if (!*list) *list = q_alloc(); q_store(*list, item);` | Unknown, SHADOW + stored-through-`*list` | Unknown, IND + stored-through-`*list` |
| `cb(*out);` (`cb` a function-pointer formal) | Unknown, IND | Unknown, IND (refused before the guard) |
| `s->cb(*out);` | Unknown, IND | Unknown, IND |
| `undeclared_sink(*out);` (uncontracted) | Unknown, SHADOW | Unknown, `call to uncontracted function` |
| `void *p = q_alloc(); q_observe(*out);` | Unsafe, with a SHADOW leaf | Unsafe, no SHADOW leaf (the leak; provisional, synthetic) |
| `q_observe(*out); q_observe(*out);` on one line | Unknown, SHADOW | Unknown, IND (see 4) |
| the same pair on two lines | Unknown, SHADOW x2 | **Safe** |
| `q_observe((*out)); q_observe((*out));` | Safe | Safe |
| `{ int (*out); }` | Unknown, SHADOW | Unknown, SHADOW |
| `{ void (*out)(void); }` | Unknown, SHADOW | Unknown, SHADOW |
| `{ T (*out)[4]; }` (typedef `T`) | Unknown, SHADOW | Unknown, SHADOW |
| `{ T (*out); }` (typedef `T`), also under `if` | Unknown, SHADOW | Unknown, SHADOW |
| `{ PTR(*out); }` (`#define PTR(n) int n`) | Unknown, SHADOW | Unknown, SHADOW |
| `{ __typeof__(x) (*out); }` | Unknown, IND | Unknown, IND (pinned as SHADOW-or-IND; JUMPED-SHADOW's gap) |

SHADOW = "local declaration may shadow a formal resource identity"; IND =
"indirect memory access is outside the resource projection".

**Unknown -> Safe justification.** The four rows that move have one
or two statements, each a call to a contract that only borrows (`q_observe`:
`resource_borrowed(p)`; `cg_pair`: both actuals borrowed). The actual `*out`
reads a pointer through the formal, and the borrowed-actual reading already
admits that read: `q_observe((*out));` is Safe at base with the same
obligations. Nothing is acquired, released or stored, so once the guard no
longer refuses, every obligation discharges. An uncontracted callee stays
Unknown at the call.

## 4. Observation (not a regression)

Two equal `q_observe(*out);` statements on one line are now told apart by
occurrence, so the guard no longer refuses them. The access gate then refuses
the unparenthesized `*out` actual. The same pair on two lines, and the
parenthesized pair on one line, are Safe. This limit lies in the call-actual
reading, not in this rule. It fails closed, and it is pinned.

## 5. Lean

`proof/QCore/CallNotDeclarator.lean`, audited by
`proof/AuditCallNotDeclarator.lean` (standard axioms only: `propext`). The
module defines:
- the four readings;
- `declaratorShaped` and the old and new tests;
- the shadow guard `shadowRefused`.

Theorems:

| theorem | statement |
|---|---|
| `vectors_match` | the 64-entry decision table equals the Rust test's vectors (`legacy_call_not_declarator::tests`) |
| `never_adds_a_declaration` | the new test implies the old one |
| `an_admission_is_a_call` | a statement the old guard refused and the new one admits has reading `call` |
| `an_admission_is_only_the_paren_star_disjunct` | such a statement fails the other three tests |
| `a_declaration_keeps_the_old_test` | every `DeclStmt` keeps the old answer |
| `a_declaration_keeps_the_refusal` | every `DeclStmt` keeps the old refusal |
| `a_non_call_keeps_the_old_test` | any reading but `call` keeps the old answer |
| `no_ast_answer_keeps_the_old_test` | no unique node keeps the old answer |
| `a_keyword_declarator_stays` | a keyword declarator is declaration-like whatever the reading |

## 6. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6
(`private/fb-046accf6`, read-only). The base `110a69c8e` and the tip `ed73934d5`
(the semantic commit) were each run from their own `git archive` tree with
their own release `quirin-audit`:
- [scripts/run.sh](scripts/run.sh), with `opt-in-current.sh`, `totals.py`,
  `moved.py` and `changed_rows.py` copied from
  `return-family-untracked-deref-cast`, paths changed;
- [scripts/compare.sh](scripts/compare.sh);
- outputs in `private/cwd/run-base2` and `private/cwd/run-tip`.

The binaries' SHA-256 values are `dc669d42…` (base) and `8ceb34a2…` (tip).

**Side-tree lock edge.** The base tree's `providers/Cargo.lock` lacks the
`qcore-c-gateway -> qcore-frontend` edge that MEMBER-EFFECT-STORES added. So
`providers/package.py checkers` refuses its `--locked` build, and the first
base attempt (`private/cwd/run-base`) ended on `QI014`. That attempt is
discarded. `run.sh` now adds the one missing edge in each side tree only
([evidence/providers-Cargo.lock.diff](evidence/providers-Cargo.lock.diff)),
identically for base and tip. The repository is not changed.

| side | Safe | Pending | Unknown | Unsafe | access rows / leaves | wall |
|---|---|---|---|---|---|---|
| base | 1,198 | 2 | 3,596 | 8 | 2,094 / 8,903 | 48.6 min |
| tip | 1,198 | 2 | 3,596 | 8 | 2,099 / 8,916 | 35.9 min |

**Status moves: none**
([evidence/moved.jsonl](evidence/moved.jsonl)).

**Leaf changes: 12 rows**
([evidence/changed.txt](evidence/changed.txt)). All 21 shadow leaves at
`IDENT ( *` calls are gone. Each is replaced at the same line by the call's
own reading, or by nothing:

| row | removed | added at the same lines |
|---|---|---|
| `unpack_basic_type` | 2 | 2 x uncontracted `cmt_map_destroy` |
| `cmt_mpack_consume_string_tag` | 2 | 2 x unresolved `output_buffer` |
| `ctr_mpack_consume_binary_tag` | 3 | unresolved `output_buffer`, 2 x IND |
| `ctr_mpack_consume_string_tag` | 3 | unresolved `output_buffer`, 2 x IND |
| `rd_kafka_broker_share_session_add_remove_toppar` | 2 | 2 x IND |
| `rd_kafka_cgrp_consumer_assignment_with_metadata` | 1 | IND |
| `rd_kafka_ApiVersions_copy` | 1 | IND |
| `rd_kafka_gzip_compress` | 2 | 2 x IND |
| `rd_kafka_handle_ApiVersion` | 2 | 2 x IND |
| `serialize_Metric` | 1 | nothing (the call's actual is admitted) |
| `mk_utils_gmt_cache_get` | 1 | IND |
| `mk_utils_utime2gmt` | 1 | uncontracted `mk_utils_gmt_cache_add` |

The 5 other rows with shadow leaves keep them, as predicted. Their leaves are
declarations, among them `rd_kafka_timers_run`'s
`void (*callback)(…);`, a real declarator:
- `rd_kafka_CreateAclsResponse_parse`;
- `rd_kafka_DeleteAclsResponse_parse`;
- `rd_kafka_timers_run`;
- `rd_map_copy`;
- `cio_file_native_lookup_group`.

**add_remove_toppar's remaining blockers** (tip), in line order of the
preprocessed unit:
- `rd_list_new`'s registry holder;
- the owned result stored through `*toppars_add_list` without an
  out-parameter contract;
- two refused branch conditions;
- IND at `rd_list_add(*toppars_add_list, …)`;
- unresolved `toppars_remove_list`;
- the uncontracted `rd_list_remove`;
- the `rd_kafka_toppar_destroy0` reference drop it does not hold;
- IND at `rd_list_destroy(*toppars_remove_list)`;
- the store through `*toppars_remove_list` without an out-parameter
  contract.

**Against the prediction** ([PREDICTION.md](PREDICTION.md)):
- Status: no move was predicted, and none was measured.
- Rows: leaf changes on about 12-13 rows were predicted; 12 were measured.
  `cio_file_native_lookup_group` was the uncertain one, and its leaves are
  declarations.
- Shadow leaves: "up to 20" removed was predicted; 21 were removed. That is
  one more than the bound: `ctr_mpack_consume_binary_tag`'s third leaf.
  Every one of the 21 removed leaves sits at an `IDENT ( *` call.
- No Safe -> non-Safe move and no Unknown -> Unsafe move.
- The `measure-prediction` block, with no move on any pinned route, stands.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host.
There are no `build/cycle19` inputs and no `private/baselines`, so it was not
run and no result is claimed.

## 7. Gate

Command:
`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 110a69c8e
--out <worktree>/private/gate`.
- Exit 1. The mode was full, because the diff reaches `qcore-c-gateway`.
- Log: `private/gate/gate.json`.

| step | exit | counts |
|---|---|---|
| submodules | 0 | 1 checked |
| fmt | 0 | |
| clippy `--all-targets -D warnings` | 0 | |
| rust-suite | 101 | 698 units; 10,132 passed, **1 failed**, 24 ignored, 148 filtered |
| probes (`reevaluation_probe`, `reevaluation_followup`) | 0 | |
| lean (`check_lean_audits.py`) | 0 | 510 audits |
| python | 0 | 807 tests |

**The one failure is a base failure.** It is
`qcore-compiler` `workspace_layering::workspace_dependencies_point_strictly_downward`:
"crates/qcore-c-gateway must not depend on `qcore-frontend`". That dependency
is MEMBER-EFFECT-STORES's, and this card does not change it. Rerun on a
`git archive` of `110a69c8e` (`private/cwd/base-layering.sh`, log
`private/cwd/base-layering.log`), it fails identically. That run also fails
`rule_packages_depend_on_the_api_only`, an artifact of the archive tree,
which has no git metadata for `tracked_files`.

The same base defect is behind the providers lock edge in section 6.

LLP64 fixture scan over the changed test paths
(`git diff --name-only 110a69c8e...HEAD -- '*tests*' | python3
docs/reports/windows-ci-llp64-fixtures-2/scripts/llp64_fixture_scan.py
--stdin`): exit 0, 0 counted, 0 found.

Targeted runs:
- `cargo test -p qcore-c-gateway --test call_with_deref_actual`: 4 passed.
- `--lib call_not_declarator`: 2 passed.
- `lake build QCore.CallNotDeclarator` and `AuditCallNotDeclarator.lean`: 9
  theorems, `propext` only.

## 8. Files likely to conflict

- `crates/qcore-c-gateway/src/legacy_classify.rs`: the one hook above.
  JUMPED-SHADOW-FORMAL-READINGS edits the same guard.
- `crates/qcore-c-gateway/src/legacy_c.rs`: one module line.
- Generated files, resolved by rerunning their generators:
  - `docs/agents/gateway.md` and `docs/agents/README.md`
    (`scripts/gen_rule_docs.py`);
  - `proof/QCore.lean` (`scripts/gen_lean_roots.py`).

## 9. Follow-ups

- **BASE-GATEWAY-FRONTEND-LAYERING** (new, registered). The base has two
  defects that MEMBER-EFFECT-STORES introduced:
  - `qcore-c-gateway`'s dependency on `qcore-frontend` fails
    `workspace_layering`;
  - `providers/Cargo.lock` misses the edge, so `providers/package.py
    checkers` refuses `--locked`.
- No other follow-up. The remaining blockers of the 12 changed rows are the
  call readings each row already owns (IND at a dereferenced actual,
  uncontracted callees, unresolved identities, out-slot stores). They are
  tracked by their existing entries, for example SLOT-NULL-TEST-ORDINARY-BUILDS
  and OUT-SLOT-MEMBER-THEN-FRESH on the OUT-SLOT branch.
- The occurrence limit in section 4 appears in no measured row, so no entry
  is registered for it.
