# FFI: importing a C struct

`source.ffi.v1` (card `W48-QR-C-STRUCT-IMPORT`). A `.qr` file imports a C
struct from a real header and calls C functions that take or return it, through
contract functions marked `[| quant #c_ffi |]`.

| Program | What it shows |
|---|---|
| [header_sum.qr](header_sum.qr) | `struct qr_header` imported with four listed fields; a construction and field reads; a call by `const` pointer and a call returning the struct by value, both with declared outcomes; rendering through `std.fmt`/`std.io`/`std.logger` |
| [record_passing.qr](record_passing.qr) | `struct qr_point` imported with three listed fields (card `W48-QR-USABLE-RECORDS`): `.qr` functions taking and returning it **by value**, a listed field **written**, and the struct handed to real C by `const` pointer and by value |

The library is [qr_header.h](qr_header.h) and [qr_header.c](qr_header.c); the
two marked contracts are [contracts/qr_header.qv](contracts/qr_header.qv).
`record_passing.qr`'s library is [qr_point.h](qr_point.h) and
[qr_point.c](qr_point.c), with [contracts/qr_point.qv](contracts/qr_point.qv).

## The file is unsafe as a whole

`#import std.ffi.unsafe;` enables the FFI, and it marks the **whole file**
unsafe. That is not the `Unsafe` verdict: it is a named assumption, carried by
every result from the file, that the imported layouts are faithful and each
`#c_ffi` function behaves as its contract states. The best result is
"Safe under the unsafe-FFI assumption", never plain `Safe`, and the check line
says so, listing every imported struct and every marked function called.

## The layout is the compiler's

The listing names fields and their `.qr` types and nothing else. The recorded
compiler (the builder's `--recorded-target`, whose `compiler` and `triple` must
be the Clang that runs) reads the header, resolved against this file's own
directory; each listed field's type and offset are compared with what it
reports, and the listed order must be the layout order. `reserved` (a `char`
array) and `label` (a pointer) have no fixed-width equivalent and stay unlisted:
opaque bytes no statement can read or write. The emitted C `#include`s the real
header, never re-declares the struct, and asserts `sizeof`, every listed
`offsetof` and every listed member's type, so a header whose layout changes
after checking does not build.

## Output

The program prints, on standard output,

```text
40
0x02
42
true
```

and logs `[INFO] 30` on standard error. A declared outcome the library does not
produce exits 70 before the line that would have published it.

## Building and running

The contract selection is a builder-side input, so this program is checked and
emitted through the library API with `examples/ffi/contracts/qr_header.qv`
selected and the recorded target of the Clang that builds it. The C it calls
is linked from a pinned artifact (card `W48-QRLIB-LINKAGE`): the build compiles
`qr_header.c` into a static library, **generates** `qr_header.qrlib` from that
archive's own symbol table and the contract module, and the program binds it
with `extern library qr_header;`. The emitted C is compiled with
`-I examples/ffi` and linked with exactly the archive the manifest pins, after
its sha256 is checked again. From the command line:

```sh
quirin qrlib generate --library qr_header --version 0.1.0 --linkage static \
    --artifact libqr_header.a --header qr_header.h --contracts contracts/qr_header.qv \
    --recorded-target host.target --unwinding abort --allocator-custody closed \
    --thread-posture single_caller -o qr_header.qrlib
quirin build --recorded-target host.target --contracts contracts/qr_header.qv \
    --native-library qr_header.qrlib -o header_sum header_sum.qr
```

run from `examples/ffi` after building `libqr_header.a`; `quirin build`
regenerates the manifest and refuses a stale one.
`compiler/tests/ffi_struct_import.rs` does exactly that, at `-O0` and `-O2`
under `-fsanitize=undefined,integer` and at plain `-O2`, all
`-Wall -Wextra -Werror -pedantic`, and compares both streams with the reference
observation.

## Writing and passing an imported struct

`record_passing.qr` writes a listed field with `RECORD.FIELD = VALUE;` — a
value of exactly the field's type; an opaque member (`note`) is never written —
and declares two `.qr` functions that take the struct **by value**:
`with_weight` writes its copy and returns it, so the caller's `p` keeps its
weight, and `show_x` renders a field of the struct C returned. A `.qr`
function is checked at every call, with the values it is given; one nothing
calls, one that calls itself, and one taking a pointer are refused. Every
function of the file is under the same unsafe-FFI assumption, and a function
in another file of the bundle that does not itself import `std.ffi.unsafe`
refuses (`Q032`). `compiler/tests/usable_records.rs` builds it with
[qr_point.c](qr_point.c) under the three builds above:

<!-- transcript: record_passing stdout -->
```text
1
5
50
7
```
