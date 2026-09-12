# `source.loop.v1` — the checked bounded counted loop

Card `LOOP-V1`. Companion to `docs/design/SOURCE_BRANCH_V1.md`, which this note
follows closely: a loop is the second control shape the language has, and the
branch card's structure is the one that fits it.

## 1. The gap, as it was recorded

QCore had no loop construct of any kind. Three recorded blockers name that
absence, and one of them names it exactly:

* **`STD-BUFFER-COPY`** — *"a bulk copy between two buffers. Needs no new
  custody shape, but needs the loop or the bulk operation the flat route has no
  statement for; `source.branch.v1`'s neighbours are where that lives."*
* HTTP header iteration, on the `source.http.v1` route.
* Any bulk byte work at all: `source.buffer.v1` writes one byte per statement.

The first is this card's payoff and the one it makes real.

## 2. The form

```ebnf
stmt   = "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}" ;
BOUND  = INTEGER | NAME ;
```

### 2.1 Why `for i in 0..N` and not `repeat N`

The brief offered `repeat N { … }` and asked the choice to be argued. It
undershoots the recorded consumer, and measurably so: `STD-BUFFER-COPY` is
`dst[i] = src[i]`, and a bare repetition with no index cannot write it. Every
one of the three recorded blockers is an *indexed* traversal, so a construct
without an index would leave all three exactly where they were.

`for i in 0..N` is also the form a reader of C, Rust or Python already knows,
and spelling the low end out rather than implying it keeps the header readable
as a range while admitting exactly one range per bound.

### 2.2 The census decided the spelling (standing constraint 14)

Over all **418** `.qcore`/`.qc` files in the tree, counting code positions
separately from `//` comments:

| Candidate | Code occurrences | Verdict |
|---|---|---|
| `for` | **0** | reserved; the reservation collides with nothing |
| `in` | **6** | **not** reserved — four are parameter names the referenced-names filter already shields (a word followed by `:`), but **two are genuine references** inside the pinned contract corpus: `ensures normal resource_borrowed(in);` in `contract/qcore/fluent_bit/input_chunk_helpers.qcore` |
| `repeat`, `range`, `iter`, `upto`, `stride` | 0 | unused; not needed |
| `loop` | 3 | a `borrow<'call, mut, EventLoop>` **parameter name** in `contract/qcore/monkey/wrapper_paths.qcore` |
| `while`, `break`, `continue`, `range`, `bound`, `idx` | — | **already** reserved |
| `count`, `body`, `index`, `next`, `span`, `step`, `to`, `over`, `through` | 1–32 | each occurs in code position; none was needed |

`in`'s two sites are exactly the shape card `STRING-SLICE` measured for `len`,
and they get exactly that card's answer: **narrow the rule rather than edit a
pinned corpus this card may not touch.** `in` is skipped by
`frontend::referenced_names` at the one position a `for` header puts it —
immediately after `for NAME` — and is an ordinary word everywhere else. Pinned
by `in_is_an_ordinary_name_outside_a_for_header`.

`..` is **two existing `Dot` tokens** read positionally. The lexer is untouched,
on the precedent that `||` was two `Pipe` tokens before `OR-AND-LOWERING` gave
it a token of its own.

## 3. The checking route: exact unrolling, and the measurement that chose it

The brief asked both routes to be measured and one argued. They were, and the
answer is not close.

### 3.1 What the tree already has

| Machinery | Present before this card? |
|---|---|
| `OwnershipOp::Branch` and its join | yes |
| `Flow::join_branches` | yes |
| a **loop terminator** in any IR | **no** — `CfgTerminator` is `Goto`, `TryAcquire`, `Check`, `Branch`, `ReturnOk`, `ReturnError`; none is a loop header, and `cleanup_lowering::SourceOp` gained `Branch` from `IF-STATEMENT` and nothing else |
| a **widening operator** on a source-side abstract domain | **no**. Every `widen` in the tree is under `crates/qcore-c-gateway/` — the *existing-C* analysis, over a different domain and for a different purpose — and no source-side model has one |
| an iteration/fixed-point driver over `SourceOp` | **no** |

So a fixed-point route would have to add a back edge to the typed CFG, a
widening to the ownership ledger, and a fixed-point driver — three new pieces,
none of which exists.

### 3.2 Why that is not the argument, though

Cost is not why the fixed point lost. It lost because **it is incompatible with
this family of routes' evidence.**

