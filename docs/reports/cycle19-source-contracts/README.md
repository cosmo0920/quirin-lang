# Cycle 19: native QCore contracts checked against typed C source

The pinned Fluent Bit source now passes two semantic checks whose contract
definitions live in native `.qcore` files. Rust imports the C bodies and
interprets those contract clauses; it has no duplicate per-function
postcondition table.

| Native contract | Actual C function | Result |
| --- | --- | --- |
| `contract/qcore/fluent_bit/busy_state.qcore` | `src/flb_input_chunk.c`: `flb_input_chunk_release_lock` | Checked over 4 Boolean pre-states |
| `contract/qcore/chunkio/lock_state.qcore` | `lib/chunkio/src/cio_chunk.c`: `cio_chunk_is_locked` | Checked over 4 Boolean pre-states |

The release function returns -1 when busy was false and leaves it false; when
busy was true, it clears busy and returns 0. Both branches preserve the ChunkIO
lock. The lock query returns the entry lock value and preserves both fields.
Negative C integer returns are normal exits, not QCore typed exceptions.

These results assume the QCore `requires` clauses: a live borrowed chunk,
exclusive access, and Boolean values for the modeled fields. They do not prove
these preconditions for arbitrary callers. Concrete pointer-to-chunk identity
binding remains a caller obligation. Clang typing and AST translation remain
trusted implementation boundaries.

## What the importer checks

The importer validates and preprocesses the translation unit with its build
definitions, then reads Clang's typed AST for each main-source function. It
supports a single typed chunk pointer, ordinary integer busy/lock fields,
integer returns, field assignments, and bounded branches and expressions.
Bitfields, volatile/atomic accesses, indirect aliases, calls, loops, unmodeled
fields, missing returns, and unsupported AST nodes remain `Unknown`. Even
unexecuted statements must belong to the supported grammar before a summary
is issued. Idempotent C writes are retained and cannot satisfy a const borrow.

The QCore checker evaluates the postconditions using the C-derived paths and
the second field's frame condition. It rejects unsupported predicates, missing
domain assumptions, wrong parameter/result types, const-borrow writes, and
postcondition mismatches. Mutating either the QCore postcondition or the C
return behavior is covered by negative tests.

The source paths can instantiate operations at a live model chunk identity.
The lifecycle checker can then compose these with a checked prefix and later
teardown. An execution prefix retains live obligations and is deliberately a
different type from a complete teardown certificate.

## Source run

Fluent Bit revision: `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`.
The audit enforces the pinned Git revision and selected-source cleanliness.
It uses the existing Fluent Bit build database and the disposable forced-
Libevent Monkey build. No Fluent Bit or bundled-library source was edited.

```sh
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json
```

- `flb_input_chunk.c`: 66 discovered functions, 1 checked native contract,
  65 explicit lifecycle `Unknown` results.
- `cio_chunk.c`: 30 discovered functions, 1 checked native contract,
  29 explicit lifecycle `Unknown` results.
- Generic C audit: 185 translation units, 0 gateway failures, 4,346/4,775
  functions projected, 2 `Safe`, 54 provisional `Unsafe`, 125 `Unknown`,
  and 4 data-only units. These totals are unchanged.

The native contract verdicts and generic C projection verdicts are separate.
Neither file receives a whole-unit lifecycle certificate. Provisional `Unsafe`
counts are not confirmed Fluent Bit defects. The audit's zero exit status does
not imply all resource/lifecycle verdicts are Safe.

## Validation

- `cargo test --workspace`: 266 tests passed (244 unit, 19 provider,
  1 Libevent submodule, 2 native contract corpus); the corpus now has 13 files.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `qcorec check` on each new `.qcore` file: passed; this command checks QCore
  compilation and contract syntax, while the audit above checks C semantics.
- `cd proof && lake build`: passed.
- `lake env lean AuditCycle19.lean`: passed; no `sorryAx` or project axioms.
  The new proofs use only Lean's standard `propext`/`Quot.sound` where needed.
- The pinned external audit above: passed.

The new executable Lean model proves coverage of both Boolean entry values,
postcondition satisfaction for every accepted source path, preservation of
other identities, and preservation of the lock when busy changes. It does not
prove the Clang-to-IR implementation or unrestricted C semantics.

See [raw source output](source-audit.log), [workspace test output](tests.log),
[Lean dependency audit](lean-audit.log), and [run metadata](results.json).
This is static verification; no runtime, Valgrind, or sanitizer claim is made.

## Cycle 19 remains in progress

Remaining work includes general identity mapping for refcounts, queues, lists,
and arenas; call and failure-path imports across the ChunkIO teardown chain;
the rest of the wrapper's lifecycle paths; and source-derived Monkey callback
traces with a real dispatcher/backend observation. Other `.qcore` predicates
remain syntax-preserved until their semantic vocabulary is implemented.
