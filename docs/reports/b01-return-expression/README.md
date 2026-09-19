# B01-RETURN-EXPRESSION

Card B01 lands two bounded evaluations behind the C gateway's existing return
refusal. It does not treat a return expression as a neutral `NoOp`:

1. exactly one **unconditionally evaluated**, contracted call with
   `ResourceReturnMode::None`, declared outside the translation unit, may occur
   inside an otherwise custody-neutral scalar expression. The ordinary
   `ContractCall` applies its parameter effects exactly once; a `NoOp` exit
   returns the resource-free scalar value;
2. one complete, direct typed member read which the existing assignment
   classifier calls `OpaqueValue` may be returned from an untracked base. The
   retained `alias_read` check still runs and the `NoOp` exit claims no resource
   identity and escapes none.

These two additional routes decline `?:`, `&&`, `||`, assignment,
increment/decrement, type operators, multiple calls, a custody-bearing embedded
call result, a protected or bound member base, a conditional pointer result, a
locally defined embedded callee and a member chain of two or more selections.
When no older independent admission applies, the pre-card generic refusal stays
byte for byte. The existing exact-whole-call and neutral-position admissions
run separately and remain unchanged; this is not a global claim that every
source containing one of those forms is `Unknown`.

## Rule and implementation

`Parser::return_expression_instruction` first delegates the existing exact-call
rule. It then checks the reserved synthetic place is absent. The scalar-call arm
uses the shared `legacy_condition_call::unconditional_call_span` boundary from
B02; it independently verifies the residual tokens with
`is_scalar_operand_expression`, classifies the isolated call through
`classify_simple`, and accepts only a destination-free `ContractCall` whose
result mode is `None`. It also requires that `find_function_body` find no local
definition for the callee. Fixed-point summaries for locally defined functions
are not declaration evidence for this card, and this premise preserves W4's
recursive pointer-actual refusal. This avoids a second call semantics.

The opaque arm asks the same synthetic assignment question as an ordinary
statement. It accepts only `OpaqueValue` into the synthetic place and only when
the expression has exactly one `.` or `->` selection. The direct-selection
bound is deliberate: an interim version also admitted
`return chunk->st->name;`; existing chained-return pins and the larger aliasing
argument remain outside this card.

Both new arms set `returned_evaluation_escapes = false`; their exit is `NoOp`.
The pre-existing whole-call arm alone retains `EscapeIfOwned`. An earlier
interim implementation sent the scalar result through `EscapeIfOwned`, which
exchanged 13 generic return leaves for 13 `UnknownAlias` leaves on the reserved
place. That run was rejected before publication, and the tests now assert that
the synthetic alias is absent.

Audited Lean module `QCore.ReturnExpressionEvaluation` models the admission
gates, conditional/collision refusals, exact call effect, visible retained
obligations, the non-vacuous release, and the opaque arm's no-custody result,
alias gate, tracked-base refusal and conditional-result refusal. The scalar
model also proves that a locally summarized callee remains refused. It is
imported by `QCore.lean` and audited by
`AuditReturnExpressionEvaluation.lean`. As
constraint 28 requires, this is an abstract rule theorem; C extraction and the
Rust/Lean correspondence remain review premises.

## Prediction, published before AFTER

The planning census at `docs/reports/w37-corpus-contracts-4/artifacts/` named
1,452 leaves over 1,120 Unknown functions and 471 sole-blocker functions. Before
any AFTER run, this card predicted **0 function status moves** and expected the
bounded scalar-call slice primarily to exchange or retain refusals. B01's raw
query counted 1,456 leaves over 1,123 functions of every status, with the same
471 sole blockers. That four-leaf/three-function difference is a query
denominator difference, not baseline drift: the fresh diagnostics artifact is
byte-identical. The zero-move prediction was wrong.

The miss is attributed, not rewritten: the statement assignment classifier's
opaque getter arm was reusable at return, and seven direct librdkafka getters
had no other blocker. Their combined struct-typedef member declarations are not
the exact tracked-pointer leaf accepted by `neutral_position::Decides`; the
older rule therefore refused them. `is_opaque_field_read` answers a different,
weaker identity question under its fully-read declaration, untracked-base and
alias premises. B01 reuses that answer without claiming or escaping an identity.

