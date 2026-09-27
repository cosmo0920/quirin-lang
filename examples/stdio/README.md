# Standard output and logging

Four programs covering `std.fmt`, `std.io` and `std.logger` — the bundled
modules that let a QCore program write a line — and the composed route that
lets a printing module also call declared C.

| Example | What it demonstrates |
|---|---|
| [renderings.qcore](renderings.qr) | `print` with both renderings, `dec` and `hex` |
| [severities.qcore](severities.qr) | `log` at all three severities, and `flush` |
| [barrier.qcore](barrier.qr) | `flush` alone: the smallest complete module on this route |
| [pipeline.qcore](pipeline.qr) | The composed route: `value \|> render \|> log` over a declared extern callee |

Everything below is checked. The transcripts are read out of this file by
[`compiler/tests/examples_suite.rs`](../../compiler/tests/examples_suite.rs) and
compared, per stream, with a real sanitized run of the emitted C — so this page
cannot drift from what the programs do:

```sh
cargo test -p qcore-compiler --test examples_suite
```

## Three modules, three capabilities

Each capability is gated by its own import and by no other:

| Import | Grants |
|---|---|
| `import std.fmt;` | the rendering words `dec` and `hex` |
| `import std.io;` | `print` — the **stdout** sink |
| `import std.logger;` | `log <severity>` — the **stderr** sink — and `flush` |

Holding one does not imply the others. A file that prints without
`import std.io;` refuses at `Q015`, and so does one that logs holding only
`std.io`. Imports are file-local: importing in another input file grants this
one nothing. A misspelled module path (`std.log`, `std.stdio`) refuses at
`Q014` rather than being ignored.

Importing is not using. A file that imports all three and never writes stays on
the ordinary sequential route; nothing drags it onto `source.std-io.v1`.

## Renderings

`print dec` writes a signed decimal `int32_t`; `print hex` writes the same value
as eight upper-case hexadecimal digits behind an `0x` prefix. Both renderings
are total on `int32_t` and on nothing else — there is no width to get wrong, and
no format string anywhere in the source.

The operand grammar admits no form that could denote a resource, which is why
this route's derived effect row is exactly `io`: never `alloc`, never `mut`.
Nothing a `print` touches can be consumed, released or retained.

`renderings.qcore` writes to **stdout**:

<!-- transcript: renderings stdout -->
```text
42
0x000000FF
-7
```

and writes nothing to stderr.

## Logger levels

There are six ordered levels: `trace`, `debug`, `info`, `warn`, `error`, and
`fatal`. Each renders its operand through the same `std.fmt` word and prefixes
the corresponding uppercase tag. The word `error` is interpreted contextually
after `log`; it remains the keyword that declares an error type elsewhere —

```qcore
error Failure { Failed; }
```

`fatal` is a tag on a line. It does not exit, abort or raise.

`severities.qcore` writes nothing to stdout, and to **stderr**:

<!-- transcript: severities stderr -->
```text
[TRACE] 42
[DEBUG] 0x000000FF
[INFO] 42
[WARN] 0x000000FF
[ERROR] -7
[FATAL] -7
```

Log lines go to stderr and `print` lines go to stdout. **No order between the
two streams is modeled anywhere**, because C gives none between two separately
buffered streams — which is why every check here compares the two streams
separately and never their interleaving.

## `flush` is a sink handoff, not a durability barrier

`flush` pushes what this program has buffered into the stream's implementation
and returns. That is all it is.

It does not reach a disk. It does not `fsync`. It promises nothing about what
survives a crash on the far side of the handoff, and nothing in the checked
model claims otherwise — so nothing in the emitted C claims it either: `flush`
emits exactly one `fflush`. If you need durability, this is not the construct,
and QCore does not currently have one.

`flush` names no rendering, so [barrier.qcore](barrier.qr) needs no
`std.fmt` import — it is `std.logger` that grants the barrier. It is the
smallest complete module on this route, and its transcript is the empty one:

<!-- transcript: barrier stdout -->
```text
```

`flush` takes no operand (`flush 1;` refuses at `Q030`) and is not a pipe stage
(`total |> flush;` refuses at `Q037`).

## The composed route

