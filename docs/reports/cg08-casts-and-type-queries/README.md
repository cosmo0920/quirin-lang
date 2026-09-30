# CG08-CASTS-AND-TYPE-QUERIES: casts as representations and type-query operands

Card CG08 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 5; base `ca634dc1` = local `main` at dispatch, which contains
the round-5 base `3e69322c` and W48-QR-C-STRUCT-IMPORT). MEASUREMENT-BASE-NOTE

- **Gap entries:** E15, E16, N11, M18 and N08 (representation only), on
  route **G** (the custody gateway) and, for N11, route **T**.
- **Evidence:** Q11, Q16, Q22, Q23, Q29, Q37, Q40 and example D.
- **Rule:** [gateway 11b](../../agents/gateway.md#constraint-11b) (opens the
  11 series; amends 5a, 10s and 10u, cross-referenced there).
- **Lean:** `QCore.CastRepresentation`, `proof/AuditCastRepresentation.lean`
  (19 theorems); `QCore.ElaborationCore`'s form table gains nine rows.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit
  `24328291`) before any implementation commit.

## 1. What changed

### 1.1 The cast table and the type-query classes (`cast_representation.rs`)

`CAST_TABLE` lists the 17 Clang cast kinds the route reads, each with a
preservation flag, a provenance flag and an admission. No row has a custody
effect; what a cast can do is hide an identity (a pointer to an integer) or
invent one (an integer to a pointer), and the admissions forbid both.

