# B03-LOOP-AND-SWITCH implementation handoff

Date: 2026-09-20. Base: `8e633261fd216a76d4d25390b0dadeda891c59aa`.
Branch: `codex/b03-loop-switch`.

This report hands the implementation to the integrator. It does **not** claim
the card closed: the corrected full standing gate and acceptance-matrix
regeneration are pending on the integration tree, as recorded in §7.

## 1. Result

The legacy C gateway now models exactly one unconditionally evaluated,
contracted call in a `while`, `for`, `do`/`while`, or `switch` controlling
expression. The lexical boundary rejects short-circuit (`&&`, `||`), ternary,
assignment, increment/decrement, type-operator, nested-call and multi-call
shapes. The call span is replaced with a literal only while checking that the
remaining expression is custody-neutral; the replacement is never interpreted
as the condition's value. The isolated call is admitted only when the existing
statement classifier returns `LegacyInstruction::ContractCall`, so contract
lookup, withdrawals, argument checks, alias obligations and effects still have
one implementation.

The call instruction occupies the existing condition node:

* `while` and `for` execute it before every body/exit decision; a `for`
  `continue` still visits the increment node before returning to the call;
* `do`/`while` executes it after each normally completed body, and `continue`
  targets it; an abrupt `break` bypasses it;
* `switch` executes it once before dispatch, and no case or fallthrough edge
  returns to it.

Every loop retains its existing body and exit successors and its
`loop_exit_edge`. The literal-zero `do`/`while` rule and the `for (;;)` rule are
unchanged. A declined probe restores `next_site`, so refused expressions keep
their site numbering and original position diagnostic.

## 2. Prediction published before AFTER

Before either AFTER audit began, the agent sent the integrator this quantified
prediction in the task mailbox (the chat record is the chronology authority;
this report does not backdate a file): **0 function-status moves, 0 exchanges,
and 0 tracker retirements**. The grounds were the four sole-blocker functions:
two atomic compare/exchange loops, one indirect ternary switch, and one
uncontracted `rd_socket_read` loop. None satisfies this card's contracted-call
and neutral-residual premises.

The expected movement was therefore only possible leaf withdrawal in
multi-blocker functions. The AFTER measurement found even that set empty.

## 3. Rust evidence

`crates/qcore-c-gateway/src/legacy_condition_call.rs` owns the shared lexical
boundary. `Parser::condition_call_instruction` masks the candidate span for the
neutral-residual check, classifies the original span through `classify_simple`,
and accepts only a modeled contract call. The four `Statement` variants carry
that instruction into their existing CFG condition nodes.

Six Clang-backed integration tests in
`crates/qcore-c-gateway/tests/loop_switch_condition_calls.rs` cover:

* repeated release on the back edge of all three loop forms;
* one switch evaluation;
* retained loop exits exposing a leak;
* short-circuit and post-release member-access refusal;
* `continue` routing for all loop kinds, including `for` increment before the
  condition call;
* `do`/`while` `break` bypassing the condition and leaving the acquisition
  live.

The targeted restored-source run passed 6/6; committed log:
`mutations-restored.log`.

## 4. Audited Lean

`QCore.LoopSwitchCallEvaluation` models loop kind and exit kind separately.
Its evaluation equations distinguish condition-false termination from an
abrupt body exit: pre-test loops have `entries + 1` versus `entries`
evaluations; a normally terminating post-test loop has `entries`, while an
abrupt final body exit has `entries - 1`. Separate theorems retain the body and
exit successors, prove the `continue` routes (including the `for` increment),
and give switch dispatch one effect.

`AuditLoopSwitchCallEvaluation.lean` registers every public theorem. The module
build and direct audit passed with only the repository's allowed `propext` and
`Quot.sound` axioms. As constraint 28 requires, these are abstract rule
theorems; Rust correspondence and faithful C extraction remain review premises.

## 5. Mutation evidence

The shipped harness rewrites source bytes on restore and was followed by a
green restored-source run. It caught **4/4** mutations:

1. admitting a short-circuit call;
2. erasing the `while` call effect;
3. erasing the `for` call effect;
4. erasing the `switch` call effect.

Artifacts: `scripts/mutations.py`, `mutations.log`, `mutations.exit`,
`mutations-restored.log`, and `mutations-restored.exit`.

## 6. Corpus BEFORE / AFTER

