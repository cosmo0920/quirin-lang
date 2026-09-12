# LOOP-V1 — the first checked source loop (wave 25)

Card brief: *"The language has NO loop construct — a recorded blocker
(`STD-BUFFER-COPY` "needs the loop or the bulk operation the flat route has no
statement for"; HTTP header iteration; any bulk byte work). This card lands the
first checked source loop."*

Base commit: main `a53fe2a4` (the wave-24 close).

Design note written before the implementation, on the `STD-BUFFER` precedent:
[`docs/design/SOURCE_LOOP_V1.md`](../../design/SOURCE_LOOP_V1.md).

**What landed.** `source.loop.v1` — `for NAME in 0..BOUND { … }` — a counted
loop whose bound is a check-time constant, whose index is a body-scoped scalar
taking `0 … BOUND-1` in order, and whose body's **net custody effect must be the
identity**. It is admitted on `source.buffer.v1`; every other route keeps the
refusal it already had, measured and pinned. Checking is **exact bounded
unrolling**; emission is a **genuine C11 `for` loop**. `STD-BUFFER-COPY` is made
real and byte-compared under ASan/UBSan/LSan. **One reserved word** (`for`),
**zero new tokens**, **`Q097`–`Q099`** under a new `Loop` pass, 36 tests, 20
audited Lean entries in one new audit file, no new axiom.

**Two measurements carry the card.** The first is why the checking route is
unrolling and not a fixed point (§3) — and it is not a cost argument. The second
is that the per-iteration custody discipline was almost entirely **already
there** (§4), which is this card's version of `IF-STATEMENT`'s finding that five
of six models already branched.

---

## 1. The census (standing constraint 14)

Over all **418** `.qcore`/`.qc` files in the tree, counting *code* positions
separately from `//` comment text (`measure/census.py`, and a second narrowed
pass over QCore source embedded in Rust string literals in
`measure/census_rust.py`).

| Candidate | Code | Comment | Verdict |
|---|---|---|---|
| **`for`** | **0** | 95 | **reserved** — collides with nothing |
| **`in`** | **6** | 170 | **not reserved** — see below |
| `repeat` | 0 | 0 | unused; rejected on grounds other than collision (§2.1) |
| `loop` | 3 | 7 | all three a `borrow<'call, mut, EventLoop>` **parameter name** in `contract/qcore/monkey/wrapper_paths.qcore` |
| `while`, `break`, `continue`, `range`, `bound`, `idx` | — | — | **already** reserved |
| `count` | 32 | 15 | `calloc(count: usize, …)` parameter names plus `ensures normal resource_borrowed(count);` references, across four pinned contract modules |
| `body` | 8 | 99 | a **binding name** in two shipped fixtures (`let body = "hello, world";`) — the word `HTTP-QUIRIN-V1` also rejected |
| `index` | 1 | 6 | inside a quoted string literal (`"/index.html"`), not a word |
| `to`, `each`, `do`, `end`, `through`, `over`, `until`, `step`, `times`, `upto`, `iter`, `stride`, `iterate`, `range` | 0 | 1–182 | comment text only |

### 1.1 `in` collides, and it is exactly the `len` shape

Four of `in`'s six code occurrences are parameter names that the
referenced-names filter already shields (a word followed by `:`). **Two are
genuine references**:

```
contract/qcore/fluent_bit/input_chunk_helpers.qcore:114
    ensures normal resource_borrowed(in);
```

and the same clause in `crates/qcore-c-gateway/tests/fixtures/contracts/o04_path.qcore`,
both naming the `in` parameter of
`extern fn generate_chunk_name(in: ptr<mut, u8>, …)`.

That is character-for-character the shape card `STRING-SLICE` measured for
`len` — a clause-word candidate occurring as a *reference* inside the pinned
contract corpus — and it takes exactly that card's answer: **narrow the rule
rather than edit a corpus this card may not touch**. `in` is skipped by
`frontend::referenced_names` at the one position a `for` header puts it (index 2,
immediately after `for NAME`) and is an ordinary word everywhere else. Both
directions are pinned: `in_is_an_ordinary_name_outside_a_for_header` compiles a
module that binds `in` and reads it back, and every accepted fixture here writes
`in` inside a header.

### 1.2 What `for`'s reservation does and does not claim

`for` is in `is_reserved_source_word`, and it is **measured load-bearing**:
without it the header's own first token is collected as a referenced name and
every loop refuses at `Q013`.

That reservation governs *reference collection* only. `let for = 2;` still binds
the name — exactly as it does for `drop`, `close` and every other reserved
statement head, which is pre-existing behaviour rather than anything this card
decided. `for_is_claimed_at_a_statement_head_and_not_everywhere` pins both uses
in one module. What this card claims is the **statement-head position**, which
was a `Q000` parse failure before it.

### 1.3 Zero new tokens

`..` is **two existing `Symbol::Dot` tokens** read positionally, on the
precedent that `||` was two `Symbol::Pipe` tokens before `OR-AND-LOWERING` gave
it a token of its own. The lexer is not touched by this card at all.

---

## 2. The form, and why this one

```ebnf
stmt   = "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}" ;
BOUND  = INTEGER | NAME ;
```

### 2.1 `repeat N` undershoots, measurably

The brief offered `repeat N { … }` and asked the choice to be argued. It
undershoots the recorded consumers, and not marginally: `STD-BUFFER-COPY` is
`dst[i] = src[i]`, HTTP header iteration is indexed, and every bulk byte
operation on `source.buffer.v1` is indexed — the route writes one byte per
statement at an index. A repetition with no index would leave all three
blockers exactly where they were, so the card would have landed a construct with
no consumer.

`repeat` is also census-clean at zero occurrences, so nothing about the spelling
decided this. The consumer did.

### 2.2 The low end is written out

`0..BOUND`, with the `0` spelled rather than implied. That keeps the header
readable as a range while admitting exactly one range per bound, so the
iteration values need no derivation — and a reader who writes `1..n` gets a
refusal naming the rule (`Q097`) instead of a loop that runs a different number
of times than it looks like it does. `LOOP-RANGE-LOW`.

### 2.3 The bound's decidable tier is narrower than the index tier, and says so

A bound is a literal, a name bound by a literal `let`, or a name bound by
`extent(NAME)` — the three a *lowering* can compute, because the count has to be
in the lowered event before the checker runs. `source.buffer.v1`'s **index**
tier is wider: it also admits a name bound by a byte read, whose value the
checker computes but the lowering does not. A bound read out of a byte therefore
refuses at `Q098`, and the narrower half is recorded as `LOOP-BOUND-FROM-BYTE`
rather than closed over by deciding the count in a second place.

`extent(NAME)` being in the tier is what matters in practice, and it is what the
payoff uses: a copy's length is the source buffer's extent.

---

## 3. The checking route, and the measurement that decided it

The brief asked for both routes to be measured and one argued. They were.

### 3.1 What the tree has

| Machinery | Present on `a53fe2a4`? |
|---|---|
| `OwnershipOp::Branch` and its join | yes |
| `Flow::join_branches` | yes |
| `CfgTerminator::Branch` | yes (card `IF-STATEMENT`) |
| a **loop terminator** or back edge in any IR | **no** — `CfgTerminator` is `Goto`, `TryAcquire`, `Check`, `Branch`, `ReturnOk`, `ReturnError` |
| a **widening operator** on a source-side domain | **no** — every `widen` in the tree is under `crates/qcore-c-gateway/`, the existing-C analysis, over a different domain |
| a fixed-point driver over `SourceOp` | **no** |

So a fixed-point route would add three pieces that do not exist.

### 3.2 But cost is not the argument

**The fixed point lost because it is incompatible with this family of routes'
evidence, not because it is more work.**

Every route that could admit this construct publishes a certificate whose
contents are *exact values* — the bytes a buffer holds, the digits a line
carries — and validates them by a **byte comparison** between the reference
execution and a native run. A fixed point over a back edge computes a sound
over-approximation of exactly those values: it is the operation that replaces
"byte 3 is 65" with "byte 3 is some byte". A reference interpreter that cannot
say which byte cannot predict the line the native run prints.

That is the same wall `STD-BUFFER-VARYING-INDEX` records for an index and
`PIPE-SCALAR-RESULT` for an address. Widening would not make this route cheaper
to check; it would make this route's **evidence unwritable**.

And since the bound is a number the checker holds, unrolling is available *and
exact*, so there is nothing left for an approximation to buy.

### 3.3 The cost is bounded rather than hidden

`MAX_ITERATIONS = 4096` (equal to `MAX_EXTENT`, so a traversal of the largest
admissible buffer is expressible and nothing larger is) and
`MAX_UNROLLED_EVENTS = 8192`. An entry above either is refused at `Q098` by
name.

The budget is a **product** over a nest rather than a depth limit, because the
product is what the checking route actually costs: two loops of eight are
cheaper than one loop of a hundred, and a depth limit would say otherwise.
`source.branch.v1` admits unbounded nesting because it does not unroll; stating
that difference is more useful than matching a number.

---

## 4. The per-iteration custody discipline was almost entirely already there

This is the card's `IF-STATEMENT` moment. Measured on the base commit, then
re-measured against the implementation:

| Body shape | Refused by | New machinery? |
|---|---|---|
| acquires without releasing | the **unedited** cycle-3 `check_ownership`, on the unrolled list: two acquisitions of one live name, or a leak at the exit | no |
| releases a resource it did not acquire | the same checker: two releases of one resource (`DoubleDrop`) | no |
| an index outside the extent at some iteration | `source.buffer.v1`'s own bounds rule, at that iteration | no |
| acquires **and** releases | **admitted** — it is the identity | no |
| any of the above at a bound of **one** | nothing would expose it | **yes** — `Q099` |

So the explicit gate exists for exactly one reason: to make the rule hold at a
bound of one, where unrolling alone exposes nothing. Both halves are pinned —
`the_inherited_checker_refuses_a_repeated_release_on_its_own` shows the
inherited refusal arriving as `OwnershipError::DoubleDrop` from the unedited
checker, and `a_body_that_acquires_without_releasing_is_not_the_identity` shows
`Q099` firing at a bound of one.

`source_loop::iteration_is_custody_identity` is written over `OwnershipOp`
alone and knows nothing about any route, so a second route admitting the
construct inherits the gate rather than restating it
(`the_identity_gate_is_written_over_the_ownership_ir_alone`).

### 4.1 What the signature compares, and what it deliberately does not

The gate compares owner names, resource types and footprints — not resource
**identities**. A body that releases a buffer and acquires an equivalent one
under the same name *is* the identity on custody, and calling that a change
would refuse the shape the discipline exists to admit.
`the_signature_reads_the_footprint_and_the_owners_and_not_the_identity` pins
that two states differing only in the resource id have equal signatures.

### 4.2 The one thing unrolling forced, and why it weakens nothing

A body-local binding appears once per iteration in the unrolled operation list,
and `check_ownership` refuses re-binding a name it has seen dropped
(`BindingAlreadyExists`). That is a fail-closed gate and it was **not weakened**.

The answer is to model the scoping faithfully: each iteration's body-local
binding carries its own scope in the name — `c#0`, `c#1` — exactly as the
emitted C's second `unsigned char *b_c` is a different object, and exactly as
this card's own scoping rule already says (a body's bindings are body-local).
`#` cannot appear in a QCore identifier, so a scoped name can never collide with
one a program wrote.

