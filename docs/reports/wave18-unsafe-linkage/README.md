# UNSAFE-LINKAGE — linking an external library that has no contract — 2026-09-10

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Card `UNSAFE-LINKAGE`, opening wave 18, user-directed:

> "We permit linking with unsafe or not-contracted contract or type system, to
> permit the linking of external libraries. It could be a reasonable and
> realistic approach to proceed in this direction."

**Base tree:** local main `862f049` (the complete wave-17 tree). Design
documents: [`docs/design/UNSAFE_LINKAGE_V1.md`](../../design/UNSAFE_LINKAGE_V1.md)
(this card) and
[`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`](../../design/NATIVE_LIBRARY_LINKAGE_V1.md)
(the contracted end of the same spectrum, adopted from a prior workflow with
recorded adjustments — §12).

---

## Headline

**The v1 landed.** An external library with no contract at all can now be
linked, from a source module, through `qcorec check` and `qcorec emit`, with
real C on the other side of the link — and every result it produces prints the
callee it rests on.

| | |
|---|---|
| What was added | `#import std.extern.unsafe;` gating an `unsafe extern fn NAME(SIG);` declaration (§2) |
| Admitted surface | **scalars by value in, a scalar or `void` out.** Every pointer, in every spelling and position, refuses by name — `ptr<const, u8>` included and deliberately (§3) |
| Assumption mechanism | `ExternAssumption::UncheckedLinkage { callee, signature }` on the **existing** declared-assumption channel, derived from the program's own callee inventory (§4) |
| Diagnostics | a new `UncheckedLinkage` pass owning `Q032`–`Q035`, through the §L21 process (§6) |
| Default | unmoved. Without the waiver an uncontracted callee still refuses at `Q023`, on the same fixture, byte for byte (§7) |
| New reserved words | **none.** `unsafe` was already reserved (§2.2) |
| Designed, not implemented | the **linkage spectrum** L0–L3 and the contract-less manifest entry (L1) that makes the premise name *which artifact* was waived — staged as `UNSAFE-LINKAGE-ARTIFACT` (§12) |
| Evidence | 28 new tests (one of them a fixed ASan/UBSan/LSan native run linking a real uncontracted C file), 26 audited Lean theorems in a new `QCore.UncheckedLinkage` namespace plus 7 in `QCore.ModuleImports`, no new axiom |

**Nothing is promoted.** A module using unchecked linkage is `Safe` **only
under its printed assumptions**, and §5 shows one such certificate with the
assumption visible in it. No raw Cycle 19 unit moves, no CT mapping changes, the
contract corpus is untouched (this card writes no contract text at all), and
full language conformance stays false.

---

## 1. What the request needed, and the one rule it is under

`source.extern-call.v1` admits a call only against a trusted C04 contract that
an explicit `CompileOptions::extern_contracts` selection supplied. An
uncontracted callee refuses at `Q023`, permanently. That is the right default
and it is not a linking story: real programs link libraries nobody has written a
contract for, and the choice today was "write a contract for every symbol you
touch, or do not link".

The third option this card adds is **link it, and say so**, under one rule:

> The escape hatch never buys silence. Every result obtained through an
> uncontracted callee carries that callee, by name and declared signature, as a
> printed premise.

That is not a relaxation of the verdict contract (hard invariant 2) — it is the
verdict contract applied. `Safe` under a named assumption, with the assumption
in the certificate, is what a declared trust boundary is *for*. What would
violate the invariant is a `Safe` that stops mentioning it, and §4 is three
independent reasons why that cannot happen.

## 2. The opt-in surface, and how the two candidate designs were decided

Two independent, file-local gates, both required:

```qcore
#import std.extern;          // calling a declared extern function at all
#import std.extern.unsafe;   // declaring one with no contract

unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;
```

### 2.1 Why not the builder-side selection

