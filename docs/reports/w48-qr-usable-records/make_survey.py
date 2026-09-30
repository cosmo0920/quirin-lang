#!/usr/bin/env python3
"""Write the Step-0 survey fixtures of card W48-QR-USABLE-RECORDS.

Usage (from the repository root):

    python3 docs/reports/w48-qr-usable-records/make_survey.py [--check]

One `.qr` file per (route, operation) cell under
`compiler/tests/fixtures/usable_records/survey/`, plus the C header the FFI
cells import and the contract the extern and FFI cells select. The fixtures
were frozen before any source change of the card; the survey test
(`compiler/tests/usable_records_survey.rs`) checks each one through the
library API with the contract selection and the recorded target the route
needs, and pins the result. `--check` rewrites nothing and fails if a file on
disk differs from what this script writes, so the fixtures and the generator
cannot drift.
"""

import sys
from pathlib import Path

OUT = Path("compiler/tests/fixtures/usable_records/survey")

RECORD = "type P { x: i32, y: i32 }\n"
NESTED = "type I { a: i32 }\ntype O { inner: I, n: i32 }\n"
OWNED = "type F { tag: i32, buf: bytes }\n"
VIEW = "type V { s: string }\n"

IO_HEAD = "#import std.fmt;\n#import std.io;\n"
BUF_HEAD = "#import std.buffer;\n#import std.fmt;\n#import std.io;\n"


def entry(effects: str, body: str, result: str = "unit", end: str = "    return;\n") -> str:
    return f"fn main() -> {result} effects {{ {effects} }} {{\n{body}{end}}}\n"


# ------------------------------------------------------------------ buffer
# `source.buffer.v1`, the one route `source.record.v1` is admitted on.
BUFFER = {
    "construct": IO_HEAD + RECORD + entry("io", "    let p = P { x: 1, y: 2 };\n"),
    "read": IO_HEAD + RECORD + entry(
        "io", "    let p = P { x: 1, y: 2 };\n    let a = p.x;\n    print dec a;\n"
    ),
    "write": IO_HEAD + RECORD + entry(
        "io",
        "    let p = P { x: 1, y: 2 };\n    p.x = 5;\n    let a = p.x;\n    print dec a;\n",
    ),
    "write_owned": BUF_HEAD + OWNED + entry(
        "io, alloc, mut",
        "    let b = bytes(2);\n    let f = F { tag: 1, buf: b };\n    let t = f.buf;\n"
        "    drop t;\n    let c = bytes(2);\n    f.buf = c;\n    drop f;\n",
    ),
    "write_released": BUF_HEAD + OWNED + entry(
        "io, alloc, mut",
        "    let b = bytes(2);\n    let f = F { tag: 1, buf: b };\n    drop f;\n    f.tag = 3;\n",
    ),
    "nested": IO_HEAD + NESTED + entry(
        "io",
        "    let i = I { a: 1 };\n    let o = O { inner: i, n: 2 };\n    let v = o.inner.a;\n"
        "    print dec v;\n",
    ),
    "param": IO_HEAD + RECORD
    + "fn get(p: P) -> i32 {\n    let v = p.x;\n    return v;\n}\n"
    + entry("io", "    let p = P { x: 1, y: 2 };\n    let v = get(p);\n    print dec v;\n"),
    "result": IO_HEAD + RECORD
    + "fn make() -> P {\n    let p = P { x: 1, y: 2 };\n    return p;\n}\n"
    + entry("io", "    let p = make();\n    let a = p.x;\n    print dec a;\n"),
    "copy": IO_HEAD + RECORD + entry(
        "io", "    let p = P { x: 1, y: 2 };\n    let q = p;\n    let a = q.x;\n    print dec a;\n"
    ),
    "move": BUF_HEAD + OWNED + entry(
        "io, alloc, mut",
        "    let b = bytes(2);\n    let f = F { tag: 1, buf: b };\n    let g = f;\n    drop g;\n",
    ),
    "loop": BUF_HEAD + RECORD + entry(
        "io, alloc, mut",
        "    let b = bytes(4);\n    for i in 0..2 {\n        let p = P { x: 1, y: 2 };\n"
        "        let a = p.x;\n        b[i] = a;\n    }\n    let v = b[0];\n    print dec v;\n"
        "    drop b;\n",
    ),
    "loop_owned": BUF_HEAD + OWNED + entry(
        "io, alloc, mut",
        "    let b = bytes(4);\n    let f = F { tag: 1, buf: b };\n    for i in 0..2 {\n"
        "        f.buf[i] = 7;\n    }\n    drop f;\n",
    ),
    "lend": BUF_HEAD + RECORD + entry(
        "io, alloc, mut",
        "    let b = bytes(4);\n    let p = P { x: 1, y: 2 };\n    let l = lend(b, 0, 2);\n"
        "    let a = p.x;\n    l[0] = a;\n    let v = b[0];\n    print dec v;\n    drop b;\n",
    ),
    "lend_owned": BUF_HEAD + OWNED + entry(
        "io, alloc, mut",
        "    let b = bytes(4);\n    let f = F { tag: 1, buf: b };\n    let c = bytes(4);\n"
        "    let l = lend(c, 0, 2);\n    l[0] = 1;\n    drop c;\n    drop f;\n",
    ),
    "view_field": IO_HEAD + VIEW + entry("io", ""),
}

