# Unsafe linkage

Calling C that nothing describes, and saying so out loud.

| File | What it is |
|---|---|
| [unchecked_calls.qcore](unchecked_calls.qcore) | Three `unsafe extern fn` declarations and three calls |
| [unchecked_api.c](unchecked_api.c) | The C behind them — no contract, and there never will be one |
| [unchecked_demo.qcorelib](unchecked_demo.qcorelib) | A `source.native-library.v1` manifest: *which build* was waived |

Checked by [`compiler/tests/examples_suite.rs`](../../compiler/tests/examples_suite.rs)
(compilation, emission, sanitized native run) and by
[`crates/qcorec/tests/examples_cli.rs`](../../crates/qcorec/tests/examples_cli.rs),
which runs the driver and compares the premise output below with what it really
printed:

```sh
cargo test -p qcore-compiler --test examples_suite
cargo test -p qcorec --test examples_cli
```

## What `unsafe extern fn` declares

An ABI signature, and **nothing else**. No modes, no custody, no effects, no
promise about what the callee does with what it is handed.

The waiver has its own import. `std.extern` grants the declared-contract
surface; `std.extern.unsafe` grants the waiver, separately, so a file's import
list says whether it waives anything at all.

The surface is **scalar-only**, and that is the entire soundness argument. Every
parameter and every result here is a value: the caller hands over nothing it owes
a cleanup for and receives nothing it now owes one for, so its ledger is
untouched and the checked accounting *around* the call still holds even though
the call's body is completely unknown.

`unchecked_api.c` is written to make that unknown-ness visible.
`demo_hidden_effect` mutates a private global and writes a line to stderr;
`demo_measure` allocates and frees behind the caller's back. Neither appears in
the checked model, and neither should — the point is that nothing notices.

## The premises are printed, never discharged

A `Safe` obtained through unchecked linkage is never printed bare. Every waived
symbol becomes a named premise, and `quirin check` prints them:

```sh
quirin check examples/unsafe-linkage/unchecked_calls.qcore
```

<!-- cli: bare -->
```text
checked 1 function(s); entry `main`
under 5 declared assumption(s):
  - unchecked linkage: `demo_hidden_effect` is linked as `(uint32_t) -> void` with no contract; nothing about its behavior is checked
  - unchecked linkage: `demo_measure` is linked as `(uint64_t) -> uint64_t` with no contract; nothing about its behavior is checked
  - unchecked linkage: `demo_unchecked_add` is linked as `(int32_t, int32_t) -> int32_t` with no contract; nothing about its behavior is checked
  - no extent transport: no declaration says how many bytes a callee touches
  - linkage: which object file supplies each symbol is the builder's premise
```

Read that list as the answer to "what does this result depend on". Three symbols
are entirely unchecked; no declaration anywhere says how many bytes a callee
touches; and **which object file supplies each symbol is the builder's act, not
the checker's**.

## What a manifest adds: which build

A bare waiver cannot say which artifact it was waived from. That is what
`unchecked_demo.qcorelib` supplies — an artifact identity and the four ABI
premises the build was produced under — and selecting it sharpens the same
premises rather than adding new ones:

```sh
quirin check --native-library examples/unsafe-linkage/unchecked_demo.qcorelib \
             examples/unsafe-linkage/unchecked_calls.qcore
```

