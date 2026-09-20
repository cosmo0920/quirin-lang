# Scalar calls and explicit output

[calls.qcore](calls.qcore) computes 42 through parameterized helper functions
without printing it. [printing.qcore](printing.qcore) uses the same scalar-call
profile and writes the computed value explicitly. Check and emit either file:

```sh
cargo run -p qcorec --bin quirin -- check examples/scalar/calls.qcore
cargo run -p qcorec --bin quirin -- emit -o /tmp/scalar-calls.c examples/scalar/calls.qcore
cc -std=c11 /tmp/scalar-calls.c -o /tmp/scalar-calls
/tmp/scalar-calls
```

The `calls.qcore` executable is silent. The emitted C contains real helper
functions and calls; returning 42 does not itself write a diagnostic or result
banner. Arithmetic uses checked signed i32, including explicit
overflow/division guards, and a guard failure exits 86 without writing to stdout
or stderr.

`printing.qcore` imports `std.io` and uses `print(f);`. That statement evaluates
`f` and writes its i32 value followed by one newline to stdout:

<!-- transcript: printing stdout -->
```text
42
```

This print form needs no `std.fmt`. With no `effects` row, `io` is inferred from
a direct print and through calls. `effects {io}` permits the same effect;
`effects {}` refuses a function whose body or callees require it.

The scalar backend supplies reusable file-scope fragments and an entry body to
the shared C-program assembler. The assembler emits the executable's single
`main` and deduplicates the common IO prelude. This mechanism does not permit
calls into another execution profile.

The profile supports no resource values, runtime branches, loops, mutation,
recursion, generics or indirect calls. The closed entry must be evaluable within
the declared bounds. A library's type check does not claim overflow freedom for
all possible arguments.

See [the exact grammar](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)
and [implementation evidence](../../docs/reports/source-scalar-calls-v1/README.md).
