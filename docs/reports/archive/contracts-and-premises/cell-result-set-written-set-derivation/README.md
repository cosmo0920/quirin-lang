# CELL-RESULT-SET-WRITTEN-SET-DERIVATION

Card brief: `private/integrator-briefs/r11-common.md` (round 11, 2026-11-04). Base `c8c7edf21`, baseline `baseline:c8c7edf21`. This report records the DISCHARGE of `named_followups` entry CELL-RESULT-SET-WRITTEN-SET-DERIVATION, which CELL-RESULT-SET-ALIGNMENT registered. The card retires that entry through its [amendments](integration-amendments.json).

> **USER LAUNCH 2026-11-04** (verbatim): "CELL-RESULT-SET-WRITTEN-SET-DERIVATION."
>
> **USER DECISIONS 2026-11-04** (verbatim, standing):
> - "Basically, Q1(a) and Q2(i) But we must not handle descriptor 0 as a valid handle. So, we need to exclude the 0 descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in [1, MAX]"
> - "Write [1, MAX]"
> - Q3: "(α) live_set suffix"
> - On patch v3's native leak: "Accept the leak"
>
> **USER DECISION 2026-11-03**: "Per-end sentinels". Standing direction: a caller's test is not evidence, so a set comes from the callee's body.

**Every set this card derives rests on `pipe`'s declared `[1, MAX]`.** That bound is the user's policy, not POSIX's set, which is `[0, MAX]`. With POSIX's set the same bodies derive `[0, MAX]` for the read end (§1, fixture and Lean). That set is not inside the declarations, so nothing would be pinned and every creation would keep refusing. Every reading the derived sets decide is attributed to the bound (follow-up CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY, amended).

Prediction: [PREDICTION.md](PREDICTION.md). It was committed at `aaefbfb32`, before any implementation.

## 1. The derivation (rule 38e as amended)

`quirin-audit pin --bounds --derive-results` now derives every declared `written_set(SLOT[K], SET) when #result == C` of a pinned declaration. It does so beside the result range, from the same configured units.

**Code.**
- The walk is `qcore_c_gateway::null_deref::derive_written_sets_c11`, in `null_deref/written.rs` (`WrittenDeriver`, `Walker`).
- The C-free rule is `qcore_core::written_set` (`combine`, `narrow`, `resolve`, `after_store`, `COMBINE_VECTORS`, `RESOLVE_VECTORS`).
- The pin is `contract_pin.rs` (`derive_results`, `DeclaredWritten`). The lock record is `contract_revision.rs` (`ResultDerivation::written_sets`, `WrittenDerivation`).
- The route table is `bounds_selection.rs` (`written_sets_of`, key `qcore_schemas::result_range::written_set_key`).

**How the walk reads a body.** It goes path by path. Each path that ends at a `return` carries two sets: the values it may return, and the values the slot holds there. A slot set of "none" means unwritten or unknown. A `return` value set is narrowed by every test `v OP k`, `!v` or `v` of a local on the path, including `(ret = G(...)) < 0`.

The derived set is the union of the slot sets of the paths that may return `C`. A path that may return `C` without a slot set refuses (`written-set-unwritten`). A body with no such path refuses too (`written-set-no-return`).

**The two shapes that give a slot set:**

- **Forwarding**, `return G(.., P, ..)`. The slot holds `G`'s written set of that position for `C`. That set is either a C library declaration's, or an in-unit helper's set derived from its own body. `F` returns exactly `G`'s value: only value-preserving conversions are stripped.
- **A store after the C library call's failure exit.** `ret = G(.., A, ..)` links `ret` to the out array `A`, or to the slot itself. A test then narrows `ret`, within `G`'s result set, on each edge.
  - If the narrowed set is exactly one class `c`, `A[k]` holds `G`'s written set of class `c`. A later `*P = A[k]` or `P[K] = A[k]` stores it.
  - If the narrowed set is empty, the edge cannot occur.
  - A `return ret` whose narrowed set excludes `C` constrains nothing. That is how `if (ret < 0) return ret;` and `if (ret != 0) return ret;` drop out.