**Nothing is laundered by it.** A body that acquires without releasing still
leaks: each iteration's distinct name is live at the exit and `NormalPathLeak`
names every one of them. `the_body_is_unrolled_exactly_and_its_bindings_are_per_iteration`
pins the four operations of a two-iteration acquire-release body and asserts the
inherited checker admits them; `a_body_that_acquires_without_releasing_is_not_the_identity`
pins the leak arriving beside `Q099` on a body that does not release.

**One honest consequence**, recorded rather than smoothed over: the inherited
checker's message is carried verbatim (that is what makes "inherited" a
checkable claim), so a reader sees `live resources escape normal exit: c#0`
rather than `c`. The scope suffix is informative — it names the iteration — but
it is a name no source wrote. `LOOP-SCOPE-SUFFIX-IN-MESSAGE`.

### 4.3 The peak-live scan, narrowed because unrolling forced it

`check_buffer_program` computed `peak_live_bytes` by re-executing **every**
prefix of the operation list, which is quadratic in the number of events — fine
at a dozen, not fine at four thousand. The live footprint of the cycle-3 model
grows at an `Acquire` and nowhere else, so the maximum over every prefix is
attained at length zero or immediately after an acquisition;
`source_loop::peak_candidate_prefixes` scans only those.

This **changes no published value**, and that is what the test says:
`the_peak_is_the_same_number_a_full_prefix_scan_gives` computes both and asserts
equality on a sequence with two acquisitions and an interleaved release. The
52-test `std_buffer` suite is unchanged and still passes.

