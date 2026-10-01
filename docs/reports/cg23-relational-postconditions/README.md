# CG23-RELATIONAL-POSTCONDITIONS: return-code dispositions and exit-specific summaries

Card `CG23-RELATIONAL-POSTCONDITIONS` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 9,
phase 4), entries K05 and K11. Base: local `main` `b6a3dc42` (measured there),
rebased onto main `db1ea764` (CG17 11u, CG20 11q, CG25 11v, CG26 11w, CG28 11t, CG27 11x) before the
gate; composes with CG25's in-place status witness (a disposition never stands
beside an in-place kind). Route G.

- **Published property.** The round-9 instruction chose the guarantee. This
  card publishes **custody under return-code-conditioned postconditions**: a
  call's resource effects may depend on the integer it returns, and a caller is
  Safe only where it tests that value against a constant where it stands (or
  right after storing it). The scope is route G, the `if` position and signed
  integer results. Projection coverage is not safety coverage, and no corpus
  coverage moved (§4).
- **Rule:** [gateway 11r](../../agents/gateway.md#constraint-11r). It amends
  rule 10x's "an out-parameter acquisition gets no split" for callees with a
  disposition.
- **Contract syntax:** contract spec
  [§C04.13](../../QCORE_CONTRACT_SPEC_r0_3.md#c0413-return-code-dispositions)
  and grammar G05 (`return_code_disposition`). §C04.4 is amended to point at it.
- **Lean:** `QCore.RelationalPost`, `proof/AuditRelationalPost.lean`
  (15 theorems), bound to the Rust tables by shared vectors.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit `e5e44937`).
  It was written after the code and its synthetic tests and before any AFTER
  build or run; this order is stated, not hidden.

## 1. What changed

**Vocabulary** (`crates/qcore-schemas/src/relational_post.rs`, the loader
clause `crates/qcore-contract/src/contract_relational.rs`).
`ensures normal resource_on_return(OP, VALUE, PREDICATE);` gives a parameter
one mode per **exit class**. An exit class is the set of returned integers
with `v OP VALUE`, where `OP` is one of `eq ne lt le gt ge`. The loader is
total-or-reject:

- The classes must be disjoint and must cover every integer. This is decided
  exactly on the breakpoints `k-1`, `k` and `k+1`.
- A parameter is either unconditional or has exactly one disposition per class.
- The entry requirements must agree across classes.
- The result must be `resource_return_none()` over a signed integer.
- Each class loads through every check a plain declaration gets.
- A declaration whose classes all agree is refused, and so is a disposition
  placed beside the incompatible markers and clauses or beside an in-place kind
  (CG25's status witness, rule 11v).

The result is a `RelationalContract` in the new
`ResourceContracts::relational` map. It is never an entry of the function map,
so a consumer that knows nothing of dispositions sees an uncontracted callee.
`check_source_assumptions` compares the declared dispositions.

**Caller** (`legacy_relational_post.rs`, hooks in `legacy_c.rs`):

- `build_function_cfg_with` gives the parser an environment in which each
  class is installed under a minted, reserved name
  (`qcore_relational_class__K__f`, added to `reserved_spelling`).
- `NAME = f(…); if (…NAME…)` is rewritten to the same C, with the assignment
  moved into the condition.
- The `if` arm first asks `relational_condition_plan`. `read_test` accepts
  `f(…)`, `!f(…)`, `f(…) OP K` and `K OP f(…)`, where the call may be
  `(NAME = f(…))` and `K` must be a plain decimal constant. Each edge takes
  `select`'s single disposition.
- The resulting plan is a `NoOp` branch followed, on each edge, by the call
  classified under that edge's class. An out slot that a class leaves
  unwritten is passed as data, and its name then holds an untracked value
  (`OpaqueValue`).
- Any other test of such a callee is lowered to its named refusal.
- Any other call refuses with `relational-untested` (the join).

**Body** (`legacy_relational_summary.rs`, hooks in `derive_custody_summaries`):

- *Exit-specific summaries (K11).* This route runs after every ordinary route,
  for an undeclared `int` definition whose every `return` is a literal, with
  at least two distinct values. For each value, the body is re-read with every
  other `return` made `for (;;);`, and the ordinary inference and validation
  run unchanged. The resulting classes are published only if they disagree.
- *The body check.* A declared disposition is compared with the body's
  ordinary or exit-specific summary. On a contradiction, the declaration is
  withdrawn and every call site refuses with `relational-withdrawn`.
- *Recursive components.* A declared disposition counts as a declaration of
  its recursive component, and the contextual route never overwrites an
  exit-specific summary.

**Shared items added, all additive:**

- `ResourceContracts::{relational, relational_contract, insert_relational,
  withdraw_relational}`, a serde field that is skipped when empty.
- `LoadedDeclaration::relational`.
- `plain_parameter_predicate`, factored out of `load_function`'s clause loop
  with no change in meaning.
- The 8th reserved prefix.
- No `LegacyInstruction` or CFG field was added.

## 2. BEFORE: the cited refusals reproduce

- CG00's controls pass at the base: `k05_no_relational_postcondition_exists`,
  `k11_an_exit_specific_disposition_has_no_summary` and
  `k_vocabulary_control_loads`, 3 passed (`private/cg23rp/tests/before-phase4.log`,
  exit 0).
- The analysis's K05 claim (Q29: a mode schema states no return-code ×
  resource relation) reproduces.
- K11's claim (Q26/Q29: a summary needs one disposition per formal across all
  exits) also reproduces. CG00's K11 control is an **argument**-conditioned
  release (`if (c) q_free(p)` with `c` a parameter). It stays uncontracted
  after this card and is `CONDITIONAL-RELEASE-BY-LITERAL`'s shape, not a
  return-code one.
