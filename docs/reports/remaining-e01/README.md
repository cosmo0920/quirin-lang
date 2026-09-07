# E01 — Source-derived obligation completeness (ChunkIO Boolean-field capability)

2026-09-07. Baseline `6c2782f`. Workstream W3, `verify_or_extend`.

This unit audits one admitted capability's **obligation inventory**: whether the
core derives an obligation, or a residual `Unknown` row, for every construct it
meets in the selected source. It is not a claim that any audited source is
`Safe`, and it promotes nothing. Machine-readable results are in
[`inventory.json`](inventory.json).

## Capability under audit

| Field | Value |
|---|---|
| Capability | ChunkIO Boolean-field source contract |
| `VerifierCapability` | `ResourceContracts` |
| Obligation property | `BooleanFieldContract` |
| Semantic profile | `qcore.c_boolean_field_contract.v2` |
| Source importer | `crates/qcore-c-gateway/src/boolean_source.rs` |
| Task derivation | `crates/qcore-verification/src/source_effect.rs` |
| Witness checker | `BooleanFieldSourceTask` (`impl WitnessChecker`) |
| Production route | `crates/qcore-audit/src/{source_audit,source_contract}.rs` |

The inventory rule, read out of the code: `import_chunk_source_with_profile`
iterates `legacy_c_source_functions(unit.source)` and returns, per function,
either a sealed `ChunkSourceSummary` or `ChunkSourceResult::Unknown { function,
reason }`. `BooleanFieldSourceTask::derive` turns each summary into **exactly
one** `BooleanFieldContract` obligation, subject `Function { symbol,
translation_unit }`, scope `Functions { symbols: {symbol} }`. There is no other
obligation shape in this capability.

New evidence:
`crates/qcore-verification/tests/source_effect_obligations.rs` (6 tests). Each
fixture carries a hand-enumerated inventory written in the test table, and the
test asserts the derived inventory equals it, that every enumerated raw
definition is accounted for, and that each checked function carries exactly the
obligation shape above.

## Acceptance 1 — nested calls are visited, and unsupported forms stay Unknown

`Importer::statement` and `Importer::expression` are total matches over the
Clang typed AST whose final arm is `Err(format!("unmodeled …"))`. There is no
skip arm, so a construct outside the grammar cannot be dropped from the
imported program; it aborts the function's import, and the function reappears
as a residual `Unknown` row with a reason.

Measured, against a positive control that differs only in the call's position
(`inventory.json`, `fixtures`):

| Position | Fixture body | Result | Reason |
|---|---|---|---|
| supported control | `… cio_chunk_reset(p); p->lock = 0; …` | contract | — |
| return | `return probe(p);` | Unknown | `unmodeled expression CallExpr` |
| argument of an admitted helper call | `cio_chunk_reset(pick(p));` | Unknown | `helper requires direct borrowed parameter` |
| if condition | `if (probe(p)) { … }` | Unknown | `unmodeled expression CallExpr` |
| declaration initializer | `int v = probe(p);` | Unknown | `unmodeled statement DeclStmt` |
| binary operand | `return p->lock == probe(p);` | Unknown | `unmodeled expression CallExpr` |
| unary operand | `return !probe(p);` | Unknown | `unmodeled expression CallExpr` |
| assignment right-hand side | `p->lock = probe(p);` | Unknown | `unmodeled expression CallExpr` |
| discarded call statement | `probe(p);` | Unknown | `helper signature or parameter binding mismatch` |

Verdict: **complete for the audited positions.** No fixture produced a summary
whose imported program had erased the call, and no function disappeared from the
inventory.

One honest qualification, recorded as boundary **E01-B3**: a local declaration
statement is itself outside the grammar (the `initializer_control` fixture with
`int v = 1;` and no call is equally `Unknown`), so an initializer's nested call
is refused by the enclosing declaration rather than by call analysis. The
outcome is fail-closed either way; the residual reason names the declaration.

The frame-helper importer is checked the same way: helper bodies containing a
call, or a local declaration, refuse to import, so an unchecked effect cannot
enter a caller's inventory through an admitted helper.

## Acceptance 2 — indirect calls, unreachable code, scope exclusions, provider coverage

### Indirect calls and callbacks

