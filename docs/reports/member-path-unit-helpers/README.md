# MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: rule 30's body check through the unit's helpers

Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS (wave 48, card-high, small).
Gateway rule **30** amended in place; contract specification **C04.19.6**
amended; Lean `QCore.MemberPathUnitHelpers`. Base: local `main`
`d708641bc`. The [prediction](PREDICTION.md) was frozen (`cd1239503`)
before any implementation code. Authority: the ledger entry
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

MEASUREMENT_PLACEHOLDER