- The 10x note ("an out-parameter acquisition gets no split") reproduces: no
  contract could state the relation.

Every BEFORE total equals the brief's baseline:

| Route | Totals (deduplicated) | `resource-diagnostics.json` |
|---|---|---|
| Corpus `66910c10` | 629 / 10 / 4,195 | `3d53a5d0…` |
| System | 44 / 2 / 188 | `725aab4d…` |
| Tail-pause slice | 29 / 1 / 239 / 1 Pending | `8c17467d…` |

The system and slice digests differ from the brief's `79404fed…` and
`93cc660c…` because the report embeds the worktree's profile path (for example
`…/agent-a3d7…/contract/profiles/fluent-bit/../../../providers/…`). Their
totals are equal.

## 3. Synthetic evidence

`crates/qcore-c-gateway/tests/relational_postconditions.rs` (13 tests) runs
route G over the `rp_*` fixture (`fixtures/contracts/relational_post.qv`,
trusted test assumptions). Every Safe result rechecks its certificate. The
tests cover:

- The brief's three tests. `if (rp_take(p) == 0) {} else q_free(p);` is Safe,
  along with four other spellings. The same call untested is not Safe
  (`relational-untested`), including `int rc = …` with a later test. A
  disposition its body contradicts is withdrawn (`relational-withdrawn`); the
  same declaration without a body stands.
- Each edge carries its own class: freeing on the transfer edge, or keeping
  the resource on the failure edge, is provisional Unsafe.
- A test that does not separate the classes (`>= 0`) refuses, and so does a
  test under `&&`.
- The stored-return form, including for `lt`/`ge` classes.
- The status-witnessed out slot (the `getaddrinfo` shape): Safe with the
  release, a leak without it, and refused untested.
- An exit-specific helper, instantiated by `!= 0` and `< 0`. It refuses
  untested and under `> 0`, a test that reaches no returned value.
- A helper with one disposition on every exit stays ordinary.
- A non-literal return gets no exit-specific summary.
- A declaration the body establishes stands.
- Loader refusals: overlap, not total, all classes agree, unconditional and
  conditional on the same parameter, a missing class mode, entry and
  nullability disagreement, a stored mode, `u32`/pointer/`unit` results, and
  placement beside a review marker.

There are also unit tests for `read_test` (18 shapes), `meets` (exact on a
dense window), selection, the rewrite (5 refused neighbours) and the class
environment.

## 4. AFTER and attribution

| Route | AFTER totals | Digest |
|---|---|---|
| Corpus | 629 / 10 / 4,195 | `3d53a5d0…`, byte-identical |
| System | 44 / 2 / 188 | `725aab4d…`, byte-identical |
| Slice | 29 / 1 / 239 / 1 Pending | `8c17467d…`, byte-identical |

No function moved, and no leaf changed on any route (`artifacts/*-comparison.json`).
There is no correction, and no leak or violation was added.

The first AFTER slice run (`cg23rp-after`) crashed: exit 101, a poisoned
parity-sink lock. Free disk had fallen to 46 GB at the time. The run was
repeated with the same binary as `cg23rp-after2` using `scripts/slice.sh`, and
that run is the one reported.

**Why nothing moved:**

- No corpus module declares a disposition.
- All 105 lexical candidate helpers of the census have an **Unknown** row of
  their own, so no per-value body validates.