**What the walk refuses (fail-closed).**
- A value it does not know is its type's whole range.
- After the slot is written, a call or a store of another type leaves the slot unknown.
- An `int` store through any other pointer joins its value to the slot set, since it may alias the slot.
- A local whose address is taken is never narrowed.
- An out array is read only when every call it is handed to is a C library declaration stating a written set of that position. Under `libc-c-standard-abi` such a call writes the array and keeps no pointer to it. The array's every other mention must be a read `A[k]` at a constant.
- A loop (other than `do { } while (0)` without jumps), `switch`, `goto`, `?:`, a statement expression, or an effect under `&&`/`||` refuses the whole derivation.

**Every configuration is read.** The pin reads every configuration of the defining file in the compile database. `src/flb_pipe.c` is built into `fluent-bit-static` and `fluent-bit-shared`, so it is read twice; the range derivation went through the same loop. Each derived set must lie inside the declared one, or nothing is pinned. The record is their union.

**Each configuration's body decides its own set.** A backend whose C library call states no written set derives nothing:
- libevent's `_mk_event_channel_create`, through `evutil_socketpair`, gives `[MIN, MAX]`;
- the Windows `flb_pipe_create` refuses `written-set-unwritten`.

The pin never borrows another configuration's or another tree's body.

## 2. The pinned declarations (per creator, backend and pin)

The declarations are:
- `mk_event_channel_create` in `monkey/result_range.qv`: `written_set(r_fd[0], [1, MAX])` and `written_set(w_fd[0], [1, MAX])` when `#result == 0`, beside its `nonnull_result_range(-1, 0)`;
- `flb_pipe_create` in `fluent_bit/result_range.qv`, a new module of the Fluent Bit bounds view: `written_set(pipefd[0], [1, MAX])` and `written_set(pipefd[1], [1, MAX])` when `#result == 0`. It carries no range and no custody.

`providers/contracts.py sync` updated the load path and both inventories. Both bounds locks were written by `quirin-audit pin --bounds --derive-results --compile-database FILE --crate C --function F` at each tree. The reviewed lock diffs are in the same commit (`641d6c408`).

| Creator | Pin | Configured backend (database) | Derived | Helpers | Rests on |
|---|---|---|---|---|---|
| `mk_event_channel_create` | 66910c10 | epoll, `mk_core.dir/mk_event.c` (corpus `fluent-bit-clang-source/build`) | `r_fd[0]` [1, MAX], `w_fd[0]` [1, MAX], result [-1, 0] | `_mk_event_add`, `_mk_event_channel_create` | `epoll_ctl`, `pipe` |
| `mk_event_channel_create` | 34155955 | epoll (slice `fluent-bit-clang-production`) | the same | the same | the same |
| `mk_event_channel_create` | 046accf6 | epoll (system `fluent-bit-clang-production`) | the same | the same | the same |
| `flb_pipe_create` | 66910c10, 34155955, 046accf6 | POSIX body, two configurations each (static, shared) | `pipefd[0]` [1, MAX], `pipefd[1]` [1, MAX] | none | `pipe` |

**Identities.** The libevent lock keeps two identities, as before:
- 046accf6 with 34155955;
- 66910c10 on its own, because the poll backend differs.

The contract text changed, so both earlier identities were voided and re-recorded. The Fluent Bit lock's `flb_pipe_create` has one identity at all three trees. Its sites are the header declaration and both definitions in `src/flb_pipe.c`: Windows and POSIX.

**Backends no reviewed database configures.** kqueue, poll, select and libevent for Monkey, and Windows for `flb_pipe_create`, are not configured. Their definition sites are in the identities, so a change to any of them takes the pin out of force. But the derivation records the configured body's set only.

