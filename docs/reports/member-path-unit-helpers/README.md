# MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: rule 30's body check through the unit's helpers

Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS (wave 48, card-high, small).
Gateway rule **30** amended in place; contract specification **C04.19.6**
amended; Lean `QCore.MemberPathUnitHelpers`. Base: local `main` `d708641bc`,
rebased onto `5b1ceecd5` (FB-QUANT-CONTRACT-CENSUS and LEAN-4-35-TOOLCHAIN,
no product code) before the measurement, then onto `711287771`
(LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED, which names `rk_no_plugin_library` on
34 other `client.qv` declarations; the eight here already named it) before
the gate. The measurement below is on `5b1ceecd5`; the combined tree is not
re-measured (the two cards change disjoint inputs: body evidence of
member-path declarations here, premises of other declarations there). The [prediction](PREDICTION.md)
was frozen (`cd1239503`, `cdbf5e25f` after the rebase) before any
implementation code. Authority: the ledger entry
`MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS` and its DISCHARGE, rule 30,
C04.19.6, rules 13i–13k, the
[OP-CALLBACK-MEMBER-PATH-RELEASE report](../op-callback-member-path-release/README.md).

## 1. Mechanism

`crates/qcore-c-gateway/src/member_path_reading.rs` (`Reader`,
`validated_exits`), called by `member_path_check::validates`, which keeps
its contradiction scan of the declaration's own body unchanged (a
contradiction still withdraws) and now takes the unit's function lookup
(`unit.function`, the gateway's unique body per name) and returns
`Standing::{Validated, Assumption(reason)}`.

* **Graph.** The body is lowered to a control-flow graph: `if`,
  `switch`/`case`/`default`, `while`/`do`/`for`, `break`/`continue`, `goto`
  and labels, `return`, and a statement expression without a transfer out
  of it (glibc's `assert`) as a nested graph. Any other statement, an
  indirect `goto` or asm ends the reading.
* **Values.** Each stated path of the formal carries a set of `ORIG`
  (untouched), `ORIG_NULL` (untouched, tested null), `NULL` (a null pointer
  constant stored, or a touched value tested null) and `OTHER` (anything),
  read to a fixpoint (the sets only grow). A null test of the path (`P->x`,
  `!P->x`, `== NULL`, `!= NULL`, under `__builtin_expect`) refines each
  branch; an empty set is an infeasible branch.
* **Views and calls.** A binding holds a *view* of the formal: the object
  itself or the subobject at a member path. An expression's value holds a
  view when it is a binding, `&P->path` (or an array member decaying), pointer
  arithmetic or `?:` over views, or a pointer-typed call that received one; a
  member's value, an element's value, a comparison or a test holds none. A
  direct call to a function the unit defines with storage class `static` (a
  header `static inline` included) that receives a view is **followed**: its
  body is read from the caller's state with its parameters bound to the
  views (depth 8, never recursively, a budget of 50,000 nodes over every
  frame); `return f(…)` keeps the callee's outcome per returned value, so a
  wrapper reads as its worker. Any other call receiving a view (indirect,
  external, non-static, recursive, a variadic position, past the bound)
  leaves every stated path the view reaches `OTHER`.
* **Ends.** A view copied into a variable, a binding reassigned, effects
  touching the formal in two unsequenced operands, and a pointer store into
  an unstated path inside a followed helper end the reading (a declared
  assumption, never a withdrawal).
* **Exits.** At every exit where a state's guard may hold (an exit
  returning exactly the enumerator of a `!=` guard is exempt; any other
  returned value is read): `#released` / `#moved` read `NULL` or
  `ORIG_NULL`; `{V, #kept}` reads no `OTHER` and the reading saw a null store
  into the path; `#stale` / `#kept` read `ORIG` or `ORIG_NULL`.

**Frame.** A call reaches the formal's members only through its arguments;
a pointer to the formal stored in memory (as `rd_kafka_enq_once_reenable`
stores the op into its eonce) is not followed. The base check read no call
at all, so this reading is strictly more conservative about calls and more
permissive only through followed helpers, `goto`, branches and guards.

Tests (`member_path_check.rs`): the brief's five (a helper that clears the
member validates; a helper on only some paths does not, unless the other
branch tests the path null; a `goto` to a shared exit that clears
validates; a `goto` that skips the clear does not; recursion does not),
plus a header-style helper reached through `&P->…replyq` (validated, and
not once it hands its view to an external call), a wrapper returning a
static worker under a `!=` guard (validated; unguarded, the `KEEP` exit
must establish it too), a call not followed after the store (not
validated; a member's value passed on is), a copy of the formal into a
local (not validated), and the base check's four cases. The route-G tests
(`tests/member_path_release.rs`, 8) pass unchanged.

## 2. Probe

[`examples/member_path_probe.rs`](../../../crates/qcore-c-gateway/examples/member_path_probe.rs)
runs the same check over one unit's functions (dumped by
[`unit_ast.py`](scripts/unit_ast.py) and
[`unit_functions.py`](scripts/unit_functions.py) from the pinned compile
database) with the pointee read as `rd_kafka_op_s`; a reading aid that
names the reason, not the arbiter.