The brief left the choice open between a source form and "a CompileOptions-side
unsafe selection mirroring `extern_contracts`". The builder-side form is
strictly cheaper — it needs no parser change at all — and it was **rejected**:
it makes the trust invisible where it matters. Two byte-identical source files
would mean different things depending on how the builder invoked the compiler,
and a reader of the source could not tell which. The requirement is an
*explicit, recorded trust declaration*; a declaration that is not in the
declaration is not one.

Recording the cost honestly: this decision is what forced the parser change of
§2.2 and the driver restructuring of §7.1.

### 2.2 Measured: `unsafe` is already a reserved word

The reserved-word discipline expected a compatibility hazard here. There is
none. `unsafe` is already in `is_reserved_source_word` and already introduces
the `unsafe { … }` statement form (`StatementKind::Unsafe`), so this card mints
**no new reserved word** and has no golden-surface collision to resolve.

What it did need was one line in `is_declaration_start`, so an
`unsafe extern fn` declaration may follow a function body. That predicate is
consulted only at the token after a top-level brace group, where a statement
cannot begin, so the widening is strictly more accepting and never differently
accepting. It is pinned in both directions:
`an_unsafe_statement_inside_a_body_still_parses_as_it_did` (the statement form
still reaches `Q500` exactly as before) and `a_waiver_may_follow_a_function_body`.

The declaration keeps `DeclarationKind::ExternFunction`; the waiver is read back
from its own leading token. No elaboration, name-resolution or typed-AST path
changed shape.

### 2.3 The gate is on the declaration

`Q032` fires on an `unsafe extern fn` declaration in a file that did not import
the module, whether or not the file calls it. Declaring an uncontracted symbol
is itself the trust act: a file that links one and never calls it has still
linked it. A file that imports the waiver and never calls anything is an
ordinary sequential module and publishes no premise — no call, no trust
consumed. Both halves are pinned in
`the_gate_is_on_the_declaration_not_on_a_call_to_it`.

## 3. What may cross, and the argument for what may not

| Position | Admitted | Refused |
|---|---|---|
| Parameter | §C04.1 scalar by value: `bool`, `i8`…`i64`, `isize`, `u8`…`u64`, `usize` | `ptr<mut, u8>`, `ptr<const, u8>`, `ptr<mut, ptr<mut, u8>>`, `unit`, any spelling outside the ABI table |
| Result | §C04.1 scalar, or `unit` | every pointer spelling, any spelling outside the ABI table |

The refusals are named constructors of `UncheckedRefusal`, not an unhandled
case, so widening the profile later means deleting a refusal on purpose. All
eleven admitted scalars are exercised
(`every_admitted_scalar_crosses_and_the_table_is_not_vacuous`), so the refusals
are not vacuous.

### 3.1 Why pointers are refused, including read-only ones

The brief asked for the argument if `borrow<Buffer>`-style read exposure were
admitted, and said the expected conclusion was REFUSE. **It is REFUSE**, and the
reason is sharper than "conservative": for a pointer at this boundary *there is
no assumption text to write*.

Take the weakest form, a `borrow<Buffer>` lent as `ptr<const, u8>`:

1. **Extent.** C04 pointers are extentless `u8` projections; this profile
   transports no length. A *contracted* callee's over-read is a premise the
   declaration at least states. An uncontracted callee's over-read is not even
   stated — recording "assume `k` reads only what it should" records nothing,
   because there is no *should*.
2. **Retention after return.** `resource_borrowed` is a lexical loan and the C04
   contract is what makes it lexical. Remove the contract and the loan has no
   end: the caller's borrow ends at the call, the callee's does not. This is
   O00's B03/R13 publication non-model — a pointer escaping into a store nobody
   models is exactly the residual this repository refuses to reason about.
3. **`const` is not a guarantee.** It constrains the declaration this file
   wrote, not an implementation that may cast it away. Under a contract that is
   a declared premise about the implementation; with no contract there is no
   declaration to premise.
4. **Aliasing and threads.** The callee may publish the pointer to another
   thread; nothing in the caller's model would see it.

