# C02-P — borrowed pointer-field reads as contracted-call actuals — 2026-09-08

The card as briefed asked for the **implementation** of the pointer-field
extension C02 §7 named and O04 needed. The first measurement of this worktree
found that the implementation had already landed, inside card O04's increment,
as `crates/qcore-c-gateway/src/borrowed_actual.rs`
([O04 §2.1](../remaining-o04/README.md)). This card therefore did **not**
re-implement it. It did three things instead, and the boundary between them
matters to the integrator:

1. **Adversarially re-verified** the landed admission against every refusal the
   card names, including the released base the C00 lesson is about. **No
   soundness failure was found, so no production Rust file was changed by this
   increment.** The one boundary the measurement did surface is a property of
   the whole resource projection, not of this admission, and is recorded in §6
   with its control.
2. **Supplied the evidence O04 did not**: an independent held-out acceptance
   family for the admission itself (13 integration tests), and the first
   unit-level tests of the untracked-base closure (23 module tests that need no
   Clang).
3. **Proved the premise that was previously assumed.** `QCore.ExitResiduals`
   models the whitelist but takes `Access.baseBound` as a supplied flag. The new
   `QCore.BorrowedActual` (25 audited theorems) models the closure that
   *produces* that flag — the premise C02 §2 had to add — including its
   flow-insensitivity, its monotonicity and both precision directions.

**Nothing is promoted.** No verdict, diagnostic code, contract corpus count, CT
mapping or schema changed. No gate was weakened, no pinned expectation deleted,
no diagnostic downgraded.

## 1. What the admission is

`crates/qcore-c-gateway/src/borrowed_actual.rs`. Two shapes, both **reads**:

* a **borrowed actual** — a whole argument of a contracted call that is exactly
  `base -> field` or `base . field`, at an argument index the contract in force
  declares `Borrowed`;
* an **opaque field read** — an assignment whose entire right-hand side is one
  such access and whose destination is a plain local name. It binds
  `LegacyInstruction::OpaqueValue`: an unknown, possibly-null pointer, so every
  downstream use stays fail-closed.

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | `base` is a direct identifier whose declared type is a single-star pointer to the record (`->`) or the record itself (`.`) | computed, parenthesized or chained bases |
| 2 | the record is a `struct`, defined exactly once, every member declaration read; the member has no array modifier, no `volatile`/`_Atomic`, and is a scalar or a pointer | union, ambiguous tag, incomplete record, aggregate member |
| 3 | the access does not continue into `->`, `.`, `[` or `(` | `a->b->c`, `a->b[i]` |
| 4 | the base carries no tracked identity (`untracked_bases`) | see §3 |
| 5 | the actual occupies a declared `Borrowed` position of the contract in force; a variadic tail never qualifies | consuming/releasing/owning/out/stored positions, uncontracted callees |

The **destination** side of the lexical gate is untouched: a store through `->`,
`.` or `[` keeps its existing rejection and its existing diagnostic. That is
what makes premise 4 hold by construction — an admitted place can never be
bound.

The classification is a pure function of the certificate's own content: it is
read from the declarations of the same preprocessed token stream that
`analyze_legacy_c_translation_unit_selected_partial` receives, so W4 derivation,
W4 validation and `check_legacy_safety_certificate` all repeat it without any
extra environment. Both directions are pinned in §4.

## 2. Admitted / refused matrix — measured, not asserted

All rows measured through the public gateway entry points on a held-out `b_*`
API deliberately unrelated to ChunkIO, Fluent Bit and O04's `g_*` family
(`crates/qcore-c-gateway/tests/borrowed_field_actuals.rs`,
`crates/qcore-c-gateway/tests/fixtures/contracts/c02p_generic.qcore`).

### 2.1 Admitted

| Shape | Result |
|---|---|
| `storage = in->storage; chunk = b_open(storage->cio, storage->stream, n);` with two cleanup arms — the `flb_input_chunk_create` → `cio_chunk_open` shape | **Safe** certificate, zero diagnostics, every exit `Discharged`, two arms `Released` |
| the same through a record-valued base (`local.cio`, `local.stream`) | **Safe** certificate, zero diagnostics |
| a scalar member at a borrowed position | admitted (module test) |
| the same fixture with one cleanup arm removed | **Unsafe**, `Leak` — the admission is a precision gain in the defect direction too |
| a callee summarizable **only** because of the admission | `InferredValidated` record, arity preserved, re-derived by the recheck |

### 2.2 Refused — each keeps its existing `Unknown` **and** its existing
`indirect memory access is outside the resource projection` diagnostic

