# Loops — `source.loop.v1`

Two programs: the checked bounded counted loop `for NAME in 0..BOUND { … }`,
on `source.buffer.v1`, which is the route v1 admits it on. Card
`LOOP-V1`; the evidence package is [`docs/reports/loop-v1/README.md`](../../docs/reports/loop-v1/README.md)
and the design note written before it is
[`docs/design/SOURCE_LOOP_V1.md`](../../docs/design/SOURCE_LOOP_V1.md). The
second program is card `LOOPS-ON-COMPOSED-ROUTES`' (wave 39), whose evidence
package is [`docs/reports/loops-on-composed-routes/README.md`](../../docs/reports/loops-on-composed-routes/README.md):
a `for` beside a **write lend**, admitted with the loan's window computed over
iterations. Its sanitized native run is `compiler/tests/loops_on_composed_routes.rs`'s.

| Program | What it shows |
|---|---|
| [`buffer_copy.qcore`](buffer_copy.qr) | A bulk copy between two owned buffers — `STD-BUFFER-COPY`, the blocker this construct was landed for — and a second loop that renders its own index |
| [`lend_fill.qcore`](lend_fill.qr) | A loop filling a lend's range **through the loan**, index by index, while the owner writes a byte it kept — the shape `SLICE-PERMISSIONS-LOOP-WINDOW` refused until the window was computed over iterations |

## The form

```ebnf
stmt   = "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}" ;
BOUND  = INTEGER | NAME ;
```

The low end is written out rather than implied, so a header reads as a range and
admits exactly one range per bound. `BOUND` is a check-time constant: a literal,
a name a literal `let` bound, or `extent(NAME)` — a number the checker reads out
of the ownership ledger's own footprint. `for` is a reserved statement head;
`in` is **not** reserved, only skipped at the one position a header puts it, so
`let in = 2;` still binds that name.

## The three rules a loop adds, and nothing else

1. **The bound is decided before anything runs.** Every index the body takes is
   therefore a number the checker has, which is what keeps every `src[i]` and
   `dst[i]` inside a proved range without a run-time bound check the route would
   otherwise have to model.
2. **One pass through the body is the identity on custody.** It must leave the
   ownership ledger holding the same resources, under the same names, with the
   same footprints it found. A body that acquires without releasing would owe a
   different obligation on each iteration, and the loop would have no single
   thing to say about any of them.
3. **The body has one exit.** No `break`, no `continue`, no `return` — each is
   an early exit whose per-exit cleanup discharge no lowering on this route has.

Everything else is inherited. `i` and `v` are body-local exactly as an `if`
arm's bindings are; the byte read and the byte write are `source.buffer.v1`'s
own statements, unchanged; the custody refusals are `qcore_core::ownership`'s,
unedited.

## Checking unrolls; emission rolls

The checker's route is **exact bounded unrolling** — four passes over the copy
body, each with its index fixed — and it is deliberately not a fixed point: an
admitting route's evidence here is a byte comparison between the reference
execution and a native run, and widening over a back edge is precisely the step
that replaces "byte 3 is 114" with "byte 3 is some byte", leaving that
comparison with nothing to predict.

The **emitted C is a real `for` loop**, and its body is written once rather than
once per iteration:

<!-- emitted: copy-loop -->
```c
for (int32_t s_i = INT32_C(0); s_i < INT32_C(4); s_i += INT32_C(1)) {
```

So the native run is a check of the unrolling rather than a restatement of it —
the reference execution unrolled the loop, the C rolled it, and the two agree
byte for byte on both streams.

## Transcript

Two streams, and no order is claimed between them: C models none between two
separately buffered streams, so each is compared on its own. Standard output
carries the second loop's three indices and then the four copied bytes —

<!-- transcript: buffer_copy stdout -->
```text
0
1
2
81
67
111
114
```

— and standard error carries the one logger line, whose number is the bound the
checker read out of the buffer's footprint:

<!-- transcript: buffer_copy stderr -->
```text
[INFO] 4
```

`81 67 111 114` is `QCor` in ASCII, copied byte by byte from `src` into a `dst`
no source statement ever writes at a spelled index.