---

## 5. Emission: a real `for`, and why the asymmetry is right

The checker unrolls; the emitter does not.

```c
for (int32_t s_i = INT32_C(0); s_i < INT32_C(4); s_i += INT32_C(1)) {
    int32_t s_v = qcore_bytes_load(b_src, (size_t) 4u, (size_t) (s_i));
    qcore_bytes_store(b_dst, (size_t) 4u, (size_t) (s_i), s_v);
}
```

The two halves are asymmetric on purpose, and each is right for its own reason.
The checker needs each iteration's exact values, because the certificate
publishes them and the evidence is a byte comparison. The emitted C needs none
of that: it needs to be the program a C reader would have written, and it needs
to stay one program rather than growing linearly in a bound that can reach 4096.

The asymmetry also buys something. Because the emitted C **computes the index
itself**, the native run is a *check* of the unrolling rather than a restatement
of it: a reference observation computed by unrolling and a native observation
produced by a rolled C loop that disagreed would show up as a byte difference.
`the_emitted_c_is_a_real_for_loop` pins the loop header, the index-parametric
store, and that the body's store is written exactly **once**.

The emitter's environment is scoped exactly as the model's is — a body's
declarations do not outlive the C block either — and the selective accessor
prelude `STD-BUFFER` established now scans at every depth, so a program whose
only byte write is inside a loop body still carries a writer
(`the_accessor_prelude_reaches_a_write_that_only_a_loop_body_performs`).

