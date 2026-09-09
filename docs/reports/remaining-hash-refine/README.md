# HASH-REFINE — the literal-argument clause's two boundaries, decided — 2026-09-09

Closes the two boundaries card **HASH-CLAUSE** recorded open in
[`docs/reports/remaining-hash-clause/README.md`](../remaining-hash-clause/README.md)
§10:

* **`HASH-CLAUSE-TYPE`** — "the clause accepts a premise on *any* parameter,
  including a pointer. Only a scalar premise is meaningful in practice, but the
  directive imposed no such restriction either, and adding one would be a new
  refusal with no measured need. Recorded, not repaired."
* **`HASH-CLAUSE-LITERAL`** — "the literal vocabulary is a decimal integer with
  an optional minus. Character constants, `enum` constants, macro-expanded
  spellings and anything a call site writes as `0L`, `(0)` or `0 + 0` are all
  **refused**, which is the fail-closed direction but also means a real call
  site that passes a differently spelled zero is not covered."

**Base tree:** main `19fbaa5` ("Consolidate wave 14"), submodules initialised,
`third_party/libevent` at `d82464a2`. A parallel lane (NEIGHBOURS) touches
`contract/qcore/` in this wave; **this card changes no file under
`contract/qcore/`**, so every count below is stated relative to base **64
modules** and none of them moves.

---

## Headline

**`HASH-CLAUSE-TYPE` is decided as an admission, with an argument and with
evidence.** The clause stays available on a parameter of any declared type. The
decision is *closed*, not open-ended, because the v1 type vocabulary is: a
parameter is a `SCALAR_ABI_TYPES` projection or a `POINTER_ABI_TYPES` one, so
the only possible restriction would be "scalars only", and three measured
reasons stand against it (§2). The legitimate pointer use the card was asked to
look for exists and is now exercised end to end: a premise naming a pointer
parameter and the literal `0` bounds a declaration to the call sites passing a
**null pointer constant**.

**`HASH-CLAUSE-LITERAL` is repaired, in the applying direction, and the movement
is the point.** The declaration names an integer *constant*; the matcher
compared token vectors byte for byte, so a call site writing `0L` or `(0)` was
refused with a diagnostic that reads as a claim about a *value*. That was a
silent narrowing of the declared premise. Two erasures are now admitted —
matched enclosing parentheses and an integer suffix — each decided from the
token stream with **no evaluation whatever**; every spelling that would need
arithmetic, a base, a cast or symbol resolution keeps its refusal, with the
capability it would need named (§3).

**Two measured corrections to the recorded boundary text.** Its list of refused
spellings is wrong in one place and incomplete in another:

* **macro spellings are not refused.** The analyzer reads *preprocessed* text,
  so an object-like macro expanding to the literal has already become the
  literal before the premise sees it — it matched at base and still matches.
  What is refused is a macro expanding to something else, `NULL`'s
  `((void *)0)` among them.
* **octal is a separate refusal from "alternate base"**, and a load-bearing
  one: `010` is 8, so a leading zero must never be stripped. It is refused on
  both sides.

**No real call site newly matches.** A census of every `flb_hash_table_add`
call site in the pinned tree (§4) finds **50 call sites: 23 spelling the size
exactly `0`, 27 non-literal, and zero** spelling zero as `0L`, `(0)`, `0U`, hex,
octal, an `enum` constant or a macro. The widening is therefore a faithfulness
repair with **zero measured corpus reach**: all five 185-unit artifacts are
**byte-identical between a base-tree control run and the after run in the same
worktree** (§4.2), so this card's entire corpus effect is zero changed bytes.
That byte-identity is a real check on this change, not a coincidence: §7's Lean
theorem says the relation is a strict widening, so any movement would have had
to be an admission, and there is none.

**One deliberate loader tightening**, in service of the above: a declared
`LITERAL` may no longer carry a leading zero or spell a negative zero, so the
declared space is **canonical** — one spelling per value — which is what gives
the call-site relation a unique target. Neither spelling occurs in the corpus;
both reject the whole module.

**Nothing is promoted.** 185 units stay **2 Safe / 42 provisional Unsafe /
137 Unknown / 4 data-only**. The contract corpus stays at base's 64 modules,
`qcore-schemas` stays 11.0.0, and no crate version moves.

---

