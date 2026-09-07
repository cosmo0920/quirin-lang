# C02 — Typed direct scalar-field access classification — 2026-09-07

Implementation of the blocker family C00 ranked first,
`indirect_member_or_deref`, as the bounded capability C00 recommended: **typed
direct scalar-field access classification**.

The resource projection's lexical gate (`contains_unmodeled_access`,
`legacy_c.rs`) rejects any statement whose *tokens* carry `->`, a non-numeric
`.`, `[`, `]` or a unary `*`. It carries no declared type, so it cannot separate
a scalar field store from a pointer store, a union write or an address escape,
and every one of them poisons the enclosing function. This card supplies the
missing typed premise and admits one conjunctive whitelist of accesses as
resource-projection no-ops. Everything outside the whitelist keeps the lexical
rejection and its existing diagnostic, unchanged.

**No verdict is promoted.** The CMetrics raw unit remains **provisional
Unsafe**, unpromoted; the source-contract layer is unchanged at one checked
function contract and six Unknown; CT coverage stays 11/50 mapped; the contract
corpus stays 59 modules. No gate was weakened, no pinned expectation deleted, no
diagnostic downgraded.

## 1. What is admitted

`crates/qcore-c-gateway/src/typed_field_access.rs` reads the record, typedef and
enumeration declarations of the same preprocessed translation unit the importer
is already projecting, then decides one **conjunctive** whitelist. A statement
is a custody-neutral no-op only when all of these hold:

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | no call and no braces in the statement | call keeps its own rules |
| 2 | no `&`, `*`, `[` or `]` token anywhere | address-of, deref, subscript |
| 3 | every access is exactly `base -> field` / `base . field`, `base` a direct identifier whose **declared** type is a single-star pointer to the record (for `->`) or the record itself (for `.`) | computed or chained bases |
| 4 | the record is a `struct`, is defined **exactly once** in the unit, and **every** member declaration was read | union tag, ambiguous tag, bitfield, nested anonymous aggregate, function-pointer member |
| 5 | the field's **declared** type is a non-pointer, non-array arithmetic or enumeration type with neither `volatile` nor `_Atomic` | pointer field, array field, aggregate field, qualified field |
| 6 | the access does not continue into `.`, `->`, `[` or `(` | `a->b->c`, `a->b.c`, `a->b[i]` |
| 7 | every remaining identifier is an enumeration constant or a variable of the same function with a declared **scalar** type | casts, type names, every pointer-valued operand |
| 8 | the base is a name this function never binds to a resource identity | see §2 |

Anything else stays **Unknown** with its existing diagnostic.

**Purely type-based.** No library, record, field or function name appears in any
rule. The classification is decided from declared C types and expression shape
alone, and the same code runs on the held-out fixtures and on CMetrics.

### Fail-closed granularity

Two deliberate all-or-nothing rules make the reader's own limits safe:

- **Whole record.** A record body carrying anything the reader cannot resolve —
  a bitfield, a nested anonymous struct or union, a function-pointer member —
  makes the **entire record** inadmissible, not just that member. Reading only
  the members it understands would risk attributing a scalar type to a name it
  had in fact misparsed.
- **Whole function.** A declaration in the function the reader cannot read
  refuses **every** access in that function. This is what lets premise (a) of
  §3 hold: a declaration the reader did not understand cannot silently be
  shadowed by one it did.

## 2. The premise the card added beyond C00's recommendation

C00's soundness sketch argued that a custody-neutral statement can be admitted
because "if the access traps, that path does not reach an exit". Implementing it
that way **failed a pinned expectation**, and the failure was real, not a test
artifact:

```
struct Cell { int field; };
int subject(void) { struct Cell *p = q_alloc(); q_free(p); p->field; return 0; }
```

`p->field` is custody-neutral by C00's argument, but it is a read through a
*released* pointer, and a use-after-free does not trap. Admitting it turned an
honest `Unknown` into `Safe`, which
`legacy_c.rs::unmodeled_memory_reads_do_not_become_neutral_effects` exists to
forbid.

