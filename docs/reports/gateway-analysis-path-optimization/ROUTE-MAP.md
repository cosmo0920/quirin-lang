# Gateway analysis paths: route map

Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION, task 1. Read at base `2ecd39531`
(line numbers are that revision's; `LC` = `crates/qcore-c-gateway/src/legacy_c.rs`,
every other path is under `crates/qcore-c-gateway/src/`). The consultation
orders were read from the code (four read-only census passes, spot-checked);
the Clang figures and the profile are measured (sections 7 and 8). Since the
card's split, `LC` code lives in `legacy_c.rs` and its child modules
(`legacy_classify.rs` holds `classify_simple*`, `legacy_condition_routes.rs`
the condition helpers, `legacy_return_routes.rs` the return-expression
routes, `legacy_cfg_builder.rs` `CfgBuilder::compile`); the order is
unchanged. Rule ids
are the [gateway rules](../../agents/gateway.md); a card name stands where
the code names no rule.

A **route** decides a position; a **fallback** is asked only after an older
route refused. "Clang" marks a reader that needs the unit's typed AST: every
such reader goes through `Parser::typed_positions_index`
(`legacy_typed_positions.rs:44`), a per-CFG-build `OnceCell` over
`typed_access_path::StatementIndex::read`, whose first use in a source scope
launches Clang for the whole unit (`typed_access_path::unit_ast`, 710). Every
Clang reader is fail-soft: no AST, no answer, the refusal stays.

## 1. Before the parser: one unit's token stream

`analyze_translation_unit_under_holds` (LC:2902) enters the source scope
(`typed_access_path::enter_source`, 2910), tokenizes once, then rewrites the
stream in this order before any reader: parenthesised callees (W45), rule
**11p** indirect-call candidates (`legacy_indirect_calls.rs:587`, Clang),
reserved spellings (W45-SR5), the **11y** dialect refusal
(`legacy_dialect_hook.rs`), review and pin holds, the LOADER-ABI withdrawal
(`declared_abi`), then the callee scope, `refuted_contract::settle` (the W4
derivation, which builds every in-scope function's CFG, possibly several
rounds), and finally one `analyze_defined_function` per selected name. Each
CFG build (`build_function_cfg_with`, LC:3230) re-reads the whole unit's
token stream for its function: `_Generic` resolution, literal premises, the
**11r** stored-return rewrite, the out-alias normaliser,
`find_function_body`, `function_parameters`, the protected formals and the
typed access facts are recomputed per build, and a function is built once per
derivation round plus once for its row.

Finding: `recursive_component_census` (LC:2816-2822) reads the stream without
the 11p rewrite, though its comment says it reads the stream the analysis
reads (census only; registered, section 9).

## 2. Statements (`Parser::parse_statement`, LC:6133)

Keywords first (`if`, `while`, `for`, `do`, `switch`, `case`, `default`,
`goto`, `break`, `continue`, `return`), then `{` (6599) and labels (6600); the
default arm (6612) takes every expression statement and declaration through
`expression_statement` (6728). A `for` clause goes to `classify_for_clause`
(`legacy_parenthesised_callee.rs:526`), which calls `classify_simple`
directly and asks none of the statement fallbacks.

### 2.1 `expression_statement` (LC:6728), in order

