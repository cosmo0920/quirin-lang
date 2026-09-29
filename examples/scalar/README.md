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

## The typed arithmetic error

[typed_errors.qr](typed_errors.qr) uses the typed arithmetic error (card
`W48-QR-SCALAR-TYPES-R2`). A function whose `effects` row declares
`throws<ArithmeticError>` raises `ArithmeticError::Overflow`,
`::DivisionByZero` or `::OutOfRange` where an operation on a value the checker
did not fold would fault; its caller propagates the error with `CALL?` (under
the same permission) or handles it with `match CALL { Result::Ok { value: v }
=> { … } Result::Err { error: e } => { … } }`, and `raise e;` re-raises it. A
fault the checker can decide from constants alone is still refused, and so is
a fault in a function without the permission. Here `80 * 3` fits `u8`,
`90 * 3` overflows it, and a zero-second window divides by zero:

<!-- transcript: typed_errors stdout -->
```text
60
```

<!-- transcript: typed_errors stderr -->
```text
[WARN] 255
[WARN] 0
```

An entry that propagates the error exits with status `80 + tag` (81, 82, 83)
after the lines it wrote; this one handles every raise and exits 0.

## Typed record fields

[record_fields.qr](record_fields.qr) declares a record whose fields are
fixed-width scalars (card `W48-QR-SCALAR-TYPES-R3`) on the byte-buffer route:
each field value is checked against its field's type — `65535` is a `u16`,
`true` a `bool` — and a field read binds a value of that type, rendered at it.
A buffer index takes an `i32`, so the `u8` version is converted with `as`
before it indexes; a `u8` at the index would be refused, not widened:

<!-- transcript: record_fields stdout -->
```text
2
0xFFFF
7
```

<!-- transcript: record_fields stderr -->
```text
[INFO] true
```

Every value on this route is known when the program is checked, so a literal
outside its field's type or a conversion that does not fit refuses the module
at check time.

See [the exact grammar](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)
and [implementation evidence](../../docs/reports/source-scalar-calls-v1/README.md);
the fixed-width types are [§G02t](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
and [their evidence](../../docs/reports/w48-qr-scalar-types/README.md); the typed
error is [its evidence](../../docs/reports/w48-qr-scalar-types-r2/README.md), and
the typed record fields are [card R3's](../../docs/reports/w48-qr-scalar-types-r3/README.md).
