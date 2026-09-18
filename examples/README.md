# Examples

Runnable QCore programs, one directory per feature family. Every program in
these directories compiles through the real pipeline; every program that emits C
has that C built under `-Wall -Wextra -Werror` and run under the sanitizers in a
workspace test that compares its output with the reference execution — and every
transcript printed in a README is read out of that file by a test, so no page
here can drift from what its programs do.

One published program deliberately emits nothing:
[`events/write_interest.qcore`](events/write_interest.qcore) is the event
route's check-only arm, accepted and checked in full but not executed, and its
page says so. It is the only one.

| Directory | Family | Route |
|---|---|---|
| [stdio/](stdio/) | `print`, `log`, `flush`, and the composed printing-and-calling route | `source.std-io.v1`, `source.std-io-extern.v1` |
| [pipes/](pipes/) | The `\|>` operator: single stages and chains | (token tier — no route of its own) |
| [branching/](branching/) | `if`/`else`, the custody join, and the condition tier | `source.branch.v1` |
| [extern/](extern/) | Declared extern calls, `succeeds`, `borrow NAME` | `source.extern-call.v1` |
| [unsafe-linkage/](unsafe-linkage/) | `unsafe extern fn` and native-library manifests | `source.extern-call.v1` |
| [threads/](threads/) | Scoped tasks, owned buffers, read shares | the checked task profiles |
| [events/](events/) | One event context, a registration on a descriptor, declared dispatch turns — and a **checked socket** registered with that reactor, the server shape | `source.event.v1`, `source.socket.v1` |
| [strings/](strings/) | Immutable UTF-8 views, slices, text reaching a stream, text a program computes — and a `for` inside the `std.buffer` composition | `source.string.v1` |
| [sockets/](sockets/) | Checked TCP and UDP over loopback, both ends in one program — with a kernel-chosen port, or over a **declared** IPv4/IPv6 address and port | `source.socket.v1` |
| [http/](http/) | A checked, executed HTTP/1.1 subset — including a `Content-Length` the program computes — and the same bytes written by hand | `source.http.v1`, `source.socket.v1` |
| [loops/](loops/) | The bounded counted loop `for i in 0..n`, the bulk buffer copy it was landed for, and a loop beside a write lend | `source.loop.v1` on `source.buffer.v1` |
| [records/](records/) | The first aggregate type: `type NAME { … }`, construction, field reads — a plain copyable value, shown alone and beside an owned buffer | `source.record.v1` on `source.buffer.v1` |

`compiler/tests/examples_suite.rs` holds **every** directory in the table above
to one standard: each published program has a row on its family page, each row
names a file that exists, and each transcript a page prints is read out of that
page and compared with the pipeline's own reference execution. Run it, with the
driver checks beside it:

```sh
cargo test -p qcore-compiler --test examples_suite
cargo test -p qcorec --test examples_cli
```

The sanitized native runs are each owned by one test file — the suite runs the
ones no other file does and references the rest rather than building them twice:

| Directory | Native runner |
|---|---|
| stdio/, pipes/, branching/, extern/, unsafe-linkage/, loops/ (`buffer_copy`), records/ | `cargo test -p qcore-compiler --test examples_suite` |
| loops/ (`lend_fill`), strings/ (`looped_fill`) | `cargo test -p qcore-compiler --test loops_on_composed_routes` |
| threads/ | `cargo test -p qcore-compiler --test thread_examples` |
| events/ | `cargo test -p qcore-compiler --test std_event` (`self_pipe`, `write_interest`), `--test compose_event_lines` (`wake_payload`), `--test socket_event_compose` (`socket_server`) |
| strings/ | `cargo test -p qcore-compiler --test string_slice_io` (`status_line`), `--test string_concat` (`computed_length`) |
| sockets/ | `cargo test -p qcore-compiler --test std_socket` |
| http/ | `cargo test -p qcore-compiler --test std_http` |

One file here belongs to no family: [`cycle12.qcore`](cycle12.qcore) predates
the directories and is the smallest sequential-cleanup program in the tree —
one acquisition, one check, one release. It has no page of its own and claims
none of the guarantees the table above does; the suite holds it to one thing
only, that it still checks.

## How these programs are spelled: pipe the flow, parenthesize the assembly

`|>` and the statement form desugar to the same tokens before any checker runs,
so choosing between them is a choice about reading and never about meaning. The
examples make it the same way every time:

* **Pipe the flow.** Where a value moves in a straight line from one binding
  into one sink — a scalar rendered, a string written, a loop index printed —
  write `value |> print dec;`. The value goes where the eye starts, and nothing
  is lost, because a pipeline is exactly linear.
* **Parenthesize the assembly.** Where a value is *built* from several pieces,
  write the call: `concat("Content-Length: ", dec(length), "\r")`. Pipe
  insertion is first-argument-only by design, so an n-ary construction has no
  honest pipeline spelling — one operand would go to the front and the rest
  would stay behind, which reads as a claim about the pieces that is not true.
* **A value that fans out stays a statement.** Where one binding is read by two
  or more later statements — `first` printed twice in
  [sockets/tcp_echo.qcore](sockets/tcp_echo.qcore), `length` both declared in a
  header and printed in [http/computed_length.qcore](http/computed_length.qcore)
  — the flow is not linear, and piping the first reader would suggest the value
  ends there. Keep `print dec first;`.
* **Never contort one into the other.** A pipeline is linear and first-argument
  only; those are design decisions, not gaps. Where the shape does not fit,
  the statement form is the right spelling and not a fallback.
* A **written literal at a sink** is not a flow: `print "HTTP/1.1 200 OK";` has
  nothing to move to the front, so it stays as written.

One published program is **not** yet spelled to the rule, and saying so here is
cheaper than a rule with a silent exception:
[strings/status_line.qcore](strings/status_line.qcore) writes `body |> print;`
and then `body[7:12] |> print;`, so `body` is read by two statements and belongs
on the fan-out side. Whether a *slice of* a value counts as a second reader, or
as the assembly half of this rule, is an open question about the rule rather
than about that program, and it is the one case where moving a line would mean
un-piping rather than piping (`EXAMPLES-STYLE-SLICE-FANOUT`).

Two families sit above this rule rather than under it, and deliberately:

* [events/](events/) keeps its own standing rule — **every line in those
  programs is spelled as a pipeline**, fan-out included — stated on that page
  and unchanged here. Where the two rules disagree, the events page wins.
* [pipes/](pipes/) and [stdio/](stdio/) are the *demonstration* of the two
  spellings: `pipes/rendered_lines.qcore` and its unpiped twins
  `stdio/renderings.qcore` and `stdio/severities.qcore` exist to be read side by
  side and emit the same C. Neither side is respelled toward the other, because
  the contrast is the content.

Two things worth knowing before reading any of them:

* **A route is selected by what a file uses, not by a flag.** Importing
  vocabulary is not using it, and a module that uses two feature sets is checked
  by the composition of both routes rather than by whichever is more permissive.
* **Premises are printed, not discharged.** Where an example rests on something
  declared rather than proved — a contract about C, an artifact identity, an ABI
  model — its README says so in those words, and `quirin check` prints the list.