All four have the same shape, and it is the shape this card's whole discipline
turns on: **an assumption that cannot be stated cannot be carried.** So every
pointer is refused, in both directions, in every mode. `own<Buffer>`,
`resource_out` slots and every other C04 mode are refused a fortiori — they are
pointer modes, and each additionally needs a contract clause that does not exist
here.

`a_read_only_pointer_is_refused_deliberately_and_not_by_oversight` asserts the
refusal *message* names `const uint8_t *` and says "scalars by value only", so
the refusal cannot silently become incidental.

### 3.2 The consequence: no custody crosses

The admitted surface carries no custody at all. Derived custody is the empty
vector, the caller-side flow is the zero flow, and the interprocedural summary's
parameter list is empty. An unchecked call **cannot** leak, double-free or
invalidate an identity, because it is handed none. That is the entire soundness
argument for the v1 surface, and it is why the v1 surface is scalars. It is
pinned on the real certificate
(`an_uncontracted_library_links_with_no_contract_selection`) and proved in Lean
(`an_admitted_unchecked_callee_carries_no_custody`).

### 3.3 The derived effect row is maximal

An unchecked callee derives the whole modeled row — `{Io, Alloc, Mut}` —
unconditionally, on the stated ground that nothing is known about it: the row
cannot honestly be narrower than everything this profile can name. The practical
effect is that an entry calling one must declare `effects { io, alloc, mut }`,
so the opt-in is visible **interprocedurally, in the entry's own signature**,
not only at the declaration. All three proper sub-rows are shown to refuse, in
Rust and in Lean.

The fixture's C makes this concrete rather than theatrical:
`k_unchecked_side_effect` mutates a private global and writes to a stream, and
`k_unchecked_measure` allocates and frees behind the caller's back. The caller's
ledger is untouched in both cases — it was handed nothing — which is §3.2 in
executable form.

## 4. Trust is carried, never laundered

The premise is
`ExternAssumption::UncheckedLinkage { callee, signature }`, derived inside
`check_extern_program` from the program alone, **beside** `DeclaredContract`, on
the same list, in the same sort order. This is the existing declared-assumption
channel — the same one a trusted C04 contract travels on — not a new parallel
one, as the brief required.

The derivation is a **total match on the callee's trust**: a contracted callee
contributes `DeclaredContract`, an uncontracted one contributes
`UncheckedLinkage`, and there is no third arm that contributes nothing.

Three independent things make omission impossible:

1. **Structural.** `ExternCallee` carries `trust`, so every callee lookup in the
   checker yields it. There is no representation of an uncontracted callee that
   is not marked. `an_admitted_unchecked_callee_is_marked` proves the admission
   function only produces marked ones.
2. **Rederivation.** The driver publishes no certificate it did not rederive
   from the same program (`recheck_extern_certificate`, pre-existing, `Q027`).
   `a_tampered_assumption_list_does_not_survive_the_independent_recheck` strips
   the waiver premises from a real published certificate and shows the recheck
   refuses it.
3. **A publication tripwire.** `certificate_carries_unchecked_linkage` compares
   the published inventory against one recomputed from the program's own callee
   list, at the driver's publication boundary; a mismatch refuses at `Q035`. It
   cannot fail by construction, and asserting it anyway is the point — the day
   it can, it is caught there rather than inside a `Safe` that stopped saying
   what it rests on.

And in Lean, `assumption_totality`: for every callee list and every uncontracted
member, the derived inventory contains that member's record. With three
companions that close the other directions —
`an_uncontracted_callee_is_never_published_as_contracted` (the one substitution
that would look like evidence), `a_waiver_premise_names_an_uncontracted_callee`
(no premise appears out of nowhere) and
`every_callee_contributes_exactly_one_premise` (none can be silently skipped).

### 4.1 Where a reader sees it

`CompiledModule::declared_assumptions()` returns the certificate's premises for
any compiled module, so a consumer that does not know the route still sees them;
routes with no declared premise return the empty slice, which is the honest
answer and not a placeholder (`the_premises_reach_a_consumer_that_does_not_know_the_route`).