# ------------------------------------------------------------------ scalar
# `source.scalar-calls.v1`: the module also calls a scalar helper.
HELPER = "fn twice(v: i32) -> i32 {\n    return v * 2;\n}\n"


def scalar(decls: str, body: str, head: str = IO_HEAD, effects: str = "io") -> str:
    return head + decls + HELPER + entry(effects, body + "    let t = twice(3);\n    print dec t;\n")


SCALAR = {
    "construct": scalar(RECORD, "    let p = P { x: 1, y: 2 };\n"),
    "read": scalar(RECORD, "    let p = P { x: 1, y: 2 };\n    let a = p.x;\n    print dec a;\n"),
    "write": scalar(RECORD, "    let p = P { x: 1, y: 2 };\n    p.x = 5;\n"),
    "nested": scalar(NESTED, "    let i = I { a: 1 };\n    let o = O { inner: i, n: 2 };\n"),
    "param": IO_HEAD + RECORD + HELPER
    + "fn get(p: P) -> i32 {\n    let v = p.x;\n    return v;\n}\n"
    + entry("io", "    let p = P { x: 1, y: 2 };\n    let v = get(p);\n    let t = twice(v);\n    print dec t;\n"),
    "result": IO_HEAD + RECORD + HELPER
    + "fn make() -> P {\n    let p = P { x: 1, y: 2 };\n    return p;\n}\n"
    + entry("io", "    let p = make();\n    let a = p.x;\n    let t = twice(a);\n    print dec t;\n"),
    "copy": scalar(RECORD, "    let p = P { x: 1, y: 2 };\n    let q = p;\n"),
    "move": scalar(OWNED, ""),
    "loop": scalar(RECORD, "    for i in 0..2 {\n        let p = P { x: 1, y: 2 };\n    }\n"),
    "lend": scalar(
        RECORD,
        "    let p = P { x: 1, y: 2 };\n    let b = bytes(4);\n    let l = lend(b, 0, 2);\n"
        "    l[0] = 1;\n    drop b;\n",
        BUF_HEAD,
        "io, alloc, mut",
    ),
}

# ------------------------------------------------------------------ extern
# `source.extern.v1`, with `survey_scale.qv` selected.
EXTERN_HEAD = "#import std.extern;\n#import std.logger;\n"
EXTERN_DECL = "extern fn survey_scale(value: i32, factor: i32) -> i32;\n"
CALL = "    let s = survey_scale(3, 2) succeeds 6;\n"