---

## 6. The payoff: `STD-BUFFER-COPY` is real

`compiler/tests/fixtures/source_loop/copy.qcore`:

```qcore
let src = bytes(4);
src[0] = 81; src[1] = 67; src[2] = 111; src[3] = 114;
let dst = bytes(4);
let n = extent(src);
for i in 0..n {
    let v = src[i];
    dst[i] = v;
}
for k in 0..3 { print dec k; }
```

**The copy is written with no new byte statement at all**: the read and the
write are `source.buffer.v1`'s own, and what the loop adds is that they run four
times with a different index each time. The bound is `extent(src)`, read out of
the ownership ledger's own footprint, so all eight accesses are proved in range
before a byte of C is emitted.

The second loop is the other half of the payoff and it was free: `k` is an
ordinary integer scalar, so rendering it needs no new operand form — the
existing `source.std-io.v1` `Scalar` operand already denotes it.

Evidence: `a_bounded_copy_between_two_buffers_is_admitted_and_exact` pins the
four writes into `dst` with their values, and
`the_emitted_program_matches_the_reference_per_stream` compiles the emitted C
under `-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer` with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and compares stdout against
the reference's stdout projection and stderr against its stderr projection.

**A positive leak control**, as `STD-BUFFER`'s is: the fixture allocates two
buffers and releases both, and one of the four writes into the second is
performed by the loop body. Had the loop written past an end, or had a release
been lost, ASan or LSan would have said so rather than this test passing.

### 6.1 What `STD-BUFFER-COPY` needed and what it still does not have

The follow-on read: *"Needs no new custody shape, but needs the loop or the bulk
operation the flat route has no statement for."* That is discharged: a bounded
copy is expressible, checked and executed.

Not discharged, and named rather than implied: a copy whose length is not a
check-time constant (`STD-BUFFER-VARYING-INDEX`, unchanged), and a **bulk**
operation — one `memcpy`-shaped statement rather than a loop
(`STD-BUFFER-BULK-COPY`). The loop is the capability the follow-on asked for;
the bulk statement is a different one and this card does not claim it.

---

## 7. Route admission, measured

v1 admits the construct on **`source.buffer.v1` only**, on the `IF-STATEMENT`
precedent, and every other route's outcome is measured and pinned rather than
widened:

| Route | Refusal | Pinned by |
|---|---|---|
| `source.std-io.v1` | `Q030` — "models one flat statement sequence; this statement opens a nested block" | `the_standard_io_route_still_refuses_a_nested_block` |
| scoped tasks | `Q500` — "statement is outside the copy-scalar task execution profile" | `a_task_scope_still_refuses_a_loop` |
| the sequential cleanup route | its own refusal | `the_sequential_route_still_refuses_a_loop` |
| `source.event.v1`, `source.socket.v1`, `source.http.v1`, `source.string.v1`, extern | their own nested-block refusals; **not** pinned by this suite, because each is reached only through its own route selector | recorded as a coverage gap, not a claim |

Follow-on: `LOOP-ROUTE-WIDENING`.

**The sequential route was considered and deliberately not taken.** A loop on
that route would be a custody-identity body over `drop`/`move`/`qcore_check` —
which is to say a repetition of statements that, being the identity, does
nothing observable. It has no recorded consumer, and landing it would have cost
a second lowering and a second emission path for a construct nobody asked for.
`LOOP-SEQUENTIAL`.

---

## 8. Diagnostics band

A new semantic pass, `Loop`, owning **`Q097`–`Q099`** — the three codes
`HTTP-QUIRIN-V1` returned to the pool from its pre-assigned `Q090`–`Q099`, and
the band the integrator pre-assigned to this card. Spec §L21 and grammar §G07.11
updated in the same change.

