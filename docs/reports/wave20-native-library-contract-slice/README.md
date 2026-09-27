# NATIVE-LIBRARY-CONTRACT-SLICE — where pointer symbols belong — 2026-09-11

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

Card `NATIVE-LIBRARY-CONTRACT-SLICE` of wave 20: **where pointer symbols
belong**. Wave 18 established that pointers cross exactly where a contract exists
to state their premise; wave 19 built the manifest but refused a `contract fn`
entry **by name**, leaving a visible seam. This card fills that seam.

**A note on the level number, because the brief and the tree number differently.**
The brief calls this card "level L2 of the linkage spectrum", meaning the second
of the manifest's two entry kinds. The in-tree spectrum table
(`docs/design/UNSAFE_LINKAGE_V1.md` §1.1) numbers by *decreasing knowledge*, and
there **L2** is the already-implemented ad-hoc declared-contract selection while
**L3** is manifest-bound contracted. This card lands L3's **trust artifact**
(artifact identity + ABI premises + each contract) and L3's **callable surface**
(every mode `source.extern-call.v1` calls, pointers included). It does **not**
land L3's *selection*: the table's `#import std.buffer;` — one name binding all
four — stays follow-on `NATIVE-LIBRARY-SOURCE-BINDING`, and selection here is
still the builder's two lists (`extern_contracts` plus `native_libraries`). The
in-tree numbering is used everywhere below; where this report says "L1" it means
the table's L1, wave 19's contract-less manifest entry.