**Where a set is read.**
- `flb_pipe_create` has no resource or cell declaration, so no route reads its sets yet (follow-up FLB-PIPE-CREATE-CELL-DECLARATION).
- The corpus's standalone Monkey units come from a libevent build. They read Fluent Bit-tree pins (CELL-GUARD-RESULT-RANGE-CORPUS-MONKEY-IDENTITY, amended), but none creates into a cell with a declared live set.

## 3. The reading (rule 38f)

The route table carries an in-force written set as `FUNCTION#P[K]==C`. Here `P` is the parameter position, `K` the element and `C` the class.

It is in force when all three hold:
- the declaration is not held, which needs its pin in force and every C library declaration it rested on unchanged;
- its lock derivation lists that slot and class;
- otherwise it is `held` or `unchecked`, exactly as a range.

Rule 38f's creation check reads only the written set of the class that the cell's `#fresh` state holds in. `mk_event_channel_create`'s `when #result == 0` reads `==0`. A state under `!= C`, or under every return, reads none. Before this card the reader looked up `FUNCTION#P` without a class. No such entry was ever produced, so nothing was read; the class check is new.

## 4. Lean

`QCore.WrittenSetDerivation` is audited in `proof/AuditWrittenSetDerivation.lean` and registered under rule 38. It uses standard axioms only, with no `sorry`. Its theorems:

- `combine_sound`: the union of the slot sets of the paths that may return `C` holds the slot on every run returning `C`.
- `narrow_sound` and `a_narrowing_without_the_class_constrains_nothing`.
- `resolve_sound` and `an_empty_narrowing_is_infeasible`.
- `after_store_sound`: an aliasing store joins its value.
- `forwarding_sound`: a function's runs are its callee's runs.
- `the_declared_bound_admits_the_creation` and `posix_s_set_derives_descriptor_zero`: the attribution to the declared bound.
- `combine_vectors_hold` and `resolve_vectors_hold`, bound entry for entry to the Rust `COMBINE_VECTORS` and `RESOLVE_VECTORS` by `crates/qcore-core/tests/written_set.rs`, which reads the Lean text.

## 5. Fixtures

**`crates/qcore-c-gateway/tests/written_set_derivation.rs`** (7 tests, real Clang AST, bodies reduced from 046accf6):
- the epoll chain gives `[1, MAX]` for both ends, with the helpers and C library sets above;
- `flb_pipe_create` forwards `pipe`'s sets;
- POSIX's `[0, MAX]` gives `[0, MAX]` for the read end;
- refusals:
  - the libevent backend (`[MIN, MAX]`);
  - the Windows body (`written-set-unwritten`);
  - forwarding to `evutil_socketpair` ("states no written set");
  - a store before the failure exit (`[MIN, MAX]`);
  - an unnarrowed local: address taken (`written-set-unwritten`), or a test that leaves 0 (`[MIN, MAX]`);
  - a call after the store;
  - an out array handed to an undeclared callee;
  - a loop;
- the inline `(ret = pipe(fd)) < 0` shape derives.

**`crates/qcore-c-gateway/tests/cell_protocol_per_end_guards.rs`** (2 new tests):
- With the written sets in force inside `live_set([1, MAX])`, `input_instance_channel_events_init`'s creations pass the live-set check. They refuse `descriptor-cell-undecided`, since the path holds no dead reading of the ends at entry; the row stays `Unknown`.
- A set held, unchecked, of another class, or POSIX's `[0, MAX]` keeps `descriptor-cell-live-set-unproved`.

**`qcore-audit` unit test `only_a_derived_unheld_written_set_is_in_force`:** the table's standing per slot and class, held and unchecked included. **`qcore-core`:** the module's own vector, narrowing and aliasing tests.

## 6. Prediction and measurement

[PREDICTION.md](PREDICTION.md) predicts no status move on any route. Its one content change is in the slice's `input_instance_channel_events_init`, units 0 and 1:
- the two `descriptor-cell-live-set-unproved` leaves at :1380 become `descriptor-cell-undecided`;
- the :1401 `protocol-broken` and `escapes` leaves may return as at `baseline:32169290d`.

