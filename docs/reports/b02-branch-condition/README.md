# B02-BRANCH-CONDITION — one modeled call before an `if` decision

Base `8e633261`.  This card implements the branch-only member of the B series.
It moves one projected function from Unknown to Safe, removes 45 refusal leaves
at 26 functions in eight units, adds no leaf, exchanges no refusal, and moves no
unit verdict.  The result is still relative to the selected `.qcore` contracts;
it is not full-language conformance and it promotes no raw corpus unit.

## Rule and boundary

An `if` condition may contain one ordinary contracted call when all of these
conditions hold:

* C evaluates the call unconditionally on every visit to the condition;
* there is exactly one ordinary call and it is not under `sizeof`, `_Alignof`
  or another recognized type operator;
* replacing the whole call span by an opaque scalar leaves an expression the
  existing `ExpressionRole::Decides` rule already admits; and
* classifying the isolated call with the ordinary statement classifier yields
  `LegacyInstruction::ContractCall`.

The CFG executes that existing `ContractCall` node once, then reaches a neutral
branch node with both successors.  The opaque replacement is a classifier
device only: it is never a computed value and never chooses an arm.  Contract
effects therefore have one implementation, and both arms see the post-call
state.  A declined speculative classification restores `next_site`.

The lexical boundary rejects `&&`, `||`, `?:`, assignment,
increment/decrement, multiple ordinary calls, type-operator contexts and every
residual the existing expression rule refuses.  It also refuses an
uncontracted call, a discarded custody-bearing result, or any other isolated
instruction.  Loops and `switch` are B03; generalized nested-call arguments are
B04; destination-side indirect access remains W42-INDIRECT-ACCESS.

The deliberate old-pin update is
`neutral_expression_positions::a_releasing_call_in_a_branch_condition_is_never_admitted`:
it is now `an_unconditional_releasing_call_in_a_branch_condition_is_modeled`.
Before B02, `(q_free(p), 1)` had to refuse because the only alternative was to
erase the release into a `NoOp`.  B02 gives it a real call-transfer node, so the
fixture is Safe with no unsupported leaf and no leak.  The short-circuit and
conditional neighbours stay refused and are tested beside it.

## Prediction and measured correction

[`PREDICTION.md`](PREDICTION.md) was committed before the changed corpus audit.
It predicted exactly one Unknown-to-Safe function, zero exchanges and no
retirement.  Those counts reproduced.  Its named candidate was wrong:
`cio_file.c::cio_file_is_up` contains
`cio_file_native_is_open(cf) && cio_file_native_is_mapped(cf)`, so B02's
short-circuit boundary intentionally retains it.  The actual mover is
`flb_input_chunk.c::flb_input_chunk_down`, whose only baseline leaf was
`if (cio_chunk_is_up(ic->chunk) == CIO_TRUE)`; `cio_chunk_is_up` has a selected
contract, its borrowed member actual is already admitted, and the call is
unconditional.  This is a prediction identity miss, recorded rather than
rewritten after measurement.

| Measure | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| Units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | identical | 0 |
| Functions | 213 Safe / 84 provisional Unsafe / 4,537 Unknown | 214 / 84 / 4,536 | +1 Safe, −1 Unknown |
| `leak` | 272 | 272 | 0 |
| `unknown_alias` | 4,309 | 4,309 | 0 |
| `unsupported` | 49,753 | 49,708 | −45 |
| `summary_conflict` | 1 | 1 | 0 |

Coordinate-multiset attribution found 45 removed branch-condition leaves and
zero added leaves: no same-position exchange.  Twenty-six functions in eight
units lose at least one leaf; 24 remain Unknown, one remains provisional
Unsafe, and `flb_input_chunk_down` becomes Safe.  The chunk-contract artifact
is byte-identical (`c4f438e4…` both sides).  Full hashes and counts are in
[`artifacts/delta.json`](artifacts/delta.json).