<!-- cli: manifest -->
```text
checked 1 function(s); entry `main`
under 5 declared assumption(s):
  - unchecked linkage: `demo_hidden_effect` is linked as `(uint32_t) -> void` with no contract, from artifact `libqcore_unchecked_demo.a` of library `qcore_unchecked_demo 0.1.0` at sha256:0000000000000000000000000000000000000000000000000000000000000000, under stated ABI premises (data model lp64, no unwinding across the boundary, closed allocator custody, single-threaded caller); the artifact identity is declared, not verified here, and nothing about the callee's behavior is checked
  - unchecked linkage: `demo_measure` is linked as `(uint64_t) -> uint64_t` with no contract, from artifact `libqcore_unchecked_demo.a` of library `qcore_unchecked_demo 0.1.0` at sha256:0000000000000000000000000000000000000000000000000000000000000000, under stated ABI premises (data model lp64, no unwinding across the boundary, closed allocator custody, single-threaded caller); the artifact identity is declared, not verified here, and nothing about the callee's behavior is checked
  - unchecked linkage: `demo_unchecked_add` is linked as `(int32_t, int32_t) -> int32_t` with no contract, from artifact `libqcore_unchecked_demo.a` of library `qcore_unchecked_demo 0.1.0` at sha256:0000000000000000000000000000000000000000000000000000000000000000, under stated ABI premises (data model lp64, no unwinding across the boundary, closed allocator custody, single-threaded caller); the artifact identity is declared, not verified here, and nothing about the callee's behavior is checked
  - no extent transport: no declaration says how many bytes a callee touches
  - linkage: which object file supplies each symbol is the builder's premise
```

The count is still five. Nothing was discharged; the same waiver now names a
build.

### The identity is a pin, not a probe

Nothing is hashed at check time. No file is opened, no digest is computed, and
whether the linker used this artifact remains the builder's act. The printed
line says exactly that: *"the artifact identity is declared, not verified here"*.

Which is why the digest in `unchecked_demo.qcorelib` is **all zeros**: this
repository ships no prebuilt archive, so the field is an honest placeholder that
a real deployment replaces with the digest of the archive it really links. A
builder-side hash check is the named follow-on
`UNSAFE-LINKAGE-ARTIFACT-PROBE`.

A manifest that does not load **refuses the module** rather than being ignored.
Change `data_model lp64;` to `data_model ilp32;` and the check fails at `Q039`.

### The four ABI premises

| Field | Value here | Means |
|---|---|---|
| `data_model` | `lp64` | the scalar widths the declarations were written against |
| `unwinding` | `abort` | no exception unwinds across the boundary |
| `allocator_custody` | `closed` | the library frees what it allocates; no allocator is shared |
| `thread_posture` | `single_caller` | the caller is single-threaded |

Each is a **stated** premise about the build, in the same sense as everything
else on this page. None is checked.

## Building and running

```sh
quirin emit examples/unsafe-linkage/unchecked_calls.qcore -o /tmp/unchecked.c
cc -std=c11 -Wall -Wextra -Werror -pedantic \
   /tmp/unchecked.c examples/unsafe-linkage/unchecked_api.c -o /tmp/unchecked
/tmp/unchecked
```

Unlike the contracted examples in [../extern/](../extern/), these run through the
CLI end to end — there is no contract environment to select.

The program's own trace goes to **stdout**:

<!-- transcript: unchecked_calls stdout -->
```text
call demo_unchecked_add d:2 d:40
call demo_hidden_effect d:7
call demo_measure d:16
return
```

and the callee's own line goes to **stderr**:

<!-- transcript: unchecked_calls stderr -->
```text
demo_hidden_effect: hidden state now 7
```

Those two blocks are the lesson. The first is the checked model reporting the
calls it made, with the arguments it passed. The second is a line the checked
model knows nothing about, written by a callee that also changed state the
caller cannot see — and neither the checker nor the emitted program has any
opinion about it.

A sanitized run establishes exactly one thing: the emitted program performed
these calls across a real link boundary, in the checked order, and left nothing
behind. It establishes nothing whatever about what the callees do, which is why
the premise is printed rather than discharged.

## Not expressible here

* **No pointer parameters.** The v1 unsafe surface is scalar-only. A library's
  pointer symbols belong at the contract-slice level of the linkage spectrum —
  a manifest whose entries are `contract fn … from …` rather than
  `unchecked fn …` — where the symbol is read through a declared contract and
  the manifest only selects and attributes.
* **A hash is not a link check.** Even a real digest here would say what a build
  *is*, not that the linker used it.