| # | Route (file:function) | Rule / card | Asked when | Reads | Decides |
|---|---|---|---|---|---|
| 1 | `legacy_call_exits.rs:30` `call_exit_boundary` | 11o | always, first | tokens | asm/SEH/`cleanup` boundary: `Unknown`, unbounded writes |
| 2 | `legacy_storage_duration.rs:57` `static_duration_declaration` | 11h | a storage keyword | Clang `DeclStmt` | `NoOp` |
| 3 | `legacy_cast_representation.rs:215` `pointer_test_store` | 11b | any `=` | Clang (`cast_index`); asks 10s `elaborated_statement` and `root_never_bound` inside | `None`, an elaboration, or the pointer-test refusal |
| 4 | `classify_simple` (LC:7167) | section 2.2 | always | | the instruction |
| 5 | `legacy_cast_semantics.rs:267` `uncast_call_actuals` | 12h (H-U0) | a `ContractCall` with `* )` | Clang; re-runs `classify_simple` on the uncast tokens | replaces the instruction (admit or refuse) |
| 6 | `legacy_call_exits.rs:80` `call_exit_statement` | 11o | a discarded `ContractCall` of a declared no-return callee | Clang (`call_exits::resolve`) | `PathEnd` or `Unknown` |
| 7 | `stepped_access` (LC:7579) | W43-INDEX-TRAVERSAL | formal-mutation or typed refusal, no call/braces | typed model | block |
| 8 | `legacy_call_argument_access.rs:643` `call_argument_access` | W44-INDIRECT-CALL-ARGUMENT-ACCESS; SR5; 10k, 11h (Clang) | exactly the gate refusal, no braces | re-runs `classify_simple` with the probe set (900) | reads, then the call |
| 9 | `legacy_elaboration.rs:458` `elaborated_statement` | 10s (+11n) | any `Unknown` not declined by `elaboration_declines` (423) | Clang; may re-enter `expression_statement` (676) | block |
| 10 | `legacy_cast_representation.rs:252` `uncast_pointer_statement` | 11b (E15) | `Unknown`, `=` then `(` | Clang; re-enters `expression_statement` (280) | the uncast statement |
| 11 | `legacy_call_instantiation.rs:523` `substituted_call` | 10u | K03 refusal **or an admitted** `ContractCall` with an unbound custody-taking position | Clang | block (can replace an admission) |
| 12 | `legacy_nested_call.rs:402` `nested_call_statement` | 10y | the nested-call or formal-embedded refusal | Clang; `nested_call_declines` scans the whole unit stream (358) | block |
| 13 | `legacy_value_connective.rs:132` `connective_statement` | 10z | `Unknown` spelling `?`/`&&`/`\|\|` | Clang; re-enters `expression_statement` (448) | statement |
| 14 | `legacy_statement_expression.rs:133` `statement_expression_statement` | 11g | `Unknown`, `({`, origin in the stream | Clang; re-enters `expression_statement` | statement |
| 15 | `legacy_aggregate_value.rs:50` `aggregate_statement` | 11f | `Unknown` in its refusal list, no opaque callee | Clang | aggregate lowering |
| 16 | `legacy_atomic_route.rs:443` `atomic_routed_statement` | 11i | any `Unknown` | Clang | block **or a re-worded `Unknown`**: `Some` either way, which ends the chain |
| 17 | `legacy_layout_extents.rs:92` `vla_declaration_statement` | 11u (M12) | `Unknown` with `[` | Clang | `ForgetScalar` |
| 18 | `legacy_layout_extents.rs:171` `container_of_statement` | 11u (E18) | `Unknown`, `=`, `-` | Clang; re-enters `expression_statement` | statement |
| 19 | `legacy_variadics.rs:506` `va_list_statement` | 11w | an uncontracted `va_*` refusal | Clang (`va_list_protocol::check`) | `NoOp`/`ForgetScalar`/`VaArgPointer` or a named refusal |
| 20 | `legacy_variadics.rs:363` `variadic_tail_statement` | 11w, 12g | the tail refusal of a callee with a declared tail | Clang; re-runs `classify_simple` widened (398) | block |
| 21 | `legacy_cast_semantics.rs:299` `uncast_value_statement` | 12h (H-U) | `Unknown` with `(` | Clang; re-enters `expression_statement`; `uncast_tail` re-runs `classify_simple` twice | statement |
| 22 | `record_instantiation_parity` (`legacy_call_instantiation.rs:183`) | 10u | a `ContractCall`, parity runs only | | no effect |
| 23 | `legacy_uncontracted_access.rs` `uncontracted_access_attribution` (landed after the base, carried through the rebase) | 12k | the gate's refusal | the callee's contract state | re-words the refusal to name an uncontracted callee where it decides; never admits |

Rows 7-21 are the statement fallback table the card made explicit
(`legacy_route_table.rs`; [report](README.md) section 2).

### 2.2 `classify_simple` (LC:7167)

`classify_simple` = list rule 4 ∘ 11v in-place guard ∘ W45 opaque-callee
refusal ∘ W43 descriptor guard ∘ `classify_simple_unguarded` ∘
`classify_simple_base`; it saves `(next_site, element_lowerings)` and restores
them only in its own guards.