`qcorec check` prints them:

```
$ qcorec check demo.qcore
checked 1 function(s); entry `main`
under 3 declared assumption(s):
  - unchecked linkage: `k_unchecked_add` is linked as `(int32_t, int32_t) -> int32_t` with no contract; nothing about its behavior is checked
  - no extent transport: no declaration says how many bytes a callee touches
  - linkage: which object file supplies each symbol is the builder's premise
```

A `Safe` obtained through unchecked linkage is therefore never printed as a bare
`Safe`.

## 5. The example certificate, with the assumption visible

The published `ExternCallCertificate` for
`compiler/tests/fixtures/unsafe_linkage/round_trip.qcore`, verbatim:

```
ExternCallCertificate {
    entry: "main",
    calls: [
        ("k_unchecked_add",         []),
        ("k_unchecked_side_effect", []),
        ("k_unchecked_measure",     []),
    ],
    acquisitions: [], out_acquisitions: [], consumptions: [],
    releases: [], reallocations: [], stored: [], error_dispositions: [],
    effects: EffectRow({ Io, Alloc, Mut }),
    assumptions: [
        UncheckedLinkage { callee: "k_unchecked_add",
                           signature: "(int32_t, int32_t) -> int32_t" },
        UncheckedLinkage { callee: "k_unchecked_measure",
                           signature: "(uint64_t) -> uint64_t" },
        UncheckedLinkage { callee: "k_unchecked_side_effect",
                           signature: "(uint32_t) -> void" },
        NoExtentTransport,
        LinkageSuppliedByBuilder,
    ],
}
```

Read it as the card's whole claim in one object. Every custody field is empty —
nothing crossed. The effect row is maximal — nothing is known. Every callee is
named with the ABI it was linked at, and **no** `DeclaredContract` premise
appears, because no contract exists to claim one
(`every_uncontracted_callee_is_named_on_the_certificate_with_its_declared_abi`
asserts that absence explicitly: publishing an uncontracted callee as contracted
is the substitution that would look like evidence).

## 6. Diagnostics

A new semantic pass `UncheckedLinkage` owning the free band `Q032`–`Q035`
(`Q022`–`Q027` are `ExternCall`, `Q028`–`Q031` are `StandardIo`), through the
§L21 process. Spec §L21 gained the row, §L14.1 the normative section, and
`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.1 the grammar — all in this change.
**No existing code changed meaning.**

| Code | Refusal | Pinned by |
|---|---|---|
| `Q032` | an `unsafe extern fn` declaration in a file without `#import std.extern.unsafe;` | 4 tests, incl. cross-file non-leakage |
| `Q033` | signature outside the closed unchecked table: pointer parameter (3 spellings), pointer result, `unit` parameter, spelling outside the §C04.1 table | 5 tests |
| `Q034` | a waiver beside a checkable declaration: an inline `contract { … }` block, or a name the selected environment declares | 2 tests + 1 control |
| `Q035` | a published certificate missing a required waiver premise (publication tripwire) | the tripwire predicate, asserted both ways |

The `Q034` pair deserves a note. It is LOADER-ABI's rule applied to this
surface: *where a declaration is visible, it is compared*. Here the contracted
route simply wins — a name the environment declares refuses the waiver **whether
or not the signatures agree**, because you may not launder away a contract you
already have. The control
`the_same_waiver_with_no_such_contract_selected_refuses_on_its_own_merits`
proves that test was about the contract and not about the pointer: with the name
absent from the environment the same declaration refuses at `Q033` instead.

## 7. The default does not move

* `an_uncontracted_callee_without_the_waiver_still_refuses_at_q023` — the same
  source, with `extern fn` instead of `unsafe extern fn`, refuses at `Q023` with
  and without a contract selection.