| Shape | Premise |
|---|---|
| the base was released (`b_release(storage)`) earlier or later in the body | 4 |
| the base was consumed through a direct lexical alias | 4 |
| the same field place was read into a second local that is released | 4 |
| the base is reassigned from an uncontracted call anywhere in the body | 4 (flow-insensitive) |
| a store through the field between the read and the call | destination side of the gate |
| the callee has no contract in force | 5 |
| a consuming position | 5 |
| a releasing position | 5 |
| an actual beyond the declared parameter vector (variadic tail) | 5 |
| a chained access `in->storage->cio` | 3 |
| a union base | 2 |
| an address-of escape `&storage->cio` | 1/3 |
| a cast operand | whole-argument |
| a partial argument `storage->state + n` | whole-argument |
| a parenthesized actual `(storage->cio)` | whole-argument |

Module-level rows additionally pin: address-taken base, incremented base,
compound-assigned base, base assigned from an opaque expression, transitive
closure over assignments, an unreadable declaration refusing the whole function,
and — in the precision direction — a base assigned from an admitted field read
staying admissible.

## 3. Premise 4, and the C00 lesson

`untracked_bases` over-approximates the names of one body that may bind a
tracked resource identity: the protected formals, plus every name that is
address-taken whole, incremented, compound-assigned, passed at a non-`Borrowed`
position of a contracted call, passed at *any* position of an **uncontracted**
call, or assigned from anything but an admitted field read or a call-free,
`&`-free expression over names that already survive; closed over assignments to
a fixed point. Refusing too many names only narrows the whitelist.

This is C02 §2's premise 8, strengthened: a borrowed actual survives into the
modeled call rather than disappearing into a no-op. The pinned expectation
`legacy_c::unmodeled_memory_reads_do_not_become_neutral_effects` is unchanged
and still passes; it was not weakened, and the two integration rows
"released base" and "second read of a released field place" pin the same
behavior for the actual-argument form.

## 4. Independent rederivation

| Substitution (statement bytes unchanged) | Result |
|---|---|
| the record becomes a `union` | recheck **refuses** |
| the read member becomes `volatile` | recheck **refuses** |
| the read member becomes an array | recheck **refuses** |
| the record body gains a member the reader cannot read | recheck **refuses** |
| the carried contract's `Borrowed` mode becomes `Consumed` | recheck **refuses** |

The unmodified certificate is re-checked at the end of both tests, so each
refusal is caused by its substitution and not by an unrelated failure. The
contract-mode row is new here: it pins that the admission rests on the declared
mode carried in the certificate, not only on the declarations.

## 5. Lean inventory

New module `proof/QCore/BorrowedActual.lean`, namespace `QCore.BorrowedActual`,
imported by `proof/QCore.lean` and audited from `proof/AuditW4.lean` (the audit
count stays 34; `scripts/check_lean_audits.py` globs `Audit*.lean`).
**25 theorems, no new axiom, no placeholder, no `sorry`** — every one depends
only on `propext` (one also on `Quot.sound`).

It models the *producer* of the `baseBound` premise that
`QCore.ExitResiduals.admitsActual` consumes, which was previously assumed:

* refusal by use — `a_protected_formal_is_refused`,
  `an_address_taken_name_is_refused`, `a_mutated_name_is_refused`,
  `an_owned_argument_is_refused`, `a_consumed_argument_is_refused`,
  `a_released_argument_is_refused`, `an_out_argument_is_refused`,
  `a_stored_argument_is_refused`,
  `an_opaque_assignment_refuses_its_destination`;
* the closure — `the_refusal_closes_over_assignments`,
  `the_refusal_closes_transitively`;
* flow-insensitivity and monotonicity — `refusal_depends_only_on_membership`,
  `the_refusal_ignores_statement_order`, `adding_a_fact_only_refuses_more`,
  `protecting_more_formals_only_refuses_more`;
* non-triviality — `seeded_is_mentioned`,
  `an_unmentioned_name_is_not_refused`, `an_empty_body_refuses_nothing`,
  `a_borrowed_argument_alone_does_not_refuse_its_name`,
  `an_admitted_read_alone_does_not_refuse_its_destination`;
* what the closure buys — `a_refused_base_is_never_admitted`,
  `an_admitted_actual_has_an_unrefused_base`,
  `an_actual_through_a_released_base_stays_indeterminate`,
  `a_statement_with_a_refused_base_is_refused`,
  `the_admission_survives_the_closure` (non-vacuity: without it every refusal
  above would hold for an empty admitted set).