`classify_simple_base` (LC:7896), in order: W45 list rule 3
`list_element_binding` (`legacy_list_iteration.rs:907`, mutates the list
frames); braces refusal; the formal-mutation gate (7919); formal shadowing
(7949); increments (7986, AOM-POISON-NOOP-INCREMENT); fixed scalar queries
(8013); W42 scalar member store (8040); scalar-field no-op (8062); the
exempted actual spans (8078-8189: C02-P borrowed actuals, O04-READ-BOUND,
O04-B09B10 member addresses, NEIGHBOURS-CHAINED-TAIL, CONSTRUCTOR-RESIDUALS,
W44 probe spans); opaque field read (8190); B08 admitted store (8213);
unevaluated `sizeof` spans, with Clang only for a `sizeof (` outside the
lexical spans (8231, 11b); the destination gate (8254); the seven call holds
(8280-8359: review hold, pin hold, refuted contract, reserved spelling,
literal premise, declared ABI, declared modes); the lexical gate (8360); a
contracted call (8431: O04-TRACE nested calls, nested refusal, formal
embedded, B08 result store, discarded owned result, excess actuals with
11w/12g widening, borrowed tail, K03 formal address with 11v in-place
addresses, 11v in-place actuals (Clang), result alias, container operands,
site minting at 8752); an uncontracted call (the seven holds again at
8811-8879, 11o `undeclarable_call` 8883, 11r `untested` 8891, "call to
uncontracted function"); scalar-only no-op (8908); B08 aggregate value
(8914); plain assignment (8962); `NoOp` (9008).

`classify_simple_unguarded` (LC:7251) then asks, for a refusal: W43 element
store/release (7257, re-runs `classify_simple_base` on rewritten tokens);
the W43 pointer step of a formal-mutation refusal (7270, which otherwise ends
here); W45 sub-object store of a call value (7304); then, only for a
`TYPED_SCALAR_CONSULTED` refusal or an indirect `Alias`/`Clear` without call
or braces: W43 typed scalar (7317), W44 chain (7325), W45 sub-object store
(7346), W43 pointer step (7355), W46/10v/11j typed path (7373, Clang).

## 3. Conditions

The order a condition's routes are **computed** at parse time is not the
order their results **take effect**: `CfgBuilder::compile` (LC:9805-10097)
applies the stored results by its own precedence. Both orders are listed.

Shared helpers: `supported_expression` (LC:5737, pure), rule **11r**
`relational_condition_plan` (`legacy_relational_post.rs:187`, mutates sites
and lowerings; a refusal plan is still `Some`), `condition_call_instruction`
(LC:5791, B02/B03; restores `next_site` only), `condition_access` (LC:5846;
pure routes, but the wrapper writes a parity row and, in parity runs, may
force the Clang index), `null_condition` (LC:5913, pure),
`distinct_condition` (LC:6010, pure), `element_condition` (LC:7879, pure; the
caller counts the lowering), `planned_condition`
(`legacy_condition_desugar.rs:482`, W44 then 10s/10x/12c inside; heavy side
effects, a no-op when not refused), `counted_loop` (LC:7820, pure).

| Statement | Parse-time order | Compile precedence |
|---|---|---|
| `if` (6136) | supported; 11r relational; call; access (null **not** checked, though the doc at 5836 says so); element; planned (only without a relational plan); bodies; null; distinct | plan, element, null, distinct, supported/call (`NoOp`, the call node first), access, `Unknown` |
| `while` (6213) | supported; call; access; planned(Loop); body; null | plan, null, call, supported, access, `Unknown` |
| `for` (6256) | W45 list iteration (ends the arm); supported; call; access; planned(ForLoop); body and clauses; counted loop; null | plan, call, `branch_instruction` (null, supported, access, `Unknown`); a counted loop replaces it |
| `do` (6375) | body; supported; call; access; planned(DoWhile); literal zero; null | plan, call, `branch_instruction` |
| `switch` (6428) | supported; call; access; planned(Switch); body | plan, call, supported, access, `Unknown` |
| `?:`/`&&`/`\|\|` value (`legacy_value_connective.rs:357`) | supported; call; access; null (once); element; distinct (once); planned(Branch); no 11r | as `if` |

Inside `planned_condition`: not refused or opaque, nothing; a call-free
condition goes to `typed_call_free_plan` (`legacy_condition_desugar.rs:963`),
otherwise `condition_plan` (558), whose leaves (`leaf_plan`, 666, rule 10x)
are decided call-free by null, descriptor refusal, distinct (branch only),
supported, `condition_access`, 10k `typed_condition_decision` (Clang),
`typed_assignment_operand` (Clang), 10s `elaborated_leaf` (Clang), and
call-bearing by `assignment_group`, `acquisition_split`, `instantiate_call`
(10u), the carrier check (`condition_on_cycle`, which reads the cursor, so
`planned_condition` must stay before the body parse) and the residual
decision. With no plan: 10s `elaborated_condition`
(`legacy_elaboration.rs:968`), then 12c `va_arg_condition`
(`legacy_va_condition.rs:59`).

Re-derivations: `null_condition` is evaluated up to four times per `if`/`for`
condition (`if`: 6168, 6188, 6205, plus the census at 6162; `for`: 6314,
6333, 6368, plus 6304), three times per `while`/`do`; `distinct_condition`
twice per `if`. Both are pure over fields nothing mutates during a statement,
so one evaluation per condition is the same answer (the card computes each
once, [report](README.md) section 2). `opaque_callee` (pure) runs six to nine times per refused
condition. The call-free desugar re-asks null, distinct, supported and
`condition_access` on the leaf; `condition_access` is not pure (a parity
row), so it is not hoisted.

## 4. Returns (`parse_statement`, LC:6530)

R = `return_refused` (LC:9105: not supported and no evaluation); E = the
evaluation `return_expression_instruction` produced.

1. 11b `pointer_test_return` (`legacy_cast_representation.rs:165`), first;
   it asks 10s `elaborated_return` inside (183).
2. `return_statement` (6624): W44-LEAK-PIN-AUDIT `cast_return::classify`
   (`legacy_cast_return.rs:78`; a refusal is final, an operand is re-sliced);
   11b `pointer_integer_return` (Clang); `supported_expression(Returned)`, then
   `(Decides)`; then `return_expression_instruction` (7044): opaque callee;
   W4 `return_call_instruction` (6987, `classify_simple(PLACE = expr)`); B01
   scalar call (7070, `classify_simple(call)`); B01 opaque value (7114, the
   same synthetic `classify_simple(PLACE = expr)` again); W43
   `lowers_refused_whole_call` (`legacy_return_call.rs:332`, re-attributes a
   refused whole call, so R becomes false); W45 families
   (`legacy_return_family.rs:586`; K re-attributes too); 10k
   `typed_return_instruction` (Clang); 11h `static_address_return` (Clang);
   then 10n `typed_connective_return` (`legacy_typed_residuals.rs:32`) and 12i
   `return_call_argument_access` (`legacy_return_call_access.rs:80`, for E
   absent or the gate refusal; `classify_simple(PLACE = expr)` a third time);
   last (since CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, after the census hook),
   12k `uncontracted_return_attribution`, which re-words the gate's refusal
   of a whole-call return to name its uncontracted callee.
3. Back in `parse_statement`, the return fallbacks, each for R unless noted:
   10s `elaborated_return`, 10u `instantiated_return`, 10y
   `nested_call_return` (R or the nested-call refusal), 10z
   `connective_return`, 11g `statement_expression_return`, 11i
   `atomic_routed_return` (R or an uncontracted atomic builtin), 11u
   `container_of_return`, 12f `scalar_return_value`. These eight are the
   return fallback table ([report](README.md) section 2).

Consequence of the order: a refused whole call is re-attributed in step 2,
so only 10y, 11i and 12i can still act on it; 10s, 10u, 10z, 11g, 11u and
12f never see it. The cast-operand return decides on the uncast operand, but
steps 3's routes receive the cast spelling.

## 5. Declarations

There is no declaration route of its own: a declaration is an expression
statement (section 2.1). The routes that read declarations specifically are
11h `static_duration_declaration` (row 2), the formal-shadowing gate
(LC:7949), 11u `vla_declaration_statement` (row 17), 11f
`aggregate_statement`'s `DeclStmt` reading (row 15) and the plain-assignment
arm of `classify_simple_base` for an initialised declaration; `int x;` lands
on the final `NoOp` (LC:9008).

## 6. Calls

A call is classified inside `classify_simple_base` (section 2.2): holds
before the gate, the gate, then the contracted or uncontracted arm. After
it: 12h `uncast_call_actuals`, 11o `call_exit_statement`, W44
`call_argument_access`, 10s (with 11n call footprints), 11b, 10u
`substituted_call`, 10y, 10z, 11g, 11f, 11i, 11u, 11w (`va_*`, tails), 12h
(section 2.1). In a condition, `condition_call_instruction` (B02/B03) and
the desugar's `instantiate_call` (10u) read calls; in a return, W4, B01,
W45 K and 12i (section 4).

Re-derivations: one `classify_simple_base` of a call statement evaluates
`find_call` thirteen times and `contains_call` (which rebuilds the text
vector) up to eight; `call_argument_spans` three times; the token-text
vector is rebuilt about 35 times across the hooks of one statement; a
refused `return f(a);` classifies the same synthetic `PLACE = f(a)` three
times (W4, B01, 12i) plus the W44 probe's fourth; `elaboration_node` (a walk
of the function's AST, twice when no exact match) is repeated for the same
statement by every Clang statement hook asked (rows 2, 3, 5, 9-21).

The seven call holds after the uncontracted arm (LC:8811-8879) repeat the
seven before the gate (8280-8359) with the same pure predicates on the same
callee (`find_call(&texts)` over unchanged texts), so the second block is
unreachable (section 9).

## 7. The Clang layer

### 7.1 Launch sites (read)

Every launch goes through `clang::command()` (`clang.rs:10`), which resolves
the executable once per (`QCORE_CLANG`, `PATH`) per process (one `--version`
probe). Modes and sites on the `pinned` and `current` routes:

| Mode | Site | Input | Frequency |
|---|---|---|---|
| `-E -P` | `legacy_preparation.rs:166` `preprocess_plain` | raw source (reads include files) | per validation |
| `-fsyntax-only` | `legacy_preparation.rs:224` `validate_syntax` | preprocessed text | per validation |
| `-E` (line markers) | `legacy_enumeration.rs:441` | raw source (reads include files) | per enumeration and per analysis: the pinned route runs it at `pinned_route.rs:906` **and** LC:2309 for the same unit; `boolean_source.rs:747` a third time for chunk-source units |
| `-ast-dump=json` (whole unit) | `typed_access_path.rs:710` `unit_ast` | preprocessed text on stdin | once per **source scope**: the analysis, the review-standing settle of a `Pending` row (`legacy_review_standing.rs:221`), the certificate self-check of a `Safe` unit (LC:2680) and, on `current`, the admission recheck (`certificate_context.rs:172`) each open a scope; the bounds side pass (`index_extent.rs:3849`, only under `QCORE_BOUNDS_PROPERTY`) one more |
| `-fdump-record-layouts-complete` | `typed_access_path.rs:669` | preprocessed text | **per consultation** of the 10w/G layout readers (`union_bitfield.rs:839`, `footprint_connection.rs:523`) |
| `-ast-dump-filter NAME` | `typed_c_ast.rs:14`; `typed_access_path.rs:640` (11y fallback); the source importers `typed_source.rs:439, 466, 522, 566, 603`, `boolean_source.rs:327, 456, 678, 748, 794`, `typed_source_match.rs:63, 94, 96`, `cast_representation.rs:290` | preprocessed text (or text + probe) | per record, typedef, enum constant and function of each importer call (chunkio, fluent-bit chunk, Monkey units) |
| `-dM -E`, `-print-target-triple` | `c_numeric.rs:405` | empty | once per process per executable (cached) |
| `-dM -E` with unit flags | `index_extent.rs:194` | empty | once per distinct command (cached), bounds side pass only |
| `--version`, `-dumpmachine` | `clang.rs:62` `ClangIdentity::capture` | — | uncached: twice per `current` run; two or three per pinned importer call |

Every production output buffer is parsed once (the whole-unit AST is
streamed into `AstIndex`, keeping function bodies and fact tables); the
redundancy is identical launches, not repeated parses of one buffer.

### 7.2 Measured: the corpus arbiter (`scripts/clang-census.sh`)

The BEFORE binary over the pinned corpus with `QCORE_CLANG` set to the
logging wrapper (`private/gapo/census-before-clang.jsonl`; its
`resource-diagnostics.json` is `d2f5a3f7…`, the BEFORE digest, so the
wrapper changed no output). A *repeat* is a launch whose argument vector and
standard input equal an earlier launch's.

| Mode | Launches | Clang seconds | Output | Repeats (seconds) |
|---|---:|---:|---:|---:|
| `-E` / `-E -P` | 762 | 31.4 | 505 MB | 390 (16.1) |
| whole-unit AST | 188 | 25.2 | 6,652 MB | 7 (0.3) |
| filtered AST | 276 | 14.9 | 425 MB | 88 (2.9) |
| `-fsyntax-only` | 221 | 11.8 | — | 34 (1.3) |
| record layouts | 84 | 6.5 | 26 MB | **82 (6.3)** |
| `--version`, `-dumpmachine`, `-dM`, other | 18 | 0.2 | — | 0 |
| **total** | **1,549** | **90.1** | | **601 (26.9)** |

378 distinct inputs; a median of 3 launches per input, at most 73. Against
the corpus arbiter's 658 s at four workers (about 2,630 worker-seconds),
Clang processes are **3.4 %** of worker time, which the profile (section 8)
confirms; parsing their 6.65 GB of AST JSON is a larger share (16.7 %). The
preprocessing repeats read include files from disk, so their output is not a
function of the launch's arguments and standard input alone; the card does
not reuse them ([report](README.md) section 3).

The system and slice routes cannot be censused this way: `quirin-audit
current` refuses an executable other than the compilation database's
compiler ("audit requires the configured Clang executable"), a fail-closed
check the wrapper trips. Their Clang share is read from the profile instead.

### 7.3 The test suite

`run_test_binaries.py` at the base with the wrapper as `QCORE_CLANG`
(`scripts/tests-timing.sh bcensus 1`; measurement only: the wrapper doubled
the run phase, and one test, `qcore-audit::test::configured_cli`, failed
because it records the configured Clang path; [summary](artifacts/clang-census-suite-before.json)).
Unlike the arbiter, the suite is dominated by Clang:

| Mode | Launches | Clang seconds | Repeats (seconds) |
|---|---:|---:|---:|
| filtered AST (`-ast-dump-filter`) | 27,558 | 1,399.5 | **23,720 (1,180.6)** |
| other (compiling and linking generated C) | 3,347 | 1,272.0 | 0 |
| `-fsyntax-only` | 14,550 | 774.1 | 10,461 (493.2) |
| `-E` / `-E -P` | 15,278 | 760.9 | 10,122 (453.2) |
| whole-unit AST | 8,180 | 585.7 | 3,891 (237.9) |
| `--version` | 11,590 | 439.8 | 0 (each a fresh identity capture) |
| `-dumpmachine` | 9,497 | 318.2 | 0 |
| `-dM -E` | 701 | 48.4 | 694 (47.9) |
| record layouts | 334 | 17.0 | 297 (14.4) |
| **total** | **91,035** | **5,615.7** | **49,185 (2,427.1)** |

The Clang seconds are summed over concurrent processes (the run phase was
595 s with the wrapper, about 300 s without). The heaviest launchers are the
source-import and route-parity tests: `quirin-source-analysis`'s tests
(4,267 launches, 406 s), `scalar_tasks` (693 compilations, 399 s), the
`*_composition_mutations` and `*_composition_source` tests (6,000-7,000
launches each, about two thirds repeats) and the `*_route_parity` tests;
the importers ask the same record names of the same text thousands of times
(`cio_ctx` 2,327, `cio_file` 2,161, `cio_chunk` 2,125, `cio_stream` 2,125).
Step 7 of the card answers a repeated filtered-AST launch from the first
one; the repeated `-fsyntax-only` and preprocessing launches are not reused
(preprocessing reads include files; the preparation's invocation count is a
reported figure), and `ClangIdentity::capture` stays uncached
(`GAPO-REMAINING-REDERIVATIONS`).

## 8. Profile (wall-clock samples, `scripts/gdb_sampler.py`)

perf is unavailable to this user (`perf_event_paranoid = 4`), so the
profile is gdb interrupting the arbiter every 0.5 s and recording every
thread's stack (`scripts/profile-run.sh`, summarised by
`scripts/profile.py`). Shares are of *busy worker* thread-samples (the
coordinating thread waiting for its workers excluded), so they approximate
shares of worker wall time; the sampled runs' outputs equal BEFORE
(`d2f5a3f7…`).

