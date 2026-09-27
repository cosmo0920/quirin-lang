# The pipe operator

`|>` puts the value first and the function second. That is the whole feature.

| Example | What it demonstrates |
|---|---|
| [rendered_lines.qcore](rendered_lines.qr) | One stage: a value into a sink |
| [scalar_stages.qcore](scalar_stages.qr) | A multi-stage chain, and hygienic temporaries |
| [scalar_stages_hand_written.qcore](scalar_stages_hand_written.qr) | The same program with the temporaries named by hand |

Checked by [`compiler/tests/examples_suite.rs`](../../compiler/tests/examples_suite.rs):
the transcripts below are read out of this file and compared with real sanitized
runs, the refused program below is read out of this file and compiled, and the
two `scalar_stages` spellings are compared byte for byte.

```sh
cargo test -p qcore-compiler --test examples_suite
```

## It is a token permutation, and nothing else

```qcore
total |> print dec;      // is exactly
print dec total;
```

"Exactly" is meant literally. The desugaring happens before any checker runs, so
the pipe adds no operation, no checker rule and no IR node. Nothing downstream
can tell a `|>` was written: the two spellings emit the same C11 and produce the
same observation.

So reach for it where it reads better and not where it does not. On a single
stage what it buys is reading order — the value moves to the front, where the
eye starts. On a chain it buys you the absence of names.

`rendered_lines.qcore` writes to **stdout**:

<!-- transcript: rendered_lines stdout -->
```text
42
0x000000FF
-7
```

and to **stderr**:

<!-- transcript: rendered_lines stderr -->
```text
[INFO] 42
[WARN] 0x000000FF
[FATAL] -7
```

## Chains, and the temporaries you do not name

```qcore
let scaled = b |> demo_sum(8) |> demo_scale(2) |> demo_scale(3);
```

`x |> f(a) |> g(b)` means `g(f(x, a), b)`. Three stages, two intermediate
values, no names.

Those intermediates are **hygienic temporaries**. The desugarer invents them,
they cannot collide with anything you wrote, they cannot be mentioned, and they
never reach the emitted C — which names values `vN` by identity, exactly as it
names resources `rN`. [scalar_stages_hand_written.qcore](scalar_stages_hand_written.qr)
is the same program with those two values named `stage0` and `stage1`, and a
test asserts the two files emit **byte-identical** C11 and observe identically.
That comparison is the argument for the piped spelling: `stage0` says nothing,
and inventing better names for values that live for one statement is work that
buys a reader nothing.

Because `scalar_stages.qcore` explicitly says `log_level trace;`, it renders
the retained system events to **stdout**:

<!-- transcript: scalar_stages stdout -->
```text
acquire demo_alloc r1 ok
call demo_fill b:r1 d:7
value demo_sum v1 b:r1 d:8
value demo_scale v2 v:v1 d:2
value demo_scale v3 v:v2 d:3
call demo_release c:r1
return
```

The compiler retains those calls and custody transitions as structured trace
events regardless of presentation. Removing the declaration restores the
default null sink and therefore removes these lines from program output.

Note what is *not* on those `value` lines: no `-> N`. None of these calls
declares a `succeeds` outcome, so there is no declared number for the emitted
program to compare against and none to print. Compare
[../stdio/pipeline.qcore](../stdio/pipeline.qr), whose calls do declare one.

## Why chains carry scalars

A chain's intermediate stages carry **data**, not custody.

That is not a restriction bolted on: it is what makes a chain possible at all.
Every stage before the last binds a value the surrounding code never names, so a
stage holding a *resource* would be holding an obligation nothing can discharge
on a failure arm. Scalars have no such obligation — contract spec §C04.1, "a
scalar may carry a descriptor number as data, but cannot carry descriptor
custody" — so they can travel a chain, and resources travel the ordinary way.

The resource in `scalar_stages.qcore` does exactly that: acquired once, lent at
the first stage, released by name at the end. And a pipeline's operand is an
ordinary expression, so the acquisition and the release can be piped too:

```qcore
let b = 8 |> demo_alloc succeeds true else raise Failure;
move b |> demo_release;
```

`move b` still moves. Using `b` after that statement refuses exactly as it does
without the pipe — the checker never sees a pipe.

## A refusal worth knowing: a sink cannot feed a later stage

`print` produces no value, so it can only ever be a pipeline's **last** stage.
This program is refused:

<!-- refusal: Q037 -->
```qcore
import std.fmt;
import std.io;
import std.logger;

fn main() -> unit effects { io } {
    let total = 42;
    total |> print dec |> log info dec;
    return;
}
```

and refused like this:

<!-- diagnostic: Q037 -->
```text
refusal.qcore:7:14: Q037: `print` produces no value, so it cannot feed a later stage: it is a sink, and only a call stage binds a result. Make it the pipeline's last stage
    total |> print dec |> log info dec;
             ^
```

Both blocks are read out of this file by the test: the program above is
compiled, and the diagnostic below is compared with what the compiler really
said. Neither is transcribed prose.

This is the narrow, permanent rule — it is about `print` producing nothing, not
about chains being unsupported. Chains are supported; the earlier refusal that
covered this shape for being a chain was deliberately replaced.

Two neighbours of the same rule:

* `let x = total |> print dec;` refuses at `Q036` — a bound pipeline binds its
  last stage, and this one produces nothing to bind.
* `total |> flush;` refuses at `Q037` — a barrier takes no operand.

## Building and running

```sh
quirin check examples/pipes/rendered_lines.qcore
quirin emit examples/pipes/rendered_lines.qcore -o /tmp/rendered_lines.c
cc -std=c11 -Wall -Wextra -Werror -pedantic /tmp/rendered_lines.c -o /tmp/rendered_lines
/tmp/rendered_lines
```

`scalar_stages.qcore` and its twin call declared extern functions, and the CLI
selects no extern contract environment (`Q023`; named follow-on
`UNSAFE-LINKAGE-CLI`). They are compiled through the library API against
[../extern/contracts/buffer_api.qcore](../extern/contracts/buffer_api.qv) and
linked against [../extern/buffer_api.c](../extern/buffer_api.c), which is what
the test suite does.

## Not expressible here

* **A resource cannot be a chain's intermediate stage**, for the reason above.
  A chain over custody is not a missing feature waiting on syntax; it is a
  shape whose failure arms have no owner.
* **There is no `|>` in an expression position.** `|>` builds a *statement*: a
  pipeline is a statement or the right-hand side of a `let`. `(1 |> 2) == 3` is
  not a condition — see [../branching/](../branching/) for what a condition may
  contain.