* `a_contracted_module_is_unchanged_by_this_card` — the pre-existing contracted
  round trip still refuses without its environment, still publishes
  `DeclaredContract`, and publishes no waiver.
* The whole pre-existing `extern_calls.rs` suite passes unchanged.
* `a_contracted_row_is_the_existing_one` (Lean) — a contracted callee's derived
  effect row is byte-for-byte the one `QCore.ExternCall` derives.
* `the_contracted_inventory_is_unchanged` (Lean) — a callee list with no
  uncontracted member derives exactly the inventory it derived before.

### 7.1 One deliberate strengthening, found while implementing

The first implementation loaded a selected contract environment *leniently* when
every callee was waived, so that an all-unchecked module would not be refused
for an unrelated module's load failure. That is wrong and it was corrected: the
`Q034` identity gate would be silently skipped, so shipping a deliberately
broken contract module would become a way to hide that a callee has a contract.
A selected environment that does not load now refuses (`Q023`) even in the
all-unchecked case. An *absent* selection is still fine — that is the ordinary
all-unchecked module, and it is the case `qcorec` compiles. Both halves are
pinned by `a_broken_contract_selection_refuses_even_when_every_callee_is_waived`.

### 7.2 The last laundering vector, closed upstream

Declaring one symbol twice — once `extern fn`, once `unsafe extern fn` — and
hoping the driver picks the waiver is refused by name resolution at `Q010`
before any of this card's gates run, in one file and across two
(`a_name_cannot_be_declared_both_ways_in_one_file`, `…_across_two_files`). No
change was needed; the tests exist so the property cannot quietly regress.

## 8. Emission and native evidence

**Measured: neither the backend nor the reference interpreter needed a line.**
An unchecked callee's parameters are all `CallParameter::Data` and its result is
`CallResult::Data`, so `acquires()` is false and the call takes the existing
plain-call arm in both `emit_extern_c11` and `interpret_extern_program`. This is
a consequence of §3's admitted surface, not a coincidence.

Emitted C for the round trip:

```c
/* Generated by qcore `source.extern-call.v1`. */
#include <stdint.h>
#include <stdio.h>

int32_t k_unchecked_add(int32_t p0, int32_t p1);
uint64_t k_unchecked_measure(uint64_t p0);
void k_unchecked_side_effect(uint32_t p0);

int main(void)
{
    (void) k_unchecked_add(2, 40);
    printf("call k_unchecked_add d:2 d:40\n");
    k_unchecked_side_effect(7);
    ...
}
```

