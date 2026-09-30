# CG14-AGGREGATE-VALUES-AND-INITIALIZERS

Card CG14 of the [Clang gap queue](../../plans/clang-gap-cards/README.md), wave
48 round 7. It covers entries M08 (whole-record copies), M09 (initializers),
M10 (compound literals) and M11 (string initializer extents) on route G, with
evidence Q09, Q11, Q12, Q25, Q36, Q37 and Q38.

- **Rule.** Gateway rule [11f](../../agents/gateway.md#constraint-11f).
- **Lean.** `QCore.AggregateValue`
  ([`proof/QCore/AggregateValue.lean`](../../../proof/QCore/AggregateValue.lean)),
  audited in `proof/AuditAggregateValue.lean`.
- **Base.** `c374de96`, local `main` = `origin/main` (round 6 landed).
- **Prediction.** [PREDICTION.md](PREDICTION.md), frozen at `20b61647` before
  any analyzer change was committed and before any AFTER run. The rule's first
  draft had been run on fixtures only; the prediction says so.

## 1. What changed

- **The rule** ([`aggregate_value.rs`](../../../crates/qcore-c-gateway/src/aggregate_value.rs),
  route-G adapter [`legacy_aggregate_value.rs`](../../../crates/qcore-c-gateway/src/legacy_aggregate_value.rs)).
  One fallback, asked last in `expression_statement` and only for a statement
  every older route refused with the braced-initializer, indirect-access or
  unrecognised-destination refusal (hard invariant 3). The statement is
  matched to one Clang node by line and exact tokens, as rule 10s matches.
  - **Record facts.** `AggregateFacts` reads the unit's complete `RecordDecl`s
    and `TypedefDecl`s once. `class_of` resolves a type through typedef ids,
    `struct`/`union` spellings, inline unnamed members and array elements to
    scalar, pointer, custody-free, pointer-bearing or unread. Ambiguous,
    incomplete and `volatile`/`_Atomic` types are unread.
  - **M08 copies.** `P = Q` and `T l = Q;` of a custody-free record are the
    source's typed read and the destination's typed store
    (`Reader::path` with `aggregate_target`), so each side carries its own
    per-hop obligations, CG13's union reading and W43's frame for a store
    through `*`/`[]`. Each side must be a typed place or an automatic object
    of the frame. A pointer-bearing record refuses (`pointer-members`), as do
    an unread record and an operand that is a call, a conditional or a
    static object.
  - **M09 initializers.** One automatic record or array local (an inline
    unnamed record is its type) with a braced initializer is read from
    Clang's semantic `InitListExpr`. Designators are resolved there, and an
    unnamed member or element is its `ImplicitValueInitExpr` or array
    filler. Every leaf must be admitted:
    - a zero;
    - a scalar the typed reader reads (places with their obligations);
    - null, a string literal or a function;
    - a character array's string, or a custody-free record read as in M08.

    An address, a formal's or local's pointer, a pointer read, a call, a
    pointer-bearing record value or a nested literal refuses by class.
    A local that can hold a pointer goes through **rule 10v's flow**
    (`PointerStore`, owner `None`, value `null`). It keeps its older
    refusal where a place rooted at the local is bound with a target or
    unknown, every such place is forgotten afterwards, and the body
    publishes no summary. The flow sees the stale binding a loop's second
    declaration would otherwise contradict.
  - **M10 compound literals.** A literal is admitted only as a whole actual
    of `CALL(…)` or `NAME = CALL(…)` at a `Borrowed` formal of a contracted
    call. Its initializer must be admitted as in M09 and read no memory.
    The call is instantiated once (rule 10u) with the literal respelled `0`,
    since it carries no identity. A custody-taking formal
    (`literal-custody-position`), an uncontracted callee
    (`literal-callee-uncontracted`) and any other position
    (`literal-position`) keep the gate's refusal. So does every position the
    fallback does not read: a return, a store, or a pointer's initializer.
  - **M11 strings.** `char buf[N] = "…";` of an automatic character array is
    a declaration with no custody effect. Its extent (array bytes, the
    decoded literal bytes, the terminator fact) is a parity row for CG16.
    No bound is claimed.
- **Hooks.** These are thin and additive:
  - `legacy_c.rs`: the module and the last fallback of `expression_statement`;
  - `lib.rs`: the module;
  - `typed_access_path.rs`: `Reader::aggregate_target`, `StatementIndex::aggregates`,
    and the unit's `AggregateFacts` in `AstIndex`/`RetainedDeclarations`.

  **Shared items added**, for the integrator composing CG07 and CG15:
  `Reader::aggregate_target` (lifts only the scalar-type requirement of the
  next `path`, as CG11's `pointer_store_target` does),
  `StatementIndex::aggregates` and `AggregateFacts::collect` in
  `RetainedDeclarations::accept`.
  - `elaboration.rs`, `typed_place.rs`, `pointer_store.rs` and
    `call_instantiation.rs` are **not changed**. The flow guard reuses the
    existing `LegacyInstruction::PointerStore` variant with its meaning
    unchanged: a store of values with no identity into places of an
    untracked object.
- **Route T** is unchanged. Its one initializer is the two-field
  `struct timeval`, and no T consumer decides a wider one (CG01's per-consumer
  admission).
- **Parity.** `position: "aggregate-value"` rows record each consultation:
  form, class, leaves or sides, and refusal code. The guard's flow decisions
  are CG11's `pointer-store-flow` rows, with the place spelled
  `NAME (initializer)`.

## 2. Cited claims

| Claim | Outcome |
|---|---|
| M08: whole-struct copies are refused (Q37/Q38, `h->value = h->value`) | **reproduced** at base (CG00's `m08_…` control, `access_path_statements::pointer_store_and_union_still_refuse`). The fixture's record is custody-free, so it is **flipped** (§5). `struct S` copies keep their refusal: it holds pointers. |
| M09: braced and designated initializers are refused (Q11/Q12) | **reproduced**. Identity-free lists are admitted; an initial alias keeps the refusal. |
| M09: T accepts only a two-element `struct timeval` `InitListExpr` | **reproduced**, unchanged |
| M10: compound literals have no identity (Q09) | **reproduced** on G and T. G's borrowed actual is admitted; T is unchanged. |
| M11: string/array initializer extents are not recorded (Q25) | **reproduced**, with CG00's correction: G did not read a string-initialised local array at all (an unrecognised destination). Now it is a declaration with its extent recorded. |
| CG00's 17 freed-alone functions (`ae17152b`) | All 17 were **still Unknown** on the base ([rederive.json](artifacts/before/rederive.json)). By reading, 14 are outside this card's forms: by-value, return and condition positions, the address of a `TAILQ` head, and literals in returns or compound assignments. |
| CG00 attributes the four `TAILQ_FOREACH` `…_find` bodies to M08 | **correction** (by reading the expansion `for (v = (&mcluster->brokers)->tqh_first; …)`): the record-typed node is the embedded head under `&`, not a copy, so the entry is not M08. They stay Unknown, with their leaves unchanged. |
| CG00 predicts `rd_kafka_toppar_pause`/`_resume`/`rd_kafka_MetadataRequest` Safe | **not held**. `RD_KAFKA_NO_REPLYQ` reaches an uncontracted callee (`rd_kafka_toppar_op_pause_resume`), so no formal mode says the literal is only borrowed. |
| CG00's per-entry sizes (M08 290, M09 432, M10 99, M11 16) | Found again on the base by stable identity (unit, function, origin, description): 282, 427, 97, 15. The rest were lifted by cards since `ae17152b` ([entries.json](artifacts/after/entries.json)). |

## 3. Measurements

[scripts/measure.sh](scripts/measure.sh) runs the corpus arbiter
`run_pinned_audit.py … 66910c10 --jobs 4`, the system route and the tail-pause
slice with `QCORE_TYPED_PATH_PARITY`, each into a fresh `private/cg14/NAME*`
directory. Every exit is 0.
[scripts/archive.sh](scripts/archive.sh) writes [artifacts/](artifacts/).
Binaries are in [artifacts/binaries.sha256](artifacts/binaries.sha256).

| Route | BEFORE (base `c374de96`) | AFTER (`7ebbdc45`) | Moves |
|---|---|---|---|
| corpus | 608 / 10 / 4,216, `3a25c4e9…` (the baseline, byte-identical) | **616 / 10 / 4,208**, `5ebd8394…` | +8 Unknown → Safe |
| system | 41 / 2 / 191, `5a8ff057…` | 41 / 2 / 191, `07f8bef7…` | 0 (1 function loses a braced-initializer leaf) |
| tail-pause slice | 28 / 1 / 240 / 1 Pending, `99fa1363…` | 28 / 1 / 240 / 1 Pending, `ba224f12…` | 0 (7 functions: braced −7, alias −1) |

The corpus sidecars (`monkey-source.json`, contracts, chunk contracts,
source-effects) are byte-identical ([sidecars.sha256](artifacts/after/sidecars.sha256)).

**A first AFTER** (`d4f80fd7`, [artifacts/after1/](artifacts/after1/)) moved
nothing. By reading its parity rows, two defects of the draft showed:

- a declaration whose type is an unnamed record defined inline counted the
  `RecordDecl` as a second declarator;
- a literal at an uncontracted callee was reported as a position.

`7ebbdc45` fixes both, with a test for the inline union. The AFTER above is
that binary's.

**Every moving function, attributed by reading.** No Safe → non-Safe, no
Unknown → Unsafe, no leak or violation added.

- `lz4.c::LZ4_isLittleEndian` and `rdxxhash.c::XXH_isLittleEndian`:
  `const union { U32 u; BYTE c[4]; } one = { 1 }; return one.c[0];`. This is
  a custody-free inline union with a scalar leaf, the zero-filled rest, and
  a union-member read (rule 10w). Leaves: braced −1 each.
- Six callers, through the derived summaries those two bodies now publish
  (custody-free locals, so no flow guard and no summary gate):
  - `LZ4_readLE16` and `LZ4_hash5`, whose condition `if (LZ4_isLittleEndian())`
    was refused;
  - `XXH_readBE32` and `XXH_readBE64`, whose `return` was refused;
  - `XXH32_hashFromCanonical` and `XXH64_hashFromCanonical`, through
    `XXH_readBE32`/`XXH_readBE64` in turn.

**Leaves** (corpus, [compare-corpus.json.gz](artifacts/after/compare-corpus.json.gz),
per function in [leaves-corpus.json.gz](artifacts/after/leaves-corpus.json.gz)):

- unsupported −414 net: braced −304, indirect −80, the rest in call leaves
  the new summaries lift;
- unrecognised destination −3;
- `unknown_alias` **+60**. By reading, an admitted copy's per-hop obligation
  on a root the flow cannot resolve now reports the root where the indirect
  refusal stood. For example, `LZ4F_getFrameInfo`'s
  `*frameInfoPtr = dctx->frameInfo` gains `dctx` and `frameInfoPtr`.
  These are the W46 scalar-access obligations; no function's status moves
  for them.
- The two `unmodeled expression may mutate a formal resource identity`
  leaves are `LZ4_decompress_generic`'s statements, reclassified once
  `LZ4_readLE16` has a summary (still Unknown).

**Census** ([census.json](artifacts/after/census.json), first row per
statement, admission at classification):

| Form | Consulted | Admitted | Functions with an admission |
|---|---:|---:|---:|
| copy | 191 of record type (5,308 others consulted) | 76 | 61 |
| initializer | 1,645 (914 of them not a record or array) | 319 | 196 |
| string initializer | 18 | 18 | 13 |
| compound literal | 48 | 0 | 0 |

- The flow guard admitted all **564** of its decisions: no stale place
  binding reached one.
- The system census admits 1 initializer and the slice 7.

**Residuals** ([residuals.json](artifacts/after/residuals.json)): see §7.

## 4. Prediction against outcome

| Prediction | Outcome |
|---|---|
| 1. BEFORE reproduces on all three routes | **Held** (`3a25c4e9…`; 41 / 2 / 191; 28 / 1 / 240 / 1 Pending) |
| 2. No Safe → non-Safe | **Held**, on every route |
| 3. Corpus Unknown → Safe: at least 2 (the endianness probes), at most 15 | **Held: 8.** The 2 direct and 6 through summaries, a cascade the prediction did not name. The 3 replyq bodies and 3 telemetry bodies did not move: an uncontracted callee, and remaining indirect leaves. |
| 4. Unknown → Unsafe at most 3 | **Held: 0** |
| 5. Braced refusal −150 or more; indirect falls; destination −4 or more; alias attributed | **Held**, except that destination fell by 3, not 4 or more: one M11 site is static storage (CG15). Braced −304, indirect −80, alias +60 attributed above. |
| 6. System and slice: at most 2 moves each | **Held: 0 each** |
| 7. Sidecars byte-identical | **Held** |
| 8. Test flips as listed | **Held, with one more flip**: `address_of_member`'s custody-free union copy (§5) |
| 9. Ledger amended, residuals registered | **Held** (§7) |

## 5. Tests and flips

New tests:

- [`tests/cg14_aggregate_values.rs`](../../../crates/qcore-c-gateway/tests/cg14_aggregate_values.rs),
  9 tests;
- [`tests/unit/aggregate_value.rs`](../../../crates/qcore-c-gateway/tests/unit/aggregate_value.rs),
  7 tests: the three tables, their exactly-when characterisations over every
  input, the Lean vectors bound row by row, and the string decoder.

**Flips, each named in its test:**

- `access_path_statements::pointer_store_and_union_still_refuse`: the
  `h->value = h->value` row goes from Unknown (indirect) to **Safe**, with
  the certificate rechecked. A pointer-bearing `A` keeps the refusal
  (a new row).
- `address_of_member::an_aggregate_typedef_member_is_not_a_store_destination`:
  `hold->lock = hold->aliased_lock`, a copy of the custody-free union
  `a_lock_t` (`int`, `long`), goes from Unknown to **Safe**.
  `sink->packer = hold->packer` (pointer-bearing) keeps its refusal.
- CG00 `m09_braced_initializers_are_refused`: `{ .a = 1 }` goes from
  refused (braced) to **Safe**. `{ .p = q }` keeps the refusal, as do the
  two T rows.
- CG00 `m10_compound_literals_have_no_identity`: the G row goes from refused
  (braced) to **Safe**. The T row is unchanged.
- CG00 `m11_string_initializers_are_refused_or_unresolved`: both
  `char buf[] = "abc"` rows go from refused (destination) to **Safe**.
  `buf[9]` is read as any array element is, and no bound is claimed.
- CG00 `m08_aggregate_copies_are_refused`: unchanged, plus one added
  custody-free row.

**Refusal controls the brief names:**

- `copying_a_record_with_an_owned_field_then_releasing_both_is_never_safe`:
  an acquired record with an acquired member, copied and released through
  both copies, plus two formal-record variants. Each is refused
  (`pointer-members`) and never Safe.
- `a_returned_block_literal_address_is_never_safe`:
  `return &(struct S){…}`, `return (int[]){…}`, and a literal's address
  stored in a pointer that is returned. Each is Unknown.

**Other controls:**

- a copy through a released or possibly-null pointer is never Safe;
- initial aliases (address, formal or local pointer, pointer read, call),
  a static local and an unnamed pointer-bearing record with an alias are
  refused;
- a literal at a custody-taking formal, with an identity-carrying
  initializer, bound to a pointer or inside an actual is refused.

## 6. Lean

`QCore.AggregateValue`, 20 theorems, registered in `proof/AuditAggregateValue.lean`
and `QCore.lean`. There is no `sorry` and no project axiom; `lake build`
passes.

- **Field-wise copy preserves custody (aliases, never duplicate owners).**
  - `copy_adds_no_owner` and `a_copy_only_aliases`: every identity the
    destination holds after a copy, the source held.
  - `a_custody_free_copy_moves_no_identity` (with
    `custody_free_has_no_identity`).
  - `releasing_both_copies_is_a_double_release`: why a pointer-bearing copy
    refuses.
  - `the_owned_field_model_agrees`, which reuses
    `QCore.AggregateOwnedField.an_aggregate_is_never_copied`.
- **Zero-fill yields no identity.**
  - `zero_fill_has_no_identity` and `an_admitted_initializer_holds_no_identity`.
  - `admitted_leaf_denotes_no_identity`, and
    `an_unadmitted_leaf_may_carry_an_identity`: the refusal is needed.
- **A compound literal's lifetime bounds its borrows.**
  `a_literal_borrow_ends_in_its_block` and `a_literal_never_escapes`.
- **Unlisted forms are refused.** `an_unlisted_form_refuses`,
  `leaf_admitted_exactly_when`, `copy_admitted_exactly_when` and
  `literal_admitted_exactly_when`.
- **Vectors and extents.** `copy_vectors_hold`, `leaf_vectors_hold`,
  `literal_vectors_hold`, and `the_terminator_fact` (a fact, not a bound).

The Rust test `cg14_lean_vectors_match_the_rust_tables` binds `copyVectors`,
`leafVectors` and `literalVectors` to `COPY_VECTORS`, `LEAF_VECTORS` and
`LITERAL_VECTORS` row by row, reading the Lean text.

Stated, not proved:

- faithful Clang extraction of semantic initializer lists, record members,
  typedefs and types (trust boundary 28);
- that the typed reader's scalar and place reads are the custody meaning of
  a leaf's evaluation (the W43/W46 rules);
- that the reused rule-10v flow computes the guard's decision.

## 7. Follow-ups

**Amended, not retired.** `CG14-AGGREGATE-VALUES-AND-INITIALIZERS` stays
open. Its DISCHARGE covers pointer-bearing copies, and aggregate values in
every position. No follow-up was absorbed (CG00 lists none).

**Registered, 659 → 662** ([scripts/update_ledger.py](scripts/update_ledger.py)):

- `CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES`: 75 copies on 61
  functions and 99 initializer leaves that may carry an identity, plus 22
  whose scalar leaf the reader refuses.
- `CG14-AGGREGATE-VALUE-POSITIONS`: 221 of CG00's M08 leaves (by-value
  actuals, returns, conditions, 34 copies from a call or conditional) and all
  97 M10 leaves (40 literal actuals at uncontracted callees on 31 functions,
  8 other positions, the rest outside call statements).
- `CG14-DECLARATION-FORMS`: several declarators 96 / 84, static or thread
  storage 102 / 85 (CG15's), no single initializer 93 / 70.

The matrix's `dimensions.named_followups` read 658 against 659 entries on the
base (a hand edit). It is now 662, the list length.

## 8. Gate

See [gate.json](gate.json) and [gate/](gate/). The commands are those of
[scripts/gate.sh](scripts/gate.sh), with logs in `private/cg14/gate`.
