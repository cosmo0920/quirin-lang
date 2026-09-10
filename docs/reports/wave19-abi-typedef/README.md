# CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF — the declared-ABI reader's two unread typedef shapes, widened and measured — 2026-09-10

The named continuation of card CONSTRUCTOR-RESIDUALS
([`docs/reports/remaining-constructor-residuals/README.md`](../remaining-constructor-residuals/README.md)
§4.2), which measured the remaining residual of production
`flb_input_chunk_create` down to one family — three callees whose parameter
types card LOADER-ABI's declared-ABI reader would not commit to — and named the
capability **`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`** for widening that reader.

**Base tree:** main `c7c0a3c` ("Close wave-19 partial: ledger, gate numbers,
increment record"), submodules initialised, `third_party/libevent` at
`d82464a2`. Every measurement below was taken on that tree with only this
card's own diff applied, on the pinned corpus (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`,
Ubuntu clang 21.1.8), through the canonical pinned route.

---

## Headline

**The card's own premise is refuted by its own measurement, and that is the
result.** Card CONSTRUCTOR-RESIDUALS §4.2 concluded that "the complete residual
of the production constructor is now known exactly, and it is one family: three
statements whose callees have a parameter type card LOADER-ABI's reader will not
commit to." This card widened that reader, landed all three declarations, and
re-ran the production probe. `flb_input_chunk_create` did **not** move:

| | wave-19 base tree | after this card |
|---|---|---|
| `flb_input_chunk_create` diagnostics | **6** | **6** |
| its exits that `Discharge` | 5 of 6 | **5 of 6** |
| its function status | `Unknown` | **`Unknown`** — unchanged |
| `fluent-bit-chunk/flb_input_chunk.c` | provisional `Unsafe` | **provisional `Unsafe`** — unchanged |
| 185-unit headline | 2 / 42 / 137 / 4 | **2 / 42 / 137 / 4** — unchanged |
| leak diagnostics | 272 | **272** — the constraint-7 pin holds |

The probe over the production unit is **byte-identical** to card
CONSTRUCTOR-RESIDUALS' own archived `probe-after.txt` — every CFG node, every
diagnostic, every exit ([`logs/probe-constructor-after.txt`](logs/probe-constructor-after.txt),
§3.3). The `abi-unreadable-declaration` withdrawal that card measured was a
**second refusal stacked on a first one**: the importer never builds a call node
for `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write)` or
`pthread_mutex_lock(&in->metrics_chunk_lock)` at all, because the
*address-of-a-member* actual is outside the resource projection, and that is
decided before any contract is consulted. Removing the outer refusal therefore
reveals the inner one and moves nothing. The residual is **renamed, not
removed**: `ABI-TYPEDEF-ADDRESS-OF-MEMBER`.

That is not a reason the reader widening should not land. It lands on its own
merits, and it is measured:

* **Two typedef shapes move from unreadable to read** — a `struct`/`union`/`enum`
  the typedef *defines* (`typedef union { … } pthread_mutex_t;`,
  `typedef struct msgpack_packer { … } msgpack_packer;`) and a function-pointer
  typedef (`typedef int (*msgpack_packer_write)(void *, const char *, size_t);`).
  The unreadable class is **narrowed, never deleted**: aggregate values,
  untagged `enum` values, multi-declarator typedefs, inline function-pointer
  parameters and doubly-declared names all keep the refusal they had.
* **A new projection, `fnptr`**, deliberately outside `POINTER_ABI_TYPES` and
  deliberately not spelled `ptr<…>`, so the loader's two existing custody gates
  make every custody mode on it — and every owned result of it — a **load-time
  rejection**. The admission is in the *refusing* direction for exactly the
  fail-open card LOADER-ABI exists to close (§2.2).
* **The widening is decision-neutral on everything already landed, measured
  rather than argued.** Over 247 preprocessed corpus units and the base tree's
  own 14-module manifest, the declared/C ABI census is **byte-identical** before
  and after — 1,644 `agreed`, **0 withdrawn**, 16 `undeclared` (§4.1). Since the
  reader's entire blast radius is that decision (§2.5), that is the containment
  statement in full.
* **Nothing is promoted, at any level.** Across three 185-unit runs: zero unit
  status moves, zero function status moves across all 4,834 projected functions,
  zero unit `reason_code` moves, no newly surfaced provisional `Unsafe`, and the
  leak pin holds at **272**. The whole corpus movement is **two diagnostic
  leaves removed and none added**, in one function of one unit (§4.5) — and it
  was written down as a prediction before it was measured (§3.4).

**One of the three declarations is deliberately not landed.** Measured
separately (§4.4), `pthread_mutex_lock` and `pthread_mutex_unlock` are worth one
diagnostic leaf each and land; `msgpack_packer_init` is worth **zero** and does
not, on the same ground card PROD-CONTRACTS left `flb_errno_print` undeclared —
its call sites refuse on the actual before any contract is consulted, so a
declaration cannot decide them. Its reading is kept in full in §3.1 and the
follow-on is named `ABI-TYPEDEF-MSGPACK-UNDECLARED`.

---

## 1. The reader boundary, and what it really was

Card LOADER-ABI compares a declared contract's opaque C ABI projections against
the analysed translation unit's own C declaration, and withdraws the declaration
for that unit when they disagree. The comparison reads the C side from the
preprocessed token stream a certificate already holds — never from the typed
Clang AST (`AGENTS.md` constraint 8) — and resolves through `typedef`s, because
a typedef is transparent in C and resolving one is exact rather than
approximate.

Two typedef shapes it would not commit to:

| shape | what the earlier reader did |
|---|---|
| `typedef union { … } pthread_mutex_t;` | `read_typedefs` scanned from `typedef` to the **first** of `;`, `{`, `}` and required it be `;`. An aggregate *definition* hits `{` first, so the name was never recorded at all and every declaration using it was unreadable. |
| `typedef int (*msgpack_packer_write)(void *, const char *, size_t);` | the declarator ends in `)`, not an identifier, so this reader also recorded nothing. |

Both are **reader boundaries**, not disagreements: the C declaration is present
and faithful, and the vocabulary could describe it. Card CONSTRUCTOR-RESIDUALS
measured the consequence: with `msgpack_packer_init`, `pthread_mutex_lock` and
`pthread_mutex_unlock` declared, all three were withdrawn at every call site of
production `flb_input_chunk.c` with `abi-unreadable-declaration`, and it
declined to land the declarations on card PROD-CONTRACTS' rule — a premise the
comparison withdraws everywhere has no measured payoff.

The tagged-aggregate case is worth naming too, because it is the same defect and
was equally unread: `typedef struct msgpack_packer { … } msgpack_packer;` was
abandoned at its own brace, so **`msgpack_packer *pk`** — the *first* parameter
of `msgpack_packer_init`, not only its third — was unreadable as well.