[pipeline.qcore](pipeline.qr) both calls declared C and prints. It needs no
marker to say so: a module using `std.extern`'s calls *and* `std.io`'s lines is
checked by `source.std-io-extern.v1`, which runs both routes' own checkers and
publishes both certificates side by side. Composing weakens nothing — every
refusal either route gives a module on its own is still given, under that
route's own code, at that route's own site.

The chain is the payoff:

```qcore
total |> demo_scale(2) succeeds 112 |> log info dec;
```

Read left to right: `x |> f(a)` means `f(x, a)`, and the last stage consumes the
value. See [../pipes/](../pipes/) for the operator on its own.

### `succeeds N` and the `-> N` observation

`succeeds 56` is a **declared outcome**. The checker takes it as given, and the
emitted C does not: it compares the real result against the declaration and
exits `70` rather than carrying on with a number nobody computed. That is what
makes a rendered line worth reading — the number in it is the number the run
produced, or the run stopped.

The declared outcome is retained in the structured diagnostic trace.
`pipeline.qcore` explicitly selects `log_level trace;`, so that trace is
rendered together with its source-authored stdout lines:

<!-- transcript: pipeline stdout -->
```text
acquire demo_alloc r1 ok
call demo_fill b:r1 d:7
value demo_sum v1 b:r1 d:8 -> 56
56
value demo_scale v2 v:v1 d:2 -> 112
value demo_scale v4 v:v3 d:2 -> 84
call demo_release c:r1
return
```

and to **stderr**:

<!-- transcript: pipeline stderr -->
```text
[INFO] 112
[WARN] 0x00000054
```

Line 3 is the call trace saying `demo_sum` returned 56; line 4 is `print dec
total` rendering it. Line 5's `-> 112` and the `[INFO] 112` on stderr are the
same pair. A call with no `succeeds` clause has no `-> N` field, because there
is no declared number to compare against — see
[../pipes/scalar_stages.qcore](../pipes/scalar_stages.qr) for that shape.

The `vN` and `rN` names are model identities, not source names: the hygienic
temporary between the two pipe stages has no name to print. `v3` is the literal
`42`, and it appears only where a call reads it.

## Building and running

```sh
quirin check examples/stdio/renderings.qcore
quirin emit examples/stdio/renderings.qcore -o /tmp/renderings.c
cc -std=c11 -Wall -Wextra -Werror -pedantic /tmp/renderings.c -o /tmp/renderings
/tmp/renderings
```

The same works for `severities.qcore` and `barrier.qcore`.

**`pipeline.qcore` cannot be checked through the CLI today.** The driver selects
no extern contract environment, so a declared extern call reaches it as

```text
Q023: calling a declared extern function requires an explicit extern contract
selection; there is no default environment
```

That is fail-closed and correct, and a selection flag is the named follow-on
`UNSAFE-LINKAGE-CLI` rather than a default. Until it lands, the composed route
is reachable through the library API — `compile_files` with
`CompileOptions::extern_contracts` — which is how the test suite compiles this
example, against [../extern/contracts/buffer_api.qcore](../extern/contracts/buffer_api.qv).
The callees' C is [../extern/buffer_api.c](../extern/buffer_api.c); link it
beside the emitted translation unit.

## What is and is not established here

The programs run clean under ASan/UBSan, and under LSan on Linux (Apple's
AddressSanitizer ships no leak detector, so leak checking stays a Linux
observation). For `pipeline.qcore` that is a **positive** control: it allocates
a real buffer and releases it, so a lost release would be reported rather than
passing silently.

What none of it establishes is that `buffer_api.c` behaves as its contract says.
That is a declared premise (`DeclaredContract("demo_sum")` and friends), printed
rather than discharged. A sanitized run can refute such a premise for these
programs; no run establishes one in general.

## Not expressible here

* **A printing module cannot spawn.** Route selection puts `source.std-io.v1`
  last, so a module that both spawns and prints stays with the task model and
  keeps its refusal (`Q020` if the `io` effect is declared, `Q500` at the
  statement if it is not). The threaded programs in [../threads/](../threads/)
  print only compiler-synthesized trace instrumentation, never a source `print`.
* **Only `int32_t` is renderable.** A callee declared to return `i64` is refused
  at the rendering rather than emitted through a conversion that does not
  describe it.
* **No string, no float, no formatting.** `dec` and `hex` are the vocabulary.