`the_emitted_program_calls_the_real_uncontracted_library` compiles that against
`compiler/tests/fixtures/unsafe_linkage/unchecked_api.c` — a real C file with no
contract anywhere in this repository — with
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined`
under `detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`. **Exit status 0**, and the
native observation equals the reference observation line for line:

```
call k_unchecked_add d:2 d:40
call k_unchecked_side_effect d:7
call k_unchecked_measure d:16
return
```

What that establishes: the emitted program performed those calls across a real
link boundary, in the checked order, and left nothing behind. What it does
**not** establish: anything whatever about what those functions do. That is the
premise, and printing it is the point.

## 9. Lean

`proof/QCore/UncheckedLinkage.lean` (new) + `proof/AuditUncheckedLinkage.lean`
(new, registered), plus additions to `QCore.ModuleImports`. **26 theorems audited in the
new namespace (27 written, one a private helper) and 7 in `ModuleImports`, no new
axiom** (only
`propext`, `Classical.choice`, `Quot.sound`).

The ABI vocabulary, bridged parameter forms and custody vector are **imported**
from `QCore.ExternCall` rather than restated, so there is one table in the model
and not two that could drift.

* Assumption-carrying totality: `assumption_totality`,
  `every_callee_contributes_exactly_one_premise`,
  `an_uncontracted_callee_is_never_published_as_contracted`,
  `a_contracted_callee_contributes_no_waiver`,
  `a_waiver_premise_names_an_uncontracted_callee`.
* Default unchanged: `the_contracted_inventory_is_unchanged`,
  `no_waiver_without_an_uncontracted_callee`,
  `a_contracted_row_is_the_existing_one`.
* Scalar-only admission, fail-closed: `a_pointer_parameter_refuses` (general,
  over an arbitrary scalar prefix), the three per-spelling instances including
  `a_read_only_pointer_parameter_refuses`, `a_unit_parameter_refuses`, both
  pointer-result theorems; non-vacuity via `the_unchecked_table_is_inhabited`
  and `a_unit_result_is_admitted`.
* Custody-freedom: `an_admitted_unchecked_callee_carries_no_custody`,
  `…_acquires_nothing`, `…_is_marked`.
* The maximal row: `an_unchecked_row_is_maximal`,
  `a_narrower_declared_row_refuses_an_unchecked_call` (all three proper
  sub-rows, not one convenient one), `an_empty_declared_row_admits_no_…`, and
  `the_whole_declared_row_admits_…` so those are not vacuous.
* The module gate: `admitted_unchecked_linkage_requires_its_module`,
  `the_extern_import_alone_grants_no_waiver`,
  `the_waiver_alone_grants_no_extern_call`,
  `the_two_extern_modules_are_independent`,
  `a_near_miss_waiver_spelling_rejects`.

**Stated boundary of the Lean model.** `QCore.ExternCall.AbiType` is a collapsed
projection (one `scalar` constructor, the two flat pointer spellings, the
pointer-to-pointer projection, `unit`), so `signature` there models the *shape*
of the rendered declaration and not the Rust rendering byte for byte. The
theorems are about the record existing, naming the callee, and being
underivable-without; the exact rendered string is Rust-tested
(`every_uncontracted_callee_is_named_on_the_certificate_with_its_declared_abi`
pins all three signatures verbatim). Source extraction and the agreement of the
Rust code with the transcription remain premises, as they are for every model in
this repository.

### 9.1 Two existing Lean modules widened

`QCore.ModuleImports.Imports` gained a sixth field, in the same "deliberate
widening" the `STD-MODULES` card recorded for its third through fifth. Every
existing theorem name and meaning is preserved; the structure literals grew one
field, `unknown_module_rejects` gained one hypothesis, and
`setter_preserves_other_fields` gained one conjunct and one case.
`QCore.StdIo.no_module_substitutes_for_another` grew the same field, and its
rows now hold the waiver too — so the three IO capabilities are shown
unobtainable from the waiver as well.

## 10. Corpus neutrality

By construction, and the construction is checkable in the diff: **no file under
`crates/qcore-c-gateway/`, `crates/qcore-audit/`, `crates/qcore-contract/` or
`contract/qcore/` is touched.** The complete production diff is
`qcore-core` (extern-call model, module registry), `qcore-frontend` (parser
branch, one predicate line, the pass enum, the import gate), `qcore-verification`
(driver admission + tripwire + the `declared_assumptions` accessor) and
`qcorec` (printing).

`qcore-frontend` is touched and the gateway's contract loader parses `.qcore`
contract modules with that parser, so the two parser changes are argued
explicitly rather than waved past: the new declaration branch requires a leading
`unsafe` token, which no existing corpus text has, and the `is_declaration_start`
line is consulted only after a top-level brace group where a statement cannot
begin. Both are strictly more accepting and never differently accepting.

**The canonical CMetrics artifacts were not re-measured here**: no prepared
Clang/CMake build tree exists on this host and preparing one was out of scope
for the card's disk budget. The expectation for consolidation is stated plainly:
**byte-identical**, on the structural argument above. The hermetic
configured-audit fixtures that *are* in the workspace suite (the second
source set, the synthetic demo, the route-parity families) all pass unchanged,
which is the available executable half of that claim.

The contract corpus count is untouched — this card writes no contract text at
all — and no CT manifest row moves.

## 11. Gate

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2910 passed, 0 failed, 8 ignored** (see §11.1) |
| `compiler/tests/unsafe_linkage.rs` | **28 passed, 0 failed**, native sanitizer run included |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **Passed 77 Lean audits** (76 at base + `AuditUncheckedLinkage`) |
| `python3 -m unittest discover -s scripts/tests` | 186 run, **1 expected failure** (§11.2) |
| Sanitizer fixture | ASan/UBSan/LSan, exit 0, native == reference line for line |

### 11.1 Submodule note

The workspace run initially failed three tests
(`libevent_submodule_matches_the_audited_revision` and two `profile_loader`
pins) because `third_party/libevent` was not initialised in this fresh
worktree. `git submodule update --init --recursive` fixed all three; they are
not related to this card.

### 11.2 The one expected Python failure

`test_the_derived_facts_are_recomputed_rather_than_carried` asserts the
acceptance matrix's derived `lean_audits` row, which reads `76` and must read
`77` now that this card registers an audit file. **Deliberately left for the
integrator**, per the card brief: acceptance files are not this card's to edit.
Reported honestly rather than worked around.

## 12. The linkage spectrum, and the adopted `NATIVE-LIBRARY-LINKAGE` design

A prior workflow designed `source.native-library.v1` on branch
`claude/qcore-c-abi-linking-c8c443` (`e48786c`; ledger framing at `ecede86`): a
**library manifest** binding a C04 contract slice, the library's exported C
header, an artifact identity (name/version/content hash) and the ABI premises
(data model, unwinding, allocator custody) into one declared, checkable linkage
unit — turning the extern family's "linkage is the builder's premise" into
something a certificate can name, with `LOADER-ABI`-style withdrawal on header
disagreement and provider-package-style artifact verification.

**It is design input, and this card integrates it three ways.**

### 12.1 Adopted into the tree

`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` now lives here, provenance-noted
(authored at `e48786c` in a prior workflow, adopted with adjustments), status
unchanged at **scoped only**. Four adjustments, recorded in its own §0 and
marked at their sites, all specifics the tree moved under it:

| # | As written | Adjusted to | Why |
|---|---|---|---|
| A1 | "the five-name … precedent" | **six** names | this card added `std.extern.unsafe` to the registry |
| A2 | "the first free band after `Q031`" | **`Q036` onward** | this card took `Q032`–`Q035`; the one load-bearing collision |
| A3 | "the 67-module corpus pin" | **69** | `CONSTRUCTOR-RESIDUALS` moved it in wave 17 |
| A4 | "constraint 8 / 18 / 28" | kept, referents named | the numbering is the authoring workflow's; the repo-native referents are `LOADER-ABI`, `P01-F2` and hard invariant 2 |

Checked and found *not* to need adjustment: its `single-threaded-caller` premise
is still vacuous for exactly the reason it gives. Its `AGENTS.md`,
`DEVELOPMENT.md` and `acceptance_matrix.json` hunks are **deliberately not
carried** — acceptance and status files are integrator surface, and its
`named_followups` entry remains for the integrator to enter.

### 12.2 The spectrum, written down

`UNSAFE_LINKAGE_V1.md` §1.1 positions this card as the **uncontracted end of one
spectrum** whose contracted end is that document, with the trust artifact each
level carries into a certificate named explicitly:

| Level | Selection | Trust artifact in the certificate | Surface | Status |
|---|---|---|---|---|
| **L3** manifest-bound contracted | `#import std.buffer;` | artifact identity + ABI premises + each contract | pointers included | designed |
| **L2** declared contract | `CompileOptions::extern_contracts` | `DeclaredContract` per callee | pointers included | implemented today |
| **L1** manifest-resident unsafe | contract-less manifest entry | artifact identity + ABI premises + `UncheckedLinkage` | scalars only | designed (§12.3) |
| **L0** standalone unsafe | `unsafe extern fn` | `UncheckedLinkage { callee, signature }` | scalars only | **implemented — this card** |