The integrator's shared BEFORE audit at base `8e633261` exited 0 and reproduced
the three verdict-bearing artifacts byte for byte. Its private record is
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline/`.

The corrected B03 AFTER audit used `/usr/bin/clang`, the pinned source and
Monkey compile database, and
`build/cycle19/provider-packages-wave20/qcore.chunkio`. It exited 0. Private
record: `/tmp/agent-b03/private/b03-after2/`.

The result matches the prediction exactly:

| Measure | BEFORE | AFTER | Movement |
|---|---:|---:|---:|
| Units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | same | 0 |
| Functions | 213 Safe / 84 provisional Unsafe / 4,537 Unknown | same | 0 |
| Diagnostics | leak 272; unknown_alias 4,309; unsupported 49,753; summary_conflict 1 | same | 0 |
| B03 position leaves | 1,372 | 1,372 | 0 |
| Function-status moves | — | — | **0** |
| Exchanges | — | — | **0** |

`diagnostics`, `chunk-contracts`, and `source` are byte-identical between the
shared BEFORE and B03 AFTER. `contracts` differs only in raw worktree-root keys;
the normalized diff has 33 before, 33 after, 0 added/removed/changed.
`source-effects` has the same 96 results and three identity-only changes to
build/configuration/task/obligation digests for the changed analyzer; no result
was added or removed and no semantic result field changed. The exhaustive
diff against the archived baseline is
`/tmp/agent-b03/private/b03-after2/artifact-diff.json`.

Two non-evidence runs are retained and excluded. The first release build was
interrupted while moving this card's Cargo target out of `/tmp`. The first
AFTER omitted `QCORE_CHUNKIO_PACKAGE`, so it produced only four artifacts; it
exited 0 but is not the comparable five-artifact measurement. They remain in
`private/b03-after/`.

## 7. Validation boundary

Targeted Rust tests, Lean module/audit, mutations, and the corrected corpus
audit are green. One sandboxed `cargo test --workspace --no-fail-fast` run
encountered LeakSanitizer ptrace failures and was interrupted; it is an invalid
platform observation, not a product failure. A corrected unsandboxed run was
then stopped at the integrator's request to free the agent slot. The integrator
owns the complete unsandboxed standing gate, matrix regeneration, and final
card-close evidence on the merged tree. Private incomplete log:
`/tmp/agent-b03/private/b03-gate-20260920T0047Z/`.

## 8. Integrator reconciliation list

1. **No status or headline-number edit.** The corpus and all four status counts
   are unchanged. Do not describe this zero-reach implementation as coverage or
   conformance growth.
2. **No retirement.** The prediction named zero entries, and the measurement
   meets no registered follow-up's discharge condition.
3. **Proposed gateway rule (new constraint 5d, exact text):**

   > A contracted call in a C controlling expression is modeled only when one
   > ordinary call is unconditionally evaluated, the residual expression is
   > custody-neutral, and the ordinary statement classifier accepts the
   > isolated call. Short-circuit, conditional, assignment, increment,
   > type-operator, nested-call and multi-call shapes stay refused. The call is
   > the loop condition node, so every back edge and `continue` route revisits
   > it (`for` after its increment); `do`/`while` `break` bypasses it; `switch`
   > evaluates it once. Modeling the instruction never removes a loop exit or
   > back edge. A declined probe restores site numbering.

4. Add the corresponding router row for 5d under gateway rules if the paragraph
   is accepted.
5. Regenerate the acceptance matrix from the integrator's final gate rather
   than editing derived sections by hand. Record this report's reconciliation
   items individually as applied/declined/deferred with verification claims.

## 9. Limits

The route deliberately does not model uncontracted calls, indirect calls,
multiple calls, conditionally evaluated calls, nested calls, or a residual
whose typed custody reads are not neutral. That boundary explains the measured
zero corpus reach. Native execution is not proved, and no provisional Unsafe
row in this report is a validated upstream defect.

## 10. Parent validation and corrections — 2026-09-20

The integrator completed the previously pending isolated standing gate at
`0e8c5c08`: **4,820 Rust passed, 0 failed, 10 ignored; 247 result summaries over
245 targets; 146 Lean audits**. Submodule initialization, fmt, Clippy and both
reevaluation probes exited 0. Clang 21.1.8 was invoked in this measuring
worktree. The acceptance matrix was regenerated, then the Python suite passed
305/305, exit 0. Provider122 remains carried evidence. Commands, durations,
source commit and absolute private log paths are in `gate-facts.json`; the
private run is `/tmp/agent-b03/private/b-series/parent-b03-gate/`.
The combined B-series gate will supersede these isolated counts.

Two wording corrections to the handoff are necessary. In §6, **two** artifacts
are verdict-bearing (`diagnostics` and `chunk-contracts`); `source` is separately
byte-identical. The three source-effect identity differences are checkout and
configuration identities, not measured semantic changes to the analyzer.
In the mutation log, the harness's generic `<compile error>` label also matched
Cargo's final `error: test failed` line. All four measured runs include named
failed test assertions; none is counted solely from a compilation failure.

The proposed global rule in §8 is declined as worded: older neutral-expression
rules can already admit shapes outside that sentence. The integration lands an
explicitly scoped B01–B03 additional-lowering rule instead and records this
choice in its reconciliation ledger. No existing neutral admission is revoked.
