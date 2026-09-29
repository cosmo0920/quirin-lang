# Scalar calls and explicit output

[calls.qr](calls.qr) computes 42 through parameterized helper functions
without printing it. [printing.qr](printing.qr) uses the same scalar-call
profile and writes the computed value explicitly. Check and emit either file:

```sh
cargo run -p qcorec --bin quirin -- check examples/scalar/calls.qr
cargo run -p qcorec --bin quirin -- emit -o /tmp/scalar-calls.c examples/scalar/calls.qr
cc -std=c11 /tmp/scalar-calls.c -o /tmp/scalar-calls
/tmp/scalar-calls
```

The `calls.qr` executable is silent. The emitted C contains real helper
functions and calls; returning 42 does not itself write a diagnostic or result
banner. Arithmetic uses checked signed i32, including explicit
overflow/division guards, and a guard failure exits 86 without writing to stdout
or stderr.

`printing.qr` imports `std.io` and uses `print(f);`. That statement evaluates
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

## Fixed-width types

[fixed_width.qr](fixed_width.qr) uses the fixed-width scalar table (card
`W48-QR-SCALAR-TYPES`): `bool`, `i8` … `i64`, `u8` … `u64`, and `isize`/`usize`
when the builder supplies a recorded target (`--recorded-target FILE`). Every
operand of one operation has one type, every conversion is written with `as`,
and every operation is checked, so `u8` values 250 and 240 are averaged
through `u16` rather than overflowing. It renders through `std.fmt`, `std.io`
and `std.logger` — `print dec|hex X;`, `log LEVEL dec|hex X;`, `flush;` — never
through `print(…)`, which stays the `i32`-only form above:

<!-- transcript: fixed_width stdout -->
```text
245
0xF5
0xFFFFFFFFFFFFFFFF
true
```

<!-- transcript: fixed_width stderr -->
```text
[INFO] true
```

`hex` renders the type's own width (`u8` two digits, `i64` sixteen); a `bool`
renders `true`/`false` and has no `hex`. An overflow, a zero divisor, a
conversion whose value does not fit, or a literal outside its type refuses the
module at check time.

See [the exact grammar](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)
and [implementation evidence](../../docs/reports/source-scalar-calls-v1/README.md);
the fixed-width types are [§G02t](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
and [their evidence](../../docs/reports/w48-qr-scalar-types/README.md).
