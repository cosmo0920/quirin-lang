# LOADER-ABI — the declared contract's ABI types, compared against the C declaration — 2026-09-10

The named continuation of card **NEIGHBOURS**
([`docs/reports/remaining-neighbours/README.md`](../remaining-neighbours/README.md)
§2.6), which could not declare `flb_time_now` faithfully in the v1 vocabulary
and, in passing, recorded what that exposed:

> the loader never compares a fixture's declared ABI type against the C prelude.

This card is the gate. **A declared contract whose parameter or result ABI
projections the analysed translation unit's own C declaration contradicts is
withdrawn for that unit**, at every call site, with a named keyed refusal — and
the sweep that gate forces over the existing declarations is where most of this
card's content is.

**Base tree:** main `8069a99` ("Consolidate wave 15: corpus re-pin, follow-ons,
ledger"), submodules initialised, `third_party/libevent` at `d82464a2`.

---

## Headline

**The fail-open is real, it is reproduced, and it is closed** — and closing it
found **eight unfaithful declarations**, one of them in the production contract
corpus and seven in the gateway test fixtures. One of those seven was
load-bearing for five cards' published fixture results, which are corrected
here rather than preserved.

| | |
|---|---|
| The fail-open | a contract may say `-> unit` for a C function returning `void *`, or name its parameters in the other order, and the analyzer applies the custody modes positionally anyway. §1 reproduces both, with a `Safe` verdict that hides an acquisition. |
| Where the gate sits | **bind time, per translation unit** (§2), reading the C declarations from the same preprocessed token stream the importer projects and the declared spellings from the environment's own retained texts. |
| What a disagreement does | withdraws the declaration for that unit: every call site is rewritten to a marked uncontracted callee refusing with a named key, and the name leaves the environment the derivation seeds. §2.2. |
| Production findings | **one**: `flb_sds_create_len`'s `len` was declared `usize`; C declares `int len`. Corrected against the read upstream body. §4.1. |
| Fixture findings | **seven**, in three families: five `const char *` parameters declared `ptr<mut, u8>`, one `double` result declared `u64` (NEIGHBOURS' own recorded instance), and one variadic callee declared at a call site's arity. §4.2. |
| Cards corrected | **O04-ARMS**, **O04-B08**/**O04-B09B10**/**O04-READ-BOUND** and **O04-TRACE** each published a fixture result that rested on the three-parameter `flb_error` declaration. Every one of those results is re-measured and re-pinned here, with the load-bearing halves kept. §5. |
| Corpus | 185 units re-measured through the canonical pinned route. See §6. |
| Evidence | 13 new tests in a held-out `k_*` family (33 declared/C rows decided one by one) plus one new gateway control, 38 audited Lean theorems in a new `QCore.DeclaredAbiAgreement` namespace, no new axiom. |

**Nothing is promoted.** Every `Safe` in this card's own family is `Safe` for a
synthetic declared custody projection under premises the fixture names; no raw
Cycle 19 unit moves.

---

## 1. The fail-open, reproduced

`qcore_contract::contract_loader` checks a declaration's ABI spellings against
its own closed tables — `SCALAR_ABI_TYPES`, `POINTER_ABI_TYPES`, `unit` — and
against **nothing else**. The spellings are then **discarded**:
`ResourceFunctionContract` carries custody modes and a result mode, and no type
at all. Nothing downstream could compare them even if it wanted to.

That is not a documentation problem, because the loader's *admissibility rules
are keyed on the types it discards*:

* a custody mode (`resource_owned`/`consumed`/`released`/`out`/`stored`/
  `removed`) is admitted only on a **pointer** parameter;
* `resource_out` additionally requires exactly `ptr<mut, ptr<mut, u8>>`;
* a non-`None` result mode requires a **pointer** return type.

So an unfaithful spelling is exactly what buys a custody mode the real C type
could not carry. Two reproductions, both in
[`crates/qcore-c-gateway/tests/loader_abi.rs`](../../../crates/qcore-c-gateway/tests/loader_abi.rs):

### 1.1 A hidden acquisition, and a `Safe` that hides it

```qcore
extern fn k_alloc(size: usize) -> unit
contract { requires resource_contract_v1;
           ensures normal resource_borrowed(size);
           ensures normal resource_return_none(); };
```

against the C the unit really declares, `void *k_alloc(unsigned long size);`,
over the body `void w(void) { k_alloc(8); }`. The declaration says the call
produces nothing, so the projection has nothing to account for and the unit is
**`Safe`**. Declare the callee faithfully instead and the same body is
`Unknown`, reporting a discarded owned result
(`the_faithful_result_type_is_admitted_and_the_acquisition_becomes_visible`).

**After this card** the unfaithful pair is refused:

```
call to `k_alloc` whose declared contract was withdrawn against its C
declaration in this translation unit (abi-return-type-disagreement)
```

### 1.2 A release applied to the wrong operand

```qcore
extern fn k_close(handle: ptr<mut, u8>, flags: i32) -> unit
contract { … ensures normal resource_released(handle, nonnull); … };
```

beside `void k_close(int flags, void *handle);`. The declaration's `Released`
mode lands on the integer and the real handle is at the position declared
`i32` — the modes are applied positionally, and neither position is the one C
declares. Refused now as `abi-parameter-type-disagreement`.

### 1.3 The reproduction that is deliberately *still* open

The same unfaithful `-> unit` declaration, over a unit that does **not** declare
`k_alloc` at all, still applies and is still `Safe`
(`without_a_c_declaration_the_unfaithful_result_type_still_applies`). There is
nothing to compare, and requiring a declaration is a different and much larger
gate — every callee whose prototype this reader cannot see would be refused.
Recorded as **`LOADER-ABI-UNDECLARED`** (§8), measured rather than assumed away
(§6.2), and pinned by that test so the boundary is honest instead of implied.

---

## 2. Where the comparison happens, and what a disagreement does

### 2.1 Bind time, not load time — decided by what each side can see

The loader has **no translation unit**. One contract module is loaded for 185
units whose declarations of a callee may differ, and one unit's disagreement
must not reject the environment for the other 184. The comparison must also be
reproducible by W4 summary derivation, W4 validation and
`check_legacy_safety_certificate`, which hold the preprocessed token stream and
the environment's retained trusted texts and nothing else.

Both sides are therefore read from exactly those two, which is card C02's rule:

| side | source | mechanism |
|---|---|---|
| the C declaration | the same preprocessed token stream the importer projects | a new declaration reader in `crates/qcore-c-gateway/src/legacy_declared_abi.rs` |
| the declared spellings | the environment's own retained module texts | `contract_loader::declared_abi_signatures_of`, rederived exactly as `declared_variadic_data_tails` and `literal_argument_premises` are |

The typed Clang AST (`typed_c_ast`) was **not** used, deliberately: it is not in
the certificate, so a recheck could not reproduce a comparison that rested on
it. The cost is recorded — this reader is a token reader, and every C form it
will not commit to is a refusal (§3).

### 2.2 Withdrawal, in both directions that matter

A disagreeing declaration is withdrawn **for that unit**:

* every **call site** is rewritten to `qcore_abi_withdrawn__<key>__<callee>`, so
  the contract is not applied at any of them and the statement refuses with the
  key. This is card HASH-CLAUSE's technique, chosen for its reason: the
  withdrawal then holds uniformly at every consumer of the token stream — the
  resource CFG, the C02-P untracked-base seeding, the aggregate-store bases, the
  member-address and bound-read admissions, the neutral-position reader, the
  out-alias normalizer — rather than at whichever contract lookup remembered it;
* the name **leaves the environment** the derivation seeds, so a function
  *defined* in this unit is no longer checked against, and its callers no longer
  trust, an interface its own C declaration contradicts.

The gate runs at each entry that owns an environment: in
`analyze_legacy_c_translation_unit_selected_partial` (the production and
certificate-recheck route), in `recursive_component_census`, and in
`analyze_legacy_c_tokens` (the single-function entry point). It is deliberately
**not** repeated per function inside `build_function_cfg`: every caller below
the translation-unit pass is handed the tokens and the environment that pass
already withdrew from, and re-deciding there would re-read every declared
contract module once per analysed body. An earlier implementation did exactly
that and was measured as a needless cost, not as extra coverage.

### 2.3 What withdrawal can and cannot do to a verdict

It is monotone in the refusing direction: an uncontracted call is `Unknown` and
poisons the enclosing function's state, so **no verdict can move towards
`Safe`**. Proved in Lean (`withdrawal_only_removes`,
`reading_a_first_declaration_never_admits`) and pinned in Rust
(`a_withdrawal_never_moves_a_verdict_towards_safe`).

**What it can do, measured and recorded rather than implied
(`LOADER-ABI-WITHDRAWAL-UNSAFE`):** withdrawing a callee whose declared
`Released` mode was discharging an acquisition leaves that acquisition
unaccounted, and the analyzer then reports a **provisional `Unsafe`**. That is
the existing behaviour for *any* uncontracted release and is exactly why hard
invariant 2 forbids presenting a provisional `Unsafe` as an upstream defect. No
withdrawal occurs anywhere in the pinned corpus (§6.2), so the direction is not
reachable there; the test pins it so a future one is not a surprise.

An inferred W4 summary may still be derived for a defined function whose
declared contract was withdrawn. That is sound and deliberate: an inferred
summary is published only after re-analysis validates it, and it never rests on
the withdrawn declaration.

---

## 3. The admission and refusal table

Default is refusal. Every admission direction is argued; the whole table is
decided row by row in `the_admission_and_refusal_table_decides_row_by_row`.

### 3.1 The refusal vocabulary

| key | category (§L21.1) | when |
|---|---|---|
| `abi-return-type-disagreement` | `stale-evidence` | the declared result is not an admissible projection of the C return type |
| `abi-parameter-type-disagreement` | `stale-evidence` | likewise at a parameter position |
| `abi-arity-disagreement` | `stale-evidence` | the declaration names a different number of parameters |
| `abi-variadic-disagreement` | `stale-evidence` | the declaration claims a variadic data tail for a C function that is not variadic |
| `abi-ambiguous-declaration` | `stale-evidence` | two declarations in the unit project differently |
| `abi-unspecified-parameters` | `unsupported-syntax` | the C parameter list is empty (`f()`), which specifies nothing |
| `abi-unreadable-declaration` | `unsupported-syntax` | the C declaration carries a type this reader will not commit to |
| `abi-unreadable-declared-sources` | `missing-premise` | the retained contract texts could not be re-read |

A *disagreement* is `StaleEvidence` because that is what it is: evidence bound
to a different declaration than the one under check. No refusal is typed as an
environment statement, so none of them can be read as saying the analysis could
not run (asserted in `the_refusal_vocabulary_is_closed_and_round_trips`).

### 3.2 What is admitted, and why

| admission | argument |
|---|---|
| **`unsigned long` admits both `u64` and `usize`** (and `long` both `i64` and `isize`) | on the LP64 targets audited here they are two spellings of one eight-byte reading, and the vocabulary is an *opaque ABI projection*, not a C type. Requiring one spelling would report the faithful `size_t` declarations of `malloc`, `flb_calloc`, `cio_chunk_open` … as disagreements. Measured: the first implementation did exactly that. |
| **specifier order is a multiset** | C11 6.7.2p2 lists the admissible specifier sets as multisets; a preprocessed `size_t` is `long unsigned int`. Measured: sequence matching made 42 of 115 real declarations unreadable. |
| **plain `char` admits `i8` and `u8`** | C11 6.2.5p15 leaves its signedness implementation-defined; `signed char`/`unsigned char` fix it and admit one spelling each. |
| **typedefs are resolved** | a typedef is *transparent* in C — the declared type **is** the target type — so resolving is exact rather than approximate. A typedef this reader will not read, and a name typedef'd twice to different targets, both make the declaration unreadable. |
| **array parameters adjust to pointers** | C11 6.7.6.3p7. Only an empty or plainly constant bound; `[static 4]` and `[*]` are refused. |
| **`__attribute__`/`__asm__` suffixes are stepped over** | they carry no type, and glibc's declarations of `malloc`/`free` are unreadable without this. |
| **`const`/`volatile`/`restrict` on the pointee do not change the pointer projection** | the projection is the pointer's ABI; `const` on the pointee selects `ptr<const, u8>`, and a qualifier *between* two pointer levels has no spelling here and is refused. |
| **a zero-parameter contract survives an unspecified C list** | §3.3. |

Everything else refuses: enumerations (C11 6.7.2.2p4 leaves the compatible
integer type implementation-defined), `long double`, function-pointer
parameters, three-level pointers, `const char **`, K&R identifier lists, and
every declaration form the reader does not parse.

### 3.3 The one admission that is not exact equality

A C declaration that is not a definition and whose parameter list is empty
specifies **nothing** about the parameters (C11 6.7.6.3p14), so a declared
parameter vector cannot be checked against it — refused.

One case is admitted, on a stated ground rather than by exception: a contract
that names **no** parameters and claims **no** variadic tail asserts nothing
about parameters at all, and the analyzer's existing call-site arity gate
already refuses every call that passes an actual to it
(`arguments.len() > contract.parameters.len()` with no tail). There is
therefore nothing an unspecified list could contradict. Both halves are pinned
in Rust (`a_zero_parameter_contract_survives_an_unspecified_c_list`) and proved
in Lean (`a_nullary_contract_survives_an_unspecified_list`,
`a_nullary_contract_admits_only_the_empty_call`).

The measured instance is Fluent Bit's `double flb_time_now();`
(`include/fluent-bit/flb_lib.h:94`) — an empty list, which the first
implementation withdrew in `flb_input_chunk.c`. Without this rule the card would
have withdrawn a declaration that is in fact faithful, and moved the corpus for
the wrong reason.

### 3.4 Composition of two declarations of one name

A prototype and a definition, or two prototypes, compose: an unspecified list
composes with a specified one to the specified signature (C11 6.2.7p3), two
specified readings must agree, and disagreeing results or parameter vectors are
`abi-ambiguous-declaration`. Composition never invents a third reading
(`composition_returns_one_of_its_inputs`), and any reading the reader refused
wins over one it read — the fail-closed direction.

---

## 4. The sweep: every declaration re-validated against its source

### 4.1 The production corpus — one finding

The 12 modules the Cycle 19 manifest selects — every module the pinned audit
loads, carrying all **31** `resource_contract_v1` declarations the loader admits —
were compared against the pinned upstream declarations in the prepared Fluent
Bit `66910c10` clone through the production decision itself
(`cargo run -p qcore-c-gateway --example declared_abi_census`), unit by unit,
over the whole 185-unit corpus (§6.2).

That is the whole comparable population: `compiler/tests/contract_loader.rs`
pins the corpus split at `(12, 31, 145)` — twelve selected modules, 31 v1
declarations, 145 rejected — and a module without the `resource_contract_v1`
marker is rejected by this loader rather than skipped, so it declares no ABI
projection for anything to disagree with.

The fixture sweep of §4.2 is by **execution**: every fixture contract the
workspace suite exercises against a C source that declares its callees. A
fixture declaration whose C source never declares the callee is not compared
there either — the same `LOADER-ABI-UNDECLARED` boundary — so §4.2 lists what
the comparison *could* decide, not a proof that every fixture declaration is
faithful.

**Finding P1 — `flb_sds_create_len`.**

```
contract/qcore/fluent_bit/resource_memory.qcore (before)
  extern fn flb_sds_create_len(str: ptr<const, u8>, len: usize) -> ptr<mut, u8>

include/fluent-bit/flb_sds.h:99 (pinned upstream)
  flb_sds_t flb_sds_create_len(const char *str, int len);
```

`len` is an `int`, not a size: different width **and** different signedness.
Corrected to `len: i32` against the read declaration and definition, with the
reason recorded in the module. The mode is unchanged — a scalar carries no
custody either way — so nothing the analyzer does moves; what moves is that the
declaration now says what the source says.

The other 30 declarations agree wherever a unit declares them, including the
two shapes the first implementation got wrong and that §3.2 records
(`size_t` → `usize`/`u64`, `ssize_t` → `isize`/`i64`, `flb_sds_t` → `char *`).

### 4.2 The test fixtures — seven findings, one family each

| # | fixture | declared | C says | class |
|---|---|---|---|---|
| F1 | `borrow_residuals.qcore` ×5 (`y_probe`, `y_retire`, `y_retire_late`, `y_hand_over`, `y_fill_from`) | `ptr<mut, u8>` | `const volatile void *` | parameter type |
| F2 | `o04_read_bound_generic.qcore` (`r_probe`) | `ptr<mut, u8>` | `const volatile void *` | parameter type |
| F3 | `o04_path.qcore` (`cio_chunk_open`'s `name`, `flb_routes_mask_set_by_tag`'s `tag`, `strdup`, `strlen`, `cio_log_error`'s `message`) | `ptr<mut, u8>` | `const char *` | parameter type |
| F4 | `o04_b09b10_callees.qcore` (`flb_hash_table_add`'s `key`) | `ptr<mut, u8>` | `const char *` | parameter type |
| F5 | `o04_b08_callees.qcore` (`flb_time_now`) | `-> u64` | `double` | result type |
| F6 | `o04_arms_callees.qcore` (`flb_error`) | three parameters | `(const char *, ...)` | arity |

F1–F4 are one family — a `const`-qualified pointee declared mutable — and are
corrected to `ptr<const, u8>`. The modes are unchanged, so no fixture's subject
moves. **F5 is the instance card NEIGHBOURS recorded**: card O04-B08's fixture
wrote `-> u64` beside a prelude declaring the real `double`, "acceptable in a
fixture, where its header says so". It is not acceptable to a comparison, and it
is now `-> f64` — writable because NEIGHBOURS added the float spellings.

**F6 is the consequential one**, and it is the follow-on `O04-ARMS-VARIADIC`
closing in the direction that card predicted. `flb_error` was declared at its
one call site's arity because the vocabulary then had no tail form; card
O04-TRACE added `resource_variadic_data_tail()`, and the faithful declaration is
now one parameter plus the tail — the form `flb_trace` already used.

---

## 5. What F6 cost, measured, and every pin it moved

The three-parameter declaration put the tail actual `storage->stream->name` at a
**declared `Borrowed` position**, where card O04-ARMS' chained-access admission
decided it. Under the faithful declaration it is a *tail* actual, and a
two-level chained read whose final field is itself pointer-typed is refused
there.

**This is not a regression this card introduced.** Card NEIGHBOURS had already
measured, on the *production* declarations where the real callee `flb_log_print`
carries a declared tail, that this exact statement refuses and
`flb_input_chunk_create` stays `Unknown` — recorded as the named follow-on
`NEIGHBOURS-CHAINED-TAIL`, with `neighbours.rs::
the_constructor_keeps_exactly_one_refusal_under_production_declarations` pinning
exactly one `indirect memory access` refusal on it. The O04 fixtures had been
**overstating the production result**; they now agree with it.

Every affected pin, and what was kept:

| test | moved | kept and asserted |
|---|---|---|
| `o04_arms.rs::the_logging_arm_discharges_…` → `…_stops_discharging_under_the_faithful_variadic_declaration` | 4 → 3 `Discharged` exits | the three cleanup arms still release the same acquisition site once; the logging arm still crosses **no** acquisition and is never a residual |
| `o04_arms.rs::the_routes_mask_arm_records_its_named_residual` | 2 → 3 `Unknown` exits | the two arms *carrying* sites still carry both identities `Mixed`; no exit is a `Residual` |
| `o04_arms.rs::the_upstream_link_declarations_admit_both_chains` | the probe reads the chain at `cio_log_error`'s declared `Borrowed` parameter instead of `flb_error`'s tail | still `Safe`, still both chains; a **new** test pins the tail direction refusing |
| `o04_arms.rs::substituting_a_chain_link_withdraws_…` | retargeted from the logging arm to the chain probe | four live controls, with the baseline asserted `Safe` first so none can pass by **vacuity** — measuring the old quantity would have made all four vacuous |
| `o04_arms.rs::weakening_the_…_premise_withdraws_the_arm` | retargeted to `cio_log_error`'s `message` premise | the original claim — the admission is a consequence of a *declared* contract, not of the statement's shape — measured where it is still live |
| `o04_b08.rs::the_routes_mask_arm_discharges_…` | 5 → 4 `Discharged` | C5 still settles carrying **two** released sites, which is that card's subject |
| `o04_b09b10.rs::the_success_exit_residual_narrows_to_one_statement` | 0 → 1 `INDIRECT`; 5 → 4 `Discharged` | the card's own eight are still gone, asserted by *naming* the survivor and by a substitution that removes it |
| `o04_b09b10.rs::the_logging_control_settles_every_exit` | a second labelled substitution | still one clearly labelled control, still not a claim about upstream |
| `o04_read_bound.rs` (3 tests) | 8 → 9 `INDIRECT`; 5 → 4 `Discharged`; 1 → 2 `Unknown` | the ninth is named and attributed to the `!chunk` arm, not to C6's path |
| `o04_trace.rs::the_success_exit_discharges_and_the_caller_reaches_safe` → `…_and_only_the_chained_tail_actual_remains` | the function-level `Safe` is **withdrawn** | **C6, the success exit, still discharges** — the exit that card existed to settle — escaping all three identities; five of six exits settle; withdrawing the chained actual alone restores the whole result, pinned |
| `hash_contracts.rs` (2 tests) | the pinned real-source vector gains the `INDIRECT` leaf | the four publications still decide, and none is refused as outside its premise |

O04-TRACE's headline is the largest correction: **`flb_input_chunk_create` no
longer reaches function-level `Safe` on that fixture.** It never reached it in
production — that card recorded `O04-TRACE-REACH` (production reach measured
zero) itself — and the fixture's own residual is now the production one.

---

## 6. Corpus

### 6.1 The 185-unit measurement

Freshly prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, forced-Libevent Monkey, explicit
`Ubuntu clang version 21.1.8`, provider packages rebuilt for this run), the
canonical pinned compatibility route, exit 0, zero gateway failures:

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 --clang clang --clangxx clang++
python3 providers/package.py build --output build/cycle19/provider-packages-loader-abi
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

| scope | units | Safe | provisional Unsafe | Unknown | data-only |
|---|---|---|---|---|---|
| cmetrics | 31 | 0 | 16 | 15 | 0 |
| ctraces | 17 | 1 | 2 | 13 | 1 |
| librdkafka | 94 | 0 | 10 | 81 | 3 |
| chunkio | 13 | 0 | 2 | 11 | 0 |
| fluent-bit-chunk | 1 | 0 | 1 | 0 | 0 |
| monkey-libevent | 29 | 1 | 11 | 17 | 0 |
| **total** | **185** | **2** | **42** | **137** | **4** |

**Unchanged**, and more strongly than the headline: the whole diagnostic surface
is **byte-identical**.

| artifact | against the wave-15 baseline |
|---|---|
| `resource-diagnostics.json` (11,135,336 bytes) | **byte-identical**, SHA-256 `1da0555b…` — every unit verdict, every function status and every diagnostic leaf |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical**, SHA-256 `c4f438e4…` |
| `resource-diagnostics.contracts.json` | the retained text of **exactly one** module differs — `fluent_bit/resource_memory.qcore`, this card's §4.1 correction — verified by comparing the inventory with the absolute-path keys normalised away; all eleven other modules and every non-`sources` field are identical |
| `monkey-source.json` | only the recorded **path-literal** residue: the `compile_entry` paths and the `__FILE__` expansions inside four `mk_event_*` wrappers, which carry this worktree's absolute path instead of the baseline's checkout path |
| `resource-diagnostics.source-effects.json` | 33 leaves on 3 of 96 results — the three `checked_function_contract` rows `cio_chunk_is_locked`, `cio_chunk_unlock` and `flb_input_chunk_release_lock` — all of them identity digests: the contract-environment digest (`build_digest`, `configuration_digest`, `obligation_id`, `task_digest`, `witness.task_digest`) moved because §4.1 changed a contract module's bytes — which is card G01's compile-task identity behaving as designed — plus the rebuilt `package_digest`, plus one `preprocessed_source_digest` carrying the same path literal. **No result's verdict, coverage shape or bounds moved.** |

The baseline is **not** re-pinned: no integrator consolidation is claimed here,
and the movement is one deliberate contract-text correction plus two recorded
residue classes.

**On the before-run.** No separate BEFORE measurement was taken, and the reason
is that the AFTER run supplies the same evidence more directly: the 11,135,336-byte
`resource-diagnostics.json` reproduces the wave-15 manifest's SHA-256 exactly. A
difference in this environment — a different Clang, a differently prepared clone, a
different provider build — could not have produced that digest, so the
environment parity a BEFORE run exists to establish is *measured*, not assumed.
What a BEFORE run would additionally have separated is nothing: every changed
leaf below is attributed, and the only content change is one contract module
this card edited.

Archived under [`artifacts/`](artifacts): the SHA-256 of all five produced
artifacts ([`after-digests.txt`](artifacts/after-digests.txt) — the first two
are the baseline's own digests), the one changed artifact
([`contracts.after.json.gz`](artifacts/contracts.after.json.gz)), the complete
33-leaf source-effects diff
([`source-effects-diff.txt.gz`](artifacts/source-effects-diff.txt.gz)) and the
census below ([`abi-census.txt.gz`](artifacts/abi-census.txt.gz)). The
byte-identical artifacts are not re-archived: their digests match the wave-15
manifest exactly.

### 6.2 The declared/C agreement census over the pinned corpus

The production decision itself
(`cargo run -p qcore-c-gateway --example declared_abi_census`), over the
preprocessed form of 183 of the 185 pinned units — two are absent for reasons
that are not this card's: `mk_core/mk_utils.c` and `mk_server/mk_utils.c` share
a base name and the census keys by name, and `tls_stub.c` has no entry in the
production compile database — against the production contract manifest:

| outcome | rows |
|---|---|
| `agreed` | **1,168** |
| `withdrawn` (any class) | **0** |
| `undeclared` | 12 |

**Zero withdrawals in the pinned corpus** — which is what makes §6.1's
byte-identical diagnostic surface the expected result rather than a lucky one.

The twelve `undeclared` rows are an artefact of the census tool's own filter,
not twelve uncompared call sites: the tool lists a contracted name when the
unit's text *mentions* it, and in all twelve the mention is `malloc` inside
`__attribute__((__malloc__))` or inside the longer identifier
`rd_kafka_mem_malloc` (verified by reading `lz4.c`, `lz4hc.c`, `pb_common.c`
and `common.pb.c`). No call site in the pinned corpus loses its declaration to
`LOADER-ABI-UNDECLARED`.

Two method notes, recorded rather than glossed: the census preprocesses each
unit with its **own full compile command**, while the pinned compatibility route
projects a historical option subset (`-I`, `-isystem`, `-D`, `-std=`), so the
two texts are not byte-identical; and the census measures the comparison, not
the analysis, so it says nothing about verdicts beyond §6.1.

---

## 7. Validation

Run on the final tree (Linux, rustc 1.98.0, clang 21.1.8):

| gate | result |
|---|---|
| `cargo test --workspace` | **2,652 passed, 0 failed, 8 ignored**; 13 new tests in `loader_abi.rs` plus one new gateway control in `o04_arms.rs` |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **68 audits** pass (67 → 68), including the new `AuditDeclaredAbiAgreement.lean` with **38** audited theorems; no new axiom |
| `providers` workspace | 108 tests pass; all five packages rebuilt for the corpus run |
| 185-unit pinned audit | §6.1, exit 0, zero gateway failures |

**One load-flaky failure, unrelated to this card.** Under the machine load of a
concurrent multi-agent wave,
`qcore-verifier-host::confined_process_execution::
a_confined_fixture_cannot_write_outside_the_admitted_paths` failed once with
`Io: Broken pipe (os error 32)` where it expects `Terminated`, and passes on its
own. It is the same class as the recorded `P02-FLAKE`; the assertion was **not**
weakened.

**Not run:** the Windows and macOS legs. This card's reader is
target-independent, but the LP64 admission of §3.2 is not: on an LLP64 target
`unsigned long` is four bytes, and the projection would have to say so.
Recorded as part of `LOADER-ABI-LP64` below.

---

## 8. Named follow-ons

| name | what |
|---|---|
| **`LOADER-ABI-UNDECLARED`** | a contracted name this reader finds **no** declaration for in a unit is not compared and its contract still applies (§1.3). Requiring a declaration is a different, larger gate: it would refuse every callee whose prototype this token reader cannot see — a K&R definition, a declaration built by a macro the reader does not follow — and it must be sized against a measured census, not assumed. |
| **`LOADER-ABI-WITHDRAWAL-UNSAFE`** | a withdrawal can turn a `Safe` unit into a **provisional** `Unsafe` when the withdrawn declaration was discharging an acquisition (§2.3). Unreachable in the pinned corpus today; if a future withdrawal fires there, the resulting allegation must be recorded as what it is. |
| **`LOADER-ABI-LP64`** | the scalar projection assumes the **LP64** data model of the audited targets: `long` and a pointer are eight bytes, so `unsigned long` admits `u64` and `usize`. The reader has no target, and nothing here consults one. On an LLP64 target (Windows) `unsigned long` is four bytes and the admission would be too wide; on ILP32 the pointer-width spellings would be too wide as well. The direction matters and is bounded: a projection that admits too much degrades to the *pre-card* behaviour for that position — the declaration applies, exactly as it did before this card — while one that admits too little withdraws, which is the refusing direction. Making the projection target-aware needs the compile target at the reader, which the certificate does not carry today. |
| **`LOADER-ABI-SILENT-DEFINITION`** | when the withdrawn name is a function **defined** in the unit, the definition is then analysed contract-free and produces no diagnostic of its own; only its *callers* report the withdrawal. Fail-closed (no entry custody is assumed), but silent if nothing in the unit calls it. Pinned by `a_defined_function_is_withdrawn_from_its_own_callers`. |
| **`LOADER-ABI-MODES`** | this card compares **types**. It does not compare a declared *mode* against anything the source says — nothing does, and nothing can without body-level evidence. A faithful signature with an unfaithful mode is exactly as trusted as before. |
| **`NEIGHBOURS-CHAINED-TAIL`** (existing) | admitting a chained read whose final field is pointer-typed at a *variadic tail* actual would restore the O04 fixture results §5 corrects. It is checker work with a real soundness obligation, and this is a contract-comparison card. |

---

## 9. Increment paragraph (draft, for the integrator)

> Declared/C ABI agreement (LOADER-ABI): a declared resource contract whose
> parameter or result ABI projections the analysed translation unit's own C
> declaration contradicts is now **withdrawn for that unit** — every call site
> refuses with one of eight named keys and the name leaves the environment the
> derivation seeds — closing the fail-open card NEIGHBOURS recorded, where the
> loader checked those spellings against its own table and against nothing
> else. Both sides are read from what a certificate already holds (the
> preprocessed token stream; the environment's retained texts), so W4
> derivation, W4 validation and certificate rechecking reproduce the decision.
> Withdrawal is proved monotone in the refusing direction, with the one
> non-equality admission argued rather than excepted (a nullary tail-free
> contract survives an unspecified C list, because the existing arity gate
> already refuses every call that passes an actual). The sweep it forces found
> **eight unfaithful declarations**: `flb_sds_create_len`'s `len` was `usize`
> against C's `int` (corrected in `contract/qcore`), and seven fixture
> declarations — five `const char *` parameters declared mutable, `flb_time_now`
> declared `-> u64` against `double` (NEIGHBOURS' own recorded instance), and
> `flb_error` declared at a call site's arity against `(const char *, ...)`,
> which closes `O04-ARMS-VARIADIC`. That last one was load-bearing: the fixture
> results of **O04-ARMS**, **O04-B08**, **O04-B09B10**, **O04-READ-BOUND** and
> **O04-TRACE** rested on it, and each is re-measured and deliberately re-pinned
> here with its load-bearing half kept — including the withdrawal of O04-TRACE's
> function-level `Safe`, which card NEIGHBOURS had already measured absent in
> production. The 185-unit corpus was re-measured through the canonical pinned
> route: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only unchanged,
> with `resource-diagnostics.json` **byte-identical** and every other leaf
> attributed to the one contract-text correction or to the recorded path-literal
> and package-identity residue classes; a census over 183 preprocessed pinned
> units measures 1,168 agreeing rows and **zero** withdrawals. 13 new tests in
> a held-out `k_*` family plus one new gateway control, 38 audited Lean
> theorems in a new `QCore.DeclaredAbiAgreement` namespace (68 audits, no new
> axiom), and spec §C04.1 gains the agreement
> requirement and loses a stale sentence about float types. Nothing is
> promoted. Follow-ons: `LOADER-ABI-UNDECLARED`, `LOADER-ABI-LP64`,
> `LOADER-ABI-WITHDRAWAL-UNSAFE`, `LOADER-ABI-SILENT-DEFINITION`,
> `LOADER-ABI-MODES`. Evidence: `docs/reports/remaining-loader-abi/README.md`.
