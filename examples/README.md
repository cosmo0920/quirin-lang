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
| [strings/](strings/) | Immutable UTF-8 views, slices, text reaching a stream, and text a program computes | `source.string.v1` |
| [sockets/](sockets/) | Checked TCP and UDP over loopback, both ends in one program | `source.socket.v1` |
| [http/](http/) | A checked, executed HTTP/1.1 subset — including a `Content-Length` the program computes — and the same bytes written by hand | `source.http.v1`, `source.socket.v1` |
| [loops/](loops/) | The bounded counted loop `for i in 0..n`, and the bulk buffer copy it was landed for | `source.loop.v1` on `source.buffer.v1` |

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
| stdio/, pipes/, branching/, extern/, unsafe-linkage/, loops/ | `cargo test -p qcore-compiler --test examples_suite` |
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

Two things worth knowing before reading any of them:

* **A route is selected by what a file uses, not by a flag.** Importing
  vocabulary is not using it, and a module that uses two feature sets is checked
  by the composition of both routes rather than by whichever is more permissive.
* **Premises are printed, not discharged.** Where an example rests on something
  declared rather than proved — a contract about C, an artifact identity, an ABI
  model — its README says so in those words, and `quirin check` prints the list.
