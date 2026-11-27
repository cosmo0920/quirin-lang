# LOOP-UNROLL-BUDGET — the three-layer unrolling budget (wave 26)

Card brief, quoting the user's three statements verbatim: *(1)* "the loop
unrolling should now ceiling 1024 or 4096… it should be bounded upper bound and
this can be managed by `#unroll` like attributes with bound hints — so this can
be easily to unrolling loops." *(2)* "`[| #unroll, where #bound < 1024 |]` like
grammar can be programmer friendly". *(3)* "With the upper conditions, we can
permit the relatively longer loop."

Base commit: main `e26bb0c3` (the wave-25 close). Closes card `LOOP-V1`'s
follow-on `LOOP-BUDGET`.

**What landed.** `source.loop.v1`'s single ceiling becomes a **three-layer
budget**: an **absolute ceiling** of 4096 no declaration may raise, a **default
budget** of 256 a `for` with no attribute is held to, and a **licence** a
statement-adjacent `[| #unroll, where #bound < N |]` clause list declares
immediately before the `for` it governs. The declaration is **checked, never
trusted**. **Zero new diagnostic codes** — every refusal is a condition
extension of `Q097` or `Q098`, per the standing `Q-NAMESPACE-DECISION` note.
**One new token** (`#`), **no new reserved word**, and the `[|`/`|]` brackets are
the quant-index alias's own, borrowed rather than invented. 29 new tests, 22
audited Lean entries in one new audit file, no new axiom, 107 → **108** audits.

**The card is carried by one measurement, and it is not the one the brief
expected.** The cost of exact bounded unrolling is *not* a function of the bound
alone: three of the four bodies measured are linear and cheap at every bound the
construct admits, and the fourth — a body that acquires and releases once per
iteration — was **quadratic**, costing **56.4 s** of check time at a bound of
1024 where the same body costs **0.39 s** after a one-pass repair this card
lands — a factor of **145**. That repair is what made a ceiling of 4096
defensible; without it the honest answer would have been 1024, and it would have
**withdrawn** the full-extent traversal `LOOP-V1` deliberately made expressible.

---

## 1. The cost measurement, and what it decided

The brief asked the ceiling to be chosen from a measurement of "the real
check-time cost of the exact unrolling at increasing bounds … on representative
bodies", and the default from the same measurement. Both were measured through
the **real entry point** — `compile_files`, the same path `quirin check` takes —
in the `dev` profile, which is the profile the standing gate's `cargo test` runs
and therefore the cost CI actually feels.

Method (`measure/`, worktree-private and gitignored, never under `target/`):
`git archive` of the base commit into a scratch tree with `MAX_ITERATIONS`,
`MAX_UNROLLED_EVENTS` and `MAX_EXTENT` raised, so the grid can run **past** the
shipped ceiling; one process per cell, so the peak RSS read from
`/proc/self/status`'s `VmHWM` is that cell's own high-water mark. Four bodies:

| case | body | events per iteration |
|---|---|---|
| `store` | `b[0] = 1;` | 1 write at a constant index |
| `copy` | `let v = src[i]; dst[i] = v;` | the `STD-BUFFER-COPY` shape, both accesses at the varying index |
| `custody` | `let c = bytes(2); c[0] = 1; drop c;` | an acquisition and a release **per iteration** |
| `lines` | `print dec i;` | one rendered line |

### 1.1 BEFORE — the base commit's checking route

Wall-clock milliseconds; `—` is a cell that was still running when the report
closed (§1.5).

| bound | `store` | `copy` | `custody` | `lines` |
|---|---|---|---|---|
| 1 | 6.2 | 6.4 | 3.6 | 6.2 |
| 64 | 6.3 | 7.6 | **44.0** | 3.4 |
| 256 | 6.0 | 9.4 | **1 215.0** | 6.3 |
| 1024 | 10.1 | 20.1 | **56 353.5** | 4.4 |
| 4096 | 14.4 | 45.1 | — (§1.5) | 11.3 |
| 8192 | 24.7 | 90.7 | — | 10.2 |
| 16384 | 64.8 | 148.6 | — | 23.9 |

Peak RSS stayed between 17.3 and 25.9 MiB across every completed cell, growing
by about 8 MiB from a bound of 1 to a bound of 16 384 — **memory is not the
binding constraint at any bound this construct admits**, and the budget is a
time budget. (Full rows with RSS in
[`artifacts/cost_before.tsv`](artifacts/cost_before.tsv).)

Three bodies are linear and cheap. **`custody` is not**: 4× the bound costs 27×
and then 46× the time, which is an exponent near 2.8 — and at a bound of 1024,
**56 seconds** to check one loop of one program.

### 1.2 The diagnosis, measured rather than reasoned

The blowup was attributed by switching one computation off in the scratch tree
behind an environment variable and re-running the same binary
(`measure/diagnose.sh`):

| bound | `custody`, peak scan on | `custody`, peak scan **removed** |
|---|---|---|
| 64 | 44.0 ms | 7.6 ms |
| 256 | 1 215.0 ms | 23.9 ms |
| 1024 | 56 353.5 ms | **202.4 ms** |
| 4096 | — | 3 022.7 ms |

The right column has the computation **removed entirely**, so it lower-bounds
the repair rather than measuring it; the repair's own numbers are §1.4's. The
column is here to attribute the cost, not to claim it.

The cause is `check_buffer_program`'s peak-live-bytes computation. Card
`LOOP-V1` §4.3 had already narrowed a scan of **every** prefix to the prefixes
that can *be* the maximum — the empty one and each one ending in an acquisition
— which is linear in the number of acquisitions. But each of those prefixes was
still **re-executed from the start**, so a body acquiring once per iteration
paid a quadratic number of ownership steps. The narrowing fixed the count of
prefixes and left the cost of each one untouched.