## 3. Standing per declaration (corpus sidecar)

[`standing.py`](scripts/standing.py) → [standing](artifacts/standing.json);
the reason column is the [probe](artifacts/probe.txt)'s, read against the
bodies (both revisions byte-identical, as the predecessor recorded).

| declaration | before | after | reason |
| --- | --- | --- | --- |
| `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` | declared assumption | **partly validated** (`30`) | the wrapper's `return` follows the static worker; every `HANDLED` exit clears `leaders.eonce` or tests it null, clears `leaders.replyq.q` inside the header-inline `rd_kafka_replyq_enq` (reached through `&rko->rko_u.leaders.replyq`) or tests it null, leaves `partitions` cleared or untouched (a null store read) and `opaque` untouched; the `KEEP` exits are exempt by the `!= KEEP` guard |
| `rd_kafka_DeleteRecords_leaders_queried_cb` | declared assumption | **partly validated** (`30`) | `goto err` read; `reply` is only read for its members, so `opaque` is untouched at both exits |
| `rd_kafka_admin_fanout_worker` | partly validated (`30`) | **declared assumption** (correction) | `fanout_parent` reads `{null, other}` at the `HANDLED` exits: the indirect `rko_fanout->rko_u.admin_request.fanout.cbs->partial_response(rko_fanout, rko)` receives the formal after its null store; the base check read no call. The callee's parameter is `const rd_kafka_op_t *`, which the reading does not trust |
| `rd_kafka_admin_worker` | declared assumption | declared assumption | the reading ends where `rd_kafka_admin_result_fail` copies `rd_kafka_admin_result_new(rko_req)` (a pointer-typed call that received the op) into `rko_result`; not followed on the way: `rd_list_move` (external, `rdlist.c`) on `admin_request.args` in `rd_kafka_admin_result_new`, the indirect `parse` in `rd_kafka_admin_response_parse` (every stated path), and, by reading, the indirect `cbs->request(…, &rko->rko_u.admin_request.args, …)` on the path to `destroy:` |
| `rd_kafka_ListOffsets_leaders_queried_cb` | partly validated | partly validated | `reply` only read for its members |
| the three share fan-outs | partly validated | partly validated | the op only read for its members after the stores |

Net (corpus, 428 declarations consulted): partly validated **45 → 46**,
declared assumptions 319 → 318, validated 63 and held 1 unchanged. As
predicted, declaration by declaration. Each declaration names the premise
`rk_no_plugin_library`, so none can be fully validated (rule 13j).

## 4. Measurement

BEFORE `before2` (`33527d73…`, the worktree detached at `5b1ceecd5`) and
AFTER `a2` (`fd6e0ee2…`, the worktree at `47333e913`, product code equal to
`bffd7b514` plus behaviour-neutral clippy changes), both built by
[`build.sh`](scripts/build.sh) from the worktree's own tree and measured on
every route with the null property ([`run-all.sh`](scripts/run-all.sh),
[`after.sh`](scripts/after.sh)); every `run.json` names its binary.

| route | BEFORE | AFTER | report sha256 | wall (s) |
| --- | --- | --- | --- | --- |
| corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `71387bd3…` → `a2e95319…` | 689.4 → 672.3 |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `143d6649…` (identical) | 78.5 → 78.8 |
| slice | 52 / 1 / 347, 4 Pending | the same | `c376ef42…` (identical) | 40.0 → 40.1 |
| null | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | every row identical (4,921, 237, 271 rows) | | |

* **No row moves** on any route ([corpus](artifacts/compare-corpus.json),
  [system](artifacts/compare-system.json), [slice](artifacts/compare-slice.json),
  [null](artifacts/null-compare.txt)). The corpus report differs only in the
  `contract_body_evidence` of `rdkafka_admin.c` and `rdkafka_partition.c`
  ([projection](artifacts/projection-corpus.json), [`projection.py`](scripts/projection.py)):
  without it the two reports are equal, every row's `contract_dependencies`
  included. The Safe roll-up is unchanged (no Safe row consults any of the
  eight; 16 Unknown rows do).