**Premise 8** closes it: the base must be a name this function never binds to a
resource identity — not a protected formal, never assigned, incremented or
address-taken, and never appearing in any call's argument list. A base the
projection never binds holds no released, escaped or reallocated identity for
the classification to erase. The set is a syntactic **over-approximation**:
naming too many bases only rejects more.

The pinned test passes unchanged, and premise 8 is now itself pinned in both
directions (`a_base_the_projection_may_bind_is_refused` and
`a_comparison_does_not_bind_the_base`, so the common `if (p != NULL) p->f = 1;`
shape stays admissible).

## 3. Soundness argument and its premises

A read or write of a non-pointer arithmetic or enumeration member of a non-union
record can neither create, destroy, alias nor transfer a tracked resource
identity, so it is custody-neutral. Unions are excluded because a scalar store
can overwrite an overlapping pointer member; `volatile` and `_Atomic` because
they carry side effects the projection does not model; address-of because it
escapes a sub-object the projection cannot name; a bound base because of §2.

Two premises are assumed and stated, not proved:

- **(a) Declaration completeness.** The declarations read are the ones in force
  at the access. The reader refuses the whole function whenever it meets a
  declaration it cannot read, which is what makes this hold for the admitted
  set.
- **(b) C effective types.** Storage declared as a record is not accessed
  through an incompatible record type whose overlapping member is a pointer.

Both are recorded in the module header.

## 4. Where the premise comes from — a recorded deviation

C00 recommended reading the premise from "the Clang typed AST (already imported
for the same unit)". This implementation reads it from the **declarations of the
same preprocessed token stream** instead. The reason is architectural and
load-bearing:

`analyze_legacy_c_translation_unit_selected_partial` — the single function that
both the resource projection and `check_legacy_safety_certificate` run — takes
only the preprocessed source, the entry name and the declared contracts. The
certificate holds exactly that. Threading a Clang round trip into it would make
W4 summary derivation, W4 validation and certificate rederivation
environment-dependent, would require a Clang AST dump per function on every
analysis and re-analysis, and would change the public pure-token API
`analyze_legacy_c_translation_unit`.

Reading declarations from the same token stream keeps the classification a
**pure function of the certificate's own content**, so acceptance item 2 holds
by construction rather than by an extra invocation. The premise is no less
typed: it is read from C declarations, and `typed_field_access.rs` commits to a
type only when it can resolve the whole declaration.

The cost of the deviation is that the reader is a trusted hand-written
declaration reader rather than Clang's parser. It is mitigated by the
all-or-nothing granularity of §1 and by 25 module tests that pin each refusal.

## 5. Acceptance

### 5.1 Positive — a scalar-field write and read stop poisoning

`crates/qcore-c-gateway/tests/typed_field_access.rs`, a held-out **generic**
fixture family (a text-editor-shaped `struct doc` and neighbours, deliberately
unlike the CMetrics unit):

| Test | Body | Result |
|---|---|---|
| `scalar_field_stores_and_reads_no_longer_poison_an_owned_lifetime` | `q_alloc`; `d->cursor = 1`; `d->revision = d->revision + 1`; `d->dirty = d->cursor`; `q_free` | **Safe** certificate, zero diagnostics |
| `a_record_valued_local_and_a_compound_assignment_are_also_admitted` | `struct doc local;` with `local.cursor = 0`, `local.cursor += 2`, `local.dirty = local.cursor` | **Safe** certificate |

Before this card each of those statements was `unsupported`, leaving the whole
function `Unknown` and its `q_alloc` obligation unexamined.

### 5.2 Negative — real violations now surface

| Test | Body | Now reported |
|---|---|---|
| `a_leak_that_hid_behind_a_scalar_field_store_is_now_reported` | `q_alloc`; `d->cursor = 1`; no release | **`Leak`**, and asserted to carry *no* `Unsupported` diagnostic |
| `a_double_release_that_hid_behind_a_scalar_field_store_is_now_reported` | `q_alloc`; `q_free`; `d->dirty = 1`; `q_free` | **`DoubleRelease`** |
| `a_release_after_transfer_that_hid_behind_a_scalar_field_read_is_now_reported` | `q_alloc`; `q_transfer`; `d->cursor = d->revision`; `q_free` | **`ReleaseAfterEscape`** |
| `a_classified_wrapper_that_stops_releasing_loses_its_caller_certificate` | callee touches a scalar field and stops releasing | caller's **`Leak`** |

