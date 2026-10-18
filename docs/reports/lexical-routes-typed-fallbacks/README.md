# LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS

Work in progress. Original prediction and brief were committed on exact base
`654e2b7eed72374b02ea39ef41059f7a9149b776` before implementation;
[PREDICTION.md](PREDICTION.md) is unchanged.

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

`QCore.ScalarPrecedence` proves the abstract same-effect selection, preservation
of old admissions, complete typed discharge, preservation of original aliases,
failed-probe identity and once-only effects. Faithful Clang extraction and
Rust–Lean correspondence remain explicit, unproved trust-boundary premises.
The Rust controls cover the W42 store, W43 scalar access, W44 condition chain,
alias, null, released/owned resources, pointer stores, unresolved calls,
scalar-fact forgetting, candidate effect mismatches and speculative null-premise
rollback. Fixtures use synthetic contracts, not production library claims.

Validation and corpus/system/slice measurements are pending; no count or
completion claim is made by this draft. Raw attempts and focused results are
worktree-private under `private/lexical-typed/`.
