# Loops — `source.loop.v1`

One program: the checked bounded counted loop `for NAME in 0..BOUND { … }`,
on `source.buffer.v1`, which is the only route v1 admits it on. Card
`LOOP-V1`; the evidence package is [`docs/reports/loop-v1/README.md`](../../docs/reports/loop-v1/README.md)
and the design note written before it is
[`docs/design/SOURCE_LOOP_V1.md`](../../docs/design/SOURCE_LOOP_V1.md).

| Program | What it shows |
|---|---|
| [`buffer_copy.qcore`](buffer_copy.qcore) | A bulk copy between two owned buffers — `STD-BUFFER-COPY`, the blocker this construct was landed for — and a second loop that renders its own index |

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