### 1.3 The repair, and why it changes no published value

`ownership::peak_live_axis` replaces the rescan with **one pass**, taking the
maximum of the values the run passes through. The maximum over every prefix of a
sequence *is* the maximum of the values the sequence passes through, so this is
an equality rather than an approximation, and three things say so:

* `the_one_pass_peak_is_the_number_a_full_prefix_scan_gives` computes the number
  three ways — a full prefix scan, `LOOP-V1`'s narrowed candidate scan, and the
  one pass — on an unrolled acquire/release body beside two overlapping outer
  buffers, and asserts all three equal;
* `the_one_pass_peak_stops_where_the_model_refuses` pins the one behavioural
  edge: a prefix the model refuses ends the walk, exactly as a refused prefix
  was skipped by the scan, so the maximum reported is the maximum over the
  prefixes that executed and nothing about a refused one is invented;
* in Lean, `the_one_pass_maximum_is_the_maximum_over_every_prefix`, stated
  abstractly over **any** stepper and **any** measure so it is a statement about
  the shape of the computation rather than about this model's footprint.

`peak_candidate_prefixes` is **retained**, and `LOOP-V1`'s own test over it still
passes unchanged: it is now the statement of the equality rather than the route
that runs.

### 1.4 AFTER — the card's own checking route

The same grid against the card's code, constants raised the same way
([`artifacts/cost_after.tsv`](artifacts/cost_after.tsv)):

| bound | `store` | `copy` | `custody` | `lines` |
|---|---|---|---|---|
| 1 | 5.9 | 4.7 | 5.9 | 4.2 |
| 64 | 4.1 | 7.2 | 9.6 | 4.1 |
| **256** | 7.5 | 10.9 | **40.1** | 4.9 |
| 1024 | 8.0 | 19.5 | **389.0** | 4.7 |
| **4096** | 28.5 | 64.3 | **5 234.9** | 10.1 |
| 8192 | 45.9 | 161.7 | 24 422.3 | 15.3 |
| 16384 | 98.8 | 264.3 | 95 427.5 | 18.3 |

**The repair is worth 145× on the body that needed it**: 56 353.5 ms → 389.0 ms
at a bound of 1024, and the three linear bodies are unmoved within noise.

**Contention is disclosed rather than smoothed over.** This grid ran while the
BEFORE `custody` 4096 cell (§1.5) was still occupying a core, and the host was
under memory pressure throughout; §1.2's quiet lower-bound column reads
23.9 / 202.4 / 3 022.7 ms where this one reads 40.1 / 389.0 / 5 234.9 ms, and
part of that gap is the repair's own extra pass and part is contention — the two
are not separated, so **every number quoted for the repair is this table's**, the
slower and more conservative one. Nothing in the decisions below turns on the
difference: the gaps being argued are factors of ten and more.

`custody` is still superlinear after the repair, and the residual is **not this
card's**: `OwnershipState::validate_invariant` runs after every operation and
iterates the whole bindings map, which retains a dropped name (that retention is
what makes `BindingAlreadyExists` a fail-closed gate). A body binding a fresh
scoped name per iteration therefore grows that map linearly and pays for it
quadratically. Repairing it means touching the cycle-3 ownership model, which is
outside this card. Named: **`LOOP-OWNERSHIP-VALIDATE-QUADRATIC`**.

### 1.5 One cell not run to completion

The BEFORE `custody` cell at 4096 was started as a background measurement and
had not finished when this report closed; from the measured 256 → 1024 scaling
its cost is **at least 40 minutes**. It is recorded as unmeasured rather than
extrapolated into the table. Nothing depends on it: the argument is carried by
the 1024 comparison, where both numbers are measured.

### 1.6 The ceiling: **4096**, and why not 1024

Both candidates the user named were considered against the data.

**4096 is chosen.** After the repair, a bound of 4096 costs 28 ms (`store`),
64 ms (`copy`) and 10 ms (`lines`); the `custody` body costs 5.2 s on the
contended grid, which is a loop that acquires and releases 4096 times and is a
cost a program **asks for by declaring it** — against a pre-repair cost of at
least 40 minutes (§1.5) that no program had to declare at all. Peak RSS at 4096
is under 23 MiB on every body.

**1024 was rejected, and the reason is a capability rather than a number.**
`MAX_ITERATIONS` is 4096 because it *equals* `std_buffer::MAX_EXTENT`: "a
traversal of the largest buffer the byte-buffer route admits is expressible, and
nothing larger is". Lowering the ceiling to 1024 would withdraw that property
and make the largest admissible buffer untraversable — an invariant-1-shaped
loss of expressible programs — in exchange for a worst case the §1.3 repair
already removes. Choosing 1024 would have been choosing to pay a capability for
a defect that had been fixed in the same change.

### 1.7 The default budget: **256**

The criterion was stated before the number was picked: *an unattributed loop
must not cost more than compiling the module around it.* A small module on this
route compiles in roughly 5 ms, so the budget is the largest power of two at
which **every** measured body stays inside a few tens of milliseconds. The
numbers below are §1.4's — the conservative, contended ones.

| candidate | worst measured body, post-repair | verdict |
|---|---|---|
| 64 | 9.6 ms | inside, but leaves ordinary loops needing a declaration |
| **256** | **40.1 ms** | **chosen** — inside, ~8× a small compile |
| 1024 | 389.0 ms | rejected — ~80× a small compile, for a loop nobody declared |

256 is also what keeps the construct usable without ceremony: every loop in
every shipped fixture and in `LOOP-V1`'s own suite is inside it except the one
written *at* the ceiling (§7).

---

## 2. The census (standing constraint 14)

`measure/census.py` over all **422** `.qcore`/`.qc` files in the tree, counting
*code* positions separately from `//` and `/* */` comment text and from string
literals, plus a second pass over that tree's **48 393** Rust string literals.