| BEFORE | Corpus (4,802 samples) | System route (901) |
|---|---:|---:|
| Contract environment re-read (`qcore_contract::contract_loader` → `qcore_frontend::lexer::lex`) | **42.9 %** | **59.5 %** |
| Clang AST JSON parse (serde) | 16.7 % | 2.4 % |
| Waiting on Clang processes | 3.9 % | 2.7 % |
| The analysis proper | 36.5 % | 35.4 % |

The contract-environment readers are asked by `build_function_cfg_with`
(32.9 % of corpus worker time; 45.8 % on the system route),
`counted_users::evaluate_function` (7.5 %; 8.1 %) and
`call_exits::declared_no_return` (2.1 %; 3.7 %): each CFG build (one per
function per W4 derivation round, plus the row's own) re-parses and re-lexes
every declared module text of the environment for each of nine readers
(literal premises, lifecycle, loop-owner and thread-sync obligations,
variadic data and borrowed tails, no-return callees, ABI signatures, checked
specifications). The top self-time functions after the lexer are
`load_function` (7.7 %), the source parser (6.4 %),
`literal_premise::restrict_to_declared_literals` (5.1 %, a whole-unit token
pass per CFG build), `typed_access_path::node_line` (4.0 %) and
`build_function_cfg_with` itself (3.6 %).

Wall time per phase is not separable further without instrumentation; the
per-unit split is the arbiter's own sequence (preparation, enumeration,
analysis), and the analysis dominates (`audit_unit` is in 96 % of busy
samples).