The census covers `int` definitions whose returns are all literals (41 of them
only through a capitalised macro), with 178 call-site leaves in 100 non-Safe
functions. The prediction's range of 0–3 Unknown → Safe holds, but its point
estimate of 1 did not. Its leaf ranges (≤124 description changes, ≤54 sites
admitted) are realised at 0. The population is registered as
`CG23-EXIT-SPECIFIC-BLOCKED-HELPERS`.

Binaries: BEFORE `06c0e75c…`, AFTER `45a912a8…`. The AFTER build is commit
`5d870cf6`. The later commits change no analysis: clippy's
`.any(&declared)` for `.any(|n| declared(n))`, plus documents and records.

## 5. Absorbed follow-ups

None is discharged by a corpus movement. Each is amended in place with sizes
re-measured on `b6a3dc42` (corpus; system where stated):

- **`CONDITIONAL-RELEASE-BY-LITERAL`:** `rd_kafka_topic_partition_destroy0` has
  2 leaves in 2 functions, 2 sole-blocked. The condition is on an argument,
  not on the returned value.
- **`W45-FB-BACKLOG-MAP-ARMS`:** `flb_input_chunk_map` returns NULL on four
  arms and closes the chunk on one (`:1512-1516`). A return-code class cannot
  separate them. Its row is Unknown (91 leaves), with 1 call leaf on the system
  route.
- **`W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL`:** unchanged. The parse test is
  provisional Unsafe with 2 leaks, and the NULL is argument-conditioned.
- **`W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`:**
  - `realpath` 1/1/0.
  - `pthread_getspecific` **24/16/1** (recorded 16/15: a correction).
  - `flb_output_thread_instance_get` 8 leaves in 4 functions on the system
    route (recorded "six call leaves").
- **`W43-DESCRIPTOR-REFUSED-SHAPES`:**
  - The status-witnessed out list is now **expressible** and exercised on the
    synthetic `rp_open_list`. Declaring `getaddrinfo`/`freeaddrinfo` is left
    to a provider card (pins and the provider gate). Every corpus spelling is
    `ret = getaddrinfo(…); if (ret …)`, the form the rewrite admits.
  - Sizes: getaddrinfo 4/4/0, freeaddrinfo 6/4/0, fcntl 1/1/1, accept4
    **1/1/1** (recorded 1/1/0: a correction), fts and glob 1/1/0 each. pipe,
    fdopen and mmap have no leaf.
- **`CJSON-DELETE-SIBLING-CHAIN`:** no leaf. It is a reachable-set release,
  not a return-code relation.

**Registered residuals:**

- `CG23-EXIT-SPECIFIC-BLOCKED-HELPERS` (above).
- `CG23-DISPOSITION-TEST-POSITIONS`. Loops, connectives, `switch`,
  `int rc = f(); if (rc …)`, later tests and a pointer result's nullness all
  refuse. The corpus population is 0 while no corpus callee carries a
  disposition; the lexical bound is 124.

The ledger moves from 672 to 673 on main `db1ea764` (666 to 667 on the base): CG23 is retired, 2 entries are registered
and 6 are amended (`scripts/update_ledger.py`).

## 6. Axes (§8)

- **Position:** the `if` condition only. `while`/`for`/`do`/`switch` and
  connective sub-leaves refuse.
- **Effect:** every v1 parameter mode except stored and removed.
- **Evaluation order:** the branch reads nothing and is placed before the
  call. Each path applies the call once, with the class its returned value
  lies in. No order C leaves open is chosen, and the stored-return rewrite is
  the same C.
- **Type:** signed integer results only.
- **Boundary:** declarations are `Declared` assumptions, checked against a
  body where one is analysed.
- **Verdict:** untested and withdrawn calls are `Unknown`.

## 7. Lean

`QCore.RelationalPost` (15 audited theorems, `propext`/`Quot.sound` only):

- `meets_sound` and `meets_complete`: breakpoint sampling decides
  intersection exactly.
- `holds_iff` and `neg_sat`.
- `every_value_reaches_one_edge`.
- `test_shape_is_exact`: the four read shapes are C's truth.
- `select_sound` and `the_edge_disposition_is_the_returned_class`:
  instantiating the selection of a decided test is the returned class's
  disposition.
- `join_sound` and `join_refuses_disagreement`: an undecided test joins, and
  disagreement refuses.
- `a_validated_summary_selects_each_exit_disposition`.
- `check_sound` and `withdrawn_selects_nothing`.
- `meets_vectors_hold` and `select_vectors_hold`, bound to `MEETS_VECTORS`
  and `SELECT_VECTORS` by `tests/unit/relational_post.rs`.

## 8. Gate

See [gate/](gate/). Commands, exits and counts are listed in the increment
paragraph and the card's reply.