The census is run against the **base** tree (`git archive` of `e26bb0c3`), so a
zero count means the spelling collided with nothing *before* this card claimed
it rather than that this card's own files were skipped.

| Candidate | `.qcore`/`.qc` code | comment | Rust literals | Verdict |
|---|---|---|---|---|
| **`#`** (character) | **0** | 112 | — (§2.2) | **claimed** — collides with nothing |
| **`unroll`** | **0** | **0** | **0** | free everywhere; **not** reserved (§2.3) |
| **`where`** | **8** | 25 | 83 | **collides** — see §2.1; contextual, not reserved |
| **`bound`** | 12 | 20 | 305 | **already reserved** (the epoch profile's `bound N effort M;`); nothing new claimed |
| `attr`, `attribute`, `hint`, `budget` | 0 | 0 | 0 / 11 / 0 / 4 | unused; none needed |
| `for`, `in` | 2 / 8 | 97 / 177 | — | `LOOP-V1`'s, unchanged |

### 2.1 `where` collides, inside the very brackets this card borrows

All eight code occurrences of `where` are inside a **quantitative index type**:
`quant<n where n := 5>` in `contract/qcore/language/quantified_indices.qcore`
and in six `docs/reports/remaining-v04/probes/` fixtures. That would be the
`len`/`in` shape again — except it is sharper here, because that type's
**readable alias is spelled `quant[| n where n := 5 |]`**: the same `[|`/`|]`
brackets, with `where` inside them.

The disambiguation is **positional and requires two tokens**. A statement head
is recognised as `[|` **followed by `#`**, and the census found `#` nowhere at
all in code position. The quant form has a `quant` word before its `[|` and no
`#` after it, and it is read by `quant_index::quant_opener_close`, which this
card does not touch. Measured rather than asserted:
`the_quant_index_alias_still_elaborates_identically_to_the_angle_form` compares
the bracket and angle spellings through `elaborate_source` and asserts equal
bindings, and `a_bracket_pipe_without_a_hash_is_not_an_attribute` pins that a
`[|` at a statement head *without* a `#` is not claimed by the loop pass at all.

`where` is therefore **contextual** and not added to
`is_reserved_source_word` — the `len` and `in` answer: narrow the rule rather
than edit a pinned corpus this card may not touch.

### 2.2 `#` in Rust string literals: read carefully, and it is not a hit

The raw count of `#` inside Rust string literals is 1 092, and **none of it is
QCore source**: the scanner's literal regex counts Rust's own raw-string
delimiters (`r#"…"#`) and the `c#0`/`c#1` scoped owner names `LOOP-V1`
introduced into the *unrolled ownership operation list*, which are internal
names no source writes. The load-bearing fact is the `.qcore`/`.qc` count, which
is **zero**.

`#` also remains **not an identifier character** — the lexer's word rule is
untouched — so `ITERATION_SCOPE_SEPARATOR`'s premise, that a scoped name `c#0`
can never collide with one a program wrote, survives this token exactly as it
stood.

### 2.3 No word is newly reserved, and that is a consequence rather than a choice

A `[| … |]` clause list contributes **no referenced name at all**:
`frontend::lexical_references` returns the empty set for the attribute
statement, because every word inside the list is a clause word of the attribute
grammar and none of them is a reference to a binding. So `unroll` needs no
reservation to be load-bearing, and a program may still write `let unroll = 1;`.
This is the first card in the family to add a clause vocabulary and reserve
**nothing**.

### 2.4 The `[| |]` adjacency pins, re-measured

Card `PIPE-OPERATOR`'s longest-match table (inherited by `OR-AND-LOWERING`) is
pinned by `qcore_frontend::lexer::tests::lexes_the_vertical_bar_family_by_longest_match`.
`#` is a **one-byte spelling with no two-byte member**, so the width-two table
cannot move. Both halves are checked:

* the pinned test itself passes **unchanged** (`cargo test -p qcore-frontend`,
  the `lexer` unit suite, no edit to that file at all);
* `no_row_of_the_vertical_bar_family_moves_when_the_hash_becomes_a_token`
  re-measures `[|>`, `[||]`, `a |> b` and `a || b` in this card's own suite,
  beside `[| #unroll |]`, so a future change has to break two tests rather than
  one.

### 2.5 `[|` at a statement head was previously unclaimed

Verified against the **base build** rather than asserted: in the constants-raised
scratch tree built from `e26bb0c3` (§1), `[| #unroll, where #bound < 1024 |]`
before a `for` fails at **lexing** — `unexpected character '#'` — so the slot
could not previously have been occupied by any program. The half of that
verification that survives into the card build is pinned in-suite by
`a_bracket_pipe_without_a_hash_is_not_an_attribute`.

---

## 3. The grammar

```ebnf
stmt = [ attr ], "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}" ;
attr = "[|", "#", "unroll", ",", "where", "#", "bound", "<", INTEGER, "|]" ;
```

**Statement-adjacent, by the integrator's placement decision** (announced to the
user, default standing): the clause list stands immediately *before* the `for`
it licenses. `for` remains the only loop head — no `loop` reservation — and
function-level attribute slots and `::`-style signature constraints are out of
scope, iceboxed as `ATTRIBUTE-SLOTS-DEFINITIONS` (§12).

Three implementation decisions inside that shape are worth stating:

**It is its own `StatementKind`, not a field of `For`.** Both halves are
deliberate. A separate statement keeps the `for` statement's own token window
unchanged, so every position `LOOP-V1` reads by index — the binder at 1, the
contextual `in` at 2 — still holds and none of that card's tests moved. And it
gives the reader a *place* to refuse a clause list that licenses nothing, which
is what a dangling attribute and an attribute before some other statement are.

