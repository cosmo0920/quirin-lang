# Bounded extern-call semantics: `source.extern-call.v1`

**Status:** scoped only. No production file changes with this document; no
checker, backend, contract-loader or Lean semantics change on its account. No
verdict, CT mapping, contract-corpus count or raw Cycle 19 unit changes.

This document scopes the increment that makes a **call to a declared extern
function** executable QCore source at one bounded boundary. It replaces
nothing: `docs/QCORE_SPEC_r0_3.md` (line 312) and
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` (§ driver body path) currently
record that arbitrary extern calls are **not** executable, and the
whole-program checker refuses any call whose target is not a source function
(`crates/qcore-frontend/src/whole_program.rs`, "call target is not a source
function"). Until the increment lands, those statements stay true.

Terminology note: `extern` in QCore is a declaration introducer (`extern fn`),
not an attribute; this increment adds **no attribute syntax**. What is already
well-defined is the extern declaration as a *contract carrier*
(`QCORE_CONTRACT_SPEC_r0_3.md` §C04, grammar G05). What is undefined — and
what this card defines — is the *call*.

## 1. Goal

A source function may call a declared extern function when, and only when:

1. the calling file explicitly opts in with a bundled-module import
   (working name `import std.extern;`), following the
   `import std.concurrency;` precedent (core-owned resolution, `Q014`/`Q015`
   refusal before checking, no cross-file leakage);
2. the callee has a **trusted declared contract** admitted by the existing
   resource-contract v1 loader vocabulary (§C04/G05), supplied through an
   explicit `CompileOptions` selection — **not** through inline source
   `contract { … }` blocks, which `Q020`/`Q021` continue to refuse unchanged;
3. the call site satisfies the bridging rules of §3 below.

Everything else keeps its current refusal. A result that depends on an extern
contract carries that contract as a **named declared assumption**: the verdict
is valid *relative to* the declaration and is never a proof of the C
implementation (`ContractTrust::Declared`, per §C06).

## 2. What already exists (reused, not rebuilt)

| Piece | Where | Reuse |
|---|---|---|
| `extern fn` structural parsing | `qcore-frontend::frontend` (`DeclarationKind::ExternFunction`) | unchanged |
| Exact ABI projection + clause grammar | `qcore-contract::contract_loader`, §C04/G05 | the *only* admitted contract vocabulary in v1 |
| Call-graph summaries | `qcore-frontend::whole_program` (`FunctionSummary`, `ParameterOwnership`) | extern contracts are compiled into summaries |
| Effect discipline | `qcore-core::typed_errors` (`Effect`, `EffectRow`) | derived rows, checked against the caller's declared row |
| Import gating | `qcore-core::module_imports` + `Q014`/`Q015` | one new bundled module |
| Typed-error/AllocError machinery | owned-task profile | nullable-owned-return adapter (§3.3) |

## 3. Bridging rules (the substance of the card)

### 3.1 Admitted callee shapes, v1

The callee's declared contract must already load under C04 (total-or-reject).
Of the admitted C04 forms, v1 makes **callable** only:

- **Parameters:** scalars passed by value (data only, per C04's "a scalar may
  carry a descriptor number as data, but cannot carry descriptor custody");
  `resource_borrowed(p)` on an admitted pointer bound to a borrowed model
  Buffer; `resource_consumed(p, nonnull)` bound to a consumed `own<Buffer>`.
- **Results:** `resource_return_none()` with a `unit` or scalar result (data
  only); `resource_return_owned(nullable)` with an admitted pointer result,
  which **must** flow into the existing checked null-branch/`AllocError`
  discipline (§3.3).

Deliberately *not callable* in v1, each a named follow-on:
`resource_out`, `resource_stored`, `resource_removed`,
`resource_return_reallocated`, `resource_released` (release through an extern
call is exactly the cleanup-relation territory §C10 reserves), and
`resource_return_owned(nonnull)` (a trusted non-nullness assertion with no
checked branch; excluded to keep every acquisition arm checked).

### 3.2 Type bridging

One explicit, closed table maps source model types to C04 ABI types; anything
outside it refuses. v1 table (working):

| Source side | C04 ABI side | Mode |
|---|---|---|
| `i32`/`u32`/… literals and scalar locals | matching scalar | data |
| `borrow<Buffer>` | `ptr<const, u8>` or `ptr<mut, u8>` | `resource_borrowed` |
| `own<Buffer>` (moved at the call) | `ptr<mut, u8>` | `resource_consumed` |
| fresh `own<Buffer>` (checked null branch) | `ptr<mut, u8>` result | `resource_return_owned(nullable)` |

Length/extent is **not** transported in v1 (C04 pointers are extentless `u8`
projections). The declaration therefore cannot promise anything about how many
bytes the callee touches; this is recorded as a premise of the trusted
contract, not checked, and buffer-extent transport is a named follow-on.

### 3.3 Nullable-owned-return adapter

Per spec line 255, a C sentinel returns *normally* unless an explicit adapter
maps it. v1 ships exactly one adapter: a null result from a
`resource_return_owned(nullable)` callee **must** be branched on at the call
site and the null arm must produce the existing typed allocation error
(`AllocError` route). Omitting the branch refuses; the certificate records the
adapter as part of the call's custody transfer. Negative-integer and errno
sentinels are **not** adapted in v1 (follow-on; interacts with O03's
post-state work).

### 3.4 Checker semantics

`whole_program` call resolution becomes a two-way dispatch: a callee is a
checked source function (unchanged path) **or** an imported extern declaration
with an admitted contract, whose `FunctionSummary` is derived deterministically
from the declared modes: `resource_borrowed → Borrowed`,
`resource_consumed → Consumed`, owned return → a fresh acquisition in the
caller's ledger. The derived `EffectRow` is `{Io}` always, plus `Alloc` iff
the contract has an owned return, plus `Mut` iff any `mut` pointer parameter;
the caller's declared row must admit it. No unsafe-block requirement: the
trust boundary is the declared contract, consistent with C04's vocabulary, and
the effect row keeps the opt-in visible interprocedurally.

Extern calls are admitted in **sequential function bodies only**. Task
workers, `with lock` bodies, error-region bodies and the exec12 driver path
keep their existing refusals (`Q600`/existing messages) — lifting any of those
is a separate card because each interacts with a separate checked model.

### 3.5 Reference execution and emission

- **Reference semantics** cannot run C. Extern calls execute as model
  operations with **literal outcome controls** declared by the fixture,
  exactly the `qcore_acquire`/`qcore_check` philosophy ("model operations, not
  general malloc calls"). Both outcome arms of a nullable return are
  exercisable.
- **C emission** prints the prototype from the ABI table (stdint types) and a
  direct call. Symbol resolution/linkage is the builder's premise: the
  increment claims nothing about which object file provides the symbol.
- **Native evidence:** fixed ASan/UBSan/LSan runs linking a real C
  implementation of each fixture callee, covering the success arm, the null
  arm (injected), a consumed-parameter release inside the callee, and
  leak-freedom of every generation the harness tracks.

## 4. Diagnostics

New codes in the free `Q022`–`Q027` band (verified free in both spec §L21 and
the workspace), owned by a new semantic pass entry; §L21 is updated in the
same change. Candidate split: missing import; callee has no admitted contract;
bridging-table refusal (type/arity/mode); adapter omission (unchecked null);
extern call in a refused context; declaration/contract mismatch. **No existing
code changes**; in particular `Q020`/`Q021` keep their exact current behavior
— this card is deliberately *not* the inline source-contract consumer those
codes are waiting for (named follow-on, and the reason they must stay
separate remains recorded in `remaining-d01`).

## 5. Lean obligations

Per the current closing standard (Rust + audited Lean; Python auxiliary):

- a model of the extern call step as an opaque effectful transition whose
  custody delta is exactly the declared summary;
- conservation: applying a derived summary preserves ledger accounting given
  the contract premise (both parameter modes, both return modes);
- adapter totality: every nullable-owned-return path either binds a live
  acquisition or takes the typed error arm — no third exit;
- refusal completeness for every non-admitted C04 mode in §3.1;
- summary derivation is a function (same contract, same summary), so
  certificate rederivation can repeat it independently;
- no new axioms; a new `Audit*.lean` registered with
  `scripts/check_lean_audits.py`.

Source extraction and the C implementation's conformance to its declared
contract remain explicit premises, stated in the theorems.

## 6. Tests (per contract-spec §C-conformance)

Positive: scalar data call; borrowed-Buffer call; consumed `own<Buffer>`;
owned nullable return with both arms; certificate rederivation.
Negative/mutation: each §4 refusal; call without import in a file that parses;
import without contract selection; contract selected but callee name absent;
duplicate extern names across selections (existing loader refusal, re-pinned
at this boundary); consumed-then-used; leaked owned return; adapter bypass
via assignment through an alias; a mutated contract file refusing replay of a
previously issued certificate. Contract fixtures live under test fixtures,
**not** `contract/qcore/`, so the 60-module corpus pin is untouched.

## 7. Out of scope (named follow-ons, not omissions)

Inline source-contract enforcement (the `Q020`/`Q021` consumer); `resource_out`
/ `stored` / `removed` / `reallocated` / `released` call modes; nonnull owned
returns; sentinel/errno adapters (O03 interaction); buffer extents; variadics,
function pointers, callbacks, structs, floats; extern calls in concurrent or
error-region contexts; cost summaries; linkage verification; any gateway,
audit-route or Cycle 19 interaction (the unrestricted-C gateway analyzes C
callers and is untouched); CT-manifest changes (no unmapped case is directly
blocked on extern calls — CT45's unknown-destructor/access-contract scenario
is adjacent and may become mappable once extern declarations are callable, but
any mapping proposal is its own stage-derived record, not this card's).

## 8. Acceptance gate

The standing per-change gate, plus: all four pinned 185-unit artifacts
untouched by construction (no gateway/audit code in the diff — assert via the
diff, not re-measurement; CORPUS-REPIN stays open and stays someone else's
card); contract corpus count unchanged; no raw unit promoted; spec §L21 and
`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` updated in the same change so line 312's
"not executable" statement moves to the bounded statement of §1 rather than
silently going stale.

## 9. Suggested staging

One card is feasible; if split, the seam is: **(a)** import gate + contract
selection + checker admission + reference-execution model + Lean
(refusal-complete, nothing emitted yet — emission keeps refusing extern calls
with the §4 context code); **(b)** C emission + native sanitizer evidence +
grammar/spec text updates. Stage (a) alone changes no observable execution
and is independently gateable.