| Code | Condition |
|---|---|
| `Q097` | the loop's **shape**: a header outside `for NAME in 0..BOUND`, a range whose low end is not the written-out `0`, a `break` or `continue` in the body, a `return` in the body, a body binding that shadows one the entry already made; also the pass's generic presentation code |
| `Q098` | the **bound**: not a check-time constant the lowering can compute; zero; negative; above `MAX_ITERATIONS`; or an entry above `MAX_UNROLLED_EVENTS` unrolled |
| `Q099` | one iteration of the body is not the **identity** on custody |

A separate pass on the `IF-STATEMENT` precedent, and for that card's reason: the
construct is a *control shape*, not a capability of any bundled module, and a
route admits it the way a route admits `if`. It is separate from
`StandardBuffer` on purpose — that pass decides what a byte access means, while
this one decides how many times a body runs and what one pass may do to custody.

**No existing code changed meaning.** A bounds violation inside a loop body is
still `source.buffer.v1`'s `Q053`, a custody refusal still its `Q054`, and a
body-local name used after the loop still `Q052` or `Q013` (§9.2). Only the
three conditions above are this pass's.

A collision discovered at integration relocates this **unshipped** band, per the
wave-3 `T07`/`T09` precedent; the code-to-condition mapping is stable under such
a shift.

---

## 9. Deviations and findings

1. **`in` collides in the pinned contract corpus** (§1.1), exactly the `len`
   shape. Handled by the narrow contextual rule rather than by a global
   reservation or a corpus edit.
2. **`repeat N` was rejected on consumer grounds, not census grounds** (§2.1).
   The word is free; the construct is not enough.
3. **`for` had to be added to `is_statement_start`, and that was found by
   measurement.** Without it, `last_top_level_block` ran past the first loop's
   body and claimed a *later* loop's block as it — so a module with two loops
   bound the first loop's index over the second loop's body and refused at
   `Q013`. The list already carries a comment saying every statement leader must
   be in it; this card is the second to learn it the hard way.
4. **A body-local name used after the loop refuses at two different codes**,
   and the difference is `STD-BUFFER` §7.3's asymmetry rather than a second
   rule. Name resolution drains every token before an `Assignment`'s `=`, so
   the name used as an *index* (`b[i] = 0;`) is never collected and this route's
   `Q052` catches it, while the same name used as the *stored byte*
   (`b[0] = v;`) sits after the `=`, is collected, and refuses earlier at
   `Q013`. Both halves are pinned.
5. **The ownership model's rebind gate forced a scoping decision** (§4.2), and
   it was resolved by modelling the scoping faithfully rather than by weakening
   the gate.
6. **The peak-live scan was quadratic and had to be narrowed** (§4.3). The
   narrowing changes no published value and a test says so.
7. **`Q099` and the inherited leak refusal can both fire on one program**, and
   that is stated rather than deduplicated: they are different conditions —
   "this body is not the identity" and "this program leaks" — and the second is
   the unedited checker's own.
8. **The loop is `source.buffer.v1`'s only nested block**, so the route's
   flat-sequence refusal (`Q051`, "this statement opens a nested block") is now
   reached by every nested block *except* a `for`. That statement in the route's
   own header and grammar is updated; the refusal itself is unchanged.
9. **No file under `crates/qcore-verification/` was touched at all.** The route
   selector already fires on a buffer construct found at any nesting depth, so
   the driver needed no edit — which is why the "driver.rs edits minimal and
   flagged" instruction has nothing to flag.

---

## 10. Evidence

### 10.1 Tests

`compiler/tests/source_loop.rs` — **36 tests**, including one sanitized native
run. Two new files under `compiler/tests/fixtures/source_loop/`:
`copy.qcore` (the payoff) and `bytes_expected.c` (the byte-identity baseline,
§10.3).

Every refusal case is written inline against the same declarations, so each
measures the rule it names. No pinned expectation was edited and no existing
fixture moved. `compiler/tests/std_buffer.rs` is unchanged and its 52 tests
still pass.

### 10.2 Lean

`proof/QCore/SourceLoop.lean` (new) and `proof/AuditSourceLoop.lean` (new) —
**20 audited entries**. `QCore.lean` imports the module, so `AuditAll.lean`
includes it in the whole-project axiom closure. **No new axiom, no placeholder,
no `native_decide`** — every audited theorem depends on nothing, or only on the
allowed `propext` and `Quot.sound`. `check_lean_audits.py` moves **103 → 104**.

Load-bearing statements, by family:

