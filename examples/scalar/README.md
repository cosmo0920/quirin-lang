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

## Fixed-width values across a task boundary

[typed_workers.qr](typed_workers.qr) passes fixed-width values to worker
threads (card `W48-QR-SCALAR-TYPES-R4`): a `copy` capture of any scalar type,
a worker parameter and result of that type, and the value statements of this
page — one arithmetic, comparison, negation or `as` per `let` — inside the
worker. A captured value is known only when the worker runs, so an operation
that can fault needs the worker's `throws<ArithmeticError>`, and a fault
raises the error in the worker. `join` collects it like any typed worker
error, and the creator consumes it with `?` or `match`; here the third
worker's `i64` product overflows and the creator's `Result::Err` arm handles
it:

<!-- transcript: typed_workers stdout -->
```text
245
0xF5
9000000000
true
```

<!-- transcript: typed_workers stderr -->
```text
[WARN] 4611686018427387904
```

## Records across calls

[record_calls.qr](record_calls.qr) passes records to functions and returns
them (card `W48-QR-USABLE-RECORDS`). A module that declares a record and a
function besides its entry takes this route; a record of scalars — here
`Point`, and `Box`, which holds two of them — is a plain value, so a call
copies it in and a `return` copies it out, and `grow` building a new `Box`
leaves the caller's `b` as it was. A field is read through its path
(`b.hi.x`), a construction names every field in any order, and a record has no
arithmetic, comparison or rendering of its own: read the field you mean. A
field write is not a statement of this route (its rows admit `io` and the
typed error only, and a write derives `mut`), and a record holding an owned
`bytes` field does not cross a call here:

<!-- transcript: record_calls stdout -->
```text
9
14
25
0x07
10
```

## Typed loops and arithmetic on the byte-buffer route

[typed_loops.qr](typed_loops.qr) counts a `for` loop over a `u8` bound (card
`W48-QR-SCALAR-TYPES-R5`): the bound is a conversion of a known `i32`, so it
is known when the program is checked, and the index has the bound's type —
here `u8`. The byte-buffer route also gains this page's value statements —
one arithmetic, comparison or negation per `let` — and since every value on
that route, the index included, is known at check time, each is decided then:
`250 + i` fits `u8` for every index this loop takes, and a bound one larger
would refuse the module rather than overflow:

<!-- transcript: typed_loops stdout -->
```text
0xFA
0xFB
0xFC
0xFD
255
1
```

<!-- transcript: typed_loops stderr -->
```text
[INFO] false
[INFO] true
[INFO] false
[INFO] true
```

## Floating point

[floating_point.qr](floating_point.qr) uses `f32` and `f64`, IEEE 754
binary32 and binary64 (card `W48-QR-FLOATING-POINT`). A float literal is
written with a point or an exponent (`2.5`, `1.5e1`), takes its context's
type (`f64` when nothing gives one) and is rounded once, at that type; an
integer literal is never a float. Arithmetic (`+ - * /`, unary `-`) never
faults — `0.0 / 0.0` is a NaN and `-1.0 / 0.0` is `-inf` — and a comparison
is partial, so it needs its effect in the row: `partial_eq` for `==`/`!=`,
`partial_ord` for the orderings, never inferred, and declared by every caller
of a function that compares (here `main` declares `below`'s `partial_ord`). A
NaN is not equal to itself. `as` converts: an integer or `f64` to `f32` rounds
to nearest, ties to even (`9007199254740993` becomes `9007199254740992`), and
a float to an integer truncates toward zero, refused when it does not fit.
`dec` prints the shortest decimal that reads back to the same value, `hex`
C's hexadecimal form, `nan`, `inf`, `-inf` and `-0` as written:

<!-- transcript: floating_point stdout -->
```text
6.25
0x1.9p+2
0.1
0.10000000149011612
nan
-inf
-0
false
true
```

<!-- transcript: floating_point stderr -->
```text
[INFO] 6
[INFO] 9007199254740992
```

The result holds under a named assumption the check prints: IEEE 754
binary32/binary64 evaluated at their own type, round-to-nearest-even,
subnormals kept, no fast-math and no contraction. The emitted C asserts the
formats and `FLT_EVAL_METHOD == 0` when it is compiled; build it with
`-ffp-contract=off`. The limitations are the specification's
[§L14.7](../../docs/QCORE_SPEC_r0_3.md#l147-floating-point-and-its-limitations).

[float_buffer.qr](float_buffer.qr) holds floats on the byte-buffer route
(card `W48-QR-FLOAT-BUFFER-PAYLOADS`): `f32`/`f64` record fields, the literal
binding `let limit = 3.75;` (an `f64`), `as` to and from floats, and value
statements at the top level and in a loop body. Every value on this route is
known when the program is checked; the comparison needs `partial_ord` in the
entry's row, and the loop's bound is the float converted with `as` (a float
bound is refused). Writing `-0.0` into the field keeps its sign:

<!-- transcript: float_buffer stdout -->
```text
7.5
0x1.ep+2
7.75
7
-0
0x1.99999ap-4
```

<!-- transcript: float_buffer stderr -->
```text
[INFO] true
[INFO] true
[INFO] false
```

`Buffer<f32>`/`Buffer<f64>` payloads (4 and 8 bytes) and `protected f32`/`f64`
are admitted on the task and mutex routes; a byte view of a float payload is
its IEEE representation, and a NaN's sign and payload bits in it are not
specified.

See [the exact grammar](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)
and [implementation evidence](../../docs/reports/source-scalar-calls-v1/README.md);
the fixed-width types are [§G02t](../../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
and [their evidence](../../docs/reports/w48-qr-scalar-types/README.md); the typed
error is [its evidence](../../docs/reports/w48-qr-scalar-types-r2/README.md), and
the typed record fields are [card R3's](../../docs/reports/w48-qr-scalar-types-r3/README.md), and the
typed task values are [card R4's](../../docs/reports/w48-qr-scalar-types-r4/README.md), and the
typed loops and buffer-route arithmetic are [card R5's](../../docs/reports/w48-qr-scalar-types-r5/README.md);
floating point is [its evidence](../../docs/reports/w48-qr-floating-point/README.md),
and floats on the buffer route and in payloads [card W48-QR-FLOAT-BUFFER-PAYLOADS's](../../docs/reports/w48-qr-float-buffer-payloads/README.md).