## 2. What the widened reader commits to, and on what argument

`read_typedefs` now finds the terminating `;` at brace/paren/bracket depth zero
and recognises three declarator shapes; everything else keeps the behaviour it
had. `TypedefTarget` became an enum so the two new readings can be represented
without pretending to be specifier words.

### 2.1 The admission table

Card LOADER-ABI's §3.2 table decides row by row and argues every admission
direction. These rows extend it, in the same form; all of them are decided
through the production entry point in
`crates/qcore-c-gateway/tests/loader_abi.rs::the_admission_and_refusal_table_decides_row_by_row`.

| admission | argument |
|---|---|
| **an untagged `struct`/`union`/`enum` definition is an object type, so a pointer to it projects exactly as a pointer to a tagged one does** | the pointer projection reads no tag and no member: `union tag *` was already readable, and `typedef union { … } t; t *` differs from it in nothing this vocabulary can spell. The equality is proved rather than argued — `QCore.TypedefProjection.an_untagged_aggregate_projects_as_a_tagged_one`, at every pointer depth. |
| **a *tagged* definition is recorded as the specifier pair `["struct", tag]` it equals**, not as a new kind | exactness, and a measured regression avoided. A unit that carries both `typedef struct tag { … } tag;` and `typedef struct tag tag;` records two readings of one name; recording the first as a distinct "aggregate" kind would make them differ, and the reader's own duplicate rule would then mark the name unreadable and **withdraw a declaration that agreed before this card**. Proved in both directions (`a_tagged_definition_composes_with_its_plain_typedef`, `an_untagged_definition_would_not_have_composed`). |
| **an aggregate under two pointer levels is the out-slot projection; under three it refuses** | that boundary is the pointer table's own (`POINTER_ABI_TYPES` has three entries), not this card's. |
| **a pointer to function projects to `fnptr`, a spelling of its own** | see §2.2. |
| **a chain of typedefs ending at a function pointer keeps its one pointer level** | typedef transparency, exactly as for every other target. |

And the refusals this card **keeps**, each narrowing rather than deleting the
unreadable class:

| refusal | ground |
|---|---|
| an aggregate **value** parameter | this vocabulary has no spelling for an aggregate, exactly as it has none for a `struct tag` value. Refused, not folded onto an integer. |
| an untagged **`enum`** value | C11 6.7.2.2p4 leaves the compatible integer type implementation-defined — the refusal card LOADER-ABI already made for `enum tag`, now reached through the typedef too. |
| a typedef declaring **several declarators at once** (`typedef struct { … } a, *b;`) | not parsed; recorded unreadable. |
| an **inline** function-pointer parameter (`int (*cb)(void *)` written out at the parameter position) | a nested declarator, refused as every nested declarator is. |
| a name typedef'd twice to different targets | unchanged: unreadable. |
| a function-pointer typedef carrying a trailing `__attribute__`/`__asm__` suffix, or more than one pointer level in its declarator | the shape recogniser requires the body to end at the parameter list and the declarator to be exactly `* NAME`. Both were unreadable before this card and stay so; the suffix-stepping card LOADER-ABI does for *function declarations* is not replicated here, because no typedef in the pinned corpus needs it. |
| **`ABI-FNPTR-DEPTH1`** — a pointer *to* a function pointer | §2.3. |

### 2.2 `fnptr`: why a new spelling and not an existing one

The loader's ABI vocabulary had no way to write a function-pointer parameter.
Three candidates, and why two are wrong:

* **`ptr<mut, u8>`** (or any `POINTER_ABI_TYPES` entry) — **rejected.** The
  loader admits a custody mode only on a declared type whose leading word is
  `ptr`, and an owned result only on one whose spelling starts `ptr<`. Admitting
  a pointer spelling for a function pointer would therefore let a declaration
  write `resource_released(callback)` on a function designator. That is exactly
  the fail-open card LOADER-ABI exists to close — "an unfaithful type is exactly
  what buys a custody mode the real C type could not carry".
* **`u64`/`usize`** on the LP64 premise — **rejected**, although the size and
  register class do coincide on every target this analyzer audits. It would make
  a declaration of an *integer* parameter and a declaration of a *callback*
  parameter indistinguishable, and the whole point of this comparison is that
  the declared spelling is checked against the C declaration. Card LOADER-ABI's
  LP64 admission is between two spellings of *one* reading; this would be
  between two different readings.
* **`fnptr`**, a new `FUNCTION_POINTER_ABI_TYPE` — **taken.** It is not a
  `POINTER_ABI_TYPES` entry and is not spelled `ptr<…>`, so the two custody
  gates that were **already there** make every custody mode on it, and every
  owned result of it, a load-time rejection. `Borrowed` is proved to be the only
  admissible mode (`borrowed_is_the_only_mode_on_a_function_pointer`), and it is
  also the true one: a function designator is never a resource identity in this
  model, so no custody is lost by refusing to express any.

The vocabulary was widened on the condition its own note states, the one card
NEIGHBOURS met for `f32`/`f64` — *a resource API in scope that passes one*.
msgpack's `msgpack_packer_init(msgpack_packer *, void *, msgpack_packer_write)`
is that API, and production `flb_input_chunk_create` calls it.

The comparison is exact in both directions: a `fnptr` declaration is refused for
every C type that is not a pointer to function, and every other spelling is
refused for one that is (four table rows, plus
`only_a_function_pointer_spelling_is_admitted`,
`no_object_pointer_admits_a_function_pointer` and
`no_arithmetic_projection_admits_a_function_pointer` in Lean).

### 2.3 Two boundaries this card names rather than hides

* **`ABI-FNPTR-OPAQUE`.** `fnptr` does not distinguish two function pointers of
  different signatures. That opacity is the *vocabulary's*, not this reader's —
  `ptr<mut, u8>` does not distinguish two object pointers of different pointee
  types either — and nothing can be bought with it, because the spelling carries
  no custody. A full parameter-list structural comparison has nowhere to be
  written: the declared side has no syntax for one. Pinned by a table row
  (`a different function signature agrees with the same spelling`).
* **`ABI-FNPTR-DEPTH1`.** A pointer *to* a function pointer refuses. It is an
  object pointer and would project into a custody-bearing spelling; no
  declaration in scope needs it, so it stays refused rather than admitted
  untested. Proved at every depth and through an alias.

### 2.4 What did **not** change

* **The refusal vocabulary is still exactly eight keys.** This card adds no
  refusal class and no diagnostic code: it moves declarations *out* of
  `abi-unreadable-declaration`, and §L21's per-code ownership table is untouched
  (constraint 1).