def extern(decls: str, body: str) -> str:
    return EXTERN_HEAD + decls + EXTERN_DECL + entry("io", body + CALL)


EXTERN = {
    "construct": extern(RECORD, "    let p = P { x: 1, y: 2 };\n"),
    "read": extern(RECORD, "    let p = P { x: 1, y: 2 };\n    let a = p.x;\n"),
    "write": extern(RECORD, "    let p = P { x: 1, y: 2 };\n    p.x = 5;\n"),
    "nested": extern(NESTED, "    let i = I { a: 1 };\n    let o = O { inner: i, n: 2 };\n"),
    "param": EXTERN_HEAD + RECORD + EXTERN_DECL
    + "fn get(p: P) -> i32 {\n    let v = p.x;\n    return v;\n}\n"
    + entry("io", "    let p = P { x: 1, y: 2 };\n    let v = get(p);\n" + CALL),
    "result": EXTERN_HEAD + RECORD + EXTERN_DECL
    + "fn make() -> P {\n    let p = P { x: 1, y: 2 };\n    return p;\n}\n"
    + entry("io", "    let p = make();\n" + CALL),
    "copy": extern(RECORD, "    let p = P { x: 1, y: 2 };\n    let q = p;\n"),
    "move": extern(OWNED, ""),
    "loop": extern(RECORD, "    for i in 0..2 {\n        let p = P { x: 1, y: 2 };\n    }\n"),
    "lend": "#import std.buffer;\n" + EXTERN_HEAD + RECORD + EXTERN_DECL + entry(
        "io, alloc, mut",
        "    let p = P { x: 1, y: 2 };\n    let b = bytes(4);\n    let l = lend(b, 0, 2);\n"
        "    l[0] = 1;\n    drop b;\n" + CALL,
    ),
}

# ------------------------------------------------------------------ task
TASK_HEAD = (
    "#import std.concurrency;\n#import std.fmt;\n#import std.io;\nconcurrency checked;\n"
)
WORKER = "fn work(a: i32) -> i32 {\n    return a;\n}\n"
TASK_EFFECTS = "throws<SpawnError>, spawn, join, may_block, io"
SCOPE = (
    "    thread scope s {\n        let h = spawn s work(copy a)?;\n        let r = join move h?;\n"
    "        print dec r;\n        return 0;\n    }\n"
)


def task(decls: str, body: str, worker: str = WORKER) -> str:
    return TASK_HEAD + decls + worker + entry(TASK_EFFECTS, body + "    let a = 1;\n" + SCOPE, "i32", "")


TASK = {
    "construct": task(RECORD, "    let p = P { x: 1, y: 2 };\n"),
    "read": task(RECORD, "    let p = P { x: 1, y: 2 };\n    let z = p.x;\n"),
    "write": task(RECORD, "    let p = P { x: 1, y: 2 };\n    p.x = 5;\n"),
    "nested": task(NESTED, "    let i = I { a: 1 };\n"),
    "param": TASK_HEAD + RECORD
    + "fn work(p: P) -> i32 {\n    let v = p.x;\n    return v;\n}\n"
    + entry(
        TASK_EFFECTS,
        "    let p = P { x: 1, y: 2 };\n    thread scope s {\n        let h = spawn s work(copy p)?;\n"
        "        let r = join move h?;\n        print dec r;\n        return 0;\n    }\n",
        "i32",
        "",
    ),
    "result": task(RECORD, "", "fn work(a: i32) -> P {\n    let p = P { x: a, y: a };\n    return p;\n}\n"),
    "copy": task(RECORD, "    let p = P { x: 1, y: 2 };\n    let q = p;\n"),
    "move": task(OWNED, ""),
    "loop": task(RECORD, "    for i in 0..2 {\n        let p = P { x: 1, y: 2 };\n    }\n"),
    "lend": task(RECORD, "    let p = P { x: 1, y: 2 };\n"),
}

