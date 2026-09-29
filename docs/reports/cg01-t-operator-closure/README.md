# CG01-T-OPERATOR-CLOSURE: route T's operators and literals, admitted per consumer

Card CG01 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 1; base `d3508e10`). It covers gap entries E01, E02, E03 (the T
part) and E14; the evidence is Q11, Q12 and Q16. Route: **T**, the bounded
typed-source importer `crates/qcore-c-gateway/src/typed_source.rs`. Route G is
unchanged. Rule: [gateway 10r](../../agents/gateway.md#constraint-10r). The
prediction was frozen before any importer change was committed
([PREDICTION.md](PREDICTION.md), commit `5d9db377`).

## What changed

**The importer reads the closure.**

- Binary `-` `<=` `>=` `^` `<<` `>>` and unary `+` `~`.
- The ten compound assignments `+= -= *= /= %= <<= >>= &= ^= |=`, as
  statements only.
- Floating and character literals, as two new `SourceExpression` variants,
  `Floating { value }` (Clang's spelling) and `Character { value }` (Clang's
  integer code), each with its Clang type.

The new logic is in a new module, `typed_source_nodes.rs`. `typed_source.rs`
gains thin hooks: the `_` arm of `Importer::expression`, the
`CompoundAssignOperator` arms of `Importer::statement`, `Path::compound`, and
the admitting entry points.

- **Types.**
  - An operator needs promoted integer operands and a promoted integer result;
    `<=` and `>=` produce `int`.
  - Pointer subtraction, pointer minus an integer and relational pointer
    comparison stay refused. They belong to gap entries M15/M16 (card CG16).
  - Floating arithmetic stays refused (N09).
- **Compound assignment.**
  - The place must be one of T's local scalar types (`int short long unsigned
    int unsigned long unsigned long long`, the only integer types T's profiles
    already admit). The right operand and Clang's `computeLHSType` and
    `computeResultType` must be promoted integer types.
  - The projection evaluates the place **once**: one `Read` of the resolved
    place, or the local's value. The old value is converted to the computation
    LHS type, the operation runs at the computation result type, the result is
    converted back to the place type (each conversion an explicit
    `IntegralCast`), and there is one `Write` or `LocalWrite`.
  - `int |= int` through this path is byte-identical to the legacy `|=`
    projection (`cg01_int_or_assign_matches_the_legacy_projection`). The legacy
    `update` now delegates to the same `update_with`, with unchanged output.
  - A right-operand call and an effectful address are refused, as for `|=` and
    `++`.
  - A compound assignment used as a value keeps
    `unmodeled callback expression CompoundAssignOperator` (CG02).
- **Sequencing.** `reject_unsequenced` sees through every new operator, since
  `effects` is structural over `Expr`. `p[a] ^ g()`, `g(a) - g(b)`,
  `(a << 1) >= g()` and `*p += g()` are refused as unsequenced; `a[i] -= i++`
  is refused because `i++` is no expression of T.

**Reading is not admission.** Every importer call now carries a
`NodeAdmission`, the consumer's own classification of each CG01 node.

- It is validated against `NODE_TABLE` before any function is read, and an
  invalid admission fails the whole call.
- A node the admission does not list is refused by name, for example
  `` typed-source node `binary ^` is not classified by the legacy typed-source
  consumer ``.
- **Classes.**
  - *Custody-neutral* is allowed only for a node with no C fault case and no
    store: `<=`, `>=`, `^`, unary `+`, `~` and the two literals.
  - *Numerically evaluated* is allowed only through `checked_binary`, which
    refuses these cases:
    - signed overflow;
    - `/` or `%` by zero;
    - `MIN / -1`;
    - a negative or too-large shift count;
    - a left shift of a negative value;
    - the implementation-defined right shift of a negative value.
  - Every other node is *obligation-bearing*, never discharged.
- `import_source_functions` and `import_source_functions_from_profile` pass
  `NodeAdmission::LEGACY`, which admits nothing. The admitting variants are
  `import_source_functions_admitting` and `…_from_profile_admitting`.

## Consumers decide

There are 20 production import call sites (the three inside `typed_source.rs`
are the entry points themselves). A read-only census of every reader of
`SourceExpression` / `SourceOperation` classified each reader's behaviour on an
unknown operator and on a new variant. The census was delegated. The one
admitting chain (the Monkey wrapper import, its only reader in `pinned_route`)
and the numeric readers were re-read before deciding. The LEGACY rows rest on
the census; keeping a consumer LEGACY needs no justification beyond it. The
verdict per import is:

| Import (consumer) | Admission | Why |
|---|---|---|
| `import_monkey_wrappers` (`quirin-source-analysis/src/libevent/monkey_source.rs`) | **`MONKEY_WRAPPER_ADMISSION`**: the 7 fault-free, storeless nodes custody-neutral, the other 13 obligation-bearing | Its only reader is `pinned_route`'s projection report. Each wrapper is reported as "opaque calls and custody Unknown" (and, with a scan loop, "loop bounds, arithmetic, termination and custody Unknown"), and `monkey-source.json` is `source_projection_only`. It certifies nothing, so no obligation is discharged. |
| `import_monkey_callbacks`; `observed_wait_inputs.rs:223` (callbacks and wrappers); `qcore-audit` `callback_task_handoff.rs:372` | LEGACY | Downstream readers would accept an unknown operator undecided: <ul><li>`cycle18_callback_obligations::timeout_steps` ignores guards and writes while discharging `AllPathExecutionCoverage`;</li><li>`event_payload::identity` turns any operator into a structural identity, and `allocation_shape` has a negative `returns_context` test;</li><li>`callback_task_handoff`, `monkey_wait_projection` and `timeout_teardown` read call sequences only.</li></ul> |
| `call_use_source.rs:314`; `chunkio/file_attempts.rs:44,74` | LEGACY | `source_call_use::symbols` recurses into any operator, so a guard such as `r ^ x` would count as a checked call result in `call_use_contract`. |
| `counter_source.rs:178`; `chunkio/teardown_source.rs:51` | LEGACY | `counter_contract` would accept a `+= 1` / `-= 1` whose shape matches `++` / `--`. |
| `teardown_source.rs:320,424`; `chunkio/teardown_source.rs:19,91` | LEGACY | `field_destructor`'s replay treats any non-`==` binary as `!=`. Only its earlier `validate_value` keeps that from accepting a new operator. |
| `composition_source.rs:628,992,1332`; `memory_close_composition.rs:331`; `filesystem_chunk_close_composition.rs:359` | LEGACY | The projections copy the operator verbatim. The core readers (`unmap`, `file_close`, `memory_close`, `unsupported_close`) refuse unknown operators, but their literal parsers take `i32`, and no reading justifies an admission. |
| `cleanup_replay_source.rs:370` | LEGACY | `cleanup_outcome` refuses unknown operators, and no classification was needed. |

- **Exhaustive matches.** Four readers match `SourceExpression` exhaustively:
  `call_use_source::value`, `cleanup_replay_source::value`,
  `composition_source::value` and `monkey_binding::Replay::eval`. Each now
  refuses the two literal variants by name, even though its import is LEGACY
  and they cannot reach it.
- **Wildcard arms.** Every other `_ =>` arm over `SourceExpression` either
  refuses or belongs to a LEGACY consumer.
- **Leaf variants.** Inside the importer, `visit_value` and `check_references`
  treat the two literal variants as leaves, which they are.
- **Numeric consumers.** No consumer evaluates a CG01 node numerically. The
  numeric readers keep their explicit operator lists:
  `cycle18_callback_obligations::evaluate_integer` (`| & +` and unary `-`) and
  `monkey_binding::eval` (`+ & | == !=`). Numeric semantics proper remain
  CG20. `checked_binary` is the kernel a future numerically evaluating
  admission must use.

## Cited claims

| Claim | Reproduced? |
|---|---|
| E01: `-` `<=` `>=` `^` `<<` `>>` absent from `Importer::expression`'s `BinaryOperator` allowlist (Q11) | **Yes.** 36 BEFORE rows (6 operators × return, assignment, branch, short-circuit guard leaf, index and call-argument positions) are `unmodeled callback expression BinaryOperator` at base. |
| E02: T's `UnaryOperator` is only `& * - !` | **Yes.** 12 rows are `… UnaryOperator`. |
| E03 (T): `CompoundAssignOperator` only `int \|=` (Q12) | **Yes.** 63 rows are `unmodeled callback statement CompoundAssignOperator`; `\|=` imports on its 5 `int`-place rows and is `compound assignment requires int operands` on the `short` and `long` places. |
| E14: no floating/character literal branch in T | **Yes.** 6 rows are `… FloatingLiteral` / `… CharacterLiteral`. |
| Q16: G's scalar reader reads all of these | **Yes, by reading** `typed_access_path.rs:970–1008`: literals, unary `+ - ! ~` and all six binary operators on scalar operands. G is unchanged and was not run. |
| Q11/Q12 line ranges `582–796` / `798–960` | **Approximate.** At base, `Importer::expression` spans 584–788 and `Importer::statement` 790–956. The content is as quoted. |

**Precision on E01.** T already *emitted* `binary -` before this card:
`x--` projects as `Read`, `Binary("-", old, 1)`, `Write` through `update`.
What E01 names is the allowlist of source operators, which reproduces. Consumers
that match `"-"` (`counter_contract`, the close compositions' unary `-`) are
unaffected: they are LEGACY and their inputs are unchanged.

The BEFORE test (`cg01_before_legacy_route_refuses_the_cited_operators`, 124
rows over the shared vectors in `tests/unit/source_operators_vectors.rs`) was
committed and executed at base as `3f6c5cc3`
([log](artifacts/before-refusals.log)). It was then retired, because its
expected reasons are the base's. `cg01_legacy_route_refuses_each_node_by_name`
now asserts the same 124 rows' refusals, with the named reasons.

## §8 axes

| Axis | Covered (admitted under an admitting consumer) | Stays refused |
|---|---|---|
| Position | return value, assignment right-hand side, branch condition, short-circuit guard leaf, index, call argument; compound assignment as a statement, including inside a branch arm | compound assignment as a value (CG02) |
| Place | local, `*p`, `p->field`, `p[i]` | places with a call (effectful address) |
| Type | `int`, `unsigned int`, `long`, `unsigned long`, `long long`, `unsigned long long` operands; `short` places through Clang's `int` computation | pointer operands (M15/M16), floating operands (N09), non-profile place types |
| Evaluation order | pure reads in AST order under the existing exclusive-access boundary | any call beside another read or call through a new operator; an RHS call in a compound assignment |
| Effect | compound writeback, one `Write` | — |
| Boundary / evidence / verdict | no verdict moves; the one admitting consumer certifies nothing | every other consumer, by name |

## Evidence

**Rust** (`crates/qcore-c-gateway/tests/unit/source_operators.rs`, 9 tests):

- the legacy route's named refusals (124 rows);
- the admitting route's imports: 124 rows, every node in every position;
- the compound assignment's single read, conversions and writeback;
- `int |=` identity with the legacy projection;
- 14 refusal controls, including `a[i] -= i++`, pointer `-` / `<=`, floating
  `-`, `x -= g()`, `p[g()] ^= 1`, compound-as-value, and a partial consumer
  refusing `-` by name;
- `x /= 0` and `a << -1`, which import only as non-neutral nodes, while
  `checked_binary` refuses both;
- admission validation (a fault-bearing or storing node called custody-neutral,
  an unknown spelling or a duplicate fails the whole import);
- 18 `checked_binary` vectors;
- the Lean binding.

In addition, `quirin-source-analysis/tests/unit/monkey_source.rs` checks that
the wrapper admission classifies all 20 nodes, each as strongly as its facts
allow and no more.

**Lean.** `QCore.TypedSourceOperators` is registered in the new
`AuditTypedSourceOperators.lean`, with 22 theorems, no `sorry` and no project
axiom.

- *Import totality and refusal*:
  - `unlisted_binary_refused` and `unlisted_unary_refused`;
  - `read_binary_node`;
  - `comma_refused`, `assignment_refused` and `increment_refused`;
  - `xor_is_a_node`;
  - `legacy_admits_nothing`, `unlisted_node_refused` and
    `invalid_admission_refuses`;
  - `admit_sound`.
- *Consumer soundness*:
  - `neutral_leaves_frame`: an admitted custody-neutral node's accesses leave
    every custody frame unchanged;
  - `faulty_not_neutral`, `compound_reads_once_writes_once` and
    `spellings_unique`;
  - for the numeric kernel, `division_by_zero_refused`,
    `quotient_overflow_refused`, `shift_count_refused`,
    `negative_left_shift_refused`, `signed_result_in_range` and
    `checked_some_exact`;
  - `kernel_vectors_hold`, 19 vectors.
- **Binding.** The module is **mechanically bound**, not only transcribed
  (compare `LEAN-PARITY-TRANSCRIPTION-CENSUS`).
  `cg01_lean_vectors_match_the_rust_tables` reads `legacyBinary`,
  `legacyUnary`, `nodeTable` and `kernelVectors` from the `.lean` file and
  compares them with `LEGACY_BINARY`, `LEGACY_UNARY`, `NODE_TABLE` and
  `checked_binary`.
- **Premises.** The Clang-JSON-to-`Expr` reading remains a review premise. So
  does the correspondence between `accesses` and the importer's
  `Read` / `Write` emission, which the Rust test
  `cg01_compound_reads_once_converts_and_writes_back` checks.

## Prediction against outcome

| Prediction | Outcome |
|---|---|
| 1. Corpus arbiter AFTER byte-identical to BEFORE | **Held.** Every verdict-bearing artifact is byte-identical to this card's BEFORE run and to the archived baseline: `resource-diagnostics.json` `ae17152b…`, `…chunk-contracts.json` `cbbcfd9c…` and `monkey-source.json` `57acfc58…`. `…contracts.json` and `…source-effects.json` are identical to BEFORE. The explained differences against BEFORE are listed below the table. **No function moves**, and there is no correction. |
| 2. Legacy route: same 124 refusals, named reasons | **Held.** |
| 3. Admitting route imports every row; `int \|=` identical | **Held.** |
| 4. Refusal controls refused | **Held.** `*p += g()` is refused one check earlier than written, as unsequenced; the opaque-call branch is exercised by `x -= g()` on a local. |
| 5. No CT or source-profile test moves | **Held.** The full suite passes (see the gate), and no pinned text changed. |
| 6. One new Lean audit | **Held.** 226 → 227. |

The explained differences against BEFORE:

- `run.json`: timestamps and paths;
- `audit.log`: the output-path line only;
- `parity.jsonl`: the same 153,414 records in `--jobs 8` streaming order.

The arbiter command was `QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3
scripts/run_pinned_audit.py OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`,
both runs exit 0. Identity records:
[before](artifacts/arbiter-before.identity),
[after](artifacts/arbiter-after.identity) and
[after, final tip](artifacts/arbiter-after-final.identity). The comparison script
is `private/cg01/identity.py`; its method is stated in the identity files. The
system slice is not due: route T does not reach it.

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 361 units, 374 summaries; **6,455 passed**, 0 failed, 11 ignored (`private/test-binaries/20260929T110141Z-526157/`) |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | a pre-gate run flagged `too_many_arguments` on `typed_source::derive`; fixed by bundling the profile and admission before the gate |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **227 audits** (226 + `AuditTypedSourceOperators.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix edit) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| clean | `cargo clean --profile dev` | 0 | |