## 1. What changed, in one place each

| File | Change |
|---|---|
| `crates/qcore-c-gateway/src/legacy_literal_premise.rs` | `satisfies` — the spelling relation; `canonical_integer`, `strip_matched_parentheses`, `outer_parentheses_match`, `decimal_digits`, `INTEGER_SUFFIXES` |
| `crates/qcore-contract/src/contract_loader.rs` | the declared literal becomes canonical (two new refusals); the type decision and the spelling boundary are documented at the clause |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | §C04.5 states the type decision; **new §C04.5.1** is the normative spelling relation |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | G05's static conditions and the clause note |
| `crates/qcore-c-gateway/tests/fixtures/contracts/hash_contracts_generic.qcore` | `p_forget` — the null-sentinel declaration |
| `crates/qcore-c-gateway/tests/hash_contracts.rs` | the spelling table at a real call site, the pointer-premise tests, two moved pinned rows |
| `proof/QCore/LiteralArgumentPremise.lean`, `proof/AuditLiteralArgumentPremise.lean` | §4 and §5 of the model, 28 new audited theorems |

`contract/qcore/` is untouched. `qcore-schemas` is untouched. The production
module `fluent_bit/hash_table_publication.qcore` is untouched and still loads:
its declared literal `0` was already canonical.

---

## 2. `HASH-CLAUSE-TYPE`: the decision, and why admission

The question the card puts is whether a type restriction belongs in the loader,
decided "from the vocabulary's own semantics". The vocabulary is closed, which
makes the question finite: a parameter's declared type is one of
`SCALAR_ABI_TYPES` (`bool`, the sized integers, `isize`/`usize`) or
`POINTER_ABI_TYPES` (three `ptr<…>` projections). There are no float types. So
"restrict the clause by type" can only mean "refuse it on the three pointer
projections", and the decision reduces to: **is a literal premise on a pointer
parameter ever legitimate?**

**It is, and the case is not exotic.** C11 6.3.2.3p3: *an integer constant
expression with the value 0* is a null pointer constant. So

```qcore
requires resource_literal_argument(v, 0);
```

on a pointer parameter `v` declares exactly: *this contract was read off the arm
where `v` is null*. That is the same shape as the card's own motivating case —
a callee with two arms and a declaration read off one of them — with the
discriminator being nullness rather than a size. The gateway fixture `p_forget`
is that callee: at a null `v` it empties the table and releases nothing, and at
a non-null `v` it takes custody of `v` and frees it. The declaration models the
first arm and the premise refuses the second, exactly as the size premise
refuses `p_publish`'s copy arm.

Note the coherence with §3: after this card, `0L` also satisfies that premise —
and `0L` is *also* a null pointer constant, for the same reason. The two halves
of this card agree rather than merely coexisting.

**Three measured reasons against restricting.**

1. **It would refuse a legitimate declaration.** The null-sentinel reading above
   is not a hypothetical; it is exercised end to end by three Clang-compiled
   integration tests on a real analysed call, and by three Lean theorems.
2. **It buys no soundness.** A premise is only ever a *withdrawal*. A premise a
   pointer call site cannot satisfy — a nonzero literal, which is not even valid
   C at a pointer parameter without a cast — is vacuous in the **refusing**
   direction: it withdraws every call. That is proved in Lean
   (`an_unsatisfiable_pointer_premise_withdraws_every_call`) and measured in
   Rust (`a_vacuous_pointer_premise_withdraws_every_call`). There is no
   configuration in which admitting the clause at a pointer parameter makes a
   consumer more permissive.
3. **It would move the migration baseline in the refusing direction with no
   need.** The retired comment directive imposed no type restriction either, so
   a restriction would be new refusal surface introduced by this card alone.

**The admission is recorded, not silent.** §C04.5 now states it as a decision
with its reason; `contract_loader`'s module documentation carries the same three
points; and both directions are pinned by test, so a later change that quietly
restricts the clause fails.

### 2.1 What is still refused at the loader

Unchanged: the premise must name a parameter *this declaration declares*, and at
most one premise may name each parameter. A premise on an unknown name, a second
premise on one parameter, and every other `requires` spelling still reject the
whole module.

---

## 3. `HASH-CLAUSE-LITERAL`: the spelling-equivalence table