`Importer::statement`'s only call arm requires the callee to be an
`ImplicitCastExpr` with `castKind == "FunctionToPointerDecay"` over a
`DeclRefExpr` whose `referencedDecl.kind == "FunctionDecl"`, whose type is
exactly `void (struct <record> *)`, whose single argument is the borrowed
parameter itself, and whose name resolves in the caller-supplied,
source-validated helper map (`call lacks source-validated frame helper`
otherwise). Consequences, tested:

- call through a file-scope function pointer — the callee is an `LValueToRValue`
  load, not a decay — `Unknown`;
- call through a record callback field — the callee is a `MemberExpr` with no
  `referencedDecl` name — `Unknown`.

No indirect target is resolved and none is assumed effect-free.

### Unreachable code

Unreachable statements are **imported, not erased**. For `return p->lock;
p->lock = 0;` the checked program is
`Block[Return(Field), Assign(Integer(0))]`, and neither Boolean prestate
executes the trailing write (`Statement::execute` returns on the first
`Return`). Dead code that is *not* modeled still refuses the whole function
(`return p->lock; probe(p);` is `Unknown`), so unreachability is never a reason
to admit an unclassified construct.

### Scope exclusions

The function denominator is `legacy_c_source_functions(unit.source)`, a token
scan of the **raw, unpreprocessed main source**.

- Header definitions are outside the audited scope by construction (boundary
  **E01-B2**). This is a scope statement carried by the certificate, not a
  silent drop within the declared scope.
- **Boundary E01-B1, found by this audit and not repaired here:** a definition
  that exists only after macro expansion, and a K&R-style definition, are absent
  from the enumeration, hence absent from the derived inventory entirely — no
  obligation *and no residual `Unknown` row*. The audit's own backstop in
  `crates/qcore-audit/src/source_audit.rs` (`for function in raw_functions`)
  cannot catch this, because `raw_functions` is the same scan
  (`crates/qcore-audit/src/lib.rs`). The reported "N checked / M Unknown" split
  is therefore not a complete function denominator for such a unit.

  What this does *not* do: promote anything. The capability only ever adds
  checked function contracts; an unenumerated function is never reported as
  checked. The same enumeration also filters the resource route's
  `MainSourceDefinitions` selection, and there the exclusion is fail-closed for
  callers — a call into an excluded definition has no contract and no validated
  summary, so the caller stays `Unknown` (asserted in the test). Repairing the
  enumeration changes the pinned 185-unit artifacts and the resource selection,
  so it is filed as follow-on unit **E01-F1** with its own parity plan rather
  than changed under an audit card.

### A provider cannot remove an obligation from the requested set

The gates, read out of `crates/qcore-verifier-host/src/lib.rs`:

- `verify_v2` rejects (`QP001`) any request whose `obligations` are not exactly
  `task.obligations_for(capability)`; the request is built by the core from the
  core-derived task, and obligation identifiers are computed by the core
  (`VerificationTask::obligation_id`) and never accepted from a provider.
- `check_evidence_v2` takes `required = task.coverage_for(capability)` as the
  denominator, runs the core's own `WitnessChecker`, and rejects (`QP009`) when
  `discharged != required` **or** `evidence.coverage != required`.
- `compose_verification_with_inventory` rejects (`QP011`) a composed `Safe`
  whose certificate coverage is not the required set.
- Any `DerivationIncomplete` marker forces `QP016` regardless of the response
  (`verify_v2`, and `enforce_derivation_completeness` in `verify_dispatch.rs`).

Tested here with an untrusted in-process provider that proposes `Safe` with the
**honest** witness and only a manipulated coverage set, so the rejection is
attributable to coverage alone:

| Proposed coverage | Outcome |
|---|---|
| empty (obligation omitted) | `Unknown`, `QP009`, no certificate |
| one forged identifier substituted | `Unknown`, `QP009`, no certificate |
| required set plus an invented identifier | `Unknown`, `QP009`, no certificate |
| exactly the required set, honest witness | `Safe` (core re-evaluates the contract itself) |
| provider answers `Unknown` | `Unknown`; composition stays `Unknown` |

In every rejecting case the obligation is still in
`task.coverage_for(ResourceContracts)` afterwards, and
`compose_verification_with_inventory` over the core's required set returns
`Unknown`. The obligation is never dropped.

