# CONSTRUCTOR-RESIDUALS — the last two roots of `flb_input_chunk_create`, measured — 2026-09-10

The named continuation of card ALIAS-AGGREGATE
([`docs/reports/remaining-alias-aggregate/README.md`](../remaining-alias-aggregate/README.md)
§5), which measured the four roots of production `flb_input_chunk_create`'s
residuals, took one of them, and left the other two as reasoned refusals with a
capability named for each: **`ALIAS-AGGREGATE-ERRNO`** and
**`ALIAS-AGGREGATE-PUBLICATION`**.

**Base tree:** main `8ec8402` ("Consolidate wave 16: corpus re-pin, follow-ons,
ledger"), submodules initialised, `third_party/libevent` at `d82464a2`. Every
measurement below was taken on that tree with only this card's own diff
applied, on the pinned corpus (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`), through
the canonical pinned route.

---

## Headline

| | wave-16 tree | after this card |
|---|---|---|
| `flb_input_chunk_create` diagnostics | **11** | **6** |
| its exits that `Discharge` | 3 of 6 | **5 of 6** |
| its function status | `Unknown` | **`Unknown`** — unchanged |
| `fluent-bit-chunk/flb_input_chunk.c` | provisional `Unsafe` | **provisional `Unsafe`** — unchanged |
| 185-unit headline | 2 / 42 / 137 / 4 | **2 / 42 / 137 / 4** — unchanged |
| unit status moves | — | **0** |
| function status moves (4,834 projected) | — | **0** |
| leak diagnostics | 271 | **272** — §6.4, named prominently |

**Both named follow-ons are dispositioned, and the second is dispositioned by
correcting its own premise.**

* **`ALIAS-AGGREGATE-ERRNO` is implemented.** A dereference of a *neutral call's
  result* is admitted as one outer argument, under a **refined** proved
  sentence, not a violated one; `__errno_location` and `flb_errno_print` are
  declared against read bodies. Measured corpus reach: **18** sites for the new
  admission and **24** more for the declaration alone, across 13 units (§6.3).
* **`ALIAS-AGGREGATE-PUBLICATION` needs no publication relation at all.** Card
  HASH-CONTRACTS had already settled the question for a publishing callee, and
  `mk_list_add` takes the same faithful declaration: `resource_borrowed` on
  both parameters, because the callee frees nothing and moves no obligation.
  The measured blocker on the two statements this card could not admit is
  something else entirely — card LOADER-ABI's declared-type reader — and it is
  named, not the publication relation.

**One pre-existing fail-open found and repaired.** Card BORROW-LIVENESS's
precondition read only the *enclosing* call's actuals, so a released identity
handed to an admitted **nested** call's `Borrowed` actual was `Safe` with no
diagnostic. Measured at the base tree before any rule of this card existed
(§3.4), repaired at the original site, pinned in both directions.

**Nothing is promoted, at any level.** No unit and no function changed verdict
in either direction; the one diagnostic-level movement in the accusing
direction is §6.4's leak, which is the seventh instance of a claim the same
function already published six times.

---

## 1. What the card was given, and what it found

Card ALIAS-AGGREGATE's §1.1 recorded four roots. R1 it took. This card takes
R2/R3 (`ALIAS-AGGREGATE-ERRNO`) and dispositions R4
(`ALIAS-AGGREGATE-PUBLICATION`). Card NEIGHBOURS-CHAINED-TAIL had already
cleared L19581, leaving **11** diagnostics on the composed wave-16 tree
([`docs/reports/wave16-corpus/README.md`](../wave16-corpus/README.md)).

The eleven, measured with card ALIAS-AGGREGATE's read-only probe
([`compiler/examples/alias_aggregate_probe.rs`](../../../compiler/examples/alias_aggregate_probe.rs),
unchanged by this card):

| # | Line | Diagnostic | Statement | Root |
|---|---|---|---|---|
| 1 | 19600 | `indirect memory access` | `flb_errno_print((*__errno_location ()), …)` | **R2** |
| 2 | 19601 | `unresolved resource identity \`chunk\`` | `cio_chunk_close(chunk, !0);` | R2 |
| 3 | 19619 | `indirect memory access` | `flb_errno_print((*__errno_location ()), …)` | **R3** |
| 4 | 19620 | `unresolved resource identity \`chunk\`` | `cio_chunk_close(chunk, !0);` | R3 |
| 5 | 19621 | `unresolved resource identity \`ic\`` | `flb_free(ic);` | R3 |
| 6 | 19628 | `indirect memory access` | `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write);` | **R4** |
| 7 | 19629 | `indirect memory access` | `mk_list_add(&ic->_head, &in->chunks);` | **R4** |
| 8 | 19637 | `indirect memory access` | `pthread_mutex_lock(&in->metrics_chunk_lock);` | R4-shaped |
| 9 | 19639 | `indirect memory access` | `pthread_mutex_unlock(&in->metrics_chunk_lock);` | R4-shaped |
| 10 | 19647 | `unresolved resource identity \`ic\`` | `return ic;` | R4 |
| 11 | 19647 | `nullable resource \`ic\` used by a non-null contract` | `return ic;` | R4 |

After this card, rows 1-5 and row 7's diagnostic are gone; row 7's line reports
the reaching state instead (§6.3).

Both ends are measured on the same prepared corpus, with the card's five changed
source and contract files checked out at `8ec8402` for the BEFORE run and
restored afterwards. The BEFORE transcript
([`logs/probe-before.txt`](logs/probe-before.txt)) independently reproduces the
wave-16 consolidation's published count of **11**, which is what makes the
comparison a measurement rather than a carried number:

| | BEFORE (`8ec8402`) | AFTER |
|---|---|---|
| diagnostics | **11** | **6** |
| `L19582` / `L19589` / `L19596` | Discharged | Discharged |
| `L19602` | `Unknown`, `site0 Mixed` | **Discharged**, `site0 Released` |
| `L19622` | `Unknown`, two `Mixed` | **Discharged**, two `Released` |
| `L19647` (success) | `Unknown`, three `Mixed` | `Unknown`, three `Mixed` — unchanged |
| exits Discharged | **3 of 6** | **5 of 6** |

AFTER transcript: [`logs/probe-after.txt`](logs/probe-after.txt).

---

## 2. R2/R3 — the dereference of a call result

### 2.1 What refuses, and why the earlier sentence cannot be stretched

`flb_errno()` is `flb_errno_print(errno, __FILE__, __LINE__)`
(`include/fluent-bit/flb_log.h:316` at the pin), and glibc defines `errno` as a
macro over a function call (`/usr/include/errno.h:37-38`):

```c
extern int *__errno_location (void) __THROW __attribute_const__;
# define errno (*__errno_location ())
```

so the preprocessed first argument is `( * __errno_location ( ) )` — a
**dereference of a call result**. The importer's lexical gate refuses any
statement carrying a unary `*` outside an admitted span, and no admitted span
covered this one.

Card W4-EXPR-POS's neutral-expression families are proved under

> an admitted expression contains no pointer-valued operand,

and `__errno_location ()` is pointer-valued **by construction**. Widening those
families to reach here would violate their own sentence, so this is a different
family with its own obligation — exactly as card ALIAS-AGGREGATE §5.1 said.

### 2.2 The refined sentence

The replacement is stated over what the **projection** tracks rather than over
what C calls a pointer:

> An admitted dereference-of-a-call-result **binds, releases, escapes and
> addresses nothing this projection tracks.**

Each conjunct is discharged by a premise this admission already required, and
the conjunction is proved from the admission
(`QCore.DereferencedResult.the_refined_sentence_holds`):

| conjunct | discharged by |
|---|---|
| binds nothing | **N6** — the nested contract's result is `None`, so no site is produced and no name binds one; and no admitted wrapper contains `=`, so the argument is an rvalue in an argument position |
| releases nothing | **N7** — every nested parameter is declared `Borrowed`, so the nested call's declared effects are the identity on the abstract state |
| escapes nothing | **N9** — every inner actual is a place the projection denotes, or data |
| addresses nothing | no admitted wrapper carries `&`, and **N10** requires the dereferenced pointee to be a **scalar**, so the whole argument's value is a scalar rvalue no position of this projection can bind |

The pointer that is dereferenced is therefore never *this projection's* pointer:
it is a value the projection declined to track at the moment the call was
declared neutral, and the dereference consumes it into a scalar in the same
expression. `the_earlier_sentence_does_not_cover_this` states the honest
relationship in Lean rather than papering over it.

### 2.3 What lands

Two files, plus one contract module and one declaration.

**The classification** —
[`crates/qcore-c-gateway/src/typed_field_access.rs`](../../../crates/qcore-c-gateway/src/typed_field_access.rs).
`TypedAccessModel` gains `scalar_pointer_results`: for every function whose
declaration or definition this reader can commit to, whether **every**
declaration in the unit returns exactly one `*` over a specifier list resolving
to `DeclaredType::Scalar`. `false` is absorbing — a second, disagreeing or
unreadable declaration retires the fact rather than competing with it — and a
name absent from the map was never read, which refuses. The declarator shape is
the one card LOADER-ABI reads for the ABI comparison, with the attribute and
assembler suffixes skipped (glibc's own declaration carries two `__attribute__`
groups after preprocessing), except that the **pointee** is classified here
instead of being projected away onto `ptr<mut, u8>`.

One precision guard was added by measurement: a declaration begins a statement.
Without it `return * f ( ) ;` and `x = a * f ( ) ;` carry the declarator shape,
and — `false` being absorbing — would retire a real declaration's fact from
elsewhere in the unit. Both directions are pinned over token vectors
(`an_expression_with_the_declarator_shape_does_not_retire_the_fact`).

**The admission** —
[`crates/qcore-c-gateway/src/variadic_nested.rs`](../../../crates/qcore-c-gateway/src/variadic_nested.rs).
Card O04-TRACE's premise **N4** ("the nested call is a whole argument") gains
**N4-D**: the argument may instead be `* f ( … )` or `( * f ( … ) )`, and
nothing else. Both spellings are admitted because `errno` is written with the
enclosing parentheses and a hand-written dereference usually is not; refusing
either would make the rule depend on parenthesisation. **N10** then requires the
scalar pointee, and is consulted *only* when the wrapper dereferences, which is
what keeps the base family byte-identical.

`legacy_c.rs` gains the span exemption the lexical gate needs — and calls
`admitted_nested_call` itself to compute it, so **there is one decision
function**, not two that could drift. Exempting the span is never on its own an
admission: the nested rule below is consulted again and still governs.

Deliberately refused, each with a test: `* * f ( )`, `& * f ( )`,
`( * f ( ) ) -> m`, `* f ( ) + 1`, `( ( * f ( ) ) )`, `f ( ) [ 0 ]`, a cast
around either shape, and every dereference of a call whose declaration this unit
does not read as returning a pointer to a scalar.

### 2.4 The declarations

Both are read against pinned bodies, to the NEIGHBOURS §2 standard.

`contract/qcore/libc/errno.qcore` (new) declares `__errno_location`: no
parameters, `resource_return_none()`. It returns a pointer to the calling
thread's `errno` object, which has static storage duration (C11 7.5p2), is not
allocated by the call and is freed by nothing in this corpus, so the caller
acquires no obligation. The module records its own portability boundary: this is
the glibc spelling, and on a host that spells it otherwise the declaration names
a function no analysed unit contains and is inert there.

`contract/qcore/fluent_bit/logging.qcore` gains `flb_errno_print`, read from
`src/flb_log.c:1321`: `errnum` and `line` are `int`s by value, `file` reaches
`flb_log_print`'s already-declared data tail, `buf` is a function-local array,
and the result is `errnum` — a scalar. Every parameter `Borrowed`,
`resource_return_none()`, declared at the one arity the macro writes and with no
tail clause because the C declaration is not variadic. Card PROD-CONTRACTS'
"deliberately left undeclared" entry for it is retired **in that module's own
text**, naming the blocker that no longer holds; the retirement is asserted by
that card's own test.

---

## 3. The pre-existing fail-open, found and repaired

### 3.1 What was measured, at the base tree

Card O04-TRACE's admitted nested call carries its own modeled actuals. Card
BORROW-LIVENESS's precondition (`require_live_borrowed_actuals`) read only the
enclosing `ContractCall`'s. Measured through the public gateway entry points on
the base tree — first with a scratch probe over a synthetic API, then reproduced
with the committed `d_*` family, which is what the pin uses:

```c
d_close(h); d_label_of(h);              /* base: Unknown, UnknownAlias("h") */
d_close(h); d_report(0, d_label_of(h)); /* base: Safe, NO diagnostic        */
```

The same released identity, at the same declared `Borrowed` mode, answered
differently purely because one of them was nested.

### 3.2 The repair

`require_live_borrowed_actuals` now examines the admitted nested call's actuals
as well, against the **same settled pre-state**. That is sound in the ordering
sense for exactly the reason O04-TRACE composes the two calls at all: an
admitted nested call's declared effects are the identity (N6/N7), so the state
entering it and the state entering the outer call are the same state.

### 3.3 What it can and cannot move

It is a **set inclusion**: every actual refused before is refused after
(`the_repair_only_adds_refusals`), and a site with no admitted nested call is
unchanged (`a_site_with_no_nested_call_is_unchanged`), so it is invisible to
every statement card O04-TRACE did not already admit a nested call in. The pass
stays read-only, so `C02-P-ALIAS-LEAK-ORDER` is undisturbed by construction as
well as by measurement.

### 3.4 Why it is this card's to repair

This card widens the reach of the family that had it. Both directions are pinned
in `a_released_identity_at_a_nested_borrowed_actual_is_refused`, with the base
tree's measured answers quoted in the test's own doc comment, across three
bodies: the control the base already refused, the bare nested call, and the same
under this card's wrapper.

---

## 4. R4 — the publication root, dispositioned by correcting its premise

Card ALIAS-AGGREGATE §5.2 recorded that `mk_list_add` and `msgpack_packer_init`
"both *publish* through their arguments", that "a faithful declaration would
have to say what that publication does to the caller's obligation", and that
this "is O00 blockers **B03/R13** and is unmodeled".

**Measured, that premise is wrong for the first callee and beside the point for
the second.**

### 4.1 `mk_list_add` needs no publication mode

Read at the pin, `lib/monkey/include/monkey/mk_core/mk_list.h:47-63`:

```c
static inline void __mk_list_add(struct mk_list *_new, struct mk_list *prev,
                                 struct mk_list *next)
{ next->prev = _new;  _new->next = next;  _new->prev = prev;  prev->next = _new; }

static inline void mk_list_add(struct mk_list *_new, struct mk_list *head)
{ __mk_list_add(_new, head->prev, head); }
```

One arm. Nothing allocated, nothing freed, nothing reallocated, one
header-local callee that does the same. **A caller that owed a release before
the call owes exactly the same release after it.**

That is precisely the question card HASH-CONTRACTS already answered, for
`flb_hash_table_add`'s reference arm, in
`contract/qcore/fluent_bit/hash_table_publication.qcore`: what this projection
reads out of a declaration is the caller's per-exit acquire/release accounting;
`resource_stored` is refused because it would delete an obligation nothing else
discharges and because §C04.2 requires an `Owned` owner no call site supplies;
and the retention itself is stated as unmodeled (B03/R13) rather than claimed.
Every clause of that argument transfers verbatim. So the faithful declaration is
`resource_borrowed` on both parameters, in the **existing v1 vocabulary**, with
no new mode, no schema change and no new model capability.

`contract/qcore/monkey/intrusive_list.qcore` (new) carries it, with the body
quoted, both refusal grounds for `resource_stored` written out, and the
retention named as unmodeled. `mk_list_del` is deliberately left undeclared: it
is the operation that *would* have to say what unlinking does to an obligation,
and no call site this card measured needs it.

**A declared publication MODE was considered and is not needed here.** The
withdrawal direction the card brief proposed — a published identity withdrawn
from settlement, never discharged — is strictly more refusing than `Borrowed`,
and it is available without a `qcore-schemas` change (the side-channel clause
pattern of `resource_variadic_data_tail()` and `resource_literal_argument`).
It is not landed because on this callee it would be **unfaithful in the
refusing direction**: withdrawing `ic` from settlement asserts that this call
made the caller's obligation undecidable, and the read body shows it did not.
A publication mode remains the right instrument for a callee that *does* move or
delete an obligation; `mk_list_add` is not one. Named `PUBLICATION-MODE` below.

### 4.2 The measured blocker is card LOADER-ABI's declared-type reader

An experiment declared `msgpack_packer_init`, `pthread_mutex_lock` and
`pthread_mutex_unlock` beside `mk_list_add` in the manifest and re-ran the probe
over the production unit
([`logs/probe-publication-experiment.txt`](logs/probe-publication-experiment.txt)):

```
L19628  Unknown: call to `msgpack_packer_init` whose declared contract was withdrawn
        against its C declaration in this translation unit (abi-unreadable-declaration)
L19629  Call mk_list_add(_, _) -> _ addresses["ic"]
L19637  Unknown: call to `pthread_mutex_lock`  … (abi-unreadable-declaration)
L19639  Unknown: call to `pthread_mutex_unlock` … (abi-unreadable-declaration)
```

`mk_list_add` is admitted; the other three are **withdrawn by card LOADER-ABI's
ABI comparison**, because each has a parameter whose declared type that reader
will not commit to:

* `msgpack_packer_init(msgpack_packer *pk, void *data, msgpack_packer_write callback)` —
  `msgpack_packer_write` is `typedef int (*msgpack_packer_write)(void*, const char*, size_t);`
  (`lib/msgpack-c/include/msgpack/pack.h:36`), a function-pointer typedef whose
  body carries `(`, which `read_typedefs` records as unresolvable;
* `pthread_mutex_lock`/`pthread_mutex_unlock` take `pthread_mutex_t *`, a
  **union** typedef, unresolvable for the same structural reason.

That is not the publication relation. It is a reader boundary, it is
fail-closed, and it is named `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` below. The two
declarations are **not** landed: a declaration the ABI comparison withdraws at
every call site is a premise with no measured payoff, which is card
PROD-CONTRACTS' rule for `flb_log_is_truncated`.

### 4.3 What `mk_list_add` is worth, measured by withdrawal rather than argued

On its own it moves the constructor's diagnostic count by **zero**: the
statement before it is refused, so the reaching state is poisoned and L19629
reports `unresolved resource identity \`ic\`` where it used to report `indirect
memory access` — a refusal replacing a refusal, in the fail-closed direction.

Its contribution is measured instead. With the three ABI-withdrawn statements
replaced by `(void) 0;` in the *preprocessed* text — an attribution experiment
on a modified program, not a claim about the real one:

| run | `mk_list_add` declared | diagnostics | exits Discharged |
|---|---|---|---|
| [`logs/probe-publication-control.txt`](logs/probe-publication-control.txt) | no | **3** | 5 of 6 |
| [`logs/probe-publication-withdraw-all.txt`](logs/probe-publication-withdraw-all.txt) | **yes** | **0** | **6 of 6**, status `Safe` |

Corpus-wide the declaration is admitted at **13 sites in 12 units** (§6.3): two
are modeled outright, eleven replace one refusal with another. No unit and no
function changed verdict.

So the complete residual of the production constructor is now known exactly, and
it is one family: **three statements whose callees have a parameter type card
LOADER-ABI's reader will not commit to.** Nothing else stands between this
projection and a full custody account of the byte-verbatim upstream body — and
what that account would then *rest on* is a separate question: the six
NEIGHBOURS declarations, this card's three, `P-B08-SHALLOW`, and everything else
those cards named as premises.

---

## 5. Lean

`proof/QCore/DereferencedResult.lean`, audited by
`proof/AuditDereferencedResult.lean`: **29 theorems**, depending only on
`propext`, `Classical.choice` and `Quot.sound`, **no new axiom**, no
placeholder. `check_lean_audits.py` moves 72 → 73.

| group | what it fixes |
|---|---|
| `the_bare_wrapper_is_exactly_the_base_admission`, `every_previously_admitted_nested_call_is_still_admitted`, `the_pointee_is_read_only_under_a_dereference`, `the_bare_subject_is_unaffected_by_the_new_premise` | the base family did not move, pointwise and for every pointee reading |
| every `admitsNested` premise lifted (N0, N2, N3, N5-N9) | fail-closed: a refused nested call refuses the candidate |
| `every_other_wrapper_shape_refuses`, `both_spellings_answer_alike` | the wrapper half |
| `a_non_scalar_pointee_refuses`, `an_unread_pointee_refuses` | N10, in both spellings; absence is not a third class |
| `the_refined_sentence_holds` | the four conjuncts, proved from the admission |
| `the_earlier_sentence_does_not_cover_this` | the honest relationship to card W4-EXPR-POS, stated rather than stretched |
| `an_admitted_candidate_is_neutral` | the composition argument inherited unchanged: argument evaluation order stays unobservable |
| `the_widening_is_strict`, `each_premise_is_load_bearing_on_the_subject` | non-vacuity, on the real subject, with each premise separately flipped |
| `the_repair_only_adds_refusals`, `a_site_with_no_nested_call_is_unchanged`, `the_measured_fail_open_is_closed`, `the_control_is_unmoved` | §3's repair as a set inclusion, plus the measured shape and its control |

`Nested`, `Contract`, `ParamMode`, `ResultMode` and `Slot` are `QCore.VariadicNested`'s
own abstractions, imported unchanged; **no theorem of any existing namespace was
retired, renamed or weakened.**

---

## 6. Corpus — before, after, every leaf attributed

### 6.1 The route, and the BEFORE reproduction

A pinned Fluent Bit `66910c10…` clone and a forced-Libevent Monkey were prepared
fresh in this worktree with `scripts/prepare_cycle19_clang_audit.py`; provider
packages were rebuilt at this tree. Both runs used the canonical pinned route
([`logs/run-audit.sh`](logs/run-audit.sh)) and exit 0 with zero gateway
failures. The BEFORE run was started from the **base tree**.

Against the wave-16 baseline
([`docs/reports/wave16-corpus/artifacts/`](../wave16-corpus/artifacts/)):

| artifact | BEFORE vs wave-16 baseline |
|---|---|
| `resource-diagnostics.json` | **byte-identical** (`46d0f178…`) |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** (`c4f438e4…`) |
| `resource-diagnostics.contracts.json` | 12 differing lines, **0 after normalising the worktree path** — the map is keyed by absolute path |
| `monkey-source.json` | 10 leaves: the `compile_entry` envelope and 7 `__FILE__` string literals with their derived `char[N]` extents |
| `resource-diagnostics.source-effects.json` | 15 re-keyed `bounds` entries plus 27 identity digests on 3 of 96 rows; **no result's verdict, coverage shape or bounds value moved** |

Both residues are the classes waves 9, 10 and 16 already record —
**path-literal provenance** and **package identity** — and neither touches a
verdict. The verdict artifact reproduces exactly, so the BEFORE measurement is
the baseline.

### 6.2 The census

[`census.py`](census.py) (card ALIAS-AGGREGATE's, reused apart from its header),
transcript at [`logs/census.txt`](logs/census.txt):

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 272 (+1);  unknown_alias: 4399 -> 4405 (+6);  unsupported: 49890 -> 49835 (-55)
units with diagnostic-count changes: 21
leaves added: 12; leaves removed: 60
```

**The 185-unit headline is `2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only`, identical before and after, with zero unit status moves in either
direction, zero unit `reason_code` moves, zero function status moves across all
4,834 projected functions and no newly surfaced provisional `Unsafe` unit.**

Three of the five artifacts are **byte-identical between the two runs** —
`chunk-contracts.json`, `monkey-source.json` and `source-effects.json`, the last
meaning no evidence row, obligation identity or task digest moved.
`contracts.json` moves by exactly the two added modules, the manifest and
`logging.qcore`'s changed text, and nothing else (measured key by key).

### 6.3 Every leaf, by rule

Seventy-two leaves moved across 21 units. All seventy-two are attributed, and
they fall into four classes:

| Δ | class | rule |
|---|---|---|
| **−24** | `call to uncontracted function \`__errno_location\`` across **12 units** | §2.4's declaration alone. These are `errno = X;` statements, which preprocess to `(*__errno_location ()) = X;` — an assignment whose destination is not a plain name, so the lexical gate (which inspects only the tokens right of `=`) never refused them and they already reached the call rule. §2's admission is not involved. That the store is **dropped and never discharges** the stored identity is measured, not assumed (`a_write_through_a_dereferenced_call_result_never_discharges_a_resource`) |
| **−31** | `indirect memory access` | **18** `flb_errno_print((*__errno_location ()), …)` sites, all in `flb_input_chunk.c` — the N4-D admission, and the corpus's only Fluent Bit unit — and **13** `mk_list_add(&X->F, &Y->G)` sites: 2 in `chunkio/cio_chunk.c` (`chunk_state_sync`), 10 across monkey, and the constructor's L19629 |
| **+11** | `unresolved resource identity NAME` | refusals **replacing** refusals at 11 of those 13 `mk_list_add` sites, in the fail-closed direction: the destination shape is admitted, so the statement is now examined and the *aggregate's* unresolved identity is what refuses, instead of the lexical gate hiding it. Exactly the phenomenon cards NEIGHBOURS and ALIAS-AGGREGATE recorded for their own added leaves. The other two — `chunk_state_sync`'s, whose base is a formal the body never binds — are fully admitted and are pure removals |
| **−5** | `unresolved resource identity` on `chunk` (×2), `ic`, `label_lengths`, `resolved_lengths` | the cascades R2/R3 were poisoning |
| **+1** | `leak` | §6.4 |

24 + 31 = 55 removed `unsupported` leaves, and 11 + 1 = 12 added: the census's
own totals, leaf for leaf. No statement moved from refused to admitted anywhere
except at those 55 sites, and **nothing became `Safe` at any level, anywhere in
the corpus.**

### 6.4 The one leaf in the accusing direction, named prominently

`fluent-bit-chunk/flb_input_chunk.c`, `flb_input_chunk_get_direct_routes`,
preprocessed line **19399**: `leak: live obligation acquired at preprocessed
line 19357`. The corpus leak count moves **271 → 272**.

It is not a new claim, and it is not a new kind of claim. The function's body
([`logs/probe-direct-routes.txt`](logs/probe-direct-routes.txt)) contains this
shape seven times, and six of them already reported at the base tree:

```
L19396  Call flb_errno_print(…) nested=__errno_location   /* was: refused, poisoning */
L19397  Call flb_free(label_lengths) -> _
L19398  Unknown: call to uncontracted function `flb_input_chunk_destroy_direct_routes`
L19399  NoOp  return -1;                                   !! leak from L19357
```

and the already-reporting L19392 exit is byte-for-byte the same three-statement
shape. What changed is only that the errno refusal at L19396 used to poison
every pointer in the reaching state, so `flb_free(label_lengths)` at L19397
reported `unresolved resource identity` instead of settling, and the exit at
L19399 could support no claim. With the refusal gone the release settles and the
function's existing claim fires on one more path.

Three things follow, and all three are stated rather than argued away:

1. **It is not a promotion.** The function was already `Unsafe`, the unit was
   already provisional `Unsafe`, and the movement is `Unknown` → `Unsafe` at the
   leaf, which is the refusing direction.
2. **It is provisional in exactly the documented sense.** The claim rests on
   `flb_input_chunk_destroy_direct_routes` being uncontracted; that callee very
   plausibly frees `result`, and the projection cannot see it. Invariant 2 —
   *a provisional `Unsafe` is never presented as an upstream defect* — governs
   here as it governs the six that already stood.
3. **The 271 pin moves, deliberately.** The card brief named it untouchable;
   this is the one place it moves, it moves by one, the mechanism is measured to
   the statement, and it is not repairable within this card's scope without
   *withdrawing* six existing claims. Recorded as `CONSTRUCTOR-RESIDUALS-LEAK-271`.

### 6.5 The baseline is not re-pinned

[`artifacts/`](artifacts/) is this card's after-measurement, archived with a
manifest carrying each SHA-256. The wave-16 baseline is untouched and stays
current until an integrator consolidation moves it; §6.3 and §6.4 are the input
to that decision.

---

## 7. What was touched, and the pins moved deliberately

### 7.1 The diff

| file | what |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | the `scalar_pointer_results` reader, its accessor and its precision guard; four new unit tests |
| `crates/qcore-c-gateway/src/variadic_nested.rs` | premise N4-D (`dereference_wrapper`), premise N10, `dereferenced_argument_span`, three new `NestedCallActual` fields |
| `crates/qcore-c-gateway/src/legacy_c.rs` | the span exemption, computed by calling `admitted_nested_call` itself; §3's borrow-liveness repair |
| `contract/qcore/libc/errno.qcore` | **new**: `__errno_location` |
| `contract/qcore/monkey/intrusive_list.qcore` | **new**: `mk_list_add` |
| `contract/qcore/fluent_bit/logging.qcore` | `flb_errno_print`, and the retirement of that module's own "left undeclared" entry for it |
| `contract/cycle19-resource-contracts.json` | the two new modules |
| `crates/qcore-c-gateway/tests/constructor_residuals.rs` | **new**: the held-out `d_*` family (17 tests) |
| `crates/qcore-c-gateway/tests/fixtures/contracts/constructor_residuals_generic.qcore` | **new**: the `d_*` declared premises |
| `proof/QCore/DereferencedResult.lean`, `proof/AuditDereferencedResult.lean`, `proof/QCore.lean` | §5 |
| pins in `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`, `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs` | §7.2 |
| `docs/reviews/r0_3/acceptance_matrix.json` | **two derived lines only** (§7.3) |

`crates/qcore-c-gateway/src/borrowed_actual.rs`, `aggregate_store.rs`,
`may_alias.rs` and `borrow_liveness.rs` are byte-unchanged. Nothing in
`report_leaks`, `alias_facts` or `AbstractState::join` moved, and the
borrow-liveness pass stays read-only, so `C02-P-ALIAS-LEAK-ORDER` is undisturbed
by construction as well as by measurement.

### 7.2 Pins moved deliberately, each keeping its load-bearing half

| pin | was | now | why |
|---|---|---|---|
| `qcore_contract_corpus.rs` corpus size | 67 | **69** | `libc/errno.qcore`, `monkey/intrusive_list.qcore` |
| `qcore_contract_corpus.rs` clause table | — | +3 rows | `__errno_location` 2, `flb_errno_print` 5, `mk_list_add` 4 |
| `contract_loader.rs` `(selected, v1, rejected)` | `(12, 31, 145)` | **`(13→14, 33→34, 145)`** | two more manifest modules, three more v1 declarations. The **rejected** count is unchanged, because both added modules are entirely v1 |
| `contract_loader.rs` module count | 67 | **69** | same |
| `contract_loader.rs` `source_assumptions().len()` ×2 | 12 | **14** | same |
| `contract_loader.rs` old-mode parity table | — | +3 names | listed **explicitly** rather than exempted, so the parity check still covers every previous name and each addition is visible as an addition |
| `neighbours.rs`, `prod_contracts.rs` `source_assumptions().len()` | 12 | **14** | same; every by-name assertion in both tests is unchanged, and `prod_contracts.rs` gains one asserting `flb_errno_print` is now declared |

No other pin moved in either direction. **No diagnostic code was added, changed
or downgraded, and no fail-closed gate was weakened**: the change is one widened
premise with a new premise added beside it, plus one strictly-more-refusing
repair.

### 7.3 One deviation from the brief, stated plainly

The brief says not to edit acceptance files. `docs/reviews/r0_3/acceptance_matrix.json`
carries two **derived** fields — `validation.lean_audits` and
`validation.lean_audit_files`, whose declared source is `proof/Audit*.lean` on
disk — and `scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
pins that the committed values equal the derived ones. Landing an audit module
therefore *requires* moving them or the standing gate fails: `72 → 73`, with
`AuditDereferencedResult.lean` inserted in the sorted list. That is the entire
diff to that file — two lines — and **no hand-maintained section, no
`named_followups` entry and no CT number was touched.** This is card
CTWIN-PUBLISH's §10 precedent, followed deliberately.

---

## 8. Gate

Run in this worktree at the card's final tree:

| gate | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2,767 passed, 0 failed, 8 ignored** across 172 targets — [`logs/workspace-tests.txt`](logs/workspace-tests.txt). The 18 tests of [`tests/constructor_residuals.rs`](../../../crates/qcore-c-gateway/tests/constructor_residuals.rs) were re-run after that transcript was taken, because one was added while it ran; all pass |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **73 Lean audits** pass, no new axiom |
| `python3 -m unittest discover -s scripts/tests` | 161 tests, OK |
| 185-unit pinned audit, twice | both exit 0, zero gateway failures (§6) |

All five provider packages were rebuilt at this tree before both corpus runs.
No `qcore-schemas` type, no provider source and no package manifest changed, so
no package version moved.

---

## 9. The increment paragraph (draft — for the integrator, not applied here)

> **The constructor's last two roots (CONSTRUCTOR-RESIDUALS):** card
> ALIAS-AGGREGATE's two reasoned refusals are dispositioned, one by
> implementation and one by correcting its own premise.
> **ERRNO**: a dereference of a neutral call's result — `errno`, spelled
> `(*__errno_location ())` after preprocessing — is admitted as one outer
> argument under a **refined** proved sentence rather than a violated one (card
> W4-EXPR-POS's *no pointer-valued operand* cannot reach a pointer-valued call,
> so the replacement is stated over what the projection tracks: an admitted
> dereference *binds, releases, escapes and addresses nothing*, with the four
> conjuncts discharged by N6, N7, N9 and the new scalar-pointee N10).
> `__errno_location` and `flb_errno_print` are declared against read bodies;
> measured reach is 18 sites for the admission and 24 more for the declaration
> alone, across 13 units.
> **PUBLICATION**: measured, `mk_list_add` needs no publication mode at all —
> card HASH-CONTRACTS had already settled the question, and the read body shows
> the callee frees nothing and moves no obligation, so plain `resource_borrowed`
> is faithful and the retention stays unmodeled (B03/R13). The real blocker on
> the two statements this card could not admit is card LOADER-ABI's
> declared-type reader (a function-pointer typedef; a union typedef), named
> `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`, and it is not the publication relation.
> A withdrawal experiment pins what remains: with those three statements removed
> the constructor reports **0** diagnostics and discharges **all six** exits.
> The constructor moves **11 → 6** diagnostics with **5 of 6** exits discharging,
> and stays `Unknown`. One **pre-existing fail-open** was found and repaired: a
> released identity at an admitted *nested* call's borrowed actual was `Safe`
> with no diagnostic, because card BORROW-LIVENESS's precondition read only the
> enclosing call's actuals. Corpus: headline unchanged at 2 Safe / 42 provisional
> Unsafe / 137 Unknown / 4 data-only, zero unit and zero function status moves,
> three of five artifacts byte-identical, 72 leaves attributed — and **one leak
> in the accusing direction (271 → 272)**, the seventh instance of a claim the
> same function already published six times, surfaced because a neighbouring
> refusal that hid it was admitted. 29 audited Lean theorems
> (`QCore.DereferencedResult`), no new axiom; contract corpus 67 → 69 by two
> deliberate additions. Evidence:
> `docs/reports/remaining-constructor-residuals/README.md`.

---

## 10. Named follow-ons

| name | what |
|---|---|
| **`CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`** | §4.2 — card LOADER-ABI's ABI reader will not commit to a **function-pointer typedef** or a **union typedef** parameter, so `msgpack_packer_init`, `pthread_mutex_lock` and `pthread_mutex_unlock` are withdrawn at every call site and cannot be declared usefully. These are the constructor's **only** remaining residual: withdrawing all three makes it `Safe` with all six exits discharged (§4.3). Widening that reader moves contracts corpus-wide in the *admitting* direction and needs its own card with its own two corpus measurements. |
| **`CONSTRUCTOR-RESIDUALS-LEAK-271`** | §6.4 — the leak count moves 271 → 272. The claim rests on an uncontracted callee that plausibly releases; six identical claims already stood in the same function. Deciding it needs either a contract for `flb_input_chunk_destroy_direct_routes` or an unresolved-effect rule for uncontracted callees that would *withdraw* the six, which is a downgrade this card is not scoped to make. |
| **`CONSTRUCTOR-RESIDUALS-ABI-EXPRESSION`** | §2.3 — card LOADER-ABI's reader has no "a declaration begins a statement" guard, so a unit containing `return * f ( ) ;` reads a second, differing "declaration" of `f`, composes it as ambiguous and withdraws the contract. Fail-closed, asserted in `a_call_that_is_not_a_declaration_does_not_retire_the_fact`, and not repaired here because un-withdrawing contracts is a corpus-wide admitting change. |
| **`PUBLICATION-MODE`** | §4.1 — a declared publication mode (a published identity withdrawn from settlement, never discharged) is implementable **without a schema change**, as a side-channel clause in the `resource_variadic_data_tail()` / `resource_literal_argument` pattern. It is not landed because no callee this card measured needs it: on `mk_list_add` it would be unfaithful in the refusing direction. It becomes the right instrument for a callee whose read body *does* move or delete a caller obligation. |
| **`CONSTRUCTOR-RESIDUALS-DEREF-SCOPE`** | The admission is one argument position of one statement-position call. A dereference of a call result in a *branch condition*, a *return expression* or an assignment right-hand side keeps its existing refusal; card W4-EXPR-POS's expression positions have their own predicate and this family is not threaded there. |