The declared literal names an integer **constant**. A call site may spell that
constant several ways. The relation admits exactly the spellings two *erasures*
reduce to the declared token vector, and refuses every other, because deciding
it would need a capability the premise deliberately lacks.

### 3.1 Admitted — decided structurally, no evaluation

| Spelling of the declared `0` | Erasure | Why it is the same constant |
|---|---|---|
| `0` | — | the declared spelling itself; this row is exact token equality, unchanged |
| `(0)`, `((0))`, `( 0 )` | matched enclosing parentheses | parenthesisation is grammar; a parenthesised expression *is* its operand |
| `0L`, `0l`, `0U`, `0u`, `0LL`, `0ll` | an integer suffix | a suffix fixes the constant's **type** (C11 6.4.4.1); the digit run is untouched |
| `0UL`, `0ul`, `0lu`, `0LU`, `0ULL`, `0uLL`, `0llu`, `0LLU` | ″ | the closed 23-entry suffix table, unsigned and long halves in either order |
| `(0L)`, `((0ULL))` | both | the erasures compose |

The suffix table is closed and case-exact: `u`/`U` combined with `l`/`L`/`ll`/`LL`
in either order, `ll`/`LL` never mixed. Anything outside it is a refusal, not an
interpretation.

### 3.2 Refused — with the capability each would need

| Spelling | Refused because deciding it needs | Where pinned |
|---|---|---|
| `0 + 0`, `1 - 1` | **arithmetic** | unit + integration |
| `-0` against a declared `0` | **arithmetic**: unary minus | unit + integration + Lean |
| `-(0)`, `+0` | **arithmetic** | unit |
| `0x0`, `0X0`, `0b0` | **a base** | unit + integration + Lean |
| `00`, `010` | **a base**: a leading zero is octal, and `010` is 8 — stripping it would be a *wrong* answer, not a missing one | unit + integration + Lean |
| `(long) 0`, `(void *) 0` | reaching into a **cast**'s operand | unit + integration |
| `(0) + (0)` | arithmetic, **and** the enclosing parentheses do not match | unit + integration + Lean |
| an `enum` constant, any identifier | **symbol resolution** | unit + Lean |
| `'\0'` | a **character** constant | unit |
| `0.0` | a **floating** constant | unit |
| `0i32` | an **unrecognised suffix** | unit |
| `(0, 0)` | evaluation of a **comma expression** | unit |

**Not refused, correcting the recorded text:** an object-like **macro**. The
analyzer reads preprocessed text, so `#define ZERO 0` at a call site
`p_publish(t, v, ZERO)` reaches the relation as the token `0` and matched at
base. `#define ZERO (0)` now matches too. `NULL` is the interesting one: it
expands to `((void *)0)` on this platform, a cast, and stays refused.

### 3.3 Why this is not a weakened gate

The relation is `actual == literal || canonical(actual) == Some(literal)`. The
first disjunct is the old test, unchanged, so **every call site the premise
admitted before is still admitted** — the widening is strict, proved in Lean
(`exact_token_equality_still_satisfies`, `the_refinement_only_admits`) and
pinned in Rust (`exact_token_equality_still_satisfies`).

In the other direction, admission is by identity or by denotation and there is
no third way in (`admission_is_identity_or_denotation`), and every admitted
spelling *denotes* the declared constant
(`an_admitted_spelling_denotes_the_declared_constant`). So the newly admitted
call sites are precisely the ones at which the declared premise was already
true. Applying a declaration where its own premise holds is not a widening of
the declaration; it is what §C04.5 says the clause means. The narrow matcher was
the deviation.

Two independent guards keep the erasures from becoming a numeric comparison:
`an_admitted_zero_never_satisfies_another_constant` (an admitted spelling of one
constant satisfies no declaration of another) and `a_sign_is_never_erased`.

### 3.4 The loader tightening, and why it is here

The declared side is now **canonical**: `010` and `-0` reject the whole module.

Without it the declared space would contain spellings whose base is ambiguous
(`010`) or whose value has two declared forms (`0` and `-0`), and the call-site
relation would have no unique target to reduce to. With it, "the declared
literal is its own canonical form" is a theorem
(`a_declared_literal_is_its_own_canonical_form`) rather than a caveat. Both
refusals are strictly refusing, neither spelling occurs in the corpus, and the
production module is unaffected.

