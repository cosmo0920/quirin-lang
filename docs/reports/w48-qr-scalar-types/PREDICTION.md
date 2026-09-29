# W48-QR-SCALAR-TYPES: frozen prediction

Frozen after the BEFORE fixture freeze and before any source, test or Lean
change. Base: local `main` at `d389700a` (the worktree's HEAD).

## BEFORE (measured)

`quirin` release binary built at the base (sha256 `e171623b…`). The freeze
script (`freeze_fixtures.py`) ran `quirin check FILE` and `quirin emit FILE`
over the 625 tracked `.qr`/`.qv`/`.qrlib` files: check exit 0 on 357, exit 1
on 268; emit exit 0 on 153, exit 1 on 472. Two reruns gave a byte-identical
manifest. Four fixtures reach `source.scalar-calls.v1` today: the admitted
`examples/scalar/calls.qr` and `examples/scalar/printing.qr`, and the refused
probes `S10_call_result_mismatch.qr` ("immutable locals require i32") and
`S32_callee_result_in_return.qr` ("return type mismatch").

BEFORE probes of the surface this card widens: `fn f(x: u8) -> u8` refuses at
Q020 ("only explicit i32/unit types are supported"); `x as i32` refuses at
Q013 (`i32` collected as an unknown name); `-> bool` refuses at Q020; an
annotated `let x: i32 = 5; print dec x;` entry refuses on the std.io route
(Q030 + Q028); an std.io entry beside an ignored i32 helper function is
admitted by the std.io route when its row declares `io`.

## Mechanism (planned)

1. **Core types (A), on `source.scalar-calls.v1`.** The route's `i32` becomes
   the §G05 table `bool i8 i16 i32 i64 isize u8 u16 u32 u64 usize` at
   parameters, results, annotated and inferred locals, literals, `+ - * / %`,
   unary `-`, the comparisons `== != < <= > >=` (producing `bool`), and one
   conversion spelling `EXPR as TYPE`. The check is still the route's closed
   evaluation, so every overflow, zero division, `MIN / -1`, out-of-range
   literal and failing conversion of an executable entry is decided and refused
   at check time (Q020, conditions widened, no new code). Literals take their
   type from context and default to `i32`, so every existing program types as
   before. Mixed-type operands, `bool` arithmetic and ordering, and any
   int/bool conversion refuse. `isize`/`usize` need a recorded target
   (pointer width, triple, compiler) supplied by the builder
   (`quirin --recorded-target FILE`); without one a program naming either type
   refuses. The emitted C checks every operation before performing it with
   explicit range tests (no `__builtin_*`, so MSVC still compiles it) and exits
   86 on a fault, the route's existing fault status; `size_t`/`ptrdiff_t`
   carry `usize`/`isize` behind `_Static_assert`s binding them to the recorded
   width.
2. **Rendering (B).** The route admits `source.std-io.v1`'s own line
   statements (`print dec|hex X;`, `log LEVEL dec|hex X;`, `flush;`), read by
   that route's own reader, in any scalar function. `dec` is signed or
   unsigned decimal by type; `hex` is `0x` plus the two's-complement pattern at
   the type's own width (`u8` 2 digits, `i32` 8 as today, `u64` 16, `usize`
   W/4); `bool` renders `true`/`false` under `dec` and refuses `hex`. Route
   selection: a module with std.io lines and scalar-call syntax that the
   std.io lowering refuses goes to the scalar route; every module the std.io
   lowering admits keeps that route byte for byte. Pipe sinks follow by
   desugaring. The builtin `print(EXPR);` stays i32-only and is refused in a
   module that uses a line statement or a non-`i32` type.
3. **Lean.** A new `QCore.ScalarTypes` module with the fixed-width ranges,
   checked operations, conversions and comparison, and the five theorems the
   brief names, registered in `AuditScalarTypes.lean`.

Records (C), loops (D), extern results (E), task captures and payloads (F),
and the composed routes' typed operands keep their existing refusals and are
registered as follow-ups with sizes.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| fixture freeze, AFTER vs BEFORE | all 625 check and emit entries byte-identical | 85% |
| if not | a std.io-refused fixture with scalar-call syntax moves to the scalar route (refusal code Q030/Q028 → Q020 or an admission), each attributed | 12% |
| existing scalar-route tests | pass unchanged (messages for the unit cases kept verbatim) | 80% |
| conversion spelling census | zero `as <scalar type>` in any tracked source file; `as` already reserved | 95% |
| native boundary table (clang -O0, -O2, UBSan `undefined,integer`) | every admitted case agrees with the reference; every faulting case exits 86 through the emitted guard; zero UBSan reports | 85% |
| Lean | new module builds with no placeholders and no project axioms | 85% |
| corpus arbiter | `resource-diagnostics.json` byte-identical (`ae17152b…`): no C-audit path touched | 97% |
| standing gate | green; test count rises by the new tests only | 85% |