**Base tree:** main `e01cac2`. Design:
[`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`](../../design/NATIVE_LIBRARY_LINKAGE_V1.md)
§4.1–§4.2 and §4.4, and
[`docs/design/UNSAFE_LINKAGE_V1.md`](../../design/UNSAFE_LINKAGE_V1.md) §1.1's
spectrum table. Parent records:
[`wave19-unsafe-linkage-artifact`](../wave19-unsafe-linkage-artifact/README.md)
(the table's L1, whose prerequisite this card is) and
[`wave18-unsafe-linkage`](../wave18-unsafe-linkage/README.md) (L0).

---

## Headline

**L3's trust artifact and callable surface landed.** A manifest may now bind a
symbol to a **declared `.qcore` contract**, so a contracted extern call resolves through the manifest and
inherits the artifact identity. A certificate for such a callee says

> `qcore_extern_alloc` behaves as its selected contract states
> **— and —** `qcore_extern_alloc` is read through the declared contract
> `…/buffer_lib.qcore`, from artifact `libqcore_buffer_demo.a` of library
> `qcore_buffer_demo 0.2.0` at `sha256:4d1f…32ed`, under stated ABI premises
> (data model lp64, no unwinding across the boundary, closed allocator custody,
> single-threaded caller)

— **two** premises, stacked. That is the card's central design decision and §3
is its argument.

| | |
|---|---|
| Entry format landed | `contract fn NAME(TYPES) -> TYPE from MODULE;` — the exported header declaration plus a **contract module reference**, which is the contract selection's own key, compared exactly and never normalised |
| Where pointers go | here, and only here. `PointerInUncheckedEntry` was **not weakened**; it acquired a destination |
| Identity inheritance | the same premise channel as L1's waivers, the same `qcorec check` printing shape, the same stated-pin trust boundary — nothing is hashed, opened or searched for at check time |
| Premise composition | **stacking, not replacement.** The declared-contract premise is untouched and the artifact premise is added beside it |
| Disagreement | **withdrawal** for the analysed unit (`Q048`), then the uncontracted refusal (`Q023`) — the *opposite* disposition from L1's `Q041`, from one shared rule (§4) |
| Diagnostics | a **second band** for the existing `NativeLibrary` pass, `Q047`–`Q049`, through the §L21 process; `Q039`'s condition list restated, its meaning unchanged |
| Contract machinery | **unchanged.** `crates/qcore-contract/` is not touched at all; the manifest selects and attributes |
| New reserved words | **none.** A manifest is not source (constraint 14 census in §8) |
| `qcore-schemas` | **untouched at 14.0.0** — no provider rebuild (constraint 18 not engaged, §8) |
| Evidence | 36 new Rust tests (21 core unit, 15 route) and **31 audited Lean theorems** in a new `QCore.NativeLibraryContract` namespace, no new axiom |

**Nothing is promoted.** A manifest-bound contracted callee is `Safe` only under
*both* its printed premises, and the artifact premise says which code, never what
that code does. No raw Cycle 19 unit moves, no CT mapping changes, the contract
corpus is untouched, and full language conformance stays false.

---

## 1. The entry format, and why it is a reference rather than an inline slice

Design §4.1 says an entry binds "`extern fn` name → contract (by content, within
the module's slice) → exported header declaration", and the brief asked for
adopt-or-adjust with provenance. **Adopted as a module reference**, not an inline
contract slice:

```
contract fn qcore_extern_alloc(uintptr_t) -> uint8_t * from fixtures/native_library/contracts/buffer_lib.qcore;
contract fn qcore_extern_fill(uint8_t *, uint32_t) -> void from fixtures/native_library/contracts/buffer_lib.qcore;
contract fn qcore_extern_sum(const uint8_t *, uint32_t) -> int32_t from fixtures/native_library/contracts/buffer_lib.qcore;
contract fn qcore_extern_release(uint8_t *) -> void from fixtures/native_library/contracts/buffer_lib.qcore;
```

Three reasons, all load-bearing, and none of them convenience:

1. **The C04 loader must stay the only contract vocabulary** (design §2). An
   inline slice would put contract text inside a manifest, which means either a
   second grammar for declarations or moving the manifest loader up a crate.
   `qcore_core::native_library` has **no dependencies at all** — that is what
   makes the manifest re-derivable in the kernel — so it cannot load a C04
   contract, and it should not learn to.
2. **The environment already rechecks itself.** `check_source_assumptions`
   reloads the selected contracts from their own retained text before use
   (`Q027`). An inline slice would create a second copy of the same declaration
   with no such identity, and the two could disagree.
3. **A reference is what a certificate can name.** The premise reads *per
   contract C*, and `C` is the selection key a rederivation resolves.

**The slice is derived, not declared.** A manifest's slice is the set of modules
its entries name; there is no separate `slice MODULE;` statement, because a
second spelling would be a second place the slice could be wrong. Design §4.1's
totality then reads directly: every entry resolves to a declared contract *in the
module it names*, and every declaration of a module in the slice is bound by an
entry (`Q047`, both directions, `check_slice_totality`).

**Two entry kinds, told apart by keyword and never by shape.** A `contract fn`
with no `from MODULE` tail is `MalformedContractedEntry`; an `unchecked fn`
carrying one is `MalformedEntry`. Neither is read as the other, which is what
keeps design §4.2's requirement — a *dropped* contract can never be read as a
waiver — true now that both kinds exist. A symbol may not be bound twice **across
both kinds**, in either order, in one manifest or across a selection: an entry at
once waived and contracted has no defined premise, and picking by line order
would make the premise depend on the order the author typed.

**Module references are compared exactly.** A contract in another module is
another contract (`SliceRefusal::ContractInAnotherModule`), so a manifest cannot
attribute a build to a declaration it never named. This is constraint 11's
no-normalisation discipline applied to a second kind of spelled name, for the
same reason.

## 2. Where pointers belong — the seam, closed from the other side

Card `UNSAFE-LINKAGE-ARTIFACT` refused `uint8_t *`, `const uint8_t *` and
`uint8_t **` in a contract-less entry, with the message that a library's pointer
symbols belong in a contract slice entry. That refusal is **unchanged**: the same
spellings still refuse as `unchecked fn` and now bind as `contract fn`, pinned in
one test row over all three spellings and the pointer result
(`the_pointer_a_contract_less_entry_may_not_declare_is_admitted_here`), and in
Lean by `a_pointer_symbol_binds_here_and_never_as_a_waiver`, which asserts both
halves in one statement.

Nothing about the **contracted** admitted surface moved either. `bridge_contract`
decides what may cross, first and alone; `manifest_contracted_callee` attaches a
residence to an already-admitted callee and does nothing else. A manifest can no
more buy a contracted callee shape than it could buy a waived pointer.

## 3. Premise stacking — and the one deliberate asymmetry with L1

The brief's requirement: *the contract's own premises stack with — never replace
— the artifact premises.* Implemented literally.

* `ExternAssumption::DeclaredContract(name)` is **untouched**, in meaning and in
  presence. A manifest-bound contracted callee still publishes it.
* `ExternAssumption::ManifestResidentContract { callee, contract_module,
  residence }` is published **beside** it.

That is the opposite shape from level L1, where `ManifestUncheckedLinkage`
*replaces* `UncheckedLinkage`, and the asymmetry is deliberate rather than
inconsistent:

| | L1 (contract-less manifest entry) | L3 (contracted manifest entry) |
|---|---|---|
| What the bare premise carries | the callee's name and its **signature** | the callee's name, as a pointer to its contract |
| What the manifest restates | the signature — the same fact | nothing the contract premise says |
| Therefore | one record is the whole statement; two would be one statement told twice, and a bare premise must stay a valid *weaker* statement rather than a malformed sharpened one | two independent statements: what the code does, and which code it is. Neither is derivable from the other, so a certificate carries both |

The Rust invariant is `assumptions_for_callee`, which returns the trust premise
first and the residence premise second, and the publication tripwire `Q049`
refuses a certificate that lost **either** half. In Lean,
`the_contract_premise_is_never_replaced` states that the declared-contract
premise is in the inventory of every admitted outcome, and
`a_resident_call_contributes_exactly_two_premises` fixes the count.

`ManifestResidence` gained one field, `contract: Option<String>`, which is `Some`
exactly for a contract slice binding and `None` exactly for a contract-less one.
The invariant is maintained by construction — `ManifestBinding::residence` and
`ContractedBinding::residence` are the only producers — and pinned on both sides
(`a_contracted_binding_names_its_module_and_a_waiver_never_does`;
`a_contracted_residence_names_its_module` /
`a_contract_less_residence_names_none`). An L1 premise's *printed* content is
unchanged.

`qcorec check` gains the matching printed line. It is **unreachable through
today's CLI** and deliberately present anyway: `qcorec` selects no extern
contract environment (follow-on `UNSAFE-LINKAGE-CLI`), so no contracted callee is
admitted through that binary at all. Without the arm the premise would render
through the `{other:?}` fallback the moment that follow-on lands, which is how a
premise stops being read. Stated here rather than left for a reader to discover.

## 4. Withdrawal — and why the direction is the opposite of L1's

Card `UNSAFE-LINKAGE-ARTIFACT` recorded a **deviation** from the design: §4.2
specifies `LOADER-ABI`-style per-unit withdrawal on header disagreement, and that
card *refused* instead. This card **withdraws**, as the design says. Two cards,
two dispositions, and they are not in tension — they are the same rule applied to
two different situations:

> **Withdraw only where the weaker admissible reading refuses.**

* **L1.** The weaker reading below a withdrawn contract-less binding is the *bare
  waiver* — and a bare waiver is **accepted**. Withdrawing to it would buy a
  certificate that says *less* about the artifact than the manifest was selected
  to make it say: a contradiction rewarded with silence. So L1 refuses outright,
  which is that setting's monotone direction.
* **L3.** The weaker reading below a withdrawn contracted binding is the
  *uncontracted callee* — and an uncontracted callee **refuses** (`Q023`, "no
  admitted declared contract"). Withdrawal is therefore already the refusing
  direction, and `LOADER-ABI`'s shape (constraint 8) is exactly right: the
  contradicted declaration is withdrawn for the analysed unit, its name is not
  seeded into the admitted environment, every call site becomes an uncontracted
  callee, and the withdrawal is **reported** rather than performed silently.

The polarity is what differs, not the rule. It is stated as a Lean theorem rather
than only in prose:
`the_weaker_reading_refuses_here_and_is_admitted_at_level_one` exhibits both
weaker readings side by side — L3's publishes nothing, L1's is a concrete
admission.

**Both halves are published.** A contradicted binding produces `Q048` (the
withdrawal, naming the disagreement and the manifest) *and* `Q023` (the refusal
every call site then reaches), pinned as the exact code sequence in
`a_manifest_contradicting_the_declared_contract_withdraws_it_for_this_unit`. A
withdrawal that only refused would be a refusal; one that refused *silently*
would be an override, which constraint 8 forbids.

**Monotone in the refusing direction**, and unlike `LOADER-ABI`'s own case it can
only refuse: there, withdrawing a release contract could turn a `Safe` unit into
a provisional `Unsafe`; here the weaker reading is a refusal, so a withdrawal
turns an admission into a refusal and nothing else. Three edited projections —
result, arity, result width — are pinned in
`a_withdrawal_is_monotone_in_the_refusing_direction`, and in Lean by
`a_manifest_can_only_withdraw` (the only outcome a manifest can produce that the
unmanifested route would not is a withdrawal) and
`withdrawal_only_removes_admissions` (over a whole environment).

**Where both sides are read from.** The manifest's stated header declaration and
the declared contract's §C04 ABI projection — the manifest text and the
environment's retained contract text, both of which a certificate already holds,
and neither of which is a typed Clang AST. That is constraint 8's reader
discipline, and it is why the decision is reproducible at a recheck.

## 5. A measured non-addition: the waiver mirror needs no gate

`Q034` refuses a waiver for a name the contract *environment* declares; `Q042`
refuses a contract for a name the manifest declares contract-less. The obvious
third face — a waiver for a name the manifest binds to a **contract** — was
implemented, and then **removed**, because it is unreachable:

`Q047` refuses a manifest whose contract slice entry the selected environment
does not declare. So wherever a manifest binds `name` to a contract, the
environment declares `name`, and `Q034` has already refused. A gate here would
never fire, and an unreachable diagnostic code is a claim a reader cannot check.

Measured, not assumed: the first implementation of that gate was written, and the
test written for it observed `Q034` instead. The row was kept, with its
expectation changed to `Q034` and the composition explained
(`a_symbol_the_manifest_binds_to_a_contract_may_not_be_waived`), and the Lean
theorem `a_manifest_contracted_symbol_is_declared_by_the_environment` states the
implication the composition rests on. Recorded as
**`CONTRACT-SLICE-WAIVER-MIRROR`**.

This is why the band is `Q047`–`Q049` and not `Q047`–`Q050`.

## 6. Closed allocator custody stops being vacuous

Design §4.4 requires a structural check: a slice containing an acquiring
declaration but no consuming or releasing declaration rejects at manifest load,
"because such a module could only ever leak or lie". Wave 19 satisfied it
**vacuously** and said so — a contract-less entry cannot acquire.

A contract slice entry can. So the check became real exactly here, and was
implemented rather than inherited as vacuous
(`check_closed_allocator_custody`, refusing at `Q047` with
`SliceRefusal::OpenAllocatorCustody`). Four decisions in it are worth stating:

* **The projection is read where contracts live; the decision is made in the
  kernel.** `qcore-core` has no dependencies, so the driver projects each bound
  contract to a `CustodyShape { acquires, discharges }` and the kernel decides.
  The decision is then a function of its inputs and repeatable at a recheck.
* **A `resource_stored` parameter is not a discharge.** It relocates custody
  beneath a declared owner and discharges nothing — that is precisely what
  `StoredCustodyUnderDeclaredOwner` records as undischarged. Counting it would
  let a slice claim closed custody on the strength of a premise nothing in this
  repository discharges.
* **An unreadable shape supplies no discharge.** A symbol whose projection is
  absent contributes nothing, which is the fail-closed direction here.
* **The check is per manifest, not per selection.** Two libraries do not close
  each other's custody: `allocator_custody closed` is one manifest's statement
  about one artifact.

Design §4.4 says "for the same resource shape". §C04 projects every resource to
one shape (an extentless `u8` buffer), so the shape match is **vacuous in v1**;
recorded rather than silently narrowed, and it becomes real work when the
buffer-extent follow-on lands.

## 7. Diagnostics

A **second band** for the existing `NativeLibrary` pass: `Q047`–`Q049`, through
the §L21 process, with spec §L21, the new §L14.3 and grammar §G02d.2 all updated
in this change.

| Code | Condition | Pinned by |
|---|---|---|
| `Q047` | the contract slice binding does not resolve, in either direction of §4.1 — an unresolved symbol, a symbol declared in another module, a declaration of a slice module that no entry binds, no contract environment selected at all — or §4.4's closed-allocator-custody check refuses | 5 route rows + 10 unit rows (5 totality, 5 closed-custody) |
| `Q048` | a **contracted** declaration and the manifest that binds it disagree in arity, a parameter or the result: the contract is **withdrawn for the unit** and the call sites fall to `Q023` | 2 route rows (the second covering three separate edited projections) + 1 unit row with four assertions |
| `Q049` | a published certificate lost either half of a manifest-bound contracted callee's stacked premises | tripwire at both publication boundaries (extern and composed), on the `Q035` precedent |

The band is **not contiguous** with `Q039`–`Q042`, because `Q043` (`ComposedStdIo`)
and `Q044`–`Q046` (`Branch`) were allocated between wave 19 and wave 20. A pass
owning two ranges is the honest record of that order; no shipped code was moved
to make the table look tidier. A collision discovered at integration relocates
this *unshipped* band per the wave-3 `T07`/`T09` precedent, and the
code-to-condition mapping is stable under such a shift.

**`Q039` was restated, not moved.** Its meaning — "a selected manifest does not
load" — is unchanged; what changed is that a contracted entry is now a grammar
this loader implements rather than one it declines. The refusal vocabulary went
from fifteen constructors to sixteen: `ContractedEntry` was deleted **on purpose**
(the module's own documented discipline: widening the grammar means deleting a
refusal deliberately), and `MalformedContractedEntry` and
`MalformedContractReference` were added, along with `SliceRefusal`'s four
constructors at the driver boundary. No existing code changed meaning:
`Q022`–`Q027`, `Q032`–`Q035` and `Q040`–`Q042` keep their exact conditions.

## 8. What did not move — pinned in both directions

* **The `Q023` default.** With no environment and no manifest, `ROUND_TRIP`
  refuses at exactly `Q023`; with an L1 manifest selected, still exactly `Q023`
  (`the_q023_default_is_byte_unchanged_in_both_directions`).
* **The contracted route without a manifest.** Same premises, same emitted C,
  same profile, same declaration count, same entry
  (`the_manifest_changes_the_premises_and_nothing_else` — the premise list is the
  *only* field that differs).
* **A manifest naming other symbols.** Byte-identical premises and C source
  (`a_manifest_naming_other_symbols_leaves_the_contracted_route_untouched`).
* **Level L1.** The whole wave-19 `native_library.rs` route suite (16 rows) and
  the wave-18 `unsafe_linkage.rs` suite (28 rows) pass unchanged, and the two
  levels coexist in one selection (`the_level_one_route_is_unchanged_beside_the_new_one`).
* **New reserved words: none.** Constraint 14's census is trivial here and is
  recorded anyway: `contract`, `fn` and `from` appear only inside a manifest,
  which is not source and shares no lexer with it. No token was added to the
  source grammar, and `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.2 is where
  the manifest grammar lives precisely because it is not §G02d's.
* **`qcore-schemas` stays 14.0.0.** The manifest types live in `qcore-core`; the
  contract projection the driver reads is an existing schemas type read
  read-only. Constraint 18 is not engaged: **no provider package was rebuilt and
  none needed to be.**
* **Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
  `crates/qcore-audit/`, `contract/qcore/` or `providers/` is touched, and
  `crates/qcore-contract/` — the contract machinery itself — is not touched
  either. The two new fixtures live under `compiler/tests/fixtures/`, **not**
  `contract/qcore/`, so the 69-module corpus pin is untouched (design §9's own
  instruction). No parser rule changed. The corpus baseline stays
  `docs/reports/wave17-corpus/`.

## 9. Lean

`proof/QCore/NativeLibraryContract.lean` (new) + `proof/AuditNativeLibraryContract.lean`
(new, registered), imported into `proof/QCore.lean`. **31 theorems audited, no new
axiom** (only `propext`, `Classical.choice`, `Quot.sound`).

The identity, ABI premises, residence record and the **premise inductive** are
imported from `QCore.NativeLibrary` rather than restated, so there is one premise
vocabulary in the model and not two that could drift — which is also what makes
the stacking claim statable. `QCore.NativeLibrary.Premise` gained the fourth
constructor and `Residence` the `contract` field, in the same change; every
existing L1 theorem is unchanged and still audited.

* **Stacking, and no premise-free path** —
  `an_unmanifested_contract_keeps_its_one_premise`,
  `a_manifest_bound_call_stacks_both_premises`,
  `the_contract_premise_is_never_replaced`, `there_is_no_premise_free_path`,
  `a_resident_call_contributes_exactly_two_premises`,
  `the_artifact_premise_names_the_contract_and_the_build`,
  `a_contracted_premise_waives_nothing`,
  `a_contracted_residence_names_its_module`,
  `a_contract_less_residence_names_none`.
* **Withdrawal, and the direction argument** — `a_contradicted_binding_withdraws`,
  `a_withdrawal_admits_nothing`, `a_withdrawal_publishes_no_premise`,
  `the_unmanifested_reading_is_always_admitted` (which is what makes the next one
  a monotonicity claim rather than a tautology about an empty set),
  `a_manifest_can_only_withdraw`, `withdrawal_only_removes_admissions`,
  `the_weaker_reading_refuses_here_and_is_admitted_at_level_one`.
* **Binding totality** — `an_unresolved_entry_is_not_total`,
  `an_entry_in_another_module_is_not_total`,
  `an_unbound_contract_in_the_slice_is_not_total`,
  `a_declaration_outside_the_slice_is_not_demanded` (the gate is total on the
  slice, not on the environment), `a_matching_slice_is_total`,
  `a_manifest_contracted_symbol_is_declared_by_the_environment` (§5's
  composition).
* **Closed allocator custody** —
  `an_admitted_slice_discharges_what_it_can_acquire` (design §8's soundness
  bullet), `an_open_custody_slice_is_refused`, `a_discharging_slice_is_closed`,
  `a_slice_that_cannot_acquire_is_closed`.
* **Determinism and non-vacuity** — `the_binding_is_a_function`,
  `admission_is_deterministic`,
  `a_pointer_symbol_binds_here_and_never_as_a_waiver`,
  `the_contract_slice_route_admits_and_stacks`,
  `an_unbound_symbol_keeps_the_single_premise`.

**Stated boundary of the Lean model.** The manifest's concrete syntax and its
total-or-reject loader are a Rust boundary and are tested there; what the model
is about is what a *loaded* manifest means. The contract machinery is not
modelled at all, deliberately — the manifest selects and attributes, and what a
declared contract means is `QCore.ExternCall`'s business, unchanged. `AbiType` is
the collapsed projection, as in the imported modules, so the modelled agreement
gate compares projections at that granularity while the Rust gate compares the
full fifteen-way table. Source extraction and the agreement of the Rust code with
the transcription remain premises, as for every model here.

## 10. Gate

| Check | Result |
|---|---|
| `cargo test` over all ten workspace members | **3,169 passed, 1 failed, 8 ignored** — the one failure is the recorded constraint-16 load flake, re-verified green in isolation (§10.1) |
| `compiler/tests/native_library_contract.rs` (new) | 15 passed |
| `crates/qcore-core` unit `native_library` | 46 passed (25 at base + 21 new) |
| `compiler/tests/native_library.rs` (wave 19, L1) | 16 passed, unchanged |
| `compiler/tests/unsafe_linkage.rs` (wave 18, L0) | 28 passed, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **Passed 86 Lean audits** (85 at base + `AuditNativeLibraryContract`) |
| `python3 -m unittest discover -s scripts/tests` | 193 run, 1 expected failure (§10.2) |

`third_party/libevent` was initialised in this fresh worktree before the run
(`git submodule update --init --recursive`), as waves 18 and 19 also recorded.

**A note on how the run was executed.** A single `cargo test --workspace`
invocation was killed twice by the harness's background-task limit under the
load of five sibling worktrees, so the suite was run in four foreground
package groups — `qcore-core`/`-schemas`/`-frontend`/`-contract` (1,083),
`qcore-c-gateway`/`-audit`/`-verification` (1,250), `qcore-compiler`/`qcorec`
(725), `qcore-verifier-host` (111 + the flake) — whose union is exactly the ten
workspace members. No test was skipped, filtered or weakened; only the process
boundaries differ, and the ignored count (8) matches the standing ledger.

### 10.1 The one failure is a recorded load flake, verified in isolation

`qcore-verifier-host::enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
failed once during the package run. That is **P02-FLAKE row one** (`AGENTS.md`
constraint 16: "the `RLIMIT_NPROC` conformance test — it counts every process of
the real UID"), and the documented condition held: five sibling agent worktrees
building and testing on the same host, with free memory down to 1–2 GB for the
duration. Re-run in isolation on the same binary, **unweakened**:

```
$ cargo test -p qcore-verifier-host --test enforced_process_limits -- --test-threads=1
test result: ok. 12 passed; 0 failed; 0 ignored
```

No assertion was touched. This card's production diff contains no file under
`crates/qcore-verifier-host/`, and the other three constraint-16 rows passed in
the package run itself.

### 10.2 One expected acceptance-matrix drift, left for the integrator

`scripts/tests` asserts the acceptance matrix's derived `lean_audits` row, which
must grow 85 → 86 now that this card registers an audit file. Acceptance files
are integrator surface and are not this card's to edit; reported here rather
than worked around, exactly as waves 18 and 19 did for the same row.

## 11. Named follow-ons

| Name | What is open |
|---|---|
| `NATIVE-LIBRARY-HEADER` | reading the library's real exported `.h` instead of the manifest's transcription of it. **Not this card**, and this card sharpens what it needs: the manifest now carries *two* kinds of header spelling, including the pointer spellings, and the reader must reproduce the §C04 projection for all of them without a typed Clang AST (constraint 8's reader discipline) so that W4-style rederivation reproduces the decision. Until it lands, that the manifest's transcription matches the header is a premise — and it is now a premise that pointers rest on, which raises its value without changing its kind. |
| `NATIVE-LIBRARY-SOURCE-BINDING` | binding a manifest from the *source* file (`#import std.buffer;`) rather than resolving by symbol name across the selection. Unchanged by this card and still needing a registry name or a new form, so still a card with a constraint-14 census. |
| `NATIVE-LIBRARY-MULTI-HASH` | hash algorithms beyond `sha256`. Unchanged; still refused rather than carried opaquely. |
| `UNSAFE-LINKAGE-ARTIFACT-PROBE` | a builder-side observation comparing a real artifact against the pin. Unchanged, and still an observation that could never be more. |
| `UNSAFE-LINKAGE-CLI` | narrowed again: `qcorec` selects manifests, and now also *renders* the L3 premise, but still selects no **contract environment** — so the L3 premise is unreachable through that binary until this lands (§3). |
| `NATIVE-LIBRARY-SLICE-SHAPE` | §4.4's "for the same resource shape" in the closed-custody check, which is vacuous while §C04 projects every resource to one shape. It becomes real work with the buffer-extent follow-on (design §12 stage (b)). |
| `NATIVE-LIBRARY-CERT-REPLAY` | design §4.3's `Q027`-class rederivation refusing a replay whose *manifest* or artifact hash differs from what the certificate names. The premise is on the certificate today, so a reader can detect a substitution; making the recheck itself refuse is a distinct step and is not claimed here. |

Buffer extents, struct/string/callback transport, sentinel adapters, extern calls
in concurrent or error-region contexts, target-aware scalar projection, dynamic
loading and verifying the link step all stay out of scope exactly as they are for
level L1.

## 12. Integrator reconciliation

1. **Diagnostic band `Q047`–`Q049`** is claimed for a *second* `NativeLibrary`
   range. If another wave-20 card claimed any of the three independently, relocate
   this **unshipped** band per `T07`/`T09`; the mapping is stable under a shift.
   Sites: `docs/QCORE_SPEC_r0_3.md` §L21 row and §L14.3,
   `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.2,
   `crates/qcore-frontend/src/frontend.rs` (`SemanticPass::NativeLibrary` doc),
   `crates/qcore-verification/src/driver.rs` (four emission sites).
2. **`docs/reviews/r0_3/acceptance_matrix.json`** is integrator surface and is not
   edited here. `NATIVE-LIBRARY-CONTRACT-SLICE` should be retired from
   `named_followups` and the six follow-ons of §11 entered
   (`NATIVE-LIBRARY-SLICE-SHAPE` and `NATIVE-LIBRARY-CERT-REPLAY` are new).
   The derived `lean_audits` row must grow by one
   (`AuditNativeLibraryContract`), which is the expected `scripts/tests` drift
   below.
3. **Ledger.** `AGENTS.md`'s headline test/Lean numbers and `DEVELOPMENT.md`'s
   increment record are integrator surface and are not edited here.
4. **`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`'s "Landed since" note** was
   updated in place, as that document's own §0 discipline provides for. Stage (a)
   of §12 is now landed but for the header reader; §6's `std.buffer` and stages
   (b)–(d) remain untouched.
5. **No provider rebuild.** `qcore-schemas` is 14.0.0 and no manifest template
   changed, so constraint 18 is not engaged.
