# Explicit entry and exit conditions

User decision 2026-10-26: `quant condition` with `entry`, `normal`, `error(E)`
and `any`. The preimplementation prediction is in PREDICTION.md. G01c and
L19.1/C03.1 specify position, bounded syntax and consumer boundaries.

The source kind retains the selector, expression and span. The schema wraps
G01p's proposition AST with an explicit selector; the frontend reuses its full
typed formula checks and limits. Typed condition inspection rejects duplicate
formal IDs. Generic inclusion/views preserve the new introducer. Strict
resource/bounds/local-condition consumers refuse it. Hypothesis-only inspection
refuses an unsupported entry condition and never uses exit assertions as
antecedents. No truth/evidence assignment or C summary is installed.

CFL hash declarations replace nine sentence-shaped postcondition clauses with
six mathematical normal-exit assertions. The three remaining statements describe
absence of a guarantee or a separate possibility witness; they become explanatory
comments, not false state negations or per-call zero-result guarantees. The
pinned nonempty NUL-free zero-digest witness remains explicit. CMetrics opts'
result-domain sentence becomes `result == 0 || result == -1` with an explicit
integer result parameter. Per-clause records and before/after source identities
are in hash-postconditions.json and opts-postconditions.json. Source/algorithm,
trace completeness, projection and model/Rust correspondence remain open.

`QCore.QuantCondition` defines selection on an explicit execution. Six audited
properties cover normal/error exits under `any`, exact error labels, separation
of entry and normal assertions, and absence of a termination guarantee. An
execution without a terminating exit is not a missing observation record.

Validation so far: focused all-target contract compilation exited 0 in
`private/quant-contract-hypotheses/condition-1/`; focused condition-test compilation
exited 0 in `condition-2/`. The targeted Lean build/audit exited 0 (six checks)
in `condition-lean-1/`. The real source inventory exited 0, retaining seven
conditions across five declarations, in `condition-models-1/`. These paths share
prefix `private/quant-contract-hypotheses/`. Rust tests have not executed; the
shared Rust slot remains occupied by another worktree. Full card/provider/native
validation and the remaining provider pre/postcondition migration are open.

The subsequent all-target `cargo clippy --all-targets --offline --locked --
-D warnings` exited 0 in `condition-clippy-1/` under the same private prefix.
The postcondition census currently finds 396 remaining bare normal-postcondition
occurrences in production provider sources, plus 11 conformance-fixture clauses
(311 unique expressions across both sets). This is a lexical inventory, not a
claim that each bare symbol has the same semantic role or that migration is done.