* **`Q033`'s unchecked-linkage table is untouched.** `qcore_core::extern_call`
  carries its **own** `AbiType` parse table, already deliberately narrower than
  §C04.1 — it does not parse `f32`/`f64` either, which card NEIGHBOURS added —
  and a spelling absent from it is refused by `Q033`, the refusing direction. So
  widening §C04.1 cannot widen what crosses an unchecked boundary. The only
  change made there is a doc comment that claimed to be "complete" and had not
  been since NEIGHBOURS (§6.1).
* **Nothing moved onto the typed Clang AST** (constraint 8), nothing touched W4
  summary validation (constraint 4), the typed scalar-field whitelist
  (constraint 5), the `report_leaks` pre-pass (constraint 6), the publication
  refusals (constraint 10) or the `#line`/identity transport (constraints
  11–12).

### 2.5 Where the widening can and cannot reach

`read_typedefs` and the projection functions it feeds are **private to
`crates/qcore-c-gateway/src/legacy_declared_abi.rs`**, and their only consumer
is `read_c_declarations` → `AbiAgreement::read`. The entire blast radius of this
change is therefore the `AbiOutcome` decided per (unit, contracted name) — which
is what §4.1's control census measures directly. Both sides of the comparison
still read only the preprocessed token stream and the environment's retained
texts, so W4 derivation, W4 validation and `check_legacy_safety_certificate`
reproduce the widened decision by construction; nothing was moved onto the typed
Clang AST (constraint 8).

---

## 3. The three declarations, read against the bodies

### 3.1 What they say, and why

Three declarations were written and read against their sources. **Two are
landed**, in
[`contract/qcore/libc/pthread_mutex.qcore`](../../../contract/qcore/libc/pthread_mutex.qcore)
— entirely resource-contract v1 and manifest-selected. **The third is not**
(§3.4, §4.4); because it is not in the tree, its declaration is preserved
verbatim below so the reading survives the decision.

| declaration | C read at the pinned revision | custody, and the ground |
|---|---|---|
| `pthread_mutex_lock(mutex: ptr<mut, u8>) -> i32` | `extern int pthread_mutex_lock (pthread_mutex_t *__mutex) __THROWNL __nonnull ((1));` (`/usr/include/pthread.h:794`) | `resource_borrowed(mutex)`, `resource_return_none()`. The call allocates, frees and reallocates nothing; the caller owns the storage on both sides, so its obligations are unchanged. |
| `pthread_mutex_unlock(mutex: ptr<mut, u8>) -> i32` | same shape, `/usr/include/pthread.h:835` | identical, for the identical reason |
| `msgpack_packer_init(pk: ptr<mut, u8>, data: ptr<mut, u8>, callback: fnptr) -> unit` | `inline void msgpack_packer_init(msgpack_packer* pk, void* data, msgpack_packer_write callback) { pk->data = data; pk->callback = callback; }` (`lib/msgpack-c/include/msgpack/pack.h:130-134`) | all three `resource_borrowed`, `resource_return_none()`. The body writes two members of `*pk` and does nothing else. Storing a pointer inside an object the caller already owns moves no obligation — the caller must still release `data` exactly as before — which is what card CONSTRUCTOR-RESIDUALS §4.1 settled for `mk_list_add` on the same ground. |

`pthread_mutex_t` is `typedef union { … } pthread_mutex_t;`
(`bits/pthreadtypes.h:67-72`), an aggregate the typedef *defines*.
`msgpack_packer` is `typedef struct msgpack_packer { … } msgpack_packer;`, a
*tagged* one — equally unread before this card, so the **first** parameter of
`msgpack_packer_init` was unreadable too, not only its third.

`msgpack_packer_new`, which does acquire, is deliberately **not** declared: no
residual this card measured reaches it, and PROD-CONTRACTS' rule is that a
premise with no measured payoff is not landed. That rule then reached
`msgpack_packer_init` itself (§4.4). Its declaration, unlanded, was:

```qcore
extern fn msgpack_packer_init(pk: ptr<mut, u8>, data: ptr<mut, u8>, callback: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(pk);
    ensures normal resource_borrowed(data);
    ensures normal resource_borrowed(callback);
    ensures normal resource_return_none();
};
```

It loads, it is `agreed` against the C declaration in both units that mention it,
and `qcorec check` accepts it. Nothing about it is wrong; it simply buys
nothing today.

### 3.2 The gates these declarations had to survive

* **The declared/C ABI comparison** (constraint 8, `LOADER-ABI`). All three are
  `agreed` in **every** unit of the pinned corpus that mentions them, with
  **zero** withdrawals of any class (§4.1).
* **The LOADER-ABI-MODES duty gate** (constraint 8, second half). This gate
  compares a declared custody duty against a *validated W4 summary derived from
  the callee's own body in the analysed unit*, and constraint 8 is explicit that
  nothing is withdrawn where W4 derives no summary — absence of evidence is not
  evidence. Neither declaration reaches it, for two different reasons worth
  keeping apart. The two pthread entry points are defined in another translation
  unit, so there is no body at all. `msgpack_packer_init` *is* defined in the
  preprocessed text — that is exactly how the ABI reader reads its signature —
  but the definition is `inline` in `pack.h`, and the audited scope
  `LegacyFunctionScope::MainSourceDefinitions` analyses "only functions whose
  definitions occur in the input source", so no body of it is projected and no
  summary is derived. The declarations are applied on their own authority as
  premises, which is what §3.1 reads them against the sources for.
* **The custody gate on `fnptr`.** `resource_borrowed(callback)` is not a
  choice among modes: it is the only mode the loader admits on an `fnptr`
  parameter, because both gates key on the leading word `ptr`. It is also the
  true one.

### 3.3 What they are worth at the constructor: nothing, measured

Card CONSTRUCTOR-RESIDUALS §4.2's experiment declared these three names
*without* widening the reader, and saw:

```
L19628  Unknown: call to `msgpack_packer_init` whose declared contract was withdrawn
        against its C declaration in this translation unit (abi-unreadable-declaration)
L19637  Unknown: call to `pthread_mutex_lock`  … (abi-unreadable-declaration)
L19639  Unknown: call to `pthread_mutex_unlock` … (abi-unreadable-declaration)
```

With the reader widened *and* the declarations landed, the same probe over the
same production unit reports, at the same three lines:

```
L19628  n15   Unknown: indirect memory access is outside the resource projection | msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);
L19637  n5    Unknown: indirect memory access is outside the resource projection | pthread_mutex_lock(&in->metrics_chunk_lock);
L19639  n3    Unknown: indirect memory access is outside the resource projection | pthread_mutex_unlock(&in->metrics_chunk_lock);
```

— which is exactly what the **base tree** reports, and the whole probe is
byte-identical to card CONSTRUCTOR-RESIDUALS' archived `probe-after.txt`:

```
$ diff docs/reports/remaining-constructor-residuals/logs/probe-after.txt \
       docs/reports/wave19-abi-typedef/logs/probe-constructor-after.txt
$ echo $?
0
```