---

## 4. The corpus: measured, and what it says

### 4.1 The call-site census

Every `flb_hash_table_add` occurrence in the pinned Fluent Bit clone
(`66910c10…`), with the fifth actual classified by spelling. Vendored `lib/`
and `tests/` excluded; script and raw output in
[`logs/`](logs/).

| Scope | occurrences | declaration + definition | literal `0` | non-literal | `0L` / `(0)` / `0U` / hex / octal / macro / enum |
|---|---|---|---|---|---|
| `src/` only | 16 | 1 | **5** | 10 | **0** |
| whole tree (`src`, `include`, `plugins`) | 52 | 2 | **23** | 27 | **0** |

The `src/`-only row reproduces the fifteen-site census card HASH-CONTRACTS
pinned in Lean (`the_census_has_fifteen_call_sites`,
`exactly_five_call_sites_satisfy_the_premise`,
`exactly_ten_call_sites_are_withdrawn`) exactly — five satisfying, ten
withdrawn — so the card brief's "five literal-0, ten non-zero" is confirmed at
its own scope, and this card extends it to the whole tree.

**The answer to the card's question is no.** Not one real call site spells the
size zero any way but the bare token `0`. The 27 non-literal actuals are
`sizeof(…)`, `strlen(…)`, `cfl_sds_len(…)`, casts and lowercase variable names;
none is an upper-case macro that could expand to a constant.

### 4.2 The 185-unit measurement

Canonical pinned route on the shared prepared pinned clone (Fluent Bit
`66910c10…`, explicit `/usr/bin/clang` 21.1.8), provider packages rebuilt at
this tree, exit 0, **zero gateway failures**.

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-hashrefine
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-hashrefine/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  <pin>/fluent-bit-clang-source \
  <pin>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

**Three measurements, not two.** The archived wave-14 baseline was produced from
the main checkout with that wave's provider packages; this card runs from a
worktree with freshly built packages, so an *archive → after* diff alone would
conflate this card's effect with the recorded environment residue classes. A
**control** run of the *base tree* `19fbaa5` — reached by detaching this
worktree to it, so the checkout path and the provider packages are identical to
the after run — separates them exactly. Logs:
[`logs/corpus-run.log.gz`](logs/corpus-run.log.gz),
[`logs/corpus-control.log.gz`](logs/corpus-control.log.gz); comparison
transcript [`logs/three-way.txt`](logs/three-way.txt).

| Artifact | archive → control | control → **after** |
|---|---|---|
| `diagnostics.json` | byte-identical | **byte-identical** |
| `source.json` | byte-identical | **byte-identical** |
| `chunk-contracts.json` | byte-identical | **byte-identical** |
| `contracts.json` | differs — worktree path literals only; **identical after path normalisation** | **byte-identical** |
| `source-effects.json` | differs — 41 leaves on 3 of 185 results | **byte-identical** |

**This card's entire corpus effect is zero changed bytes, in every artifact.**
That is the strongest available form of the §3.3 claim: the widening admitted no
call site anywhere in the pinned corpus, exactly as §4.1's census predicts.

The two archive → control classes are the ones every consolidation since
CORPUS-REPIN has recorded, and neither is this card's:

* `contracts.json` — the six absolute-path map keys, worktree location
  provenance. Verified by direct comparison past the recorded
  `DIFF-TOOL-CONTRACTS` blind spot: replacing the worktree path with the main
  checkout path makes the file byte-identical to the archive, so the six module
  *texts* are unchanged.
* `source-effects.json` — 41 leaves (15 added, 15 removed, 26 changed) on
  results 16, 24 and 63, itemized by an exhaustive leaf walk rather than
  sampling ([`logs/source-effects-archive-to-control.txt`](logs/source-effects-archive-to-control.txt)).
  Every one is an identity digest — `package_digest`, `build_digest`,
  `configuration_digest`, `task_digest`, `contract_sources` — or a
  `resource_contracts:<hash>` obligation identifier keyed by one. **No verdict,
  status, reason or diagnostic is among them.**

### 4.3 Headline

| Measure | wave-14 baseline | after |
|---|---|---|
| units | 185 | 185 |
| verdicts | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | **identical** |
| unit verdict moves | — | **0** |
| function status moves | — | **0** |
| source functions / projected | 4921 / 4834 | **identical** |
| `leak` / `unknown_alias` / `unsupported` | 271 / 4405 / 49920 | **identical** |
| gateway failures | 0 | **0** |