* BEFORE's corpus digest is main's (`71387bd3…`, the predecessor's AFTER).
* **Failed attempt, kept and discarded:** the first binaries (`before`
  `f03de150…`, `a1` `cfd6dac5…`) were built from `git archive` exports so
  the build could wait on the shared lock while the tree changed; their
  corpus runs exited 1 in the Fluent Bit audit
  (`Pin(GitUnavailable { pin: "libevent" })`: an export has no git identity
  for the submodule), so they measure nothing of the corpus; their system and
  slice runs are not used ([`build-export.sh`](scripts/build-export.sh),
  `private/mpuh/attempt-export-*`).
* Wall times were measured once each on a shared host; not a performance
  claim.

**Prediction against outcome.** As predicted on every declaration and
route: two gained, one corrected, one unchanged; no verdict or row moves.

## 5. Lean

`proof/QCore/MemberPathUnitHelpers.lean`, audited by
`proof/AuditMemberPathUnitHelpers.lean`: `aexec_sound` (every concrete run's
final reading is admitted by the reading, at every frame depth),
`a_validated_must_path_reads_null`, `a_validated_set_path_is_kept_or_null`,
`a_validated_stale_path_is_untouched`, `a_call_summary_is_inlining` and
`a_call_runs_its_body` (validation through a call summary is inlining),
`a_path_not_followed_never_validates` and `the_depth_bound_never_validates`,
`an_exempt_exit_falsifies_its_guard`, `an_unnamed_return_is_never_exempt`,
`an_eq_guard_is_never_exempt`, `untouched_end_is_entry`,
`a_null_store_validates_must`. Axioms: `propext` and `Quot.sound` only.
Lean audits 324 → **325** (built under 4.35.0).

## 6. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 784 → **784** (783 → 783 on
the measured base).

* **Retired:** `MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS` (DISCHARGE holds:
  the check follows a static callee of the unit called with the formal and a
  structured `goto`; each of the three is validated or its residual named;
  measured).
* **Registered:** `MEMBER-PATH-STATES-THROUGH-INDIRECT-AND-EXTERNAL-CALLS`
  (2 declarations, `rd_kafka_admin_worker` and the corrected
  `rd_kafka_admin_fanout_worker`; 4 Unknown corpus rows name them, no Safe
  row).

## 7. Gate

Gate **g1** ([gate.json](gate.json), exits in [gate/](gate/), logs
`private/mpuh/gate-g1/`): every code step at `8dc6581dd` (the tip on main
`711287771`; later commits change records only), the standing gate under one
exclusive flock on the shared heavy lock (2026-10-15 16:24:52Z–16:35:12Z;
[`gate-run.sh`](scripts/gate-run.sh), [`gate-whole.sh`](scripts/gate-whole.sh),
[`gate.sh`](scripts/gate.sh), then [`close-gate.sh`](scripts/close-gate.sh)
→ [`write_gate_json.py`](scripts/write_gate_json.py) and
[`finish-gate.sh`](scripts/finish-gate.sh)). No provider gate: nothing under
`providers/` or `crates/qcore-schemas/` changes.

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update | 0 | |
| `cargo fmt --check` | 0 | |
| `clippy --locked --all-targets -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 499 units, 512 summaries, **8,106 passed, 0 failed, 15 ignored** (`private/mpuh/tests-g1/`), no rerun |
| both reevaluation probes | 0, 0 | |
| `check_lean_audits.py` | 0 | **325** audits passed (Lean 4.35.0) |
| `cargo clean --profile dev` | 0 | |
| Python suite before the matrix regeneration (08) | **1** | fails only `test_the_derived_facts_are_recomputed_rather_than_carried` (the committed matrix's Lean count, 324 → 325), as for every card adding an audit |
| docs index check (09) | 0 | |
| `build_matrix.py . --gate gate.json` (09a), Python suite (09b, 374 tests OK), docs index (10b) and its check (10c) | 0, 0, 0, 0 | |

The bold rule-id duplicate check (`rg -o --no-filename '^[0-9]{1,2}[a-z]?\. \*\*'
docs/agents/*.md | sort | uniq -d`) is empty; no rule id was added (rule 30
amended in place). Clang: [`gate/clang-version.txt`](gate/clang-version.txt).

## Files

* `crates/qcore-c-gateway/src/{member_path_reading,member_path_check,legacy_callback_route,lib}.rs`,
  `examples/member_path_probe.rs`.
* Lean: `proof/QCore/MemberPathUnitHelpers.lean`,
  `proof/AuditMemberPathUnitHelpers.lean`, `proof/QCore.lean`.
* Records: `docs/agents/{gateway,README}.md` (rule 30, its router row),
  `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.6), `docs/STATUS.md`,
  `docs/development/increments/wave-48-part8.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