Three properties make it one design rather than four: the trust artifact shrinks
monotonically with knowledge and is **never empty**; **pointers cross exactly
where a contract exists to state their premise**, which is why the two
documents' opposite-looking tables are one rule; and **the ABI type gate never
relaxes** — L3 against the library header, L2 against the contract module, L1
against the manifest header, L0 against the closed §C04.1 vocabulary. What is
waived along the spectrum is the contract, never the ABI.

### 12.3 The strongest premise (L1): designed, not implemented

`UNSAFE_LINKAGE_V1.md` §4.2 designs the unification the spectrum implies. The
strongest honest form of this card's premise is not *symbol X, unchecked, from
anywhere* but *symbol X, unchecked, **from artifact Y@hash under ABI premises
Z***. That is a **contract-less manifest entry**: it inherits the manifest's
artifact identity and ABI premise fields, is still type-checked against the
manifest's exported header (principle 4 at its strongest available setting —
stronger than L0's), carries no modes and no effects, keeps this card's
scalars-only surface unchanged (an artifact hash says *which code*, not *what it
does with your pointer*), and must be **explicitly marked contract-less** so a
dropped contract can never be silently read as a waiver.

**It is designed and deliberately not implemented in v1**, because the manifest
it lives in does not exist yet — landing L1 first would mean landing the whole
manifest to carry one field. Staged as `UNSAFE-LINKAGE-ARTIFACT` (§13), aligned
with that document's own stage (a).

