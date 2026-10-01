# MSVC default-C ABI declarations

The caller fixtures for UCRT `memcmp`, `strncmp` and `strnlen` now require
`Safe` and independent certificate replay against the real preprocessed
headers. No library body or contract is substituted. The native test declares
`MainSourceDefinitions`: it certifies its caller, while unrelated UCRT inline
helpers remain outside that recorded target scope. This is conditional custody
safety under the loaded libc contracts, not string-bounds safety or a proof of
the CRT implementation. [Prediction](PREDICTION.md) was committed before code
as `af37f1e5` (base `53ffdbd3`).

The lexical declaration reader interprets one function-prefix `__cdecl` as
the default C convention, removing it before counting result pointer levels.
It reads exactly one `__int64` plus at most one `signed` or `unsigned` word,
in either order, as the existing signed/unsigned 64-bit projections. `isize`
and `usize` retain the existing 64-bit-pointer premise. Ordinary `long` still
has the reader's LP64 premise; this is not general LLP64 or x86 support.
A newly readable cdecl result or parameter involving ordinary `long` stays
unreadable, including through typedef chains, to avoid inheriting a misleading
LP64 reading on MSVC. Explicit `long long` keeps its 64-bit reading.
Other conventions, repeated annotations, mixed sized specifiers, wrong widths,
wrong signs and unproven source effects retain their refusals. No AST is used
in the ABI comparison; derivation, validation and certificate replay all
recompute from retained tokens and contracts.

`QCore.MsvcAbiReader` models the two decisions and audits 23 theorems:
exact word counts, signed/unsigned readings, no pointer/custody laundering,
preservation of existing comparisons, unreadable-convention refusal, native
query agreement and width/qualifier/hidden-pointer-result disagreements.
Compiler language facts are external premises; extraction and Lean-to-Rust
correspondence remain the review-maintained premises of [boundary 28](../../agents/evidence.md#constraint-28).

The `loader_abi` admission table adds 18 held-out MSVC rows; an additional
caller control refuses an allocator contract that hides a cdecl pointer result.
The real-header negative control deliberately declares MSVC `memcmp`'s size
as `u32` and must return `Unknown` with `abi-parameter-type-disagreement`.
Prior failure logs show why whole-header certification is not claimed: the
caller becomes Safe but unrelated UCRT inline bodies keep the whole expanded
unit Unknown until the test selects its intended main-source scope.

The additional package-inheritance fixture reconstructs a published LF parent
only when those exact bytes match the inventory's existing SHA-256. Raw bytes
are retained when they already match. This repairs checkout CRLF differences
without changing a pin or making the production bundle verifier newline-insensitive;
changed/missing-parent controls still refuse. Its eight tests pass.

Validation is being recorded in `private/msvc-abi-20261001/`. The full standing
gate will be restarted for this semantic change; previous in-flight workspace
attempts are incomplete evidence, never reported as a passing gate. No pinned
GNU/Linux corpus measurement or broader coverage increase is claimed here.

Primary facts: [Microsoft `__int64`](https://learn.microsoft.com/en-us/cpp/cpp/int8-int16-int32-int64?view=msvc-170)
and [`__cdecl`](https://learn.microsoft.com/en-us/cpp/cpp/cdecl?view=msvc-170).