The importer lowers these statements to an `Unknown` **instruction**, not to a
`Call` node: the actual `&ic->mp_pck` is the address of a member of a tracked
identity, and `&in->metrics_chunk_lock` the address of a member reached through
a parameter. Neither is in the resource projection, and that is decided by the
importer before any contract lookup happens — which is why a declaration cannot
help and why the ABI withdrawal was never the binding conjunct. **The correct
statement of the constructor's remaining residual is therefore
`ABI-TYPEDEF-ADDRESS-OF-MEMBER`**, and card CONSTRUCTOR-RESIDUALS §4.3's
attribution experiment (replacing the three statements with `(void) 0` in
preprocessed text, after which the constructor reaches `Safe`) is unaffected: it
measured that these three *statements* are the blocker, which remains true. What
it did not measure is *which* refusal binds at them.

### 3.4 What they are worth corpus-wide

Their reach is real — the declarations are in force at 234 census rows across
116 units (§4.1) — but the shape of the actuals decides what that buys:

| callee | `&member` | `&plain name` | plain name | prototype/other |
|---|---|---|---|---|
| `pthread_mutex_lock` | 21 | 1 | 0 | 116 |
| `pthread_mutex_unlock` | 29 | 1 | 0 | 116 |
| `msgpack_packer_init` | 4 | 0 | 2 | 4 |

([`callsites.py`](callsites.py) over the preprocessed corpus; "prototype/other"
is dominated by the one declaration each unit's `pthread.h` contributes.) Every
`&member` site refuses on the actual exactly as the constructor's do.

That leaves a very short list, and it is worth writing down *before* the corpus
runs rather than after, because it bounds what they can possibly show.
[`candidates.py`](candidates.py) enumerates every call statement of the three
callees whose first actual the resource projection could model at all — a plain
name, or the address of a plain name — across all 247 preprocessed units. There
are **four**, in three files:

| unit | site |
|---|---|
| `flb_input_chunk.c` (twice: the production database's entry and the Monkey database's) | `msgpack_packer_init(pk, data, callback);` — inside msgpack's own `inline msgpack_packer_new`, a **header** definition |
| `mk_scheduler.c` | `pthread_mutex_lock(&mutex_worker_exit);` |
| `mk_scheduler.c` | `pthread_mutex_unlock(&mutex_worker_exit);` |

Every other site in the corpus — all 54 `&member` calls, the constructor's three
included — is refused on its actual before any contract is consulted. So the corpus movement these two declarations can produce is confined
to at most `monkey-libevent/mk_scheduler.c`, plus a header-defined function in
`fluent-bit-chunk/flb_input_chunk.c` that the audited scope does not project in
the first place — `LegacyFunctionScope::MainSourceDefinitions` analyses "only
functions whose definitions occur in the input source", and
`inline msgpack_packer_new` is defined in `pack.h`. That unit is in any case
already proved byte-identical by §3.3's probe.

Both remaining sites are in one function, `mk_sched_worker_free`
(`mk_server/mk_scheduler.c:146`), whose body also contains
`mk_plugin_exit_worker()`, `mk_vhost_fdt_worker_exit()`, `mk_cache_worker_exit()`,
`pthread_self()`, three `pthread_getspecific()` results handed to `mk_mem_free`,
an `abort()` arm and an indirect `&ctx->workers[i]` — every one of them a refusal
this card does not touch. **The falsifiable prediction is therefore: at most two
diagnostic leaves move, both in that one function, and no function status and no
unit status can flip in either direction.** §4 reports what actually happened; a
prediction written before the measurement is only worth anything if the
measurement is allowed to contradict it.

**Verdict, applying PROD-CONTRACTS' rule to the measurement rather than to the
argument** (§4.4 measures each declaration separately):

| declaration | payoff | landed |
|---|---|---|
| `pthread_mutex_lock` | 1 leaf | **yes** |
| `pthread_mutex_unlock` | 1 leaf | **yes** |
| `msgpack_packer_init` | **zero** | **no** — `ABI-TYPEDEF-MSGPACK-UNDECLARED` |

`contract/qcore/msgpack/packer.qcore` was written, checked, loaded, measured and
then **deliberately not landed**. It is applied rather than withdrawn — the ABI
comparison agrees with it in both units that mention it — and it still decides
nothing, because every one of its call sites refuses on the actual before any
contract is consulted. That is the same ground on which card PROD-CONTRACTS left
`flb_errno_print` undeclared, and the reading above is kept in full for the day
that blocker moves, exactly as PROD-CONTRACTS kept its own.

---

## 4. Corpus — before, after, every leaf attributed

### 4.1 The containment census: the reader widening, isolated

The reader's whole blast radius is the `AbiOutcome` decided per (unit,
contracted name) (§2.5), so that decision can be measured directly and
exhaustively rather than inferred from a verdict diff. The production decision
itself (`cargo run -p qcore-c-gateway --example declared_abi_census`) was run
over **247 preprocessed units** (159 from the Fluent Bit production database,
88 from the forced-Libevent Monkey one; 240 of them mention a contracted name at
all) — the pinned corpus's own compile commands, each
unit preprocessed with its own full command
([`preprocess.py`](preprocess.py); this is card LOADER-ABI's §6.2 method, and
carries its recorded caveat that the pinned compatibility route projects a
narrower historical option subset) — in three configurations:

| run | reader | manifest | `agreed` | `withdrawn` | `undeclared` |
|---|---|---|---|---|---|
| A [`census-base-reader.txt.gz`](logs/census-base-reader.txt.gz) | base `c7c0a3c` | base, 14 modules | 1,644 | **0** | 16 |
| B [`census-widened-reader-base-manifest.txt.gz`](logs/census-widened-reader-base-manifest.txt.gz) | **widened** | base, 14 modules | 1,644 | **0** | 16 |
| C [`census-widened-reader-new-manifest.txt.gz`](logs/census-widened-reader-new-manifest.txt.gz) | **widened** | this card's, 16 modules | 1,878 | **0** | 16 |

**A and B are byte-identical** (`diff` exits 0 over all 1,660 rows). That is the
containment statement measured, not argued: on everything the base tree
declares, the widened reader decides exactly what the base reader decided, in
every unit, for every contracted name. It is the Lean sentence
`the_widening_only_adds` instantiated on the production corpus.

C adds exactly 234 rows and no withdrawal of any class: 116 `pthread_mutex_lock`,
116 `pthread_mutex_unlock` and 2 `msgpack_packer_init`, all `agreed`. The 16
`undeclared` rows are unchanged and are the artefact card LOADER-ABI §6.2
already recorded — the census lists a contracted name when the unit's *text*
mentions it, and these are `malloc` inside `__attribute__((__malloc__))` or
inside the longer identifier `rd_kafka_mem_malloc`.

