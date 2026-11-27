# MSVC ABI reader prediction

Base: `53ffdbd3`, before implementation, 2026-10-01.

The native x86-64 MSVC host-declaration controls for `memcmp`, `strncmp` and
`strnlen` currently return `Unknown`: the retained declarations use `__cdecl`
and `typedef unsigned __int64 size_t`. Reading the default C calling convention
and the explicit 64-bit integer spelling should make these three controls
`Safe` for their declared neutral custody contracts, with certificates that
recheck against the retained text. This is conditional custody evidence, not
proof of the CRT implementation.

The change reads exactly one function-prefix `__cdecl` and the closed scalar
family `__int64`, `signed __int64`, `unsigned __int64` (specifier order may vary).
The scalar readings are the existing 64-bit projections, including `isize` /
`usize` under the reader's stated 64-bit-pointer premise. Invalid mixed
specifiers, other calling conventions, nested declarators, mismatched contracts
and unsupported source effects must remain refused. Ordinary `long` retains
its existing LP64 premise; this is not a general LLP64 target-model expansion.

No movement is predicted for pinned GNU/Linux declarations that contain
neither new spelling. This prediction is not a corpus measurement. Focused
controls will record native host observations and refusal outcomes separately
from the abstract Lean decision proofs and their extraction premise.

Primary language facts: [Microsoft sized integer types](https://learn.microsoft.com/en-us/cpp/cpp/int8-int16-int32-int64?view=msvc-170)
and [default C calling convention](https://learn.microsoft.com/en-us/cpp/cpp/cdecl?view=msvc-170).