These are precision gains in the *defect* direction. A provisional `Unsafe` on a
synthetic fixture is not an upstream defect claim.

### 5.3 Ambiguous — everything else keeps its Unknown

`ambiguous_accesses_keep_their_existing_unknown_and_diagnostic` runs twelve
bodies that differ from the admitted case in exactly one premise, and asserts
each retains the existing
`indirect memory access is outside the resource projection` diagnostic:

pointer field · union base · record containing a union · record containing a
bitfield · record containing a function pointer · volatile field · array field ·
address-of escape · chained access · subscripted access · cast operand ·
call-bearing statement.

Two more pin the fail-closed granularity:
`an_unreadable_declaration_refuses_every_access_in_the_same_function` (a
function-pointer local) and
`a_shadowing_redeclaration_of_a_different_record_refuses_the_name`.

### 5.4 Independent rederivation

`a_certificate_resting_on_the_classification_is_rederived_from_source`
substitutes only the *declaration* the classification rests on, leaving the
statement byte-identical, in four ways — the field becomes a pointer, the record
becomes a union, the field becomes `volatile`, the record gains a bitfield — and
each is refused with the measured reason
`inferred custody summaries do not match the preprocessed source`. The
unmodified certificate is re-checked at the end of the test, so the four
refusals are caused by the substitution and not by an unrelated failure.

`a_custody_summary_derived_through_a_classified_body_is_validated_and_rederived`
covers acceptance item 2's second half: a callee is summarizable **only**
because its scalar-field store is classified; the published record is
`InferredValidated`, its formal arity survives, the released formal is
`Released` and the record formal stays `Borrowed`, and
`check_legacy_safety_certificate` re-derives the whole vector.

### 5.5 CMetrics — the real unit

Canonical audit, unchanged configuration, reading the shared prepared build at
`build/cycle19/fluent-bit-clang-{source,production}` and writing its report to
the session scratchpad, so nothing in that tree was modified. Archived reports:
[`cmetrics-baseline.json`](cmetrics-baseline.json),
[`cmetrics-after.json`](cmetrics-after.json); machine diff in
[`logs/cmetrics_report_delta.txt`](logs/cmetrics_report_delta.txt).

```
cmetrics/lib/cmetrics/src/cmt_counter.c: Unsafe (7/7)
  resource diagnostics: leak=2, unknown_alias=2, unsupported=35   (was 36)
source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

**Per-function delta — the complete list.**

| Function | Resource status before | after | Diagnostics before → after | Cause |
|---|---|---|---|---|
| `cmt_counter_allow_reset` | Unknown | **Safe** | 1 → 0 | its whole body is `counter->allow_reset = 1;` (original line 85, preprocessed 5491). `counter` is a borrowed formal the projection never binds; `allow_reset` is `int` in a complete non-union `struct cmt_counter`. All eight premises hold, the statement is a no-op, the function is diagnostic-free |
| `cmt_counter_create` | Unsafe | Unsafe | 20 → 20 | unchanged. `counter->aggregation_type = 2;` (original 77) is scalar, but `counter` is assigned from `calloc` and passed to calls, so **premise 8 refuses it**. Both leaks, the uncontracted `cmt_log_print` / `cmt_map_create` / `cmt_counter_destroy` calls and `counter->cmt = cmt` (a pointer field) all remain |
| `cmt_counter_destroy` | Unknown | Unknown | 5 → 5 | every statement carries a call or `&counter->_head` |
| `cmt_counter_inc` | Unknown | Unknown | 3 → 3 | uncontracted callees; the member accesses are call-bearing |
| `cmt_counter_add` | Unknown | Unknown | 3 → 3 | as above |
| `cmt_counter_set` | Unknown | Unknown | 5 → 5 | as above, plus a branch condition mixing a call and `counter->allow_reset` (branch conditions are out of scope, §7) |
| `cmt_counter_get_val` | Unknown | Unknown | 3 → 3 | as above, plus the unsanctioned `*out_val` store |

**Unit-level delta:** exactly one diagnostic disappears; `leak`, `unknown_alias`
and every other count are unchanged; the unit verdict is unchanged.

**Source-contract layer:** unchanged — one checked function contract
(`cmt_counter_allow_reset`) and six Unknown. C00 predicted the six would stay
Unknown "due to their OTHER blockers"; **confirmed**. Five need a source-contract
ABI extension and four need cross-translation-unit callee contracts, neither of
which this card touches.

C00 additionally predicted `cmt_counter_create` might lose one `unsupported`.
It does not, because premise 8 — which C00's sketch did not contain — refuses a
`calloc`-bound base. That is the honest, more conservative outcome.

## 6. The 185-unit corpus — a deliberate exclusion

This change touches shared analysis code (`legacy_c.rs`), so unlike card G01 no
disjointness argument is available: the four archived 185-unit artifacts **would
change**, in both directions, and cannot be re-measured in this environment.

Measured, not assumed ([`logs/cycle19_pin_gate.log`](logs/cycle19_pin_gate.log)):

```
Fluent Bit pin check failed: pin `fluent-bit`:
  expected 66910c10a4d7eafa810b84229cf2dcf7b0f26f97,
  found    90a010910e5b3328f496cc20fadc937547133439