BEFORE is the integrator's fresh exit-0 audit at base `8e633261`, private path
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline`; its
verdict-bearing diagnostics and chunk-contracts reproduce
W37-CORPUS-CONTRACTS-4 byte for byte.  AFTER is the exit-0 audit at
`private/b02-after`, using the same pinned Fluent Bit source, Monkey compile
database and source-policy artifact.  The BEFORE binary hash is `4d1d89eb…`;
the AFTER binary hash is `1c4600d2…`.  Raw contracts differ because their keys
carry the checkout root; they are not verdict-bearing and no contract changed.

## Rust tests, Lean and mutations

`branch_condition_calls.rs` exercises the admitted release and neutral scalar
call, both unknown-result successors, and refusals for short circuit,
conditional evaluation, two calls, type operators, an indirect residual, a
read through a released base, an owned result and an uncontracted callee.  The
existing W4 position suite carries the deliberate pin above.

`QCore.BranchConditionCall` proves the abstract admission conjunction, each
refusing frequency class, the non-vacuous admitted case, and that both unknown
Boolean-result arms receive the post-transfer state.  The audit is
placeholder-free and uses only the permitted `propext` axiom closure.  Faithful
C extraction and Lean-model-to-Rust correspondence remain explicit review
premises under constraint 28.

The shipped mutation harness and log are under `scripts/` and `logs/`.  Six of
six mutations are caught by their named tests: removing the call transfer,
admitting short circuit, admitting a call under a type operator, skipping the
residual check, accepting a non-`ContractCall`, and deleting the else successor.
The baseline and restored control are green; every restore is SHA-256 equal and
is performed by rewriting, so Cargo cannot reuse a mutant fingerprint.

## Validation

The final run is bound by [`gate.json`](gate.json) and private logs at
`private/b02-gate-final`: submodule 0; Clang 21.1.8 recorded in this worktree;
workspace tests 4,825 passed / 0 failed / 10 ignored over 247 summaries and 245
targets; fmt 0; Clippy 0; both probes 0; 146/146 Lean audits; matrix regeneration
0; Python 305/305, exit 0.  Provider tests 122 are carried, not remeasured,
because providers and `qcore-schemas` are unchanged.

The first sandboxed attempt is preserved at `private/b02-gate-run` and supplies
no evidence: LeakSanitizer refused to run under ptrace, then the run was
interrupted before Cargo wrote its aggregate exit.  It also exposed the stale
pre-B02 pin described above.  No count is carried from that attempt.

## Retained limitation and integrator reconciliation list

No registered entry retires.  The largest measured boundary remains the 196
functions carrying the indirect+branch pair: B02 does not implement
W42-INDIRECT-ACCESS.  The conservative rejection of all short-circuit
conditions also retains some calls that are unconditionally evaluated as the
left operand; admitting those requires a CFG for expression sequencing and is
not claimed here.

For the integrator, after earlier B cards are merged and the chained arbiter
re-measures the combined tree:

1. Keep the deliberate `neutral_expression_positions` pin update; it is the
   previous behavior B02 intentionally changes.
2. Regenerate the acceptance matrix so `AuditBranchConditionCall.lean` and the
   combined gate are recorded.  This branch already regenerates it for its
   standalone 146-audit gate; a later combined gate supersedes those measured
   values.
3. If the combined measurement preserves this card's isolated delta, update
   `docs/STATUS.md` to: **2 Safe / 42 provisional Unsafe / 137 Unknown / 4
   data-only units; 214 Safe / 84 provisional Unsafe / 4,536 Unknown projected
   functions; leak 272, unknown_alias 4,309, unsupported 49,708,
   summary_conflict 1.**  If an earlier merged B card changes the base, use the
   arbiter's values instead and attribute the interaction.
4. Add the single newest-first increment paragraph: **“B02-BRANCH-CONDITION
   evaluates one unconditional contracted call through the existing transfer
   before an `if` split.  At base `8e633261` it removed 45 branch-condition
   leaves with no additions or exchanges and moved
   `flb_input_chunk_down` Unknown→Safe; units did not move.  Short-circuit,
   multiple-call, type-operator and unsafe-residual shapes remain refused;
   W42-INDIRECT-ACCESS remains the quantified 196-function pair.”**
5. Record the prediction identity miss in reconciliation; do not rewrite
   `PREDICTION.md`.  There are no retirement or diagnostic-namespace changes.
