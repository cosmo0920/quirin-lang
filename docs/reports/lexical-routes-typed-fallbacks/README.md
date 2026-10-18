# LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS

Work in progress. Original prediction and brief were committed on exact base
`654e2b7eed72374b02ea39ef41059f7a9149b776` before implementation;
[PREDICTION.md](PREDICTION.md) is unchanged. The card was subsequently rebased
onto the combined W48/RK integration source
`854920e3e12a86c34231d4b46fa42918e29044ca`, preserving its provider contracts,
interceptor metadata and ledger fields. The integrator's fresh combined
three-route run will supply the exact-base observation; it will be labelled
as reused evidence with its actual source/binary identity.

The precedence rule preserves the exact older admitting route. Only a scalar
route's flow refusal consults a separately read Clang plan at the same source
occurrence, under its own declaration and closure premises. The candidate
must have the same scalar destination and refused-write frame, no pointer
store/destination/readback, and every original same-type alias read. All base,
per-hop and alias obligations must discharge on the incoming state before it
can replace the lexical refusal. Otherwise the old result and diagnostics
stand. No route is deleted and no callee's contract or entry custody changes.

`legacy_scalar_precedence.rs` attaches the bounded candidate inside the existing
outer guards. `legacy_transfer.rs` evaluates virtual scalar outcomes using a
cloned incoming state, fresh diagnostics, a nested null-premise frame and a
saved consultation log. The original result wins when it discharges. Otherwise
the typed result wins only with zero diagnostics. Only the chosen outcome and
its named premises are committed; scalar effects occur once and no candidate
can allocate a site or call a function. Other CFG consumers keep the original
instruction's identical effect and violation evidence.

`QCore.ScalarPrecedence` proves composition under each independent reader's own soundness premise, abstract same-effect selection, preservation
of old admissions, complete typed discharge, preservation of original aliases,
failed-probe identity and once-only effects. Faithful Clang extraction and
Rust–Lean correspondence remain explicit, unproved trust-boundary premises.
The Rust controls cover the W42 store, W43 scalar access, W44 condition chain,
alias, null, released/owned resources, pointer stores, unresolved calls,
scalar-fact forgetting, candidate effect mismatches and speculative null-premise
rollback. Fixtures use synthetic contracts, not production library claims.

The first complete standing gate retained a Rust exit 101 (**8,247 passed /
2 failed / 15 ignored**, 514 units / 527 summaries); all other steps passed,
including **339 Lean audits / 374 Python tests**. The two failing tests encoded
three lexical-only refusal assertions: `disable_then_store` after a removal
with no record, a scalar store through a target-free interior alias, and a
chain read through that alias. Each is now independently discharged by the
new rule. The latter two custody decisions retain their named null premise;
none establishes null safety. The three pins were deliberately amended with
source comments, while genuine literal-null, tracked, released, alias, pointer
store and unresolved-effect controls remain. The caller-held-context
certificate control additionally rejects omission of the selected W42 store's
new null premise. All 23 pin/certificate-focused cases pass. The original gate,
latent third assertion's failed focused replay, and exact reasons remain under
[attempts](artifacts/attempts/first-standing-gate/explanation.json) and private logs.

A fresh final standing gate and corpus/system/slice measurements are pending;
no completion claim is made by this draft. Raw logs are worktree-private under
`private/lexical-typed/`.

## Follow-ons and integration

`LEXICAL-SCALAR-NOOP-ALIAS-READ` is a separate observed analyzer-boundary
candidate, not a validated violation or an upstream defect: releasing a
same-type formal peer then storing a scalar through an unbound formal lowers
the store to the older `NoOp`, where this card is never consulted.
[The observation](artifacts/noop-alias-observation.json) records the exact
synthetic source, custody profile, raw failure log and source-state boundary.
Discharge needs exact-base reproduction and an independently reviewed
alias-preserving reading of that older route, with matching Lean and a re-pin.
This card does not silently change established `NoOp` admissions.

The integrator should preserve the rebased RK tracking and runtime projection
metadata, then run the normal failure-gated integration discipline. No provider
sources are changed by this card; provider measurements belong to the combined
base gate, with explicit borrowing provenance if quoted here.