```

The developer's checkout has moved off the audited pin. The pin gate correctly
refuses to run. The off-pin sibling checkout was not touched, reset or altered.

Consequently:

- **The four archived artifacts are NOT edited.** Hard invariants 3 and 5 mean
  they are re-measured, diffed and re-pinned deliberately, not adjusted to fit.
- **Named follow-up: re-audit the 185-unit corpus at the pin and re-pin the four
  artifacts**, archiving the diff with per-unit causes. That follow-up is where
  the corpus-level precision effect of this card — including any *newly
  surfaced* provisional `Unsafe` units, which C00 warned about — must be
  measured. Until then this card claims no corpus effect at all.

## 7. What this card does not do

- **Branch, loop, return and switch conditions are out of scope.** They flow
  through `supported_neutral_expression`, which is untouched. That is C00's
  rank-3 family (`branch_condition`) and belongs to its own increment.
- **C02-P is out of scope.** Borrowed pointer-field reads — O04's prerequisite —
  are explicitly not implemented; premise 5 refuses every pointer field.
- **Compound assignment through an unreadable destination** (`p->f += 1` where
  the classification refuses) keeps its existing
  `unrecognized assignment destination` diagnostic; no new diagnostic text was
  introduced anywhere.
- **No unit is promoted**, no CT case is mapped, no contract corpus count moves,
  no schema changed, no public gateway type gained or lost a variant.
- The `q_*` fixtures are synthetic **declared test assumptions**, not verified
  facts about any library.

## 8. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | new — the reader, the whitelist, 25 module tests |
| `crates/qcore-c-gateway/src/lib.rs` | declare the private module |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `Instruction`-free: two `Parser` fields, `typed_access_facts`, `function_parameter_range`, one classification call in `classify_simple` |
| `crates/qcore-c-gateway/tests/typed_field_access.rs` | new — 11 public-API acceptance tests |
| `proof/QCore/LegacyCAnalyzer.lean` | `Instruction.noop` + its transfer case + 9 theorems |
| `proof/QCore/TypedFieldAccess.lean` | new — the classification model + 22 theorems |
| `proof/QCore.lean`, `proof/AuditW4.lean` | import and axiom audit |
| `crates/qcore-verification/tests/source_effect_mutations.rs` | merge repair, §10 |

## 9. Lean parity

The classification adds one semantic rule the flow model must reflect: a
classified statement is a **no-op** in the custody transfer. `Instruction.noop`
and its transfer case were added to `QCore.LegacyCAnalyzer`, and the whitelist
itself is modeled in the new `QCore.TypedFieldAccess`.

**31 theorems, no axioms beyond `propext`, no placeholders, no `sorry`.** All
are printed in `proof/AuditW4.lean` and checked by
`scripts/check_lean_audits.py`.

`QCore.LegacyCAnalyzer` (9): `noop_preserves_state`, `noop_preserves_precision`,
`noop_preserves_ledger`, `noop_preserves_diagnostics`,
`noop_does_not_hide_a_leak`, `noop_does_not_hide_a_double_release`,
`noop_does_not_hide_a_release_after_escape`,
`noop_between_acquire_and_release_is_safe`,
`unsupported_form_of_the_same_program_is_indeterminate`.

`QCore.TypedFieldAccess` (22): `admitted_access_is_neutral`,
`refused_access_keeps_the_unsupported_effect`,
`admitted_effect_preserves_custody`, `refused_effect_is_never_safe`; one
refusal theorem per premise (`a_union_never_admits`,
`an_incomplete_record_never_admits`, `an_ambiguous_tag_never_admits`,
`a_pointer_member_never_admits`, `an_aggregate_member_never_admits`,
`a_qualified_member_never_admits`, `a_mismatched_base_never_admits`,
`a_bound_base_never_admits`, `a_chained_access_never_admits`,
`an_escaping_statement_never_admits`,
`a_call_bearing_statement_never_admits`,
`a_nonscalar_operand_never_admits`, `unread_declarations_never_admit`);
`an_admitted_access_reads_no_tracked_identity` (premise 8, under the reader's
explicit faithfulness obligation);
`an_access_through_a_released_base_stays_indeterminate` (the behavior the
fail-closed pin protects); `an_admitted_access_does_not_hide_a_leak`;
`an_admitted_access_preserves_a_discharged_lifetime`; and
`the_whitelist_is_not_empty`, without which every refusal theorem above would
hold vacuously.

The Lean model is a model of the **classification decision**. It does not prove
that the Rust reader extracts declared types faithfully from C source, and it is
not a claim about arbitrary C memory safety.

## 10. One merge repair, recorded

The base of this branch merges E02 (`worktree-agent-a1811313fbc7a2c16`) and the
integration tip, which carries P02's `PluginLimits::additional` field. E02's
`crates/qcore-verification/tests/source_effect_mutations.rs` constructs
`PluginLimits` literally and no longer compiled. It now spreads
`..PluginLimits::default()`, which is the empty declared-kind map — the
serialization is byte-identical and no limit behavior changed. This is a
semantic merge conflict between two already-merged cards, not a C02 change.

## 11. Gate results

Branch `worktree-agent-a4559311447e1d28e`. Raw logs under [`logs/`](logs/).

Baseline was measured on this same checkout with the card's tests excluded, so
no separately-timed run is trusted:
`cargo test --workspace -- --skip typed_field_access` → **912 passed**, which is
901 pre-existing plus the 11 integration tests the skip filter does not name.
The card adds **36** tests (25 module + 11 integration).

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace` | **937 passed / 0 failed** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **47 passed / 0 failed** | 0 |
| Provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | 0 |
| Provider clippy | `cargo clippy --locked --offline --manifest-path providers/Cargo.toml --all-targets -- -D warnings` | clean | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **43 ran, OK** | 0 |
| Lean build | `cd proof && lake build` | **63 jobs, completed successfully** | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **20 audits passed** | 0 |
| CMetrics audit | `cargo run -p qcore-audit -- <cmetrics-counter audit.json>` | 1 checked / 6 Unknown; provisional Unsafe; unsupported 36 → 35 | 0 |
| Pinned 185-unit corpus | `cycle19_fluent_bit_libraries_audit --pinned-evidence` | **refused: pin gate** (§6) | 1 |

Provider tests are **47**, not the 46 recorded under card C01: P02's enforced
process limits added one on this base. No provider file was touched.

## 12. Scope and non-claims

- **No raw unit is promoted.** The CMetrics unit remains **provisional Unsafe**,
  unpromoted. A provisional `Unsafe` is never an upstream defect claim.
- The one function whose resource status moved (`cmt_counter_allow_reset`,
  Unknown → **Safe**) is a *resource-projection* status. It is a separate
  verdict from its already-checked source contract, and neither promotes the
  unit.
- Removing state poisoning surfaces defects as well as clearing them. The
  fixture family exercises both directions on purpose.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- No gate was weakened, no pinned expectation edited, no diagnostic downgraded,
  no `Unknown` flipped to `Safe` without a supplied typed premise that covers
  the reason it was Unknown.
- CT coverage is unchanged at **11/50 mapped**; the contract corpus remains 59
  modules; `qcore-schemas` is unchanged.