## 9. Findings from the map (registered, none fixed here)

Each is a reading of the code, not a measured failure; each is registered
with the acceptance matrix as `GAPO-*` (the report lists the ids). The card
changed none of them: fixing any could move an output.

1. **Probe state leaks in the return position.** The W4, B01 and W45-K
   probes of a refused return (LC:6987, 7091/7111, 7117/7127,
   `legacy_return_family.rs:660`) restore `next_site` only; `classify_simple`
   also moves `element_lowerings` (which decides a `for` loop's counted
   lowering at LC:6330), the list frames and the element origins. The doc
   comment at LC:6983 says `next_site` is its only mutation. Whether a
   declined probe can leak observably is open.
2. **Rule 12h re-classifies without restoring sites.** `uncast_call_actuals`
   (`legacy_cast_semantics.rs:286`) classifies the uncast statement after the
   first classification minted sites, leaving gaps in site numbering.
3. **The return position asks fewer statement routes.** 12h H-U0, 11o
   `call_exit_statement`, the 11w tail and `va_*` hooks and 10u
   `substituted_call` are asked for a statement but never for `return
   CALL(...)` (e.g. a cast `Borrowed` actual in a returned call).
4. **11o sees only the first lowering.** `call_exit_statement` reads the
   instruction of `classify_simple`/12h; a no-return call lowered later by
   W44, 10u, 10y or 11w never becomes a `PathEnd`.
