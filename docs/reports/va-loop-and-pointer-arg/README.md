# VA-LOOP-AND-POINTER-ARG: `va_arg` in a condition and the never-owned pointer value

Wave 48 supplemental card (round 12), route G. Authority: the registered
follow-ups `CG26-POINTER-VA-ARG` and the condition part of
`CG26-VA-PROTOCOL-PRECISION` ([CG26 report](../cg26-variadics/README.md)),
and the shared acceptance probe, `mk_vhost_set` at the tip of Monkey branch
`cosmo0920-missing-va_end-on-null` (`21a2c190`). Base: local `main`
`958e76593` (unchanged through the card).

- **Rule:** [gateway 12c](../../agents/gateway.md#constraint-12c) (the gateway
  letters `11a`–`11z` and `12`, `12a`, `12b` are taken), amending rule 11w's
  lowering only.
- **Lean:** `QCore.VariadicConditions`, `proof/AuditVariadicConditions.lean`
  (16 theorems), bound to the Rust graph decider and the binding by two unit
  tests.
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen at `3b4f1fcdf` from
  the BEFORE census, before any implementation code of the card.
- **Property and scope.** Custody under variadic definitions (rule 11w's
  published guarantee), extended to a `va_arg` read in a controlling
  expression and to a pointer value read by `va_arg`. Nothing about which
  arguments a `va_arg` reads or their types (K10) is claimed.

## 1. What changed

**A `va_arg` in a controlling expression** (`legacy_va_condition.rs`, one hook
in `legacy_condition_desugar.rs`'s `planned_condition`, asked after every
older route of an `if`, `while`, `for` or `do` condition including CG02's
elaborator). Only the whole expression `(NAME = va_arg(L, T))`, matched to its
Clang node, under a protocol rule 11w admits, is lowered: a plan whose entry
is the event's statement (exactly what `NAME = va_arg(L, T);` lowers to) and
whose second node is the test of `NAME` (a null branch for a pointer, a branch
refining nothing for a scalar) with the condition's two exits. The value of
`NAME = E` is `NAME` after the store (C11 6.5.16p3) and the condition is a full
expression, so testing `NAME` after the event is C's order; the loop's back
edge and `continue` re-enter the event node, one event per evaluation. Every
other expression holding a `VAArgExpr` is refused by name (`va_list operation
refused: a va_arg in a controlling expression that is not (NAME = va_arg(L,
T))`); under a refused protocol the condition carries the protocol's reason.

**The pointer value** (`legacy_variadics.rs`). `T *x = va_arg(L, T *)` and
`x = va_arg(L, T *)` for an object pointer type (no function pointer, array or
`volatile`) into an automatic local — a body `VarDecl` with no storage class,
the only object of its spelling, not a formal, protected formal or descriptor
name — are the new instruction `VaArgPointer { x }`. Its binding is the one a
declared derived result over an unbound base already gets
(`derive_pointer(None, Exact, …)`): no target, unknown, possibly null,
`Derived`. By card W43-LIBC-RESULT-ALIAS's rules, unchanged, a null test
narrows the binding alone, a `Borrowed` position reads it, and a release,
consumption, store, escape, return or reallocation through it is never applied
and refuses with `UnknownAlias` (with no target it is never accused). Every
other destination keeps the named refusal.

**Deviation from the brief, stated.** The brief names a C02-P opaque member
read (`OpaqueValue`). That binding is an identity-unknown pointer, and
`refine_pointer_null` answers a null test of one with `UnknownAlias`
("nullness is independent of identity"), so `while ((key = va_arg(…)))` and
`if (!value)` would each have kept a refusal (measured on the first test run:
`UnknownAlias key`, `UnknownAlias value` on the tip shape). The derived binding
is the existing opaque, never-owning vocabulary whose null test narrows only
itself; everything else the brief requires (borrowed reads admitted; release,
custody store, return as owned refused) holds and is tested.