* **Unrolling is sound** — `running_the_unrolling_is_iterating_the_body` is the
  headline: running the flat list the inherited checker is handed *is* running
  the body `count` times, so the program a reader wrote and the program the
  checker decides are the same program with nothing widened between them. With
  `run_append`, `a_loop_unrolls_to_its_body_repeated`, and
  `a_nested_loop_unrolls_by_the_same_rule` (a nest is checked by the same rule
  at every depth rather than by a second one).
* **The zero bound** — `a_zero_bound_unrolls_to_nothing` and
  `a_zero_bound_runs_no_event` are the *fact* the refusal rests on: its body is
  never examined, so admitting it would be the one way a statement could enter a
  program unchecked. `a_bound_of_one_runs_the_body_once` makes the refusal about
  zero rather than about small numbers.
* **Per-iteration identity** — `an_identity_body_iterates` lifts one pass to
  every bound, which is what makes the gate a statement about the body rather
  than about the count; `a_body_of_stores_and_lines_is_the_identity` and
  `an_acquire_then_release_body_is_the_identity` are the positive controls; and
  `a_body_that_only_acquires_grows_the_ledger` /
  `a_body_that_only_acquires_is_not_the_identity` are the non-vacuous converse
  in the accusing direction.
* **The index** — `the_index_takes_every_value_below_the_bound` and
  `the_index_never_reaches_the_bound`.
* **The cost** — `a_loops_cost_is_its_bound_times_its_body`, stated as the
  product it is, which is why the budget is a product rather than a depth.
* **Neutrality** — `a_loop_free_sequence_unrolls_to_itself` against a `loopFree`
  predicate written independently of `unroll`, with
  `a_loop_is_the_one_item_that_is_not_loop_free` making the claim non-vacuous.
* **The wall as one list** — `refused_shapes_are_exactly_these`, so a widening
  must change a theorem rather than slip past one (the
  `QCore.StdIo.the_operand_forms_are_exactly_these` convention).

**Not proved**, carried over from the family's convention: that the Rust
lowering extracts these items faithfully from source tokens (a tested Rust
boundary); that a C compiler's `for` runs the body the same number of times the
model does (that is what the sanitized native byte comparison cross-checks); and
anything about a bound the checker cannot compute, which is absent from the
model by construction — `count` is a `Nat`.

### 10.3 Byte identity, measured against main rather than asserted

The base commit's compiler was built in a scratch tree
(`measure/baseline.sh`, `git archive a53fe2a4`) and used to emit the C of every
`.qcore` file under `compiler/tests/fixtures/` and `examples/` that emits at
all. The card build was then run over the same inputs with the same spelled
paths and compared byte for byte (`measure/compare.sh`):

```
byte-identical: 65   differ: 0   no-longer-emits: 0
```

**All 65 programs that emit are byte-identical.** The in-suite pin
`a_module_without_a_loop_emits_byte_identically` compares `std_buffer/bytes.qcore`'s
emission against `fixtures/source_loop/bytes_expected.c`, which is the **base
build's own output** (verified identical to it with `cmp`) and carries no
`#line`, so it is path-independent.

A measurement hazard worth recording, because it cost a full re-run: the first
comparison reported 16 differences, every one of them a `#line` directive
carrying the **spelled input path** — the baseline had been emitted with
absolute paths and the card build with relative ones. Standing constraint 11 is
exactly about this (*do not normalize the spelled input path — that changes
`__FILE__`*), and the tool was corrected to use one spelling rather than the
compiler to use one path.

### 10.4 Corpus neutrality

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `crates/qcore-contract/`, `crates/qcore-verification/`,
`contract/qcore/` or `providers/` is touched, and `qcore-schemas` is untouched.
The whole diff is:

```
crates/qcore-core/src/source_loop.rs             (new: the construct's own model)
crates/qcore-core/src/std_buffer.rs              (the Repeat event, the walk, the gates)
crates/qcore-core/src/std_buffer_c_emit.rs       (the real C for loop)
crates/qcore-core/src/lib.rs                     (one module declaration)
crates/qcore-frontend/src/source_loop.rs         (new: the header grammar and census)
crates/qcore-frontend/src/source_std_buffer.rs   (the loop's lowering into the route)
crates/qcore-frontend/src/frontend.rs            (StatementKind::For, the pass, `for`, contextual `in`)
crates/qcore-frontend/src/lib.rs                 (one module declaration)
compiler/tests/source_loop.rs                    (new)
compiler/tests/fixtures/source_loop/             (new)
docs/QCORE_SPEC_r0_3.md                          (§L21, one row)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md          (§G07.11, new)
docs/design/SOURCE_LOOP_V1.md                    (new)
proof/QCore/SourceLoop.lean, proof/AuditSourceLoop.lean, proof/QCore.lean
docs/reports/loop-v1/README.md
```