Per scope: cmetrics 31 (0 Safe / 16 Unsafe / 15 Unknown), ctraces 17
(1/2/13 + 1 data-only), librdkafka 94 (0/10/81 + 3 data-only), chunkio 13
(0/2/11), fluent-bit-chunk 1 (0/1/0), monkey-libevent 29 (1/11/17).

**The baseline is not re-pinned here.** `docs/reports/wave14-corpus/artifacts/`
remains current: this card moves nothing in it.

---

## 5. Mutation rows

Every row refuses, or decides as stated. Rows 1–12 are card HASH-CLAUSE's,
retained and re-run; the rest are this card's.

| # | Mutation | Where it is decided | Test |
|---|---|---|---|
| 1–12 | HASH-CLAUSE's twelve loader/gateway malformation rows | unchanged | `legacy_literal_premise` unit tests |
| 13 | **suffixed-literal mismatch**: `0L`, `0ULL`, `(0L)` against a declared `8` | refused — the erasures are erasures, not a numeric comparison | `an_admitted_spelling_of_one_literal_never_satisfies_another` |
| 14 | **suffix not in the table**: `0i32` | refused | `the_refused_spellings_and_their_grounds` |
| 15 | **parenthesisation depth**: `((0))`, `((0ULL))` | admitted at any depth | `the_admitted_spellings_of_the_declared_literal` |
| 16 | **unmatched parenthesisation**: `(0) + (0)`, `(long) 0` | refused — the outer pair does not match | unit + integration + `an_unmatched_parenthesis_pair_is_refused` |
| 17 | **negative literal**: `-1`, `(-1)`, `-1L`, `(-1LL)` against a declared `-1` | admitted | `a_negative_literal_admits_the_same_spellings` |
| 18 | **negative literal, crossed**: `1`, `(1)`, `1L`, `-2`, `-01` against `-1`; `-1` against `1` | refused — a sign is erased by nothing | ″, `a_sign_is_never_erased` |
| 19 | **negative zero**: `-0` against `0` | refused (arithmetic); and `-0` as a *declared* literal rejects the module | `the_refused_spellings_and_their_grounds`, `the_loader_refuses_a_non_canonical_declared_literal` |
| 20 | **octal**: `010` against `10` and against `8`; `00` against `0` | refused in both directions; `010`/`00` as declared literals reject the module | `an_octal_spelling_is_refused_rather_than_normalised`, `an_octal_run_matches_no_decimal_declaration` |
| 21 | **type decision, admitting direction**: a pointer premise at `0`, `(0)`, `0L` | applied — the null-sentinel arm | `a_pointer_premise_admits_exactly_the_null_sentinel_call` |
| 22 | **type decision, refusing direction**: the same premise at `v`, `(void *) 0`, `&t` | withdrawn, with the named refusal | ″ |
| 23 | **type decision, the premise is what refuses**: the same declaration without the premise | covers the non-null call; with it, withdrawn | `a_pointer_premise_only_ever_withdraws` |
| 24 | **type decision, vacuous premise**: a nonzero literal at a pointer parameter | withdraws *every* call, including the null one | `a_vacuous_pointer_premise_withdraws_every_call` |
| 25 | **declared-literal canonicality**: `010`, `-0`, `00`, `0L`, `0x0` as declared literals | each rejects the whole module | `the_loader_refuses_a_non_canonical_declared_literal` |
| 26 | **macro spelling** (recorded-boundary correction): a macro expanding to `0` or `(0)`; `NULL`'s `((void *)0)` | first two admitted, the cast refused | `a_macro_spelling_arrives_already_expanded` |

---

## 6. Pin updates — stated loudly