**Shared items added (for the integrator).** `LegacyInstruction::VaArgPointer`
(one transfer arm, listed with the other destinations in `rebound_names`, the
slot-destination match and `legacy_descriptor_custody`'s data instructions);
`variadics_route::{StoredClass, stored_assignment}` and
`Parser::stored_va_arg`; the module `va_condition_route`; the one hook in
`planned_condition` (after `elaborated_condition`).

## 2. Reproduced claims

| Claim (input) | BEFORE (executed) | Standing |
|---|---|---|
| Probe, tip: `:525` loop condition, `:526` pointer `va_arg`, `:533` `mk_vhost_set_property` | [probe-before-tip](artifacts/probe-before-tip.jsonl) | reproduces |
| Probe, master: four `va_end` refusals, the loop condition, `mk_vhost_set_property` | [probe-before-master](artifacts/probe-before-master.jsonl) | reproduces |
| Corpus on main 647 / 10 / 4,177, `ed384eb0…` | `private/valoop/before` | reproduces |
| CG26-POINTER-VA-ARG: 3 corpus functions, 4 slice functions | census | reproduces |
| CG26-VA-PROTOCOL-PRECISION: `va_arg` inside another expression, 3 leaves | census | reproduces; the condition form was **not** in its count (masked by the generic loop refusal): 7 corpus, 4 slice functions |

## 3. Measurements

Release `quirin-audit`, corpus arbiter `66910c10` `--jobs 4`, system route and
tail-pause slice ([scripts/measure.sh](scripts/measure.sh)); totals as
`compare.py` keys them (one row per file and function).

| Run | binary | corpus | system | tail-pause slice |
|---|---|---|---|---|
| BEFORE, main `958e76593` | `9263920a…` | **647 / 10 / 4,177** `ed384eb0…` | 44 / 2 / 188 `471de81c…` | 29 / 1 / 239 / 1 P `0787f2a2…` |
| AFTER, tip `fc0c6ca5d` | `89d5dbb0…` | **647 / 10 / 4,177** `7f7d8328…` | 44 / 2 / 188 `471de81c…` (byte-identical) | 29 / 1 / 238 / **2 P** `896bcb24…` |

(The census scripts count unit rows, so they show the system as 83 / 4 / 361
and the slice as 45 / 1 / 356 / 2 P before; the slice's four setters appear in
two units each.)

**Corpus** ([comparison](artifacts/compare-corpus.json)): no status move, no
leak, violation or Unsafe added, 10 Unknown functions change leaves:

| Function | Change | Reading |
|---|---|---|
| `ut_init_member_internal`, `verifyAssignment0`, `verifyMultipleAssignment0`, `ut_create_topic_partition_lists` (`const char *`), `rd_kafka_err_action` (`int`), `mk_config_set` (`char *`) | loop-condition refusal gone | lowered; each keeps indirect accesses, uncontracted callees or unresolved identities |
| `rd_kafka_err_action` | also `unresolved resource identity` −2 (`actions`, `request`) | the refused condition's write frame no longer poisons the formals' bindings |
| `mk_config_set`, `ut_topic_partitions`, `ut_populate_member_owned_partitions_generation` | pointer-`va_arg` refusal gone | `VaArgPointer` |
| `mk_vhost_set` (master copy) | loop-condition refusal → `va` may reach a function exit without `va_end` | **correct**: the `!value` return without `va_end` (SUP04, the defect the tip fixes) |
| `rd_kafka_producev` | loop-condition refusal → the named shape refusal | `!err && (vtype = va_arg(…)) != END`: an event under `&&`, already a protocol refusal |

**System:** byte-identical. **Slice** ([comparison](artifacts/compare-slice.json)):
**`flb_service_set` Unknown → Pending**, its only remaining leaf the call to
`flb_config_set_property`, whose declared contract awaits human review; the
other three setters (`flb_input_set`, `flb_output_set`, `flb_filter_set`) keep
their uncontracted property setter. Loop-condition −4, pointer-`va_arg` −4.

**The probe** (contracts route on the scratch copy; [after-tip](artifacts/probe-after-tip.jsonl),
[after-master](artifacts/probe-after-master.jsonl)): at the tip `mk_vhost_set`
keeps exactly one leaf, `call to uncontracted function mk_vhost_set_property`
(blocker 3, the other card's); blockers 1 and 2 are gone. On master it keeps
its five protocol refusals (the condition now among them) and the
`mk_vhost_set_property` call. `mk_vhost_set_property` is unchanged.

**Prediction against outcome.** Predicted no corpus status move, the loop
condition −7, pointer refusal −3, protocol +1 (`mk_vhost_set:525`), other
changes only in the 9 census functions; system byte-identical; slice
`flb_service_set` → Pending; the probe rows above. All hold except one leaf
not predicted: `rd_kafka_producev:564`'s generic loop refusal became the named
shape refusal (loop condition −8, not −7), a renaming in a protocol-refused
function, no status effect. `rd_kafka_err_action`'s −2 lie within the
allowance.

## 4. Tests and Lean

Rust: `tests/va_loop_pointer_arg.rs` (10: the tip `mk_vhost_set` shape Safe
over fixtures, the master shape refused for `va_end` at the condition too, the
scalar `while`/`if`/`do` forms, the back edge re-evaluating the event (a
`va_end` in the body is a read after `va_end`), comparison, bare and `switch`
shapes refused by name, a pointer passed to a borrowed callee admitted,
`q_free(va_arg(…))`, `q_free(p)`, `q_store(owner, p)`, `q_transfer(p)` and
`return p` never `Safe`, an owned binding overwritten never `Safe`, static,
formal and function-pointer destinations refused); `cg26_variadics.rs` and
`clang_gap_phase4.rs`'s K09 control flipped (no named refusal; `q_free(p)` an
unresolved alias, never `Safe`); unit tests
`valoop_lean_loop_graphs_are_decided_alike` (the Lean `whileLoop` and
`leakingLoop`, parsed from the Lean file, decided by `decide` before and after
the Rust lowering) and `valoop_the_va_arg_pointer_binding_is_the_lean_vector`.

Lean `QCore.VariadicConditions`: `lower` splits the protocol graph's head into
the event and a test node; `lower_sound` and `lower_complete` (every state the
lowered graph brings to an original node the protocol graph brings there, the
test node holding the head's post-event state, and conversely),
`lowered_no_violation` and `lowered_exit` (a certified protocol graph, lowered,
has no reachable violating node and only accepted exit states), the concrete
`while` loop certified and lowered, its master variant certified by no
assignment; `never_owned_is_never_released`, `never_owned_changes_no_custody`,
`never_owned_is_never_accused`, `never_owned_is_read_at_a_borrowed_position`,
`a_null_test_refines_only_the_binding`, and `an_owned_identity_is_released`
(non-vacuity). Premises stated, not proved: the AST reading (trust boundary
28) and the Rust lowering's correspondence to `lower` (review, pinned by the
unit test).

## 5. Follow-ups

Amended, not retired:

- `CG26-POINTER-VA-ARG`: the binding half is discharged (the derived binding,
  Lean, measured); the derived-summary half stays — a variadic definition's
  summary declaring its tail positions borrowed. Re-measured: the only
  corpus variadic definition called with a refused tail is `mk_print`
  (Unknown; 17 leaves in 9 functions), so the rule would move nothing at
  `66910c10`.
- `CG26-VA-PROTOCOL-PRECISION`: the condition form `(NAME = va_arg(L, T))` is
  lowered (rule 12c); the rest re-measured unchanged: `do { } while (0)` (2
  leaves, `rd_kafka_mock_handle_PushTelemetry_decode_error`), inner-block lists
  (`rd_kafka_error_new_v` 2, `rd_http_error_new` 4 uncontracted), events under
  `?:`/`&&` (`rd_kafka_producev` 6 + the named condition), `va_arg` inside
  another expression (3 uncontracted leaves).

No follow-up registered or retired; the ledger stays at 693.

## 6. Gate

See [gate/](gate/) and [gate.json](gate.json).
