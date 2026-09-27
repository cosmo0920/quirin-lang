# Declared extern calls

Calling C whose behaviour you have written down.

| Example | What it demonstrates |
|---|---|
| [buffer_round_trip.qcore](buffer_round_trip.qr) | One resource's whole life across the boundary |
| [scalar_results.qcore](scalar_results.qr) | Scalar results, and why they may leave the boundary |
| [borrowed_render.qcore](borrowed_render.qr) | `borrow NAME`: rendering a live resource without consuming it |

Supporting files, shared with [../stdio/](../stdio/) and [../pipes/](../pipes/):

| File | What it is |
|---|---|
| [contracts/buffer_api.qcore](contracts/buffer_api.qv) | The **declared** model of the C API |
| [buffer_api.c](buffer_api.c) | The C itself — unverified, and linked beside the emitted unit |

Checked by [`compiler/tests/examples_suite.rs`](../../compiler/tests/examples_suite.rs),
which compiles each example against the contract module, builds the emitted C
with `buffer_api.c` under `-Wall -Wextra -Werror -pedantic`, runs it under
ASan/UBSan (and LSan on Linux) and compares its output, per stream, with the
reference execution:

```sh
cargo test -p qcore-compiler --test examples_suite
```

## The contract is a declaration, not a proof

Loading `contracts/buffer_api.qcore` asserts nothing about `buffer_api.c`. It is
a `ContractTrust::Declared` assumption (contract spec §C06): the checker reads
the clauses as **given**, records `DeclaredContract("demo_sum")` and one such
premise per admitted callee in the certificate, and the driver prints those
premises so a reader can see what a result rests on.

If a clause is wrong about the C, the result is wrong. That is why the clauses
are written down where a reviewer can read them — and why a sanitized run of
these programs is worth having: it can **refute** a clause (a lost release
becomes a leak, a double release a double free) even though no run establishes
one in general.

A standing premise sits beside them: **no clause transports an extent.** C04
pointers are extentless `u8` projections, so nothing declares how many bytes a
callee touches, and the certificate carries `NoExtentTransport` by name. That
`demo_fill` writes exactly eight bytes is a property of the fixture, not of the
declaration.

## The four modes these examples use

| Clause | Means |
|---|---|
| `resource_return_owned(nullable)` | the result **may** be null and carries a fresh obligation |
| `resource_borrowed(x)` | the callee uses `x` and takes on no obligation |
| `resource_consumed(x, nonnull)` | the caller's obligation **moves** to the callee |
| `resource_return_none()` | the result carries no custody — a plain scalar |

## Deciding both arms

```qcore
let b = demo_alloc(8) succeeds true else raise Failure;
```

`succeeds true` is not an assertion the checker believes. The declaration says
the result may be null, so the caller has to **decide both arms**, and this is
the spelling of that decision: succeed and bind, or raise `Failure`. The emitted
C really compares the returned pointer against the declared outcome.

`move b` at the release is where custody leaves the function. Use `b` after it
and the module refuses; forget the release and it refuses too.

`buffer_round_trip.qcore` explicitly selects `log_level trace;`, rendering its
typed resource and call observations on stdout:

<!-- transcript: buffer_round_trip stdout -->
```text
acquire demo_alloc r1 ok
call demo_fill b:r1 d:7
value demo_sum v1 b:r1 d:8 -> 56
call demo_release c:r1
return
```

`r1` is the resource's model identity and `v1` a bound value's. The argument
prefixes are `b:` for a borrowed identity, `c:` for a consumed one, `v:` for a
bound value read at this call, and `d:` for plain data written in the source.

## Scalar results

`demo_sum` and `demo_scale` both declare `resource_return_none()` beside a
non-`unit` result. Contract spec §C04.1 is the licence: *a scalar may carry a
descriptor number as data, but cannot carry descriptor custody.* So the bound
results are ordinary values — they sit beside the buffer's accounting without
entering it, and dropping one on the floor leaks nothing.