The contract-corpus pin (73 modules), the 185-unit artifacts, the leak count of
**266** and the CT manifest are main's, and none of them moves for anything
here. Nothing is promoted; no `Unknown` became `Safe`.

### 10.5 CT candidates

None proposed. The construct adds a checked model, but mapping it would need a
conformance case whose artifact the case's own named workspace test runs; that
is a mapping lane's work, and growing the denominator without adding evidence is
what constraint 23 forbids.

### 10.6 Gate

See §11.

---

## 11. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement (checked out `d82464a277d0f42703702c4dfd9af6af38595a83`) |
| `cargo test --workspace --no-fail-fast` | *(recorded in §11.1)* |
| `cargo test -p qcore-compiler --test source_loop` | **36 pass**, including the sanitized native run |
| `cargo test -p qcore-compiler --test std_buffer` | **52 pass**, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean; one lint of this card's own (`needless_lifetimes`) was **fixed, not suppressed** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | *(recorded in §11.1)* |
| `cargo run -p qcore-compiler --example reevaluation_followup` | *(recorded in §11.1)* |
| `python3 scripts/check_lean_audits.py` | **104/104**, one new audit file, no new axiom |
| `python3 -m unittest discover -s scripts/tests` | *(recorded in §11.1)* |

Command evidence, with each step's exit code written to its own file so no
wrapper can lose it (the `STD-BUFFER` §10.5.1 hazard), is in the worktree-private
`measure/` directory: `measure/gate.sh` and the `*.exitcode` files beside it.
Measurement outputs are never under `target/` and never in the shared
scratchpad.

### 11.1 The closing run

*(filled in from `measure/*.exitcode` — see §11.2.)*

### 11.2 For the integrator

* **`Q097`–`Q099` allocated** to a new `Loop` pass; `Q100`+ is untouched. The
  `Q0xx` space is now exhausted below `Q100`.
* `AGENTS.md`'s diagnostics-namespace summary table needs one row for
  `Q097`–`Q099` → `Loop`, and its ledger's Lean-audit count moves **103 → 104**.
  **Neither was edited** — `AGENTS.md` is integrator surface and this card's
  brief forbids it.
* `docs/reviews/r0_3/acceptance_matrix.json` carries derived
  `validation.lean_audits` and `validation.lean_audit_files` rows checked by
  `scripts/tests`, so the derived-value tripwire
  `test_the_derived_facts_are_recomputed_rather_than_carried` trips on its
  `lean_audits` count. This card's brief forbids editing the matrix, so it is
  deliberately left stale for the wave integrator to regenerate — the same
  handling every wave's derived sections receive. The delta is entirely this
  card's one audit file.
* `qcore_frontend::frontend::StatementKind` gained one variant and `SemanticPass`
  one. Every existing `match` over them was extended, not defaulted.
* `qcore_core::std_buffer::BufferEvent` gained one variant and `BufferError` one.
  Same.
* **No standing constraint is touched.** No gateway, audit, contract-loader or
  provider file changed; no pinned expectation was edited; no fail-closed gate
  was weakened.

---

## 12. Named follow-ons

Each names its missing capability, because that is what makes it a boundary
rather than a backlog item.