**The `where` clause is required.** `[| #unroll |]` alone would have to mean
"license up to the ceiling" — a declaration with nothing in it to check — and
the whole point of the form is that the condition a programmer writes is
compared against the bound the checker computes. It refuses at `Q097` saying so.

**The pairing is carried by the reader that walks a statement sequence**, so
"immediately preceding" is a property of that walk rather than of a span
comparison — and because a loop **body** is such a sequence, an attribute
licenses an inner loop by the same code that licenses an outer one
(`an_attribute_licenses_an_inner_loop_too`).

---

## 4. The three layers, exactly

The decision is one total function in the core,
`qcore_core::source_loop::admit_unroll`, over **two numbers and two constants**.
It reads no body, no route and no cost.

```rust
pub fn admit_unroll(bound: u32, license: Option<UnrollLicense>) -> BudgetVerdict
```

| | layer | constant | rule |
|---|---|---|---|
| 1 | absolute ceiling | `UNROLL_CEILING = MAX_ITERATIONS = 4096` | no declaration may license a bound above it |
| 2 | default budget | `DEFAULT_UNROLL_BUDGET = 256` | a `for` with no `attr` may count at most this many iterations |
| 3 | licence | `MAX_DECLARED_LIMIT = 4097` | `#bound < N` licenses exactly `1 … N-1`, so `N` runs `2 … 4097` |

**`MAX_DECLARED_LIMIT` is `ceiling + 1`, and that is the honest spelling.**
`#bound < N` licenses bounds up to `N - 1`, so the rule binds what a declaration
can *license*, not the numeral it spells. The full-extent traversal is therefore
written `[| #unroll, where #bound < 4097 |] for i in 0..4096`, and nothing above
4096 is expressible at all. Both sides of that boundary are pinned by
`the_largest_declaration_licenses_the_full_extent_traversal_and_nothing_above_it`.

**The order of the two licensed tests is layer one's whole force.** A
declaration naming more than the construct models is refused **before** this
loop's own bound is looked at, so a two-iteration loop cannot carry a licence
for a 99 999-iteration one. In Rust that is the `if/else if` order; in Lean it is
`the_ceiling_is_judged_before_the_bound`, quantified over *every* bound.

**The licence raises the per-loop bound and nothing else.** The entry's
unrolled-event budget (`MAX_UNROLLED_EVENTS = 8192`) is untouched, so a nest of
licensed loops is still refused by what the checking route actually costs —
`a_licence_does_not_raise_the_entry_budget` pins a 300 × 300 nest of two
licensed loops refusing at `Q098` with the entry budget's own message.

**The decidable tier is untouched.** The attribute is not a way to declare a
bound; the bound is still a check-time constant the lowering computes, and a
varying bound still refuses at `Q098` as `LOOP-VARYING-BOUND`. A limit the
reader would have to compute — `#bound < n` for a bound name `n` — refuses at
`Q097` for the same reason from the other side: a licence the checker has to
compute is not one a reader can check against the loop.

---

## 5. Every refusal arm, and its condition home

**This card allocates no `Q` code.** The `Q000`–`Q099` space is exhausted and
`Q-NAMESPACE-DECISION` is in the ice box by user direction, so each arm below is
a **condition extension of a code `LOOP-V1` already owns**, on the
`SOCKET-EVENT-COMPOSE` / `NATIVE-LIBRARY-HEADER-DECORATION` precedent — the same
condition reached by a new route. The widening is stated in §L21 and in §G07.12,
not taken silently.

`Q097` owns the **shape** of the construct, and an attribute's shape is one:

| Arm | Test |
|---|---|
| an unknown clause word (`#inline`) | `an_unknown_clause_word_refuses_rather_than_being_ignored` |
| a clause list with no `where` condition | `an_attribute_without_its_condition_refuses` |
| a comparison other than `<` (`<=`, `>`, `==`) | `a_comparison_other_than_strictly_less_refuses` |
| a limit that is not an integer literal | `a_limit_the_reader_would_have_to_compute_refuses` |
| a clause list with anything further in it | (same reader, same code) |
| an attribute before a statement that is not a `for` | `an_attribute_before_a_statement_that_is_not_a_for_refuses` |
| a second clause list in a row | `a_second_clause_list_refuses_rather_than_one_winning` |
| a dangling clause list at the end of a sequence | `a_dangling_attribute_refuses` |

`Q098` owns every question about a **number**, and every budget refusal is that
same question:

| Arm | Test |
|---|---|
| unattributed, above the default — message **names the attribute and its exact spelling** | `a_loop_above_the_default_refuses_and_the_message_names_the_attribute` |
| a declared `N` above the ceiling, refused **before** the loop's bound | `a_declaration_above_the_ceiling_refuses_even_on_a_small_loop` |
| a bound the declared condition does not satisfy, naming **both** numbers | `a_bound_the_declared_condition_does_not_satisfy_refuses` |
| the condition is strict, so `N` itself is not licensed | `the_declared_condition_is_strict` |
| a condition **no** bound can satisfy (`#bound < 1`, `#bound < 0`), refused from the clause list alone and naming no bound | `a_condition_no_bound_can_satisfy_refuses_without_naming_one` |

Two boundary cases, both stated rather than left implicit:

**An *unterminated* clause list is a `Q000` parse failure**, where a *malformed*
one is this construct's own refusal. The difference is that there is no
statement to attribute a diagnostic to. `an_unterminated_clause_list_is_a_parse_failure`.

**A clause list the reader could not read suppresses the budget decision**, so
one mistake produces one refusal — `LOOP-V1` §9.9's rule applied to the
attribute. Without it, `[| #inline, … |] for i in 0..600` printed both the
unknown-clause refusal and the above-the-default refusal for the same mistake.
The suppression is the model's own third arm, `LoopBudget::Unreadable`, and it
suppresses **only** the budget: `a_refused_clause_list_suppresses_the_budget_decision_and_nothing_else`
shows the body still read and its own refusal still reported beside the
attribute's.