5. **`variadic_tail_statement` keeps counters** when its widened result is
   not a `ContractCall` (`legacy_variadics.rs:395-451`); the descriptor guard
   (LC:7209, 7224) restores nothing.
6. **The `if` arm asks `condition_access` without the null test** its doc
   comment (LC:5836) says it requires; the compile precedence lets the null
   decision win, so only a wasted consultation and a parity row result.
7. **`recursive_component_census` skips the 11p rewrite** (LC:2816-2822),
   contrary to its comment (census only).
8. **Dead code.** The seven call holds after the uncontracted arm
   (LC:8811-8879) are unreachable (section 6); `condition_call.is_none()` in
   the `for` arm's counted-loop test (LC:6333) is implied by the supported
   test before it.
9. **Rule 11i ends the statement chain with a refusal.** `atomic_routed_statement`
   returns `Some` for a re-worded `Unknown`, so 11u, 11w and 12h are never
   asked for a statement it matched.
10. **Re-derivation without observable effect** (candidates for later
    speed work, each needing its own identity proof): the repeated
    `classify_simple(PLACE = expr)` of a refused return, `elaboration_node`
    walks per hook, the token-text vector and `find_call` recomputations, and
    the unit-wide scans (`place_is_unspelled`, `find_function_body`,
    `nested_call_declines`) per consultation.