| Pin | before | after | Why |
|---|---|---|---|
| `hash_contracts.rs` `PINNED_REFUSALS[9]` (`0L`) | `withdrawn` | **`applied`** | `0L` is the declared literal `0` with a type suffix |
| `hash_contracts.rs` `PINNED_REFUSALS[10]` (`(0)`) | `withdrawn` | **`applied`** | `(0)` is the declared literal `0` in parentheses |
| `a_non_literal_size_is_outside_the_declared_premise` | `0L` in the refusal list | **removed**, and `0x0`, `00`, `(long) 0` added | the refusal list keeps every genuinely-refused row and grows by three |
| contract corpus size | 64 | **64** | no file under `contract/qcore/` is touched |
| `contract_loader.rs` `(selected, v1, rejected)` | unchanged | **unchanged** | ″ |
| `qcore-schemas` | 11.0.0 | **11.0.0** | no schema change |
| Lean audit count | 64 | **64** | an existing audit file was extended, not added |

**These are the only two moved pinned rows in the repository, and both move in
the applying direction.** That is the whole visible risk surface of this card,
and §4.2's byte-identical corpus is the measurement that bounds it. The nine
other parity rows — including `0 + 0` and `-0`, the two that would need
arithmetic — are unchanged, and the load-bearing negative half of every touched
pin grows rather than shrinks.

---

## 7. Lean

`QCore.LiteralArgumentPremise` gains sections 4 and 5 and moves **50 → 78
theorems, every one audited** (28 new); the audit count stays at **64** because
an existing audit file was extended, not added. No new axiom, no placeholder.

The relation is modelled **non-recursively** — a leading `(` run, a trailing `)`
run that must balance it, and one literal read out of the contiguous slice
between them — so every refusal row is settled by `decide`, that is, by
evaluation of the model itself, rather than by rewriting.

Four sentences, each with its non-vacuity:

* **Decided, not evaluated.** `the_read_literal_is_the_token_itself` and
  `an_admitted_spelling_returns_its_own_digits`: an admitted spelling's digit
  run and sign are *selected* from one integer token of the actual's own stream,
  never computed. `the_core_is_a_contiguous_slice` records that the token vector
  the literal is read out of is a `drop` followed by a `take` of the actual, so
  nothing is read from anywhere else.
* **Exact.** `a_declared_literal_is_its_own_canonical_form` (the declared
  spelling is the relation's fixed point — the theorem §3.4's loader tightening
  buys) and `an_admitted_spelling_denotes_the_declared_constant`. `denote`
  exists only so that this can be *stated*; the implementation never computes
  it. `denote_separates_two_constants` keeps it from being vacuous.
* **A strict widening.** `exact_token_equality_still_satisfies`,
  `the_refinement_only_admits`, `admission_is_identity_or_denotation`, with
  `the_widening_admits_more_than_exact_equality` showing the widening is not
  empty (both `(0)` and `0L` differ from the declared vector and are admitted by
  the canonical clause, not by identity).
* **It never crosses a value.**
  `an_admitted_zero_never_satisfies_another_constant`, `a_sign_is_never_erased`,
  and `an_octal_run_matches_no_decimal_declaration` — the last conditioned on
  the declaration being decided, which is exactly what §3.4's tightening
  guarantees.

Each refused spelling is audited beside the capability deciding it would need:
`arithmetic_is_refused`, `a_negated_zero_is_refused`,
`an_alternate_base_is_refused`, `an_octal_spelling_is_refused`,
`a_cast_is_refused`, `an_identifier_is_refused`,
`an_unmatched_parenthesis_pair_is_refused`.

For **`HASH-CLAUSE-TYPE`**, the premise is exercised at a parameter index the
gateway fixture gives a pointer type: `the_null_sentinel_call_is_applied`,
`the_live_pointer_call_is_withdrawn`, and
`an_unsatisfiable_pointer_premise_withdraws_every_call` — the last being §2's
reason 2 in general form.

### 7.1 One recorded model/implementation divergence

The Lean model erases an enclosing pair by *run length*; the Rust
implementation checks that the two parentheses actually match. The results
agree, because an unmatched parenthesis leaves a `(` or `)` token inside that no
rule admits, and `(0) + (0)` is exhibited refused on both sides. The divergence
is recorded in the definition's own docstring rather than hidden, and it is a
divergence in *how*, not in *what*: no admitted or refused row differs.

What is **not** proved is unchanged from the prior cards: that the upstream arms
are as transcribed, that a declared `borrowed` mode really borrows, and C's
object model. Newly: that the gateway's C tokenizer produces the token vectors
the model describes is Rust-tested (the unit table tokenizes each spelling with
the real tokenizer), not proved.

---

## 8. Tests