Verdict: **complete.** Coverage is a core-computed denominator throughout;
a provider's account of what it covered is compared to it, never substituted
for it.

## Acceptance 3 — deletion mutations

Two fixtures, both starting from the supported control:

1. **Delete the frame-helper call** (`cio_chunk_reset(p);`). The function is
   still checked, but the derived obligation changes with the source: the
   obligation identifier moves from `resource_contracts:8b4f668cba00acc2b108c1c8`
   to `resource_contracts:0530096208a34bc98b13c1d3`, `dependencies` becomes
   empty, and the checked path's `frame_writes` change. The task digests differ,
   and a witness built before the deletion is rejected against the post-deletion
   task.
2. **Delete every modeled field access.** The function leaves the checked set
   and reappears as a residual `Unknown` row with reason `function has no
   modeled chunk field access` — it does not vanish.

Derivation is therefore source-sensitive for this capability, and a stale
inventory cannot be reused: obligation identity binds the task identity, which
binds `raw_source_digest` of the whole translation unit plus the sealed
build/profile/policy snapshot (`BooleanFieldSourceTask::derive`). This check is
deliberately capability-scoped; general identity invalidation is card G01.

## Acceptance 4 — follow-on scope

The admitted set is exactly the three `impl WitnessChecker` sites. The other
admitted capabilities needing the same audit, as named units (no work done here):

- **E01-F2** — CMetrics unit-result Boolean-field capability
  (`qcore.c_boolean_field_contract.v3`). Same importer grammar, distinct result
  ABI, two prestates instead of four, unit-result contract evaluation.
- **E01-F3** — chunk lifecycle (`qcore.chunk_lifecycle.v1`,
  `ChunkLifecycleReplay`). Derives over a QCore `SourceProgram`; its
  `statement_calls` scanner already refuses nested calls and structured control
  flow into `DerivationIncomplete` markers (existing coverage:
  `crates/qcore-verification/tests/unit/obligation_derivation.rs::source_import_never_hides_unknown_effects_or_control_flow`,
  which includes `wrapper(cio_ctx_destroy(ctx));` and
  `return cio_ctx_destroy(ctx);`), and it carries an unconditional
  incompleteness marker, so it cannot reach `Safe` today. The unit is to audit
  its inventory the way this card audits ChunkIO's.
- **E01-F4** — observation hooks (`qcore.observation_hooks.v1`,
  `ContractObservationReplay`). Its denominator is the provider manifest's
  required-hook set matched against declared `extern fn` contracts, not a source
  scan, so the completeness question is different in kind.
- **E01-F1** — the enumeration boundary above, which is shared by every
  source-scanned capability and by the resource projection.

## What this unit does not establish

It does not establish that any ChunkIO or Fluent Bit source is safe; it does not
promote any raw unit; it adds no support for general C. It audits inventory
completeness for one capability, on small fixtures, under the capability's
existing trusted premises (Clang typed import, declared profile, declared
policy, live/exclusive/Boolean-domain caller conditions). Boundaries E01-B1 to
E01-B4 remain open and are pinned by tests so a later change cannot claim
completeness without addressing them.

## Validation

Standing per-change gate over `6c2782f` plus this change, all passing:

| Gate | Result |
|---|---|
| `cargo test --workspace` | 825 passed, 0 failed (819 baseline + 6 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cargo test --manifest-path providers/Cargo.toml` | 45 passed, 0 failed |
| providers `cargo fmt` / `cargo clippy -D warnings` | clean |
| `python3 -B -m unittest discover -s scripts/tests -p "test_*.py" -t scripts/tests` | 43 passed |
| `cd proof && lake build` | 62 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | 20 Lean audits passed |

Configured CMetrics Clang audit, rebuilt in this fresh worktree
(`providers/package.py build` for both package sets, then
`prepare_cmetrics_clang_audit.py`, then `qcore-audit`):

```
source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

unchanged from the pinned expectation. The pinned Fluent Bit checkout used for
that run is an independent detached clone at the profile pin
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; the developer's sibling checkout was
at a different revision, so `--fluent-bit` selected the pinned clone.

The full 185-unit Cycle 19 audit was **not** re-run here: its persistent Monkey
build inputs are not present in this worktree. This change adds no production
code — `git status` shows exactly one new integration test file and this report
directory — so the four 185-unit artifacts cannot differ.