**The refusal teaches the spelling.** The above-the-default message is not a
rule to look up:

```
Q098: a `for` with no unrolling attribute is budgeted at 256 iterations and
this one counts 600; write `[| #unroll, where #bound < 601 |]` immediately
before the `for` to license it, up to the 4096-iteration ceiling
```

---

## 6. Erasure: the attribute is the checker's budget and nothing else

**Emission is unchanged.** The real C `for` loop `LOOP-V1` emits is untouched;
this card adds no emission unrolling and the attribute appears nowhere in the
emitted C. The claim is made twice, once on each side:

* **In Rust**, `an_attributed_and_an_unattributed_loop_emit_byte_identical_c`
  compiles two modules whose only difference is the clause list and asserts the
  emitted C strings equal — and then asserts the certificates agree too, so the
  attribute changes no published value either.
* **In Lean**, over an `Attributed` type that *can* carry a licence:
  `the_licence_erases_from_the_unrolling`, `the_licence_moves_no_event`,
  `the_licence_changes_no_cost` and
  `an_attributed_program_unrolls_to_its_unattributed_one`. Each is `rfl`, and
  that is the content: there is **no place in the construction** from which a
  licence could reach the unrolling.

**Reference/native byte comparison is unchanged**, and the card adds a positive
control for it: `a_licensed_loop_runs_sanitized_and_matches_the_reference_per_stream`
checks a licensed 600-iteration loop and a licensed 300-line loop, then compiles
the emitted C under `-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer` with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and compares **301 stdout
lines** against the reference's stdout projection and stderr against its stderr
projection. The reference execution unrolled; the native run rolled; they agree
byte for byte. It is also a positive leak control — the fixture acquires and
releases a buffer, so a lost release would be LSan's finding rather than this
test's silence.

---

## 7. What moved in `LOOP-V1`'s suite, and what did not

The brief asked for `LOOP-V1`'s 39 `source_loop` and 52 `std_buffer` tests to
pass unchanged. **`std_buffer` is unchanged: 52 of 52 pass with no edit.**
`source_loop` is **38 of 39 unchanged**, and the one that moved is the one this
card is about:

`the_largest_admitted_bound_is_admitted` writes `for i in 0..4096` — a bound
above the new default — so it now carries
`[| #unroll, where #bound < 4097 |]`. **The test's claim is unchanged**: the
largest bound the construct models is admitted. What moved is that reaching it
is now something a program *declares*, which is the card's entire purpose, so
the alternative to editing this fixture was not having the card. It is flagged
in the fixture's own doc comment, and the suite still reports **39 tests**.

No other pinned expectation was edited, no fixture moved, and no assertion was
weakened.

---

## 8. Corpus neutrality

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `crates/qcore-contract/`, `crates/qcore-verification/`,
`contract/qcore/` or `providers/` is touched, and `qcore-schemas` is untouched.
The contract-corpus pin (73 modules), the 185-unit artifacts, the leak count of
**266** and the CT manifest are main's and none of them moves. Nothing is
promoted; no `Unknown` became `Safe`.

**Every existing program is byte-identical.** Two independent reasons, and both
are checked rather than argued: no shipped `.qcore`/`.qc` file contains a `#` in
code position or a `[|` at a statement head (§2), so no existing program can
reach any new code path; and `LOOP-V1`'s own in-suite byte-identity pin
`a_module_without_a_loop_emits_byte_identically` — which compares
`std_buffer/bytes.qcore`'s emission against the **base build's own output** —
still passes.

The whole diff:

```
crates/qcore-core/src/source_loop.rs            (the three layers, the decision, the refusals)
crates/qcore-core/src/ownership.rs              (peak_live_axis: the one-pass peak)
crates/qcore-core/src/std_buffer.rs             (use it; two lines of call site)
crates/qcore-frontend/src/lexer.rs              (Symbol::Hash)
crates/qcore-frontend/src/frontend.rs           (StatementKind::LoopAttribute, the head, the pass, empty references)
crates/qcore-frontend/src/source_loop.rs        (the clause-list reader; the header takes a budget)
crates/qcore-frontend/src/source_std_buffer.rs  (the statement-adjacent pairing)
compiler/tests/loop_unroll_budget.rs            (new, 29 tests)
compiler/tests/source_loop.rs                   (one fixture amended, §7)
docs/QCORE_SPEC_r0_3.md                         (§L21: Q097 and Q098 condition text widened in place)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md         (§G07.12: the attr production and the three layers)
proof/QCore/LoopBudget.lean, proof/AuditLoopBudget.lean, proof/QCore.lean
docs/design/SOURCE_LOOP_V1.md                   (one pointer line: LOOP-BUDGET closed)
docs/reports/loop-unroll-budget/
```

---

## 9. Evidence

### 9.1 Tests

`compiler/tests/loop_unroll_budget.rs` — **29 tests**, including one sanitized
native run. Every refusal case is written inline against the same declarations,
so each measures the rule it names.

Beyond the arms tabulated in §5: the census and lexing pins (§2.4, §2.5), the
inner-loop licence, the entry-budget independence, the erasure pair, the
three-layer decision exercised directly over the two numbers
(`the_three_layer_decision_is_total_and_ordered`, which also asserts that
exactly two of the five verdicts admit), the two peak-repair equalities, and
the **measured route refusals**: a clause list on the standard-IO route and on
the sequential route reaches a reader that does not model it and is refused at
that route's own code, beside the refusal the `for` after it already had
(`a_route_that_does_not_admit_the_loop_refuses_the_attribute_at_its_own_code`,
the `LOOP-ROUTE-WIDENING` precedent).

### 9.2 Lean

`proof/QCore/LoopBudget.lean` (new) and `proof/AuditLoopBudget.lean` (new) —
**22 audited entries**. `QCore.lean` imports the module, so `AuditAll.lean`
includes it in the whole-project axiom closure. **No new axiom, no placeholder,
no `native_decide`**: every audited theorem depends on nothing, or only on the
allowed `propext` and `Quot.sound`. `check_lean_audits.py` moves **107 → 108**.

Load-bearing statements, by family:

* **The decision is total** — `the_verdicts_are_exactly_these` and
  `the_decision_is_total`: five answers, no partiality, no default case, and a
  sixth answer must edit a theorem (the
  `QCore.StdIo.the_operand_forms_are_exactly_these` convention).
* **Layer one is a ceiling** — `every_admitted_bound_is_inside_the_ceiling` is
  the headline: for *every* bound and *every* declaration a program can write,
  an admitted loop's bound is at most the ceiling. It is proved **over** the
  declaration rather than assumed about it, which is the difference between a
  ceiling and a default with a longer spelling. With
  `a_declaration_cannot_raise_the_ceiling` (the same fact from the refusing
  side), `the_ceiling_is_judged_before_the_bound` (the ordering), and the exact
  boundary from both sides.
* **Layer three is checked, not trusted** — `the_declared_condition_is_checked`
  and `the_declared_condition_is_strict` are the refusals; the strictness
  theorem is what makes the spelled condition and the checked condition the same
  relation.
* **The card only refuses** — `the_budget_admits_nothing_loop_v1_refused` is
  invariant 1's shape for this change: every bound the three-layer decision
  admits was already admitted by the single ceiling it replaces, so no program
  became checkable by adding a budget. `the_budget_refuses_something_loop_v1_admitted`
  is the non-vacuous converse and `a_declaration_recovers_it` shows the licence
  giving the refused program back.
* **Erasure** — the four theorems of §6.
* **The one-pass peak** — `the_one_pass_maximum_is_the_maximum_over_every_prefix`,
  stated over any stepper and any measure, with
  `the_maximum_is_a_value_the_run_passed_through`.

**Not proved**, carried over from the family's convention: that the Rust reader
extracts a clause list faithfully from source tokens (a tested Rust boundary);
anything about cost in seconds, which is a measurement and lives in §1; and any
relation between the ceiling's numeral and a buffer's extent, which is a
decision this model records rather than derives.

### 9.3 Artifacts

[`artifacts/cost_before.tsv`](artifacts/cost_before.tsv),
[`artifacts/cost_after.tsv`](artifacts/cost_after.tsv),
[`artifacts/cost_diagnosis.tsv`](artifacts/cost_diagnosis.tsv) and
[`artifacts/census.txt`](artifacts/census.txt) beside this report, with the
scripts that produced them in the worktree-private `measure/` directory.
Command evidence for the gate is in [`gate.txt`](gate.txt), and the closing
workspace run's tail in
[`artifacts/workspace_tail.log`](artifacts/workspace_tail.log).

---

## 10. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this worktree |
| `cargo test --workspace` | see §10.1 |
| `cargo test -p qcore-compiler --test loop_unroll_budget` | **29 pass**, including the sanitized native run |
| `cargo test -p qcore-compiler --test source_loop` | **39 pass** (one fixture amended, §7) |
| `cargo test -p qcore-compiler --test std_buffer` | **52 pass**, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **Passed 108 Lean audits**, one new audit file, no new axiom |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, 1 failure** — the expected derived-value tripwire (§10.2) |

### 10.1 The closing run

**4,118 passed, 0 failed, 8 ignored, across 207 suites reporting
`test result: ok`, exit 0.** The run reached and finished its final phase — the
merged doctests — and the log ends with that summary; it contains zero
`test result: FAILED` lines. The exit status is **read rather than inferred**:
`measure/gate.sh` writes each step's status with `echo $? >` into its own
`*.exitcode` file, so no shell wrapper stands between a process and the
recorded number (the `STD-BUFFER` §10.5.1 hazard, avoided by construction).
Recorded: `workspace=0 fmt=0 clippy=0 probe=0 followup=0 lean=0 pytests=1`.

**The arithmetic reconciles exactly.** `AGENTS.md`'s wave-25 close records
**4,089** workspace tests with 8 ignored. 4,089 + 29 = **4,118**, and 29 is
precisely this card's new suite — so the card adds its own tests and moves no
other count, which is what corpus neutrality looks like from the test side.

Two environment findings, recorded rather than hidden:

1. **Two earlier gate runs were stopped deliberately by this card**, each
   because a source edit landed under them — the `0.20 s`/`0.39 s` correction to
   three doc comments, and the `LicenseLicensesNothing` refusal of §11.7a. The
   `STD-BUFFER` precedent governs: a run with a source change under it is not a
   measurement, and **no number above comes from either of them**.
2. **Machine contention.** Two sibling agents ran their own full gates against
   neighbouring worktrees for most of this run and the host was under memory
   pressure, so wall-clock timings are not comparable with a quiet run. Nothing
   about the results depends on that, and it is the same contention §1.4
   discloses for the cost grid.

**None of standing constraint 16's five load-sensitive rows tripped.** No
assertion was weakened and no test was re-run in isolation to obtain a pass.

### 10.2 The one expected failure

`scripts/tests`' `test_the_derived_facts_are_recomputed_rather_than_carried`
fails on `validation.lean_audits`: the committed acceptance matrix carries
**107** and the tree now produces **108**.

```
AssertionError: Lists differ: ['14.0.0', 107] != ['14.0.0', 108]
```

That is the tripwire working as designed. The delta is entirely this card's one
new audit file, and this card's brief forbids editing
`docs/reviews/r0_3/acceptance_matrix.json`, so the row is deliberately left
stale for the wave integrator to regenerate — the same handling every wave's
derived sections receive, and the same one `LOOP-V1` recorded.

---

## 11. Deviations and findings

1. **The cost of unrolling is not a function of the bound** (§1.1). Three of
   four bodies are linear and cheap at every admissible bound; the fourth was
   quadratic. A budget on the bound alone would have been calibrated against the
   wrong quantity, and the report says so rather than presenting one number.
2. **`LOOP-V1`'s peak-live narrowing was incomplete, and the measurement found
   it** (§1.2). The narrowing fixed the *count* of prefixes and left the cost of
   each one untouched, so the computation stayed quadratic in ownership steps.
   Repaired without changing any published value, with three checks that say so.
3. **A residual quadratic is left in place and named**
   (`LOOP-OWNERSHIP-VALIDATE-QUADRATIC`, §1.4). It is in the cycle-3 ownership
   model's per-operation invariant validation, not in this construct, and
   repairing it means touching a fail-closed gate's data structure — outside
   this card.
4. **The ceiling stayed at 4096 rather than moving to 1024**, and the reason is
   a capability rather than a number (§1.6). The user named both; the
   measurement plus `MAX_ITERATIONS == MAX_EXTENT` chose between them.
5. **`where` collides inside the very brackets this card borrows** (§2.1) — the
   `quant[| n where n := 5 |]` alias. Answered by requiring **two** tokens at
   the head, with the alias's continued behaviour measured rather than asserted.
6. **One `LOOP-V1` test fixture was amended** (§7). Flagged rather than
   absorbed: 38 of 39 are untouched, and the one that moved is the test of the
   exact property this card changes.
7. **A malformed attribute originally printed two refusals for one mistake**,
   found by a failing test rather than by reading. Fixed with the model's third
   budget arm (`LoopBudget::Unreadable`), which suppresses the budget decision
   and nothing else (§5).
7a. **A condition no bound can satisfy first reported a bound it had not read.**
   `#bound < 1` was routed through the violated-condition refusal with a
   placeholder count of 1, so the message could state something false about a
   loop counting 5. It is now its own refusal, decided from the clause list
   alone and naming no bound — found by reading the message rather than by a
   test, and the test came after.
8. **`[| #unroll |]` without the condition is refused, not defaulted.** It was
   tempting to read it as "license up to the ceiling"; that would be a
   declaration with nothing in it to check, which is the opposite of the form's
   purpose (§3).
9. **The `AFTER` grid ran under measured contention** (§1.4), disclosed with the
   factor, and one BEFORE cell was not run to completion (§1.5). No decision
   turns on either.
10. **The Lean erasure theorems are `rfl`**, and the report says so rather than
    dressing them up (§6). Their content is the *type* they are stated over: an
    `Attributed` item that could carry a licence, and no path from it to the
    unrolling.
11. **A foreign route's refusal message names an empty statement**
    (`LOOP-ATTRIBUTE-FOREIGN-ROUTE-MESSAGE`, §12). Found by measuring the route
    refusals rather than assuming them. The code and the refusal are right; the
    message is not, and repairing it means editing another route's reader.
12. **The §1.2 diagnosis column removes the computation rather than replacing
    it**, so it lower-bounds the repair rather than measuring it. Every number
    quoted for the repair is §1.4's, which is both the real code and the
    contended, slower table.

---

## 12. Named follow-ons

| Name | What it is, and what it needs |
|---|---|
| `ATTRIBUTE-SLOTS-DEFINITIONS` | **Iceboxed by user direction.** Function-level attribute slots and `::`-style signature constraints — an attribute position on a *definition* rather than adjacent to a statement. Needs a placement decision the integrator deferred, and the `Q-NAMESPACE-DECISION` question is upstream of it |
| `LOOP-OWNERSHIP-VALIDATE-QUADRATIC` | `OwnershipState::validate_invariant` iterates the whole bindings map after every operation, and the map retains dropped names because `BindingAlreadyExists` is a fail-closed gate. A loop body binding a fresh scoped name per iteration therefore costs quadratically. Needs an incremental invariant check that keeps the gate exactly as strong (§1.4) |
| `LOOP-BUDGET-MODEL-LICENCE` | The budget decision is applied at the **lowering**, so a hand-constructed `BufferProgram` bypasses the default budget. The absolute ceiling and the entry's unrolled-event budget are still enforced in the model, so nothing fail-closed rests on it. Needs `BufferEvent::Repeat` to carry the licence, which changes a public variant's shape |
| `LOOP-BUDGET-ENTRY-LICENCE` | The attribute licenses a **per-loop** bound; the entry's unrolled-event budget of 8192 has no declaration. Needs a second clause (`#events < N`) or an entry-level slot, which is `ATTRIBUTE-SLOTS-DEFINITIONS`' shape |
| `LOOP-ATTRIBUTE-ROUTE-WIDENING` | The attribute is read by the one route that admits the loop, `source.buffer.v1`. A second admitting route inherits `admit_unroll` rather than restating it, but must call the clause-list reader itself. Paired with `LOOP-ROUTE-WIDENING` |
| `LOOP-ATTRIBUTE-UNTERMINATED-Q000` | An unterminated clause list is a parse failure rather than this pass's refusal (§5). Needs a parse-error recovery position, which no route on this family has |
| `LOOP-ATTRIBUTE-FOREIGN-ROUTE-MESSAGE` | `source.std-io.v1`'s generic refusal message is built from a statement's head *word*, and an attribute's head is a symbol, so it reads ``` `` is not a statement `source.std-io.v1` models ```. The code and the refusal are right and the empty name is not. Needs that route's reader to name a statement by shape rather than by head word — another route's file, outside this card |
| `LOOP-BUDGET-RELEASE-PROFILE` | Every number in §1 is a `dev`-profile measurement, which is what the gate runs. A release-profile budget would be larger, and a construct whose admitted set depends on the build profile is not one this revision wants. Recorded as a boundary rather than a plan |

Carried forward unchanged from `LOOP-V1`: `LOOP-VARYING-BOUND`,
`LOOP-BOUND-FROM-BYTE`, `LOOP-BREAK`, `LOOP-BODY-RETURN`,
`LOOP-RESOURCE-COLLECTION`, `LOOP-INDEX-ARITHMETIC`, `LOOP-RANGE-LOW`,
`LOOP-ROUTE-WIDENING`, `LOOP-SEQUENTIAL`, `LOOP-BODY-ATTRIBUTION`,
`LOOP-SCOPE-SUFFIX-IN-MESSAGE`, `LOOP-EXAMPLE`, `STD-BUFFER-BULK-COPY`.
`LOOP-BUDGET` itself is **closed** by this card.

---

## 13. For the integrator

* **No `Q` code is allocated.** `Q097` and `Q098` each gain conditions, stated
  in §L21 and §G07.12 in place; no row is added to either table, and
  `AGENTS.md`'s namespace summary needs no new row — its `Q097`–`Q099` cell may
  optionally gain a clause naming the widening.
* `AGENTS.md`'s ledger Lean-audit count moves **107 → 108**. **Not edited** —
  `AGENTS.md` is integrator surface and this card's brief forbids it.
* `docs/reviews/r0_3/acceptance_matrix.json` carries derived
  `validation.lean_audits` / `lean_audit_files` rows checked by `scripts/tests`,
  so the derived-value tripwire trips on this card's one new audit file. The
  brief forbids editing the matrix, so it is deliberately left stale for the
  wave integrator to regenerate — the handling every wave's derived sections
  receive.
* `qcore_frontend::frontend::StatementKind` gained one variant
  (`LoopAttribute`). Every existing `match` over it was extended, not defaulted.
* `qcore_core::source_loop` gained `UnrollLicense`, `BudgetVerdict`,
  `LoopBudget` and four `BoundRefusal` variants;
  `qcore_frontend::lexer::Symbol` gained `Hash`. Same.
* `qcore_core::ownership` gained one public function, `peak_live_axis`.
* **No standing constraint is touched.** No gateway, audit, contract-loader or
  provider file changed; no pinned expectation was edited; no fail-closed gate
  was weakened. Constraint 14's census was run rather than assumed and is §2.

---

## 14. Draft increment paragraph

> The three-layer unrolling budget (LOOP-UNROLL-BUDGET): `source.loop.v1`'s
> single ceiling becomes an **absolute ceiling** of 4096 no declaration may
> raise, a **default budget** of 256 a `for` with no attribute is held to, and a
> **licence** a statement-adjacent `[| #unroll, where #bound < N |]` clause list
> declares immediately before the `for` it governs — closing `LOOP-V1`'s
> `LOOP-BUDGET` follow-on. **The measurement that chose the numbers found
> something the brief did not expect**: the cost of exact bounded unrolling is
> not a function of the bound at all. Three of four representative bodies are
> linear and cheap at every admissible bound (a 4096-iteration copy costs 64 ms,
> a 4096-line loop 10 ms), while a body that acquires and releases **once per
> iteration** was quadratic — **56.4 s** of check time at a bound of 1024 — and
> the cause was that `LOOP-V1`'s peak-live narrowing had fixed the *count* of
> prefixes scanned and left each one re-executed from the start. Replacing that
> rescan with one pass takes the same body to **0.39 s**, a factor of 145, changes **no published
> value** (three checks say so, including a Lean theorem over any stepper and any
> measure), and is what made the ceiling defensible at **4096** rather than 1024:
> lowering it would have withdrawn the full-extent traversal `MAX_ITERATIONS ==
> MAX_EXTENT` exists to keep expressible, paying a capability for a defect fixed
> in the same change. The default of **256** is the largest power of two at which
> every measured body stays inside ~40 ms, where 1024 costs 389 ms. **The
> declaration is checked, never trusted**: a bound that violates the condition
> refuses naming both numbers, the condition is strict so `N` itself is not
> licensed, and a declared `N` above the ceiling refuses *before* this loop's
> bound is looked at — so a two-iteration loop cannot carry a licence for a
> 99 999-iteration one. An unattributed loop above the default refuses **with the
> exact clause list that would license it written into the message**. **Zero new
> diagnostic codes** (`Q-NAMESPACE-DECISION` is iceboxed): the clause list's
> shape and a list that licenses nothing are conditions of `Q097`, and the three
> budget refusals are conditions of `Q098`, on the `SOCKET-EVENT-COMPOSE`
> precedent. **One new token** — `#`, promoted from a lexer error on
> `OR-AND-LOWERING`'s precedent for `^`, census-clean at zero code occurrences
> across 422 `.qcore`/`.qc` files — and **no new reserved word at all**, a first
> for this family: a `[| … |]` clause list contributes no referenced name, so
> `unroll` and `where` stay contextual, which matters because the census found
> `where` **eight** times inside `quant<n where n := 5>` whose readable alias is
> spelled with the very brackets this attribute borrows; the head is recognised
> as `[|` **followed by `#`** and the alias's behaviour is re-measured rather
> than asserted. **The attribute erases**: it governs the checker's budget only,
> an attributed and an unattributed program of the same admitted bound emit
> byte-identical C, and a licensed 600-iteration loop is byte-compared against a
> sanitized ASan/UBSan/LSan run over 301 lines. 29 tests; `std_buffer`'s 52 are
> untouched and 38 of `LOOP-V1`'s 39 are, the one amendment being the test of the
> ceiling bound itself, which now declares what it reaches. 22 audited Lean
> entries in one new audit file, no new axiom, 107 → 108 audits. Corpus-neutral
> by construction. Evidence: `docs/reports/loop-unroll-budget/README.md`.