The Rust, Lean and probe steps ran at tip `950c5744`, where the source is final.
Exits are in [gate/](gate/) and the record in [gate.json](gate.json); logs are
under `private/cg01/gate/`. `providers/` is unchanged, so the provider gate is
not due. Clang 21.1.8 was recorded in this worktree
([gate/clang-version.txt](gate/clang-version.txt)).

## Follow-ups

- **Retired:** `CG01-T-OPERATOR-CLOSURE`. Every DISCHARGE clause holds:
  - BEFORE reproduces the cited refusals;
  - the prediction is frozen;
  - AFTER has no moving function;
  - the card absorbs no follow-up;
  - there are Rust refusal controls and audited, mechanically bound Lean.

  Its scope condition, that each node is classified by every consumer or stays
  refused, holds: one consumer classifies all 20 nodes, and 19 refuse them by
  name.
- **Registered:**
  - `CG01-T-CONSUMER-ADMISSIONS`: 19 production import sites keep LEGACY. The
    blocking readers are named in the table above.
  - `CG01-CYCLE18-EVALUATOR-WIDTH`: seen in passing.
    `cycle18_callback_obligations::evaluate_integer` strips every cast,
    including `IntegralCast`, and computes in `i128` regardless of the C type.
    Its unary `-` is unchecked. It evaluates closed guard constants only.
- **Unchanged:** `CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING`.
  `c_type` now delegates to `qual_type`, but the `volatile` / `_Atomic` /
  `restrict` refusal is byte-identical.