The change is attributed to the declared bound.

The worktree's isolation guard refuses the live measurement, so the integrator runs it from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/cell-result-set-written-set-derivation-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:c8c7edf2165a2065c234f3a63b4eed23e937ced6 \
  --side tip=rev:864519c368874f0e07fdde5e19818f6294517cb7 \
  --declared docs/reports/cell-result-set-written-set-derivation/PREDICTION.md
```

**Measurement (integrator, 2026-11-05).** Run from the main checkout at
`c8c7edf21` with the command above, output
`private/measure/written-set-derivation-20261104T151649Z`; exit 0. The
declared prediction matches on every route (corpus, system, slice: `match`).
Moves 0, unit moves 0, unmatched 0 on each route; content changes: corpus 0,
system 0, slice 2:

- slice `input_instance_channel_events_init` (units 0, 1): the two
  `descriptor-cell-live-set-unproved` leaves at :1380 are replaced by two
  `descriptor-cell-undecided` leaves; the :1401 leaves did not return. The row
  stays Unknown. The change is attributed to the declared bound
  (CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY).

[native-comparison.json](native-comparison.json) (sha256
`c11907dd6f06bf55616e6cfff01cf2de16588ff92574a7f063f61a5688a74033`).

## 7. Card gate

The card gate was `python3 scripts/gate_scope.py --base c8c7edf21 --out private/gate2`. It ran on `864519c36`, the last code commit; this report, its amendments and `gate.json` were untracked prose at the time. The full record is [gate.json](gate.json).

The scope was full:
- the diff reaches `qcore-core`, `qcore-c-gateway`, `qcore-contract` and `qcore-schemas`;
- a proof root changed, so all of Lean ran;
- `providers/` changed, so the provider gate ran.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 |
| fmt, clippy | 0, 0 | |
| Rust suite | 0 | 604 units, 645 summaries; 9,177 passed, 0 failed, 16 ignored, 135 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean | 0 | 407 audits |
| Python suite | **1** | 577 tests, 1 failure (below) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider `cargo test` | 0 | 43 summaries; 133 passed, 0 failed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The one Python failure is the expected stale record: `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`, `['24.0.0', 406] != ['24.0.0', 407]`. This card adds `AuditWrittenSetDerivation.lean`, and the integrator regenerates the records.

**The earlier run**, `private/gate`, on `afbeaae3e`, failed on two things. That run's logs were removed with its polluted `target/`; the console log is kept, below.
- Three Rust units: `contract_views` (4 tests), `contract_view_census` and `contract_loader`'s rejected-declaration count.
  - The `null_deref_beside` relation of `mk_event_channel_create` refused the new `written_set` clauses.
  - The corpus gained one rejected declaration.
  - `864519c36` admits `written_set(` in that relation, as it admits `result_set(`, and updates the count to 370.
- The provider steps, on `E0464`: an `.rlib` and a `.so` of the same crate in `target/`, left by my own unconfigured `cargo build`. I removed the worktree's `target/` before the second run.

Console logs, outside the worktree:
- `/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/crswsd/gate1-console.log`
- `/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/crswsd/gate2-console.log`

The step logs are in the worktree under `private/gate2/`.

## 8. Follow-ups

- **Retired: CELL-RESULT-SET-WRITTEN-SET-DERIVATION.**
- **Registered: FLB-PIPE-CREATE-CELL-DECLARATION.** `flb_pipe_create`'s written sets are pinned and derived, but no route reads them. The function has no resource declaration and no cell states, and its callers stay `call to uncontracted function`. A slot cell names only `#pointee`, that is element 0, so `pipefd[1]` needs a per-element slot place.
- **Amended: CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY.** The derived written sets rest on the bound, and so does the slice's creation admission.
- **Amended: CELL-GUARD-RESULT-RANGE-CORPUS-MONKEY-IDENTITY.** It now also covers the derived written sets, and the configuration dimension: the libevent body derives nothing.