### 4.2 The route, and what it cost to get right

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output <worktree>/build/cycle19/clang-pin --clang /usr/bin/clang --clangxx /usr/bin/clang++
python3 -B providers/package.py build --output build/cycle19/provider-packages-abi
sh docs/reports/wave19-abi-typedef/run-audit.sh <before|after>   # the canonical pinned route
```

Three details of this measurement are on the record because each cost a run:

1. **The pinned route reads `<source>/build/compile_commands.json`, which the
   preparation script does not create.** It configures the same pinned tree into
   `fluent-bit-clang-production`, whose entries name the pinned source files and
   that build's own generated headers. That database is copied into
   `<source>/build/` unchanged, and the BEFORE and AFTER runs read the same
   bytes. Recorded so the route is reproducible, not because anything was
   projected differently.
2. **An uninitialised submodule fails the pin gate at the *end* of the run, not
   the start.** The first BEFORE attempt analysed the five scopes that precede
   `monkey-libevent` — reproducing the wave-17 baseline exactly, `cmetrics` 0/16/15,
   `ctraces` 1/2/13 + 1 data-only, `librdkafka` 0/10/81 + 3 data-only, `chunkio`
   0/2/11, `fluent-bit-chunk` 0/1/0 — and then failed
   `Pin(SourceRevisionMismatch { pin: "libevent", expected: "d82464a2…", actual:
   "<the worktree HEAD>" })` before writing any artifact, because
   `third_party/libevent` was not checked out and the pin check read the
   enclosing repository's HEAD instead. That is invariant 5 behaving exactly as
   designed — a missing pinned input is a refusal, not a silent pass — and it is
   worth knowing that it costs the whole run rather than the first second of it.
   `git submodule update --init --recursive` fixed it.
3. **Two runs were OOM-killed mid-`librdkafka`** under concurrent load from
   sibling agents on the same machine (swap fully consumed). A killed run leaves
   no exit marker and a half-written artifact set; both were **deleted
   unmeasured**, and nothing truncated was diffed.

### 4.3 The BEFORE run reproduces the frozen baseline exactly

The BEFORE measurement was taken on the **detached base tree `c7c0a3c`**, with a
binary built from that tree and executed directly, so no part of this card's diff
— source, contract module or manifest — was on disk while it ran. It reproduces
the frozen wave-17 archive
([`docs/reports/wave17-corpus/artifacts/`](../wave17-corpus/artifacts)) **exactly**,
in every number the archive carries:

| measure | wave-17 frozen archive | this card's BEFORE run |
|---|---|---|
| units | 185 | **185** |
| unit status | 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4 data-only | **identical** |
| projected functions | 4,834 | **4,834** |
| function status | 149 `Safe` / 84 `Unsafe` / 4,601 `Unknown` | **identical** |
| `leak` | **272** | **272** |
| `unknown_alias` | 4,405 | **4,405** |
| `unsupported` | 49,835 | **49,835** |

Per library, against the baseline's own table: cmetrics `0/16/15`, ctraces
`1/2/13` (+1 data-only), librdkafka `0/10/81` (+3), chunkio `0/2/11`,
fluent-bit-chunk `0/1/0`, monkey-libevent `1/11/17` — **six for six**. Exit 0,
zero gateway failures.

And it is stronger than the counts. Against the wave-17 manifest's own SHA-256
records, the BEFORE run reproduces **the verdict artifact byte for byte**:

| artifact | wave-17 manifest | BEFORE | |
|---|---|---|---|
| `resource-diagnostics.json` (11,124,610 bytes) | `412905bb…` | `412905bb…` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` (4,186,178 bytes) | `c4f438e4…` | `c4f438e4…` | **byte-identical** |
| `resource-diagnostics.contracts.json` | `529525b5…` | `a61d7499…` | differs: the map is keyed by absolute path and the baseline was produced in a different worktree (`agent-aa9f079f…`) |
| `monkey-source.json` | `8c40b21c…` (1,665,474 B) | `765cf20c…` (1,664,660 B) | differs by 814 bytes: the recorded **path-literal provenance** class — `compile_entry` paths and `__FILE__` expansions carrying this worktree's absolute path |
| `resource-diagnostics.source-effects.json` | `8c495470…` | `feeb7a35…` | differs: the recorded **identity digest** class (build/configuration/task digests, which card G01's compile-task identity binds to the checkout path) |

Run through `docs/reports/corpus-repin/diff_artifacts.py` (constraint 22; the
per-card diffs live under [`artifacts/`](artifacts) —
[`diff-wave17-to-before.json.gz`](artifacts/diff-wave17-to-before.json.gz) here
and `diff-wave17-to-final.json.gz` in §4.5 — and are outside the archived
consolidation chain, `DIFF-TOOL-CARD-DIFFS`), the same picture is itemised:

```
--- chunk-contracts: NO DIFFERENCES
--- diagnostics:     NO DIFFERENCES
--- contracts:       sources 14 -> 14 (+0 -0 ~0, keyed by module path below contract/)
                     manifest modules 14 -> 14 (+0 -0 ~0); audit_selection 5 -> 5; keys {}
--- source:          envelope key `compile_entry`; 4 wrapper rows changed
--- source-effects:  3 result rows changed
```

Reading the **normalised and raw keys together**, as constraint 22 requires:
under normalisation `contracts.json` moves by *nothing at all* — same fourteen
modules, same manifest text, same audit selection, no collisions — and the raw
keys differ only in the checkout root they embed. The whole `contracts.json`
digest difference is that root.

Every unit verdict, every function status and every diagnostic leaf therefore
reproduces *exactly*, and the three artifacts that move are the two residue
classes waves 9, 10, 15, 16 and 17 already record. That is the environment-parity
statement a BEFORE run exists to make, measured rather than argued: a different
Clang, a differently prepared clone or a differently built provider package could
not have produced `412905bb…`.

### 4.4 The three-declaration run, which is the experiment that dispositioned them

A second run was taken at the branch with **all three** declarations landed
(`pthread_mutex_lock`, `pthread_mutex_unlock`, `msgpack_packer_init`), so that
each declaration's payoff could be measured separately rather than argued. Exit
0, zero gateway failures, all six scopes.

[`census.py`](census.py), transcript [`logs/census-three.txt.gz`](logs/census-three.txt.gz):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 272 -> 272 (+0);  unknown_alias: 4405 -> 4405 (+0);  unsupported: 49835 -> 49833 (-2)
units with diagnostic-count changes: 1
  monkey-libevent/mk_server/mk_scheduler.c: {'unsupported': -2}
leaves added: 0; leaves removed: 2
    -1 mk_sched_worker_free L6523 unsupported: call to uncontracted function `pthread_mutex_lock`
    -1 mk_sched_worker_free L6539 unsupported: call to uncontracted function `pthread_mutex_unlock`
```

**This is §3.4's pre-registered prediction, met exactly.** Two leaves, both in
`mk_sched_worker_free`, both at the two `&mutex_worker_exit` sites `candidates.py`
named in advance; nothing added; no status move at any level.

And it settles the two declarations *separately*, which is what the run was for:

| declaration | measured payoff |
|---|---|
| `pthread_mutex_lock` | 1 leaf |
| `pthread_mutex_unlock` | 1 leaf |
| **`msgpack_packer_init`** | **zero** |

`msgpack_packer_init` is applied, not withdrawn — the census has it `agreed` in
both units that mention it — and it still decides nothing, because every one of
its call sites refuses on the actual first. That is *precisely* the ground card
PROD-CONTRACTS recorded for leaving `flb_errno_print` undeclared (§2.5): "the
dereference refuses the statement before any contract is consulted, so a
declaration cannot decide those sites." A premise with no measured payoff is a
premise bought for nothing.

**So `contract/qcore/msgpack/packer.qcore` is NOT landed.** Its reading is kept
in full in §3.1 so a later card can land it the day the blocker moves — which is
exactly how card CONSTRUCTOR-RESIDUALS came to land `flb_errno_print` once it
repaired that card's dereference blocker. The follow-on is named
`ABI-TYPEDEF-MSGPACK-UNDECLARED`.

### 4.5 The landed tree, measured on its own

Because §4.4's tree is not the tree this card lands, a **third** run was taken at
the final branch tip — `libc/pthread_mutex.qcore` only. Exit 0, zero gateway
failures.

[`census.py`](census.py), BEFORE → landed
([`logs/census-final.txt.gz`](logs/census-final.txt.gz)):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 272 -> 272 (+0);  unknown_alias: 4405 -> 4405 (+0);  unsupported: 49835 -> 49833 (-2)
units with diagnostic-count changes: 1
  monkey-libevent/mk_server/mk_scheduler.c: {'unsupported': -2}
description inventory deltas:
  unsupported / 'call to uncontracted function `pthread_mutex_lock`':   1 -> 0 (-1)
  unsupported / 'call to uncontracted function `pthread_mutex_unlock`': 1 -> 0 (-1)
leaf-level diff, per changed unit:
  UNIT monkey-libevent/mk_server/mk_scheduler.c
    -1 mk_sched_worker_free L6523 unsupported: call to uncontracted function `pthread_mutex_lock`
    -1 mk_sched_worker_free L6539 unsupported: call to uncontracted function `pthread_mutex_unlock`
leaves added: 0; leaves removed: 2
```

**Identical to §4.4 in every measured number — and the verdict artifact is
byte-identical to it**, SHA-256 `38839e7f…` on both. `msgpack_packer_init`
therefore contributed not "no leaves I could find" but **exactly nothing, byte
for byte**, which is as strong as the ground for dropping it can be made.

#### Every moved leaf, attributed

Two, and they are the same claim twice:

| leaf | attribution |
|---|---|
| `mk_sched_worker_free` L6523, `unsupported: call to uncontracted function \`pthread_mutex_lock\`` — **removed** | `pthread_mutex_lock(&mutex_worker_exit)`: `mutex_worker_exit` is a file-scope `pthread_mutex_t` (`mk_scheduler.c` preprocessed L13685), so the actual is the address of a plain name and the projection can model it. With the declaration in force the call is modelled as a borrowed-pointer call instead of an uncontracted one. |
| `mk_sched_worker_free` L6539, likewise for `pthread_mutex_unlock` — **removed** | the matching unlock, same object, same ground |

Both moves are **`Unknown` → modelled at a statement whose enclosing function
stays `Unknown`**: `mk_sched_worker_free` still carries `mk_plugin_exit_worker`,
`mk_vhost_fdt_worker_exit`, `mk_cache_worker_exit`, `pthread_self`, three
`pthread_getspecific` results handed to `mk_mem_free`, an `abort()` arm and an
indirect `&ctx->workers[i]` — every one a refusal this card does not touch. So
the function does not move and neither does the unit, which is why the headline
row is untouched. **No leaf moved in the accusing direction and none in the
promoting direction; two refusals were retired against a declared premise.**

#### Against the frozen baseline

`docs/reports/corpus-repin/diff_artifacts.py --archived docs/reports/wave17-corpus/artifacts`
([`artifacts/diff-wave17-to-final.json.gz`](artifacts/diff-wave17-to-final.json.gz)):

```
--- chunk-contracts: NO DIFFERENCES
--- diagnostics:     envelope key `diagnostic_counts`; units +0 -0 ~1
--- contracts:       sources 14 -> 15 (+1 `qcore/libc/pthread_mutex.qcore`, 0 changed, 0 removed)
                     manifest modules 14 -> 15 (+1, same module); audit_selection 5 -> 5; keys {}
--- source:          envelope key `compile_entry`; 4 wrapper rows changed
--- source-effects:  3 result rows changed
```

Reading **normalised and raw keys together** (constraint 22): under
normalisation `contracts.json` gains **exactly one** module and changes nothing
else — no collision, nothing removed, no other module's text moved — and the raw
keys differ additionally by the checkout root, which is the residue §4.3 already
identified. `source` and `source-effects` carry only the path-literal and
identity-digest classes, unchanged from §4.3 and unrelated to this card's diff.

**The baseline is not re-pinned.** No integrator consolidation is claimed here:
the headline row and the leak pin are untouched, so `docs/reports/wave17-corpus/`
remains the corpus baseline exactly as it stands (constraint 21 — published
beside, never edited).

---

## 5. Lean

`proof/QCore/TypedefProjection.lean`, audited by
`proof/AuditTypedefProjection.lean`: **35 theorems**, depending only on
`propext` and `Quot.sound`, **no new axiom**, no placeholder.
`check_lean_audits.py` moves 80 → 81, and `AuditAll.lean` re-checks the whole
compiled closure (31,343 declarations, standard logical axioms only) after the
one change made to an existing module — `AbiType` gains the constructor
`fnptr`. No theorem of any existing namespace was retired, renamed or weakened.

| group | what it fixes |
|---|---|
| `the_widening_only_adds`, `the_widening_loses_no_reading` | conservativity, pointwise over every target and pointer depth: a reading the earlier reader committed to is the same reading now, and nothing that was read became unreadable. This is the sentence §2.5's containment argument needs. |
| `an_untagged_aggregate_pointer_was_unreadable_and_is_now_read`, `a_function_pointer_was_unreadable_and_is_now_read` | non-vacuity, one witness per new shape, each the measured production case |
| `a_function_pointer_is_not_any_other_target`, `an_untagged_aggregate_is_its_own_target` | neither new target is quietly identified with an existing one |
| `an_untagged_aggregate_projects_as_a_tagged_one` | the aggregate admission, as an **equality at every depth** rather than as prose |
| `an_aggregate_value_refuses`, `a_named_object_value_refuses`, `an_aggregate_under_two_pointers_is_the_out_slot`, `an_aggregate_under_three_pointers_refuses` | what the aggregate admission does *not* reach, and that the depth boundary is the pointer table's own |
| `only_a_function_pointer_spelling_is_admitted`, `a_function_pointer_is_not_an_integer`, `a_function_pointer_is_not_an_object_pointer` | `fnptr` in the admitting direction, and the LP64 coincidence buying nothing |
| `a_pointer_projection_never_admits_a_function_pointer`, `no_object_pointer_admits_a_function_pointer`, `no_untagged_aggregate_admits_a_function_pointer`, `no_arithmetic_projection_admits_a_function_pointer` | and in the refusing direction, for every projection this vocabulary produces |
| `a_function_pointer_is_not_a_pointer_spelling`, `borrowed_is_the_only_mode_on_a_function_pointer`, `every_custody_mode_is_refused_on_a_function_pointer`, `an_owned_result_of_a_function_pointer_is_rejected` | the soundness half: no custody mode and no owned result is writable on `fnptr`, by the two gates that were already there |
| `the_custody_gate_still_admits_object_pointers`, `the_result_gate_still_admits_object_pointers` | and neither gate was weakened to make that true |
| `a_pointer_to_a_function_pointer_refuses`, `an_alias_to_a_pointer_to_a_function_pointer_is_unresolved` | `ABI-FNPTR-DEPTH1`, directly and through an alias |
| `an_exhausted_depth_budget_is_unresolved`, `an_absent_name_is_unresolved`, `an_unresolved_link_is_unresolved`, `a_name_recorded_twice_differently_is_unresolved` | resolution is fail-closed on every edge |
| `a_chain_to_a_function_pointer_keeps_its_level`, `an_alias_folds_an_aggregate_like_an_object`, `a_name_recorded_twice_alike_keeps_its_reading` | and still resolves what it should |
| `a_tagged_definition_composes_with_its_plain_typedef`, `an_untagged_definition_would_not_have_composed` | the regression §2.1 avoided, proved in **both** directions: recording a tagged definition as a distinct kind collapses the pair to `unresolved`, which would have withdrawn a declaration that agreed before this card |

`Projection`, `AbiType` and `admits` are `QCore.DeclaredAbiAgreement`'s own
abstractions, imported unchanged, so the projection this module produces is
literally the one that card's comparison consumes.

---

## 6. What was touched, and the pins moved deliberately

### 6.1 The diff

| file | what |
|---|---|
| `crates/qcore-c-gateway/src/legacy_declared_abi.rs` | the widening: `TypedefTarget` becomes an enum, `typedef_end` finds the `;` at depth zero, `read_function_pointer_typedef` and `read_aggregate_typedef` are new, `resolve_typedef` and the three projection functions carry the two new targets, `matching_close` learns `{`, `matching_open` is new |
| `crates/qcore-contract/src/contract_loader.rs` | `FUNCTION_POINTER_ABI_TYPE` and the `parameter_abi_types()` vocabulary it joins |
| `crates/qcore-core/src/extern_call.rs` | **comment only**: the `AbiType` doc claimed to be §C04.1 "complete" and had not been since card NEIGHBOURS added `f32`/`f64`. Corrected, with the reason the narrowness is safe — a spelling absent from that table is refused by `Q033`, the refusing direction, so widening §C04.1 never widens what unchecked linkage admits. No behaviour moved. |
| `contract/qcore/libc/pthread_mutex.qcore` | the two landed declarations (§3.1). `msgpack/packer.qcore` was written and measured but **not** landed (§4.4); its text is preserved in §3.1 |
| `contract/cycle19-resource-contracts.json` | both selected |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.1 | `fnptr` added normatively, with its refusal obligations in both directions and the "only `resource_borrowed` may apply to a scalar **or to `fnptr`**" rule |
| `contract/qcore/README.md` | the manifest paragraph, stale since card CONSTRUCTOR-RESIDUALS ("twelve modules … 31 declarations" against a live fourteen and 34), corrected to sixteen and 37 with that provenance stated |
| `proof/QCore/TypedefProjection.lean`, `proof/AuditTypedefProjection.lean`, `proof/QCore.lean` | §5 |
| `proof/QCore/DeclaredAbiAgreement.lean` | one constructor, `AbiType.fnptr` |
| four test files | §6.2 |

### 6.2 Pins moved deliberately, each keeping its load-bearing half

Editing `contract/qcore` requires updating the pinned count tables **deliberately**
(invariant 5). Six pins moved, and every one of them moved because the manifest
grew, not because a measured quantity did:

| pin | before → after |
|---|---|
| `compiler/tests/qcore_contract_corpus.rs` corpus size | 69 → **71** modules |
| its `EXPECTED_CONTRACTS` table | three rows added: `pthread_mutex_lock` (3 clauses), `pthread_mutex_unlock` (3), `msgpack_packer_init` (5) |
| `compiler/tests/contract_loader.rs` manifest-selected modules (twice) | 14 → **16** |
| its `(selected, v1_declarations, rejected_declarations)` triple | `(14, 34, 145)` → **`(16, 37, 145)`** — the rejected count is unchanged, because both added modules are entirely v1 |
| its explicit borrowed-only parity list | three names added, each `Borrowed` at every parameter with no result custody |
| its `every_abi_scalar_is_a_borrowed_only_projection` loop | now runs over `SCALAR_ABI_TYPES` **and** `fnptr`, so the new spelling is checked to reject every custody mode exactly as an integer entry does; three new asserts pin that `fnptr` is in neither existing table and does not start with `ptr` |
| `crates/qcore-c-gateway/tests/neighbours.rs`, `.../prod_contracts.rs` | 14 → **16**, each keeping the by-name assertions that are what those tests actually pin |

New coverage, not just moved pins: card LOADER-ABI's
`the_admission_and_refusal_table_decides_row_by_row` goes from **32 to 50 rows**,
all decided through the production entry point — nine on aggregate-defining
typedefs (the two new admissions, the tagged-definition composition guard, the
out-slot depth, and four refusals that stay: an aggregate value, an untagged
`enum` value, a multi-declarator typedef) and nine on function-pointer typedefs
(the admission, the chain, `ABI-FNPTR-OPAQUE`, `ABI-FNPTR-DEPTH1`, both refusal
directions against integers and object pointers, and a doubly-declared name).
One existing row's label was corrected: "a function-pointer parameter is not
read" now says *inline*, which is what it always tested.

---

## 7. Gate

The full standing per-change gate from `AGENTS.md`, at the branch tip:

| step | result |
|---|---|
| `git submodule update --init --recursive` | `third_party/libevent` at `d82464a2` (§4.2 note 2) |
| `cargo test --workspace` | **exit 0** — 178 suites, **2,986 passed**, 8 ignored, 0 failed |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | **exit 0**, zero warnings |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **81 audits pass** (80 → 81), `AuditAll` re-checked 31,343 declarations, standard logical axioms only |

Clang is `/usr/bin/clang`, Ubuntu clang 21.1.8, selected explicitly through
`QCORE_CLANG` for every existing-C test and both corpus runs.

**Load flakes (constraint 16).** One of the four named rows tripped under the
sibling load this card ran against: `a_confined_fixture_cannot_write_outside_the_admitted_paths`
failed with `assertion left == right failed: Broken pipe (os error 32); left: Io,
right: Terminated` — the exact `Io(Broken pipe)` shape constraint 16 records for
it. Re-run in isolation at load average 17.5 it **passes**
([`logs/flake-retry.txt`](logs/flake-retry.txt)). Nothing was weakened; the row is
load-sensitive exactly as `remaining-p03-f` documented. The other three named
rows did not trip.

**Providers.** No provider module, manifest template or `qcore-schemas` major
version was touched, so constraint 18's rebuild-and-verify obligation is not
triggered by this diff. Packages were nonetheless rebuilt for this card's own
corpus runs (`build/cycle19/provider-packages-abi`) because the pinned route
binds one.

---

## 8. The increment paragraph (draft — for the integrator, not applied here)

> **Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF (wave 19).** Card LOADER-ABI's
> declared-ABI reader would not commit to a `struct`/`union`/`enum` **defined by
> its own typedef** or to a **function-pointer typedef**, so any declaration
> using one was withdrawn at every call site as `abi-unreadable-declaration`.
> Both shapes are now read — the terminating `;` is found at brace depth zero,
> and a tagged definition is recorded as the specifier pair it equals so it
> composes with a separate plain typedef instead of colliding — and the
> unreadable class is narrowed, never deleted, with every remaining refusal named
> and pinned. A pointer to function gets its own projection,
> `FUNCTION_POINTER_ABI_TYPE` (`fnptr`), deliberately outside `POINTER_ABI_TYPES`
> and not spelled `ptr<…>`, so the loader's two existing custody gates make every
> custody mode on it and every owned result of it a load-time rejection; it is
> deliberately not folded onto `u64`/`usize` despite the LP64 width coincidence,
> because that would make an integer parameter and a callback parameter
> indistinguishable to the comparison. Spec §C04.1 gains `fnptr` normatively.
> **The card's own premise is refuted by its own measurement:** with the reader
> widened and all three of CONSTRUCTOR-RESIDUALS §4.2's declarations landed, the
> production `flb_input_chunk_create` probe is byte-identical to that card's
> archived `probe-after.txt` — the ABI withdrawal was a second refusal stacked on
> the importer's address-of-a-member refusal, which binds first, so the residual
> is renamed `ABI-TYPEDEF-ADDRESS-OF-MEMBER` rather than removed. Three 185-unit
> runs: BEFORE at base `c7c0a3c` reproduces the frozen wave-17 verdict artifact
> **byte-identically** (`412905bb…`); the three-declaration run measures each
> declaration's payoff separately; the landed tree carries
> `libc/pthread_mutex.qcore` only. Corpus movement is **two diagnostic leaves**,
> both `call to uncontracted function pthread_mutex_{lock,unlock}` in
> `mk_sched_worker_free`, exactly as the report predicted before measuring —
> **zero unit status moves, zero function status moves across 4,834 projected
> functions, zero newly surfaced provisional `Unsafe`, and the leak pin holds at
> 272.** `msgpack/packer.qcore` was written, loaded and measured at **zero**
> payoff and deliberately **not landed**, on the ground PROD-CONTRACTS recorded
> for `flb_errno_print`. Contract corpus 69 → 70 modules, manifest 14 → 15,
> declarations 34 → 36. Lean `QCore.TypedefProjection`, 35 theorems, no project
> axiom; audits 80 → 81.

Headline numbers for `AGENTS.md`: contract corpus **70 modules**; Lean audits
**81**; the 185-unit corpus row and the leak pin are **unchanged** (2 / 42 / 137
/ 4, leak 272), so the wave-17 baseline is **not** re-pinned by this card.

---

## 9. Named follow-ons

* **`ABI-TYPEDEF-ADDRESS-OF-MEMBER`** — the corrected statement of production
  `flb_input_chunk_create`'s residual (§3.3). The three statements card
  CONSTRUCTOR-RESIDUALS §4.2 attributed to the declared-ABI reader are refused
  by the *importer*, on the `&x->y` actual, before any contract is consulted.
  Sizing this is a resource-projection question about the address of a member,
  not a contract-vocabulary one, and it inherits every caution card
  REMAINING-CTWIN-PUBLISH recorded: the neighbouring escape model was measured
  to **promote** a publish-without-release body from `Unsafe` to `Safe`, which
  invariant 1 forbids. Nothing here should be widened without that measurement
  in front of it.
* **`ABI-TYPEDEF-MSGPACK-UNDECLARED`** — `msgpack_packer_init` is read against
  its body in §3.1 and deliberately left undeclared: measured payoff zero,
  because the address-of-a-member actual refuses every one of its call sites
  before any contract is consulted. It is the only production exercise `fnptr`
  would have had, so until this lands the function-pointer projection is
  exercised by the decision table and Lean rather than by the corpus. The day
  `ABI-TYPEDEF-ADDRESS-OF-MEMBER` moves, the declaration is already written.
* **`ABI-FNPTR-OPAQUE`** — `fnptr` does not distinguish two function-pointer
  signatures, exactly as `ptr<mut, u8>` does not distinguish two object-pointer
  pointees. The declared side has no syntax for a parameter list, so a
  structural comparison has nowhere to be written; giving it one is a
  contract-vocabulary change, not a reader change. Nothing can be bought with
  the opacity today, because the spelling carries no custody.
* **`ABI-FNPTR-DEPTH1`** — a pointer *to* a function pointer refuses. It is an
  object pointer and would project into a custody-bearing spelling. No
  declaration in scope needs it; admitting it needs a measured one.
* **`ABI-TYPEDEF-INLINE-DECLARATOR`** — an inline function-pointer parameter
  (`int (*cb)(void *)` written at the parameter position rather than behind a
  typedef) is still a nested declarator and still refuses. The typedef spelling
  is what this card reads; the inline one would need the declarator parser the
  reader deliberately does not have.
* **`LOADER-ABI-UNDECLARED`** and **`LOADER-ABI-LP64`** are carried unchanged
  from card LOADER-ABI. This card neither narrows nor widens either: it adds no
  target-dependent reading (`fnptr` is deliberately *not* justified by the LP64
  width coincidence, §2.2) and changes nothing about names the reader finds no
  declaration for.
* **`DIFF-TOOL-CARD-DIFFS`** applies to this card's artifact diff, which lives
  under `artifacts/` and is outside the archived consolidation chain
  (constraint 22).