## A loop beside a lend — `lend_fill.qcore`

The loan `part` covers `[0, 4)` of `store`; the loop copies `src` into it
**through the loan**, and in the same body writes `store[7]`, a byte the loan
does not hold. Before card `LOOPS-ON-COMPOSED-ROUTES` an entry that lends
refused a `for` by name, because the loan's window was an interval of statement
indices that a loop body contributed nothing to. Now a loan a body names is
**live across the whole loop**: `part`'s window runs through the `for`, every
body write is judged against it at every iteration, and the owner's write to
byte 7 is admitted because it lies outside the range — exactly as it would be
in a flat sequence. Writing `store[2]` in that body is refused at `Q054`.

The emitted C is one real `for`, the loan store in the loan's own coordinates
through the displaced pointer the lend declared:

<!-- emitted: lend-loop -->
```c
    unsigned char *const l_part = b_store;
    for (int32_t s_i = INT32_C(0); s_i < INT32_C(4); s_i += INT32_C(1)) {
        int32_t s_v = qcore_bytes_load(b_src, (size_t) 4u, (size_t) (s_i));
        qcore_bytes_store(l_part, (size_t) 4u, (size_t) (s_i), s_v);
        qcore_bytes_store(b_store, (size_t) 8u, (size_t) (INT32_C(7)), INT32_C(1));
    }
```

Standard output carries the four copied bytes read back through the owner and
then the byte the owner wrote inside the loop:

<!-- transcript: lend_fill stdout -->
```text
81
67
111
114
1
```

Nothing is written to the logger's stream: the program writes no `log` line.

## A refused program

A bound of zero is refused rather than admitted as a loop that does nothing:

<!-- refusal: Q098 -->
```qcore
import std.buffer;

fn main() -> unit effects { alloc, mut } {
    let b = bytes(4);
    for i in 0..0 {
        b[i] = 1;
    }
    drop b;
    return;
}
```

and refused like this:

<!-- diagnostic: Q098 -->
```text
refusal.qcore:5:5: Q098: a bound of 0 runs no iteration, so this body would never be checked; `source.loop.v1` refuses it rather than admitting an unchecked statement
    for i in 0..0 {
    ^
```

Both blocks are read out of this file by the test: the program above is
compiled, and the diagnostic below is compared with what the compiler really
said, span caret included. The ground is invariant 1's, not tidiness — a body
that never runs is the one way a statement could enter a program unexamined.

Three more refusals the rules above name, each checked by the same test:

| Written in the body | Refused at |
|---|---|
| `break;` | `Q097` (`LOOP-BREAK`) |
| `return;` | `Q097` (`LOOP-BODY-RETURN`) |
| `let t = bytes(2);` with no release | `Q099`, then the ownership checker's own `Q054` |

And one that is not this route's: naming the index after the loop is `Q013`,
unknown name, because the binding is body-local.

## Running it

```sh
cargo test -p qcore-compiler --test examples_suite
```

The test compiles this program through the real pipeline, asserts it takes the
`source.buffer.v1` route, builds the emitted C under
`-std=c11 -Wall -Wextra -Werror -pedantic`, runs it under ASan/UBSan (and LSan
on Linux), and compares **both** streams with the reference execution and with
the transcript above, which it reads out of this file.

The leak control is positive: this program allocates two buffers and releases
both, and one of the four writes into the second is performed only by the loop
body. A loop that ran past an end, or a release that went missing, would be an
ASan or LSan finding rather than a passing test.

## What this is not

* **No varying-value tier.** Every bound and every index here is check-time
  known. A loop over an index the checker cannot fix is what `HTTP-V1-PARSE`
  and `STD-BUFFER-VARYING-INDEX` record, and it is not this.
* **One route.** v1 admits the construct on `source.buffer.v1` only; the
  sequential, standard-IO, event, socket and task routes keep the refusals they
  already had, measured and pinned rather than widened (`LOOP-ROUTE-WIDENING`).
  A loop written outside the entry refuses at `Q097` rather than going
  unchecked.
* **Bounded.** 4096 iterations for one loop, and an unrolled event total of
  8192 for the entry — a product over a nest, because the product is what the
  checking route costs.