The Lean model is a model of the **decision**. It does not prove that the Rust
reader extracts these facts faithfully from C source — that obligation is
stated as `Faithful` and assumed — and it is not a claim about arbitrary C
memory safety.

The pre-existing 20 admission theorems in `QCore.ExitResiduals` (union,
incomplete record, ambiguous tag, aggregate member, qualified member,
mismatched base, bound base, chained access, partial argument, non-borrowed
position, beyond-vector, uncontracted callee, non-vacuity, and the custody
consequences) are unchanged and still audited.

## 6. Open boundaries

* **May-alias between two distinct declared names is out of scope, and is the
  projection's boundary rather than this admission's.** Measured:
  `void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }`
  — no field access anywhere, pre-existing behavior — is **Safe**; the
  field-read form
  (`b_release(dup); storage = in->storage; b_open(storage->cio, …)`) is
  **Safe** for the same reason. The two are pinned to answer *identically* by
  `the_may_alias_boundary_answers_the_same_with_and_without_a_field_read`, so
  a later increment that relates the two names moves both rows together. This
  is recorded, not endorsed: the projection has never related two names that
  may denote the same object, and a released **formal** base, a released
  **tracked** base and a released lexical **alias** are each refused today.
* **Branch, loop, return and switch conditions** remain out of scope, exactly as
  in C02 §7. A pointer-field read in a condition keeps its `Unknown`.
* **Nested calls in sibling arguments** are pre-existing behavior, not opened by
  this admission: the same body with plain-name actuals answers the same. No
  span of a nested call is ever excluded from the lexical gate.
* **The declared premises are inherited unchanged and unproved**: declaration
  completeness, C effective types, and `&base->member` not reassigning or
  releasing `base` (recorded in the module header).
* **The pinned 185-unit corpus was not re-measured.** The developer's Fluent Bit
  checkout is on branch `cosmo0920-implement-batching-io_write` at
  `ef0cb0b2e423ad492ecd2645dbe52cd4286cc14b`, off the audited pin
  `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, so the audit's own pin gate
  refuses. CORPUS-REPIN remains open and integrator-owned. **No production Rust
  changed in this increment**, so no verdict can move by construction; that is
  an argument, not a measurement.
* **`docs/reviews/r0_3/acceptance_matrix.json`, `AGENTS.md`, `CLAUDE.md`,
  `README.md`, `DEVELOPMENT.md` and `docs/reevaluation/ct_manifest.json` were
  deliberately not touched** — the integrator consolidates those.

## 7. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | **tests only** — a `#[cfg(test)] mod tests` with 23 unit tests of the closure and the span computation. No production line changed. |
| `crates/qcore-c-gateway/tests/borrowed_field_actuals.rs` | new — 13 held-out acceptance tests |
| `crates/qcore-c-gateway/tests/fixtures/contracts/c02p_generic.qcore` | new — the `b_*` declared test assumptions, deliberately outside `contract/qcore` so the corpus count stays 60 |
| `proof/QCore/BorrowedActual.lean` | new — the untracked-base closure model, 25 theorems |
| `proof/QCore.lean`, `proof/AuditW4.lean` | import and axiom audit |
| `docs/reports/remaining-c02-p/README.md` | this record |

## 8. Gate results

Branch `claude/next-implementation-waves-050ed5` in an isolated worktree.
Baseline measured on this same checkout before any change.

| Gate | Command | Baseline | After | Exit |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **1473 passed / 0 failed / 1 ignored** | **1509 passed / 0 failed / 1 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | — | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | — | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | — | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | — | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | — | **58 passed / 0 failed** | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | — | **95 ran, OK** | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | 34 | **34 audits passed** | 0 |
| Pinned 185-unit corpus | `cycle19_fluent_bit_libraries_audit --pinned-evidence` | not run | **not re-measurable** (§6) | — |

The card adds **36** tests: 23 module + 13 integration. The submodule had to be
initialized once in this worktree
(`git submodule update --init --recursive`); before that,
`libevent_submodule_matches_the_audited_revision` failed for absence, which is
the pin gate working, not a defect.

## 9. Scope and non-claims

* **No raw unit is promoted**; no unit verdict, function status or diagnostic
  changed anywhere, because no production line changed.
* `Safe` in the `b_*` family is `Safe` for a synthetic declared custody
  projection over trusted test assumptions. The fixtures are declared test
  assumptions, not verified facts about any library.
* Projection coverage is not safety coverage; unknowns stay in the denominator.
* CT coverage is unchanged at 20/50 mapped; the contract corpus remains 60
  modules; `qcore-schemas` is unchanged at 5.0.0.
