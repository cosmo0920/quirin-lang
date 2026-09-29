# Wave 48 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> record ([wave-47.md](wave-47.md)) holds the wave-47 paragraphs and stays
> unchanged. Wave 48 opens the `.qr` language track (fixed-width scalars, C
> struct import, usable records, task record captures, closures).

**`W48-QR-SCALAR-TYPES` (2026-09-29, base `d389700a`).** Card 0 of the `.qr` language track ([report](../../reports/w48-qr-scalar-types/README.md); implemented grammars §G02s and new §G02t, specification §L14.0/§L20/§L21; `QCore.ScalarTypes`, new `AuditScalarTypes.lean`, 23 theorems). The §G05 `scalar_abi` table (`bool`, `i8`–`i64`, `isize`, `u8`–`u64`, `usize`) is first-class on `source.scalar-calls.v1`: parameters, results, annotated and inferred locals, literals range-checked per context type (default `i32`), checked `+ - * / %` and negation, comparisons producing a distinct `bool`, and one explicit conversion `EXPR as TYPE` (total when widening, checked otherwise; `as` was already reserved and a type name after it is contextual, census 0 hits in 625 files). Every fault is decided at check time on this closed route (`Q020`, no code allocated) and the emitted C still tests operands before each operation (explicit range tests, exit 86). `isize`/`usize` take their width only from a recorded target (`quirin --recorded-target FILE`), a named assumption of the result, bound in the C by `_Static_assert`s; without one they refuse. Output is only `std.fmt`/`std.io`/`std.logger` lines (read by §G02g's own reader, pipe sinks included): `dec` the number, `hex` the type's own width, `bool` as `true`/`false` with no `hex`; the builtin `print(…)` stays `i32`-only. All 625 existing fixtures check and emit byte-identically; a boundary table of 2,175 native cases ran at Clang `-O0`/`-O2` under UBSan and plain `-O2` with every fault exiting 86 and 0 sanitizer reports; the corpus arbiter's `resource-diagnostics.json` is byte-identical (`ae17152b…`). Records, loops, extern values, task captures and payloads keep their refusals. Follow-ups 580 → 588, none retired.