| Name | What it is, and what it needs |
|---|---|
| `LOOP-VARYING-BOUND` | a bound the checker cannot compute. Needs the comparison discipline for an observation the reference interpreter can no longer predict exactly — the same missing piece `STD-BUFFER-VARYING-INDEX` names, and the same one that keeps `IF-RUNTIME-CONDITION` open |
| `LOOP-BOUND-FROM-BYTE` | the narrower half: a bound bound by a *byte read*, which the checker computes but the lowering does not. Needs the count to be decided in one place that runs after the values are known, or the lowered event to carry an unresolved bound |
| `LOOP-BREAK` | `break` and `continue`. Needs a per-exit cleanup discharge — the same capability `IF-ARM-RETURN` names, which no lowering on these routes has |
| `LOOP-BODY-RETURN` | `return` inside a body. Same missing capability, reached from the other side |
| `LOOP-RESOURCE-COLLECTION` | a loop over a collection of resources. Needs such a type; none exists |
| `LOOP-INDEX-ARITHMETIC` | `b[i + 1]`. Needs an expression tier, which this revision has none of |
| `LOOP-RANGE-LOW` | a range whose low end is not `0` |
| `LOOP-ROUTE-WIDENING` | admitting the construct on the std-io, string, event, socket, HTTP, extern, task, mutex or epoch routes. Each is a separate checked model, and each would inherit `iteration_is_custody_identity` rather than restate it |
| `LOOP-SEQUENTIAL` | the sequential cleanup route specifically. No recorded consumer (§7) |
| `LOOP-BODY-ATTRIBUTION` | a refusal inside a loop body is attributed to the `for` statement, because a body event has no top-level index. The `IF-ARM-ACQUIRE-ATTRIBUTION` shape: the code is right and the span is coarse |
| `LOOP-SCOPE-SUFFIX-IN-MESSAGE` | the inherited ownership checker's verbatim message names a scoped binding (`c#0`) a source never wrote (§4.2) |
| `LOOP-BUDGET` | the unrolled-event budget itself. Raising it needs the checking route to stop being linear in the unrolled total, which is the same thing as not unrolling |
| `LOOP-EXAMPLE` | a published `examples/loops/` family with a README, held to `EXAMPLES-SUITE`'s standard. This card ships a test fixture with that evidence but publishes no example, and the two are not the same deliverable — the `STD-BUFFER-EXAMPLE` precedent |
| `STD-BUFFER-BULK-COPY` | a `memcpy`-shaped single statement rather than a loop. A different capability from the one `STD-BUFFER-COPY` asked for (§6.1) |

---

## 13. Draft increment paragraph

> The first checked source loop (LOOP-V1): `source.loop.v1` lands
> `for NAME in 0..BOUND { … }` on `source.buffer.v1` — a counted loop whose bound
> is a check-time constant, whose index is a body-scoped scalar taking
> `0 … BOUND-1` in order, and whose body's **net custody effect must be the
> identity** — with every other route's refusal measured and pinned rather than
> widened. **The checking route is exact bounded unrolling and deliberately not a
> fixed point, and the reason is not cost**: a route that admits this construct
> publishes exact values and validates them by a byte comparison between its
> reference execution and a native run, so a widening over a back edge is
> precisely the operation that replaces "byte 3 is 65" with "byte 3 is some
> byte" — it would not make the loop cheaper to check, it would make the route's
> evidence unwritable. The tree has no loop terminator and no source-side
> widening operator either, but that is the smaller half of the argument. The
> card's second measurement is its `IF-STATEMENT` moment: the per-iteration
> custody discipline was **almost entirely already there** — an unrolled body
> that acquires without releasing presents the unedited cycle-3 ownership
> checker with two acquisitions of one live name, and one that releases what it
> did not acquire presents it with two releases — so `Q099` exists for exactly
> one reason, to make the rule hold at a bound of **one**, where unrolling alone
> exposes nothing. The gate is written over `OwnershipOp` alone, so a second
> route inherits it rather than restating it. Unrolling forced two things, both
> resolved without weakening a gate: a body-local binding now carries its
> iteration scope in the unrolled name (`c#0`, `c#1`), which is the faithful
> encoding of the body-local scoping rule and not a relaxation of the rebind
> gate — a body that acquires without releasing still leaks under every one of
> those names; and the quadratic peak-live prefix scan was narrowed to the
> prefixes that can *be* the maximum, which a test shows gives the same number.
> **The emitted C is a real `for` loop, not an unrolled body**, and the asymmetry
> buys evidence: because the C computes the index itself, the native run is a
> check of the unrolling rather than a restatement of it. The payoff makes
> `STD-BUFFER-COPY` real — a bounded `dst[i] = src[i]` copy written with **no new
> byte statement at all**, bound by `extent(src)`, byte-compared against a
> sanitized ASan/UBSan/LSan run as a positive leak control. **One reserved word**
> (`for`, census-clean at zero code occurrences across all 418 `.qcore`/`.qc`
> files) and **zero new tokens** (`..` is two existing `Dot` tokens); **`in` is
> deliberately not reserved**, because the constraint-14 census found it as a
> genuine reference inside the pinned contract corpus
> (`ensures normal resource_borrowed(in);`) — exactly the shape that kept `len`
> out of the global list, and it takes exactly that card's narrow contextual
> answer. `Q097`–`Q099` under a new `Loop` pass through the §L21 process, the
> band `HTTP-QUIRIN-V1` returned to the pool. 36 tests, 20 audited Lean entries
> in one new audit file, no new axiom, 103 → 104 audits. Corpus-neutral by
> construction, and byte identity is **measured rather than asserted**: the base
> commit's compiler was rebuilt in a scratch tree and all **65** emitted programs
> compare byte-identical. Evidence: `docs/reports/loop-v1/README.md`.