# ------------------------------------------------------------------ ffi
FFI_HEAD = "#import std.ffi.unsafe;\n#import std.fmt;\n#import std.io;\n"
IMPORT = (
    'struct! {\n    type R = c"struct survey_rec" from "survey_rec.h";\n'
    "    fields { len: u32, flags: u8 }\n}\n"
)
IMPORT_OUTER = (
    'struct! {\n    type Outer = c"struct survey_outer" from "survey_rec.h";\n'
    "    fields { inner: R, n: u32 }\n}\n"
)
CONS = "    let r = R { len: 4, flags: 1 };\n"


def ffi(decls: str, body: str) -> str:
    return FFI_HEAD + IMPORT + decls + entry("io", body)


FFI = {
    "construct": ffi("", CONS),
    "read": ffi("", CONS + "    let l = r.len;\n    print dec l;\n"),
    "write": ffi("", CONS + "    r.len = 9;\n    let l = r.len;\n    print dec l;\n"),
    "nested": ffi(IMPORT_OUTER, CONS),
    "param": ffi(
        "fn total(h: R) -> u32 {\n    let l = h.len;\n    return l;\n}\n",
        CONS + "    let t = total(r);\n    print dec t;\n",
    ),
    "result": ffi(
        "fn make() -> R {\n    let r = R { len: 4, flags: 1 };\n    return r;\n}\n",
        "    let r = make();\n    let l = r.len;\n    print dec l;\n",
    ),
    "copy": ffi("", CONS + "    let q = r;\n    let l = q.len;\n    print dec l;\n"),
    "move": FFI_HEAD
    + 'struct! {\n    type R = c"struct survey_rec" from "survey_rec.h";\n'
    "    fields { len: u32, flags: u8, buf: bytes }\n}\n"
    + entry("io", ""),
    "loop": ffi("", "    for i in 0..2 {\n        let r = R { len: 4, flags: 1 };\n    }\n"),
    "lend": FFI_HEAD.replace("#import std.fmt;", "#import std.buffer;\n#import std.fmt;")
    + IMPORT
    + entry("io, alloc, mut", CONS + "    let b = bytes(4);\n    let l = lend(b, 0, 2);\n    drop b;\n"),
}

HEADER = """/* The C struct the FFI survey cells of card W48-QR-USABLE-RECORDS import. */
#ifndef SURVEY_REC_H
#define SURVEY_REC_H

#include <stdint.h>

struct survey_rec {
    uint32_t len;
    uint8_t flags;
};

struct survey_outer {
    struct survey_rec inner;
    uint32_t n;
};

#endif
"""

CONTRACT = """module contract;

// The scalar contract the extern survey cells of card W48-QR-USABLE-RECORDS
// select: scalar in, scalar out, no custody in any position.
extern fn survey_scale(value: i32, factor: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(value);
    ensures normal resource_borrowed(factor);
    ensures normal resource_return_none();
};
"""


def files() -> dict[str, str]:
    out = {"survey_rec.h": HEADER, "survey_scale.qv": CONTRACT}
    for route, table in (
        ("buffer", BUFFER),
        ("scalar", SCALAR),
        ("extern", EXTERN),
        ("task", TASK),
        ("ffi", FFI),
    ):
        for op, text in table.items():
            out[f"{route}_{op}.qr"] = (
                f"// Survey cell ({route}, {op}) of card W48-QR-USABLE-RECORDS; written by\n"
                f"// docs/reports/w48-qr-usable-records/make_survey.py.\n" + text
            )
    return out


def main() -> int:
    check = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        print(__doc__, file=sys.stderr)
        return 2
    drift = []
    OUT.mkdir(parents=True, exist_ok=True)
    for name, text in sorted(files().items()):
        path = OUT / name
        if check:
            if not path.exists() or path.read_text() != text:
                drift.append(name)
        else:
            path.write_text(text)
    if drift:
        print("drifted: " + ", ".join(drift), file=sys.stderr)
        return 1
    print(f"{'checked' if check else 'wrote'} {len(files())} files under {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
