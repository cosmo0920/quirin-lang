# Examples

Runnable QCore programs, one directory per feature family. Every program in
these directories compiles through the real pipeline, emits C11 that builds
under `-Wall -Wextra -Werror`, and runs under the sanitizers in a workspace
test that compares its output with the reference execution — and every
transcript printed in a README is read out of that file by the test, so no page
here can drift from what its programs do.

| Directory | Family | Route |
|---|---|---|
| [stdio/](stdio/) | `print`, `log`, `flush`, and the composed printing-and-calling route | `source.std-io.v1`, `source.std-io-extern.v1` |
| [pipes/](pipes/) | The `\|>` operator: single stages and chains | (token tier — no route of its own) |
| [branching/](branching/) | `if`/`else`, the custody join, and the condition tier | `source.branch.v1` |
| [extern/](extern/) | Declared extern calls, `succeeds`, `borrow NAME` | `source.extern-call.v1` |
| [unsafe-linkage/](unsafe-linkage/) | `unsafe extern fn` and native-library manifests | `source.extern-call.v1` |
| [threads/](threads/) | Scoped tasks, owned buffers, read shares | the checked task profiles |
| [events/](events/) | One event context, a registration on a descriptor, declared dispatch turns | `source.event.v1` |
| [strings/](strings/) | Immutable UTF-8 views, slices, and text reaching a stream | `source.string.v1` |
| [sockets/](sockets/) | Checked TCP and UDP over loopback, both ends in one program | `source.socket.v1` |
| [http/](http/) | A checked, executed HTTP/1.1 subset, and the same bytes written by hand | `source.http.v1`, `source.socket.v1` |

Run everything the first five directories claim:

```sh
cargo test -p qcore-compiler --test examples_suite
cargo test -p qcorec --test examples_cli
```

and the threading examples:

```sh
cargo test -p qcore-compiler --test thread_examples
```

and the event examples:

```sh
cargo test -p qcore-compiler --test std_event
```

and the string example:

```sh
cargo test -p qcore-compiler --test string_slice_io
```

Two things worth knowing before reading any of them:

* **A route is selected by what a file uses, not by a flag.** Importing
  vocabulary is not using it, and a module that uses two feature sets is checked
  by the composition of both routes rather than by whichever is more permissive.
* **Premises are printed, not discharged.** Where an example rests on something
  declared rather than proved — a contract about C, an artifact identity, an ABI
  model — its README says so in those words, and `quirin check` prints the list.