| Where | Count | What |
|---|---|---|
| `legacy_literal_premise` unit tests | 10 existing + **6 new** = 16 | the spelling table over real tokenizer output, the refusal table with grounds, exact-equality preservation, negative literals, octal in both directions, the macro correction |
| `hash_contracts.rs` integration | 18 existing + **6 new** = 24 (plus 2 ignored reporting aids) | the spelling table at a real Clang-compiled call site, cross-value non-admission, declared-literal canonicality, and the four pointer-premise tests |

Every HASH-CLAUSE and HASH-CONTRACTS test is retained. No test was deleted or
renamed.

---

## 9. Gate

Run on the merged card at this worktree.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2548 passed, 0 failed, 8 ignored** (154 targets), exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean — **after** the one-line pre-existing repair recorded in §10 |
| `reevaluation_probe` / `reevaluation_followup` | exit 0 / exit 0 |
| `python3 scripts/check_lean_audits.py` | **64 audits** pass (unchanged count; an existing audit file was extended) |
| `providers/` `cargo test` | **102 passed, 0 failed** |
| all five provider packages built and verified | pass |
| `python3 -B -m unittest discover -s scripts/tests` | **135 ran, 0 failures** |
| pinned 185-unit audit | exit 0, zero gateway failures (§4.2) |

One load-flaky observation, recorded and not weakened: an earlier
full-workspace run failed
`qcore-verifier-host --test execution_quota
a_forced_override_never_weakens_an_execution` once under parallelism; it passes
in isolation (13 passed) and passed in the gate run above. Its assertion was
not touched. This is a sibling of the recorded P03 confined-write flake.

Note on counts: the Python suite measures **135** tests at base `19fbaa5`,
where the wave-14 ledger records 131. This card adds no Python test; the
difference predates it.

---

## 10. Open boundaries

* **`HASH-REFINE-EVALUATION`** — the refused spellings of §3.2 stay refused.
  `0 + 0` and `-0` are the same constant as `0` by every reading of C, and a
  call site is free to write them; deciding them needs constant-expression
  evaluation, which this premise deliberately does not have. Admitting them
  would mean either evaluating in the gateway or declaring a constant-folding
  premise, and neither has a measured need: **zero** corpus call sites spell the
  size that way (§4.1).
* **`HASH-REFINE-BASE`** — `0x0` and `00` denote the declared `0` and stay
  refused for the same reason, one step smaller: reading them needs a base.
  Note that admitting them is *not* symmetric with the suffix erasure — a suffix
  leaves the digit run alone, a base reinterprets it — which is why the leading
  zero is refused rather than stripped.
* **`HASH-REFINE-CAST`** — `(void *) 0` is a null pointer constant and stays
  refused, so the null-sentinel use of §2 covers `0`, `(0)` and `0L` but not the
  spelling C programmers most often write, `NULL`. Reaching into a cast's
  operand needs the cast's target type, which this relation does not read.
* **`HASH-CONTRACTS-PAIRING`** — unchanged and still open: the loader rejects a
  premise naming a parameter the declaration does not have, but nothing can
  *require* a premise for a declaration that needs one.
* **`HASH-CONTRACTS-RETENTION`** and **`HASH-CONTRACTS-CALL-POSITION`** —
  unchanged; this card touches neither.
* **Pre-existing, repaired in passing, recorded for the integrator:**
  `cargo clippy --all-targets -- -D warnings` fails at base `19fbaa5` on
  `crates/qcore-c-gateway/tests/prod_contracts.rs:699`
  (`clippy::manual_contains`, clippy 0.1.98). The line is base content from
  wave 14's PROD-CONTRACTS card and is not otherwise this card's business; it
  is repaired in one line with the assertion text and meaning unchanged, so the
  standing gate can pass. If the parallel contract lane edits the same line,
  either version is correct.
* **Load-flaky, not touched:**
  `qcore-verifier-host --test execution_quota
  a_forced_override_never_weakens_an_execution` failed once under full-workspace
  parallelism and passes in isolation (13 passed). It is a sibling of the
  recorded P03 confined-write flake; its assertion was not weakened.
* The corpus baseline is **not** re-pinned here: the artifacts are
  byte-identical to `docs/reports/wave14-corpus/artifacts/`, so there is nothing
  to re-pin.