| Kind | Preservation | Provenance | Admission in the elaborator |
|---|---|---|---|
| `IntegralCast`, `IntegralToBoolean`, `FloatingToBoolean`, `IntegralToFloating`, `FloatingToIntegral`, `FloatingCast` | possibly lossy | n/a | scalar value, `NoNumericClaim` (numbers are CG20's) |
| `PointerToBoolean` | possibly lossy | lost | the null test: the pointer's read, a truth value |
| `PointerToIntegral` | possibly lossy | lost | data only of a never-bound automatic pointer; otherwise `PointerEscape` refuses |
| `IntegralToPointer` | possibly lossy | n/a | a pointer value with **no** identity |
| `BitCast`, `NoOp` (object pointers) | preserving | kept | the same pointer value |
| `NullToPointer` | preserving | n/a | the null pointer |
| `ToVoid` | preserving | n/a | no value |
| `AtomicToNonAtomic`, `NonAtomicToAtomic` | preserving | kept | refused (CG19) |
| `AddressSpaceConversion` | preserving | kept | refused (CG08: no address-space model) |
| `ArrayToPointerDecay` | preserving | kept | refused (an address; CG16) |

Any other kind is refused by name. `query_operand` reads a
`UnaryExprOrTypeTraitExpr`'s operand from Clang's node: not variably modified
→ **unevaluated**, no event (`sizeof(*p)`, `sizeof p->f`, `sizeof f()`); a
`sizeof` of a variably modified type → **evaluated**, the size expressions
Clang lists, each once, reads only; `_Alignof` of a variably modified type,
an evaluated expression operand other than a plain name, and other traits →
refused. `clang_constant` is Clang's own evaluation of an integer constant
expression (the value its evaluator gives a probe enumerator).

### 1.2 Route G (`legacy_cast_representation.rs`, thin hooks)

| Hook | Where | What |
|---|---|---|
| H1 | `return`, **before** the older routes | a value Clang converts with `PointerToBoolean` is the elaborator's (events, then an exit that escapes nothing); else, only a never-bound root keeps the older answer; else refused |
| H3 | statement, **before** the older routes | the same for a declaration or assignment: a truth value binds nothing |
| H5 | CG02's hooks (statement, `return`, call-free condition) | also consulted for an explicit cast, a `GATE_KINDS` conversion or a type query; a parenthesised type-query operand is no call to the call-free premise |
| H4 | statement, after the elaborator | `T *NAME = (U *) V;` / `NAME = (U *) V;` with object-pointer `BitCast`/`NoOp` casts is lowered as the uncast statement by the older routes |
| H2 | the W44 cast return's pre-card arm | `return (INTEGER) NAME;` with Clang's `PointerToIntegral` of a possibly bound `NAME` is refused (the W44 refusal) |
| H6 | the lexical gate of an ordinary call statement | a parenthesised `sizeof ( … )` whose every Clang node on its line is unevaluated joins W44's operand spans (the marker) |

H1 and H3 are asked before the older routes because the older routes'
premise (the value is the pointer) is false for them; every other hook is a
fallback (hard invariant 3). CG03's data reading (`call_instantiation.rs`)
takes `FloatingToBoolean` and the null test of a named pointer object.

### 1.3 Route T (N11)

`typed_source.rs::enum_constants` read an enumerator's value only from an
explicit initializer's evaluated `ConstantExpr` and refused an implicit one.
An implicit enumerator's value is now `clang_constant` (Clang's evaluation,
never the host's count from its predecessor); one Clang cannot evaluate at
file scope refuses. Every other T consumer is unchanged; the Monkey binding's
`SizeOf` host table is a residual (§6).

### 1.4 Shared items added (for the integrator)

All additive; no existing variant's meaning changes.

- `elaboration.rs`: `Refusal::PointerEscape`; `Context::untracked_pointer`
  (default `false`); `Elaborator::representation`, `Elaborator::type_query`;
  `pointer_operand`; nine `FORM_TABLE` rows (`representation cast`, `pointer
  null test`, `pointer to integer`, `integer to pointer`, `pointer
  representation`, `null pointer constant`, `discarded value`, `unevaluated
  type query`, `evaluated type query`; 29 → 38). The `CStyleCastExpr`,
  `UnaryExprOrTypeTraitExpr` and implicit-cast arms consult the new methods
  only where they refused before.
- `legacy_elaboration.rs`: `holds_value_form` also names a CG08 form;
  `GContext::untracked_pointer`; the call-free premise of conditions reads
  type-query operands as no call.
- `call_instantiation.rs`: two data-reading cast rows.
- `typed_source.rs`: the implicit enumerator arm.
- Nothing in `typed_place.rs`.

## 2. The soundness repair (found reproducing E15)

On base `ca634dc1` (fixture contracts, `tests/cg08_casts_and_type_queries.rs`):

| Fixture | BEFORE | AFTER |
|---|---|---|
| `bool f(void) { void *p = q_alloc(); return p; }` | **Safe** (the return escaped `p`) | Unsafe, leak |
| `bool f(void) { void *p = q_alloc(); bool b = p; return b; }` | **Safe** (`b` bound to `p`'s identity) | Unsafe, leak |
| `bool b; b = p; return b;` | **Safe** | Unsafe, leak |
| `bool b = p; q_free(p); return b;` | Unsafe (false custody conflict) | Safe |
| `return (bool)p;`, `return p != 0;`, `return !p;` | Unsafe, leak | unchanged |

C converts the pointer to `_Bool` (C11 6.3.1.2): the caller or the
destination receives a truth value, never the pointer. **No corpus function
moved** because of H1/H3 (no Safe → non-Safe in any run); the defect had no
corpus instance on the arbiter's inputs.

## 3. The cited claims

| Claim | Reproduced? |
|---|---|
| Q11: T reads six cast kinds plus `LValueToRValue` | yes (CG00's E15/M18/N08 T controls stand unchanged) |
| Q16/Q37: the typed readers refuse casts inside paths | yes; unchanged (a path cast stays CG10's) |
| Q22/Q23, example D: `1` is a class marker, not a size | yes; no route reads a number from it, pinned by `the_marker_never_reaches_a_bound_or_a_branch` |
| Q29: the contract schema carries no pointer/integer provenance | yes; the representation lives in the elaborator only |
| Q40: G's cast return | yes; the uncommitted-operand gap reproduced (`void *p … return (unsigned long)p;` was provisional Unsafe, a leak) |
| CG02: 165 consultations refused by a cast kind, 67 the nanopb `IntegralToBoolean` | **partly**: on this base the elaborator's cast refusals were 511 rows (189 pointer `NoOp`, 177 `IntegralToBoolean`, 95 `BitCast`, …, before de-duplication); the 67 nanopb returns reproduce exactly (67 returns, 32 functions) |
| CG00's 34 candidates | 2 already Safe on this base (freed by earlier cards) |

Corrections to the input: none beyond CG00's recorded E16 partial correction.
M18's "G restricts cast-return forms" was true in the accusing direction only:
an uncommitted operand's integer return was a leak, not a refusal (now
refused).

## 4. Measurements

MEASUREMENT-SECTION

## 5. Lean

`QCore.CastRepresentation` (`proof/QCore/CastRepresentation.lean`, audited by
`proof/AuditCastRepresentation.lean`, no placeholder, no project axiom):

- `kinds_unique`, `provenance_rows` (only a pointer representation keeps
  provenance; every lost row is a null test or a pointer-to-integer);
- `cast_has_no_custody_effect`; `tracked_pointer_to_integer_refused`,
  `untracked_pointer_to_integer_is_data`; `integer_to_pointer_has_no_identity`;
  `pointer_to_boolean_is_the_null_test`; `pointer_value_keeps_identity`;
  `unlisted_cast_refused`, `refused_row_refused`;
- `constant_type_unevaluated`, `alignof_never_evaluated`,
  `unevaluated_operand_has_no_events`, `evaluated_operand_once`,
  `evaluated_effect_refused`;
- `consumed_value_is_clangs`, `marker_never_consumed`,
  `constant_vectors_consumed`;
- `non_typedef_non_tag_never_folds` (`SIZEOF-TYPE-FOLD-NO-LEAN`).

Bound to Rust by shared vectors: `castTable` = `CAST_TABLE`
(`cg08_lean_cast_table_matches_the_rust_table`); `queryVectors` through Clang
and `query_operand`; `constantVectors` against `clang_constant` (x86-64
Linux); `foldVectors` against `fold_type_sizes`
(`cg08_lean_fold_vectors_match_fold_type_sizes`). `ElaborationCore.formTable`
= `FORM_TABLE` (38 rows) by CG02's test.

## 6. Tests, flips and residuals

New: `tests/cg08_casts_and_type_queries.rs` (15 tests: the repair, the
integer round trip refused never Safe, `sizeof(int[f()])` refused, the marker
never in a bound or a branch, the uncast statements, the unevaluated
operands, T's implicit enumerator, the nanopb shape); unit tests in
`tests/unit/cast_representation.rs` and `tests/unit/elaboration.rs`
(`cg08_casts_and_type_queries_are_elaborated_by_their_rows`).

Deliberate flips of pinned refusals (each test names the card):

| Test | Was | Now |
|---|---|---|
| `clang_gap_phase1::e16_type_queries_are_read_only_in_fixed_shapes` | `sizeof(*s)` initializer IND, return refused, `sizeof(int[n])` IND | Safe (unevaluated; VLA extent read once); `sizeof(int[n++])` added, Unknown |
| `clang_gap_phase3::m12_variable_length_arrays_are_refused` | `sizeof(int[n])` IND | Safe (the VLA declaration refusal, CG17's, stands) |
| `branch_condition_calls::a_call_inside_sizeof_stays_refused…` → `…_is_never_applied…` | branch refusal | Safe, no `q_probe` node |
| `neutral_expression_positions::type_queries_do_not_hide_evaluated_calls_or_pointer_results` | 5 Unknown | 2 Unknown kept; `sizeof(q_alloc())`, `sizeof(*p)`, `sizeof(void *)` Safe |
| `neutral_expression_positions::expression_queries_do_not_erase_scalar_mutations` | branch refusal | a refusal **or** the leak (the mutation is forgotten, never the stale zero) |
| `sizeof_operand` (6 tests of `W44-SIZEOF-OPERAND-RESIDUALS`) | refused | Safe (array declarator, file-scope name, member operand, leading multiplication, no-call statement, type-operator-first statement); the two variably modified operands keep their refusal |
| `typed_field_access::ambiguous_accesses…` | `d->revision = (long) h;` refused | Safe (never-bound formal); `(long) p` of a tracked `p` added, Unknown |
| `legacy_condition_acquisition::every_refusal_arm_is_pinned` | fixture `sizeof(buf)` | fixture `sizeof(char[n++])` (the arm still pinned) |

CG00's E15, M18 and N08 controls pass unchanged: T is unchanged and G's
pointer↔integer assignments of a tracked pointer still refuse; N11's T control
(`sizeof(int)` stays a `SizeOf` node) is unchanged.

RESIDUALS-SECTION

## 7. Gate

GATE-SECTION