## BEFORE and AFTER measurement

The parent measured BEFORE once at source commit `8e633261fd216a76d4d25390b0dadeda891c59aa`
under `/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline/`:
the release audit exited 0; `resource-diagnostics.json` and
`chunk-contracts.json` reproduced the two pinned verdict-bearing baselines, and
`monkey-source.json` separately reproduced its source baseline. The recorded
source-effects/contracts identity deltas are itemized in that run's
`artifact-diff.json` rather than hidden.

The final card AFTER is the run `after-repair` under
`/tmp/agent-b01/measure/w37/runs/`, with the corrected runner copied under
`measure/b01-run/` so its `parents[2]` resolves to the worktree root. It ran the
pinned Fluent Bit source, compile database and ChunkIO package, exited 0 in
291.6 seconds at analyzer commit `d7ed1a45`, and produced the five committed
compressed artifacts here. Its five JSON artifacts are byte-identical to the
earlier reviewed-tree run, so the declared-only repair changed no corpus count.

| Measure | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| return-position generic leaves | 1,456 | 1,444 | −12 |
| functions showing that leaf | 1,123 | 1,111 | −12 |
| function Safe | 213 | 220 | +7 |
| function provisional Unsafe | 84 | 84 | 0 |
| function Unknown | 4,537 | 4,530 | −7 |
| unit Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | same | 0 |
| leak | 272 | 272 | 0 |
| unknown_alias | 4,309 | 4,309 | 0 |
| unsupported | 49,753 | 49,741 | −12 |
| summary_conflict | 1 | 1 | 0 |

There are **zero exchanged or added leaves**. The twelve withdrawals are direct
getters. Seven become Safe:

* `rd_kafka_ConsumerGroupDescription_coordinator`
* `rd_kafka_ListOffsetsResultInfo_topic_partition`
* `rd_kafka_MemberAssignment_partitions`
* `rd_kafka_TopicPartitionInfo_leader`
* `rd_kafka_group_result_partitions`
* `rd_kafka_topic_partition_result_partition`
* `rd_kafka_share_partition_offsets_partition`

The other five retain unrelated blockers. No unit moves and raw units remain
unpromoted. The 272 leak pin is unchanged. The result is evidence for this
pinned corpus and these declarations, not full C conformance.

Two attempts are excluded from the measurement. In the first, the output
directory was moved while the binary still held it open; the audit failed to
write its final artifacts and exited 1. In the second, the archived runner was
invoked in place, so its historical `__file__.parents[2]` resolved to
`docs/reports`; that run was interrupted by its own captured session before
completion. Their private outputs are under `measure/b01-after/` and
`measure/discarded-cwd-attempt/`. No count above comes from either attempt.

The first full isolated standing-gate run is also excluded. It completed every
suite but exited 101 with one real failure: W4's
`a_pointer_actual_stays_unknown` pin observed `Safe` for locally defined
recursive `1 + counted(p)`. The cause was that fixed-point summaries share the
contract environment with declared APIs. Commit `d7ed1a45` added the explicit
declared-only premise, a B01 regression row and its Lean gate. That failed run
measured 4,820 passed / 1 failed / 10 ignored over 246 summaries and is retained
at `measure/b01-gate-final/cargo-test.log`; it is not the published gate.

The final artifact run used the release analyzer built from `d7ed1a45`; the
following commit `0e91d859` changes only a test fixture's syntax to satisfy
Clippy. The integrator still owes a combined-tree remeasurement because sibling
cards share this parser.

## Held-out evidence and mutations

The focused suites exercise the real Clang gateway. They compare scalar-call
returns with explicit `call; return scalar` spellings, expose a retained
obligation through a borrowed call, pin skipped `?:`/`&&` calls, collisions,
owned results and multiple calls, and mirror the production combined-typedef
getter form. Protected and bound getter bases and a conditional pointer result
stay refused.