What v1 records instead is not nothing: `UncheckedLinkage { callee, signature }`
carries **no** artifact field, and that absence is part of the trust statement —
*no build was named*. An L0 premise and a future L1 premise are therefore
distinguishable by inspection, and an L0 premise stays a valid weaker statement
when L1 lands rather than becoming a malformed one.

### 12.4 One deviation, recorded by name

That document's §4.4 states a `single-threaded-caller` posture as a
stated-but-vacuous premise. This card states **no thread posture at all** for a
waived callee: an uncontracted callee has no declaration in which to state one,
and a premise nobody wrote is not one this repository records on its behalf.
Lifting the concurrency refusal for the unchecked route is a separate card and
must add the field explicitly. Recorded so the omission is deliberate rather
than inherited.

## 13. Named follow-ons

| Name | What is open |
|---|---|
| `UNSAFE-LINKAGE-PTR` | any pointer at the unchecked boundary. §3.1 is the argument for why this needs new *model capability* — an extent transport and a retention model — and not a wider table. Do not widen the table without one. |
| `UNSAFE-LINKAGE-ARGS` | argument forms beyond integer literals. The v1 argument grammar is the pre-existing one, so a `bool` parameter is declarable but callable only with an integer literal. Inherited, not introduced. |
| `UNSAFE-LINKAGE-CTX` | unchecked calls in task workers, `with lock` bodies and error regions, which keep every existing refusal. |
| `UNSAFE-LINKAGE-CLI` | a `qcorec` flag selecting a contract environment. The CLI selects none, which is exactly why an all-unchecked module is what it can compile end to end today. |
| `UNSAFE-LINKAGE-ARTIFACT` | the contract-less manifest entry of §12.3: a waived symbol resident in a `source.native-library.v1` manifest, inheriting its artifact identity and ABI premises and still type-checked against its header, so the premise names *which build* was waived. Designed here; blocked on that document's stage (a), the manifest itself. The single highest-value extension of this card. |
| `NATIVE-LIBRARY-LINKAGE` | the adopted document's own implementation (its §12 staging (a)-(d)). Adopted as a design here, not implemented; its `named_followups` ledger entry is integrator business. |
| `UNSAFE-LINKAGE-EXTENT-PREMISE` | `NoExtentTransport` is published unconditionally, so an all-scalar unchecked module carries a premise about pointers it does not have. Harmless and true, but noisy; making it conditional would move existing contracted certificates, so it was left alone deliberately. |

Variadics, function pointers, structs, floats and verifying the link step stay
out of scope for this route exactly as they are for the contracted one.