`succeeds 56` on such a call is a **declared outcome** and is optional: a call
without one binds whatever the callee returned. Where it is written, the emitted
C compares the real result and exits `70` rather than continuing with a number
nobody computed; the structured outcome event records that value.

`scalar_results.qcore` also specifies `log_level trace;`, so it writes:

<!-- transcript: scalar_results stdout -->
```text
acquire demo_alloc r1 ok
call demo_fill b:r1 d:7
value demo_sum v1 b:r1 d:8 -> 56
value demo_scale v2 v:v1 d:2 -> 112
call demo_release c:r1
return
```

## `borrow NAME`: a line about a resource that is still alive

[borrowed_render.qcore](borrowed_render.qr) renders something *about* a live
resource without consuming it, releasing it, or laundering its identity into a
value.

`borrow b` renders exactly one thing: the resource's **model identity** — the
number the trace above spells `r1`. Not a byte of the object, not an address,
not an extent, because no model a rendering can reach carries one.

What the borrow buys is a **checked precondition**. Each rendering is admitted
only because the extern model's own ledger holds `b` live at that statement.
Move the release above the second rendering and the file refuses at `Q043` —
that is the feature in one sentence: a diagnostic line about a resource cannot
outlive the resource. (Moving the release above the *first* rendering refuses
too, but earlier and for an older reason: the calls in between would then be
using an identity that has already been consumed, which is `Q024`. The point
stands twice over; only one of the two refusals is this feature's own.)

It needs both halves of the import list — `std.fmt` for the renderings,
`std.io`/`std.logger` for the sinks — which puts it on the composed route
`source.std-io-extern.v1`. Its explicit `log_level trace;` renders the system
events alongside the source-authored borrowed render. Its **stdout** is:

<!-- transcript: borrowed_render stdout -->
```text
acquire demo_alloc r1 ok
1
call demo_fill b:r1 d:7
value demo_sum v1 b:r1 d:8 -> 56
call demo_release c:r1
return
```

and its **stderr**:

<!-- transcript: borrowed_render stderr -->
```text
[INFO] 56
[WARN] 0x00000001
```

The bare `1` on stdout is `print dec borrow b` — the identity `r1` rendered as a
number — and `0x00000001` on stderr is the same identity in hex, rendered after
the sum and before the release, while the obligation is still open.

The sanitized run is a **positive leak control** for that claim: the program
allocates, renders the identity twice while the obligation is open, and
releases. Had a borrowed render moved, duplicated or discharged custody, LSan or
the double-free detector would have said so instead of the test passing.

## Building and running

These examples cannot be checked through the CLI today: the driver selects no
extern contract environment, so a declared extern call reaches it as

```text
Q023: calling a declared extern function requires an explicit extern contract
selection; there is no default environment
```

That is fail-closed and correct — a selection flag is the named follow-on
`UNSAFE-LINKAGE-CLI`, not a default. Until it lands the route is reachable
through the library API, `compile_files` with `CompileOptions::extern_contracts`,
which is what the test suite does. Once the emitted C exists, link it beside the
callees:

```sh
cc -std=c11 -Wall -Wextra -Werror -pedantic generated.c examples/extern/buffer_api.c -o generated
```

For C that has **no** contract, and no CLI obstacle, see
[../unsafe-linkage/](../unsafe-linkage/).

## Not expressible here

* **No rendering on this route alone.** `source.extern-call.v1` has no `print`.
  Getting a total onto a stream means composing with `std.io`, which is
  [../stdio/pipeline.qcore](../stdio/pipeline.qr).
* **A borrowed render needs a custody model.** Written on the plain
  `source.std-io.v1` route it refuses at `Q028` — not because that route
  forbids it, but because that route has no identity map at all, so there is
  nothing a borrow could name.
* **Only `int32_t` is renderable.** A callee declared to return `i64` can be
  called and bound here, but its result is refused at a rendering rather than
  emitted through a conversion that does not describe it.