`scripts/mutations.py` ships five mutations. It restores by rewriting, touches
the source, asserts the byte round trip and runs a final green baseline. A
future run respects `CARGO_TARGET_DIR` and writes to a fresh timestamped private
directory, or to a fresh `QCORE_MUTATION_LOG_DIR`; it never overwrites this
report's archived evidence. Results in `logs/mutations.txt`: scalar call
disabled exit 101;
declared-only guard disabled exit 101; opaque read disabled exit 101; collision
guard disabled exit 101; opaque result escaped exit 101; restored baseline exit
0. Thus both positive families and the three fail-closed/no-escape gates are
independently load-bearing.

## Validation

The full standing gate uses `CARGO_BUILD_JOBS=3`, Lean jobs 2 and the external
target `/home/cosmo/Documents/GitHub/qcore-c/private/b-series/targets/b01`.
Run-specific private logs live under `/tmp/agent-b01/measure/b01-gate-repair/`.
The final counts and exits are recorded in `gate.json`; the cargo suite was run
outside the ptrace sandbox so native sanitizers were not disabled.

| Command | Exit / measured result |
|---|---|
| `git submodule update --init --recursive` | 0; libevent `d82464a277d0f42703702c4dfd9af6af38595a83` |
| `cargo test --workspace --no-fail-fast` | 0; 4,822 passed / 0 failed / 10 ignored, 246 strict summaries, 235 `Running` targets + 9 doc-test sections |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 |
| `python3 scripts/check_lean_audits.py` | 0; 146/146 |
| `python3 -m unittest discover -s scripts/tests` | 0; 305 |
| `clang --version` | 0; Ubuntu clang 21.1.8 |

The first all-target Clippy attempt exited 101 on a B01 single-element test
loop. Test-only commit `0e91d859` removed the loop and the final Clippy run is
green. The pre-regeneration Python run exited 1 solely on its intended matrix
tripwire (`lean_audits` 145 committed versus 146 on disk); regeneration with
this report's `gate.json` exited 0, and the final 305-test run is green. These
attempts remain under the same private log directory rather than being hidden.

## Integrator reconciliation list

1. Regenerate the acceptance matrix on the merged tree with the merged gate;
   the card adds one Lean audit. Do not carry this card's isolated count across
   sibling B02–B04 audits.
2. Re-measure the combined release corpus. If the combined count differs from
   this card's 220 Safe / 84 provisional Unsafe / 4,530 Unknown functions or
   49,741 unsupported leaves, publish `integration-gate.json` beside this
   report and attribute each delta. The card's own gate remains its record.
3. Update `docs/STATUS.md` and the current increment from combined primary
   evidence only. If no sibling interacts, the exact proposed status values are
   function 220 Safe / 84 provisional Unsafe / 4,530 Unknown, unsupported
   49,741, leak 272, unknown_alias 4,309 and summary_conflict 1; unit counts do
   not move. The proposed increment sentence is: “B01 admitted declared-only,
   unconditionally evaluated resource-free calls inside scalar returns and
   direct opaque getters without custody escape; 12 generic leaves withdrew and
   seven functions moved Unknown to Safe on the pinned corpus.”
4. Amend gateway constraints with the two rules already drafted as 5d/5e:
   resource-free unconditional scalar-call evaluation and direct opaque getter
   evaluation with no escape, retained alias checks and one selection.
5. Retire **none** of `RETURN-POSITION-SUBSHAPE-B`,
   `RETURN-CALL-INCOMPLETE-LINE-455`,
   `RETURN-POSITION-TYPED-MEMBER-REACH` or
   `RETURN-POSITION-CONDITIONAL`. This card takes bounded slices; each named
   discharge condition still has uncovered shapes. Do not add an umbrella
   residual: these existing follow-ups track the remaining scope, including the
   1,444 surviving generic leaves. Do not claim B01 modeled all return
   expressions.
6. Reconciliation dependency: B01 is based on B02 commit `24dce8c5` and uses
   its shared `legacy_condition_call::unconditional_call_span`; do not restore
   the temporary duplicate helper used during review.