Every route that could admit this construct publishes a certificate whose
contents are *exact values* — the bytes a buffer holds, the digits a line
carries — and validates them by a **byte comparison** between the reference
execution and a native run. A fixed point over a back edge computes a sound
over-approximation of exactly those values. It is the operation that replaces
"byte 3 is 65" with "byte 3 is some byte", and a reference interpreter that
cannot say which byte cannot predict the line the native run will print.

That is the same wall `STD-BUFFER-VARYING-INDEX` records for an index the
checker cannot constant-fold, and `PIPE-SCALAR-RESULT` records for an address.
A widening would not make this route cheaper to check; it would make this
route's evidence unwritable.

Since the bound is a number the checker holds, **unrolling is available and it
is exact**, so there is nothing left for an approximation to buy. What it costs
is stated rather than hidden: `MAX_UNROLLED_EVENTS` bounds the total, and an
entry above it is refused by name.

### 3.3 What unrolling buys, measured

The per-iteration custody discipline turned out to be almost entirely
**inherited**, which is the card's own version of the `IF-STATEMENT` finding
that five of six models already branched:

| Body shape | Refused by | Needs new machinery? |
|---|---|---|
| acquires without releasing | the unedited cycle-3 `check_ownership`, on the unrolled list: two acquisitions of one live name, or a leak at the exit | no |
| releases a resource it did not acquire | the same checker: two releases of one resource | no |
| an index outside the extent at some iteration | `source.buffer.v1`'s own bounds rule, at that iteration | no |
| acquires **and** releases | admitted — the identity | no |
| any of the above at a bound of **one** | nothing would expose it | **yes** — `Q099` |

So the explicit gate exists for one reason: to make the rule hold at a bound of
one, where unrolling alone exposes nothing. It is written over `OwnershipOp`
alone (`source_loop::iteration_is_custody_identity`), so a second route
admitting the construct inherits it rather than restating it.

### 3.4 One thing unrolling did force

The unrolled operation list presents a body-local binding once per iteration,
and `check_ownership` refuses re-binding a name it has seen dropped
(`BindingAlreadyExists`) — a fail-closed gate that must not be weakened. The
answer is to model the scoping faithfully: each iteration's body-local binding
carries its own scope in the name (`c#0`, `c#1`), exactly as the emitted C's
second `unsigned char *b_c` is a different object. Nothing is laundered by
that: a body that acquires without releasing still leaks, and `NormalPathLeak`
names every one of the distinct live bindings.

## 4. Emission: a real `for`, and why the asymmetry is right

The checker unrolls; the emitter does **not**. The two halves are asymmetric on
purpose.

The checker needs each iteration's exact values, because the certificate
publishes them and the evidence is a byte comparison. The emitted C needs none
of that — it needs to be the program a C reader would have written, and it needs
to stay one program rather than growing linearly in a bound that can reach 4096.

The asymmetry also buys something: because the emitted C computes the index
itself, the native run is a **check** of the unrolling rather than a restatement
of it. A reference observation computed by unrolling and a native observation
produced by a rolled C loop that disagreed would show up as a byte difference.

## 5. What v1 refuses, by name

| Shape | Code | Follow-on |
|---|---|---|
| a bound the lowering cannot compute | `Q098` | `LOOP-BOUND-FROM-BYTE`, `LOOP-VARYING-BOUND` |
| a bound of zero | `Q098` | — (a decision, not a gap: its body would never be checked) |
| a bound above 4096, or an entry above 8192 unrolled events | `Q098` | `LOOP-BUDGET` |
| `break` / `continue` | `Q097` | `LOOP-BREAK` |
| `return` in a body | `Q097` | `LOOP-BODY-RETURN` |
| a body binding that shadows an outer one | `Q097` | `LOOP-SHADOWING` |
| a range whose low end is not `0` | `Q097` | `LOOP-RANGE-LOW` |
| a body whose net custody effect is not the identity | `Q099` | — (the discipline itself) |
| a loop over a collection of resources | — | `LOOP-RESOURCE-COLLECTION`: no such type exists |
| the construct on any route but `source.buffer.v1` | that route's own | `LOOP-ROUTE-WIDENING` |

## 6. Nesting

Admitted, and bounded by the **product** rather than by a depth. A depth limit
would say that two loops of eight are as expensive as one loop of a hundred,
which is false; the unrolled event total is what the checking route actually
costs, so that is what the budget counts.

`source.branch.v1` admits unbounded nesting because it does not unroll. Stating
that difference is more useful than matching a number.
