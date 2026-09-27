#!/usr/bin/env python3
"""Measure the *reach* of the owned-field rules: which `.qcore` shape produces
each ownership operation, and what the real pipeline answers today.

The card's spine. Run from the repository root:

    cargo build -p qcorec
    python3 docs/reports/owned-field-route-reach/scripts/reach.py . --out measure/reach

Each probe is a whole `.qcore` program written to the output directory and
handed to the built `quirin check` binary. Nothing is inferred from source
reading: the table's "what the pipeline answers" column is this run's exit code
and first diagnostic code, captured to files, never through a pipe.

`--expect <json>` compares the run against a recorded expectation file and exits
non-zero on any disagreement, so the table is reproducible rather than retyped.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

#: Every probe: id, the rule it is about, and the program.
PROBES: list[tuple[str, str, str]] = []


def probe(pid: str, rule: str, body: str) -> None:
    PROBES.append((pid, rule, body))


PRELUDE = "#import std.buffer;\n#import std.fmt;\n#import std.io;\n\n"


def program(decls: str, body: str) -> str:
    return (
        PRELUDE
        + decls
        + "fn main() -> unit effects { io, alloc, mut } {\n"
        + body
        + "    return;\n}\n"
    )


OWNING = "type Frame { tag: i32, body: bytes }\n\n"
SCALARS = "type Point { x: i32, y: i32 }\n\n"

# --- rule 1: construction moves the buffer in -------------------------------
probe(
    "r1-admitted",
    "construction moves in",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    drop f;\n",
    ),
)
probe(
    "r1-refused-use-after-move",
    "construction moves in",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    b[0] = 1;\n"
        "    drop f;\n",
    ),
)

# --- rule 2: a partial move leaves a hole ----------------------------------
probe(
    "r2-admitted",
    "partial move leaves a hole",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    let taken = f.body;\n"
        "    drop f;\n"
        "    drop taken;\n",
    ),
)
probe(
    "r2-refused-second-read",
    "partial move leaves a hole",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    let taken = f.body;\n"
        "    let again = f.body;\n"
        "    drop f;\n"
        "    drop taken;\n",
    ),
)

# --- rule 3: drop-through in declaration order ------------------------------
probe(
    "r3-admitted",
    "drop-through in declaration order",
    program(
        "type Pair { first: bytes, second: bytes }\n\n",
        "    let a = bytes(2);\n"
        "    let c = bytes(2);\n"
        "    let p = Pair { first: a, second: c };\n"
        "    drop p;\n",
    ),
)
probe(
    "r3-refused-double-drop",
    "drop-through in declaration order",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    drop f;\n"
        "    drop f;\n",
    ),
)

# --- rule 4: leak at exit names the path ------------------------------------
probe(
    "r4-refused-leak",
    "leak at exit names the path",
    program(
        OWNING,
        "    let b = bytes(4);\n    let f = Frame { tag: 7, body: b };\n",
    ),
)

# --- rule 5: non-copyability -----------------------------------------------
probe(
    "r5-copy-owning-record",
    "non-copyability",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    let g = f;\n"
        "    drop f;\n",
    ),
)
probe(
    "r5-copy-hollow-record",
    "non-copyability (hollow)",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    let taken = f.body;\n"
        "    let g = f;\n"
        "    drop f;\n"
        "    drop taken;\n",
    ),
)
probe(
    "r5-copy-buffer",
    "non-copyability (plain owner)",
    program(
        "",
        "    let b = bytes(4);\n    let c = b;\n    drop b;\n",
    ),
)
probe(
    "r5-copy-scalar-record",
    "copyability of a scalar record",
    program(
        SCALARS,
        "    let p = Point { x: 1, y: 2 };\n"
        "    let q = p;\n"
        "    let a = q.x;\n"
        "    a |> print dec;\n",
    ),
)
probe(
    "r5-copy-scalar",
    "copyability of a scalar on the buffer route",
    program(
        "",
        "    let b = bytes(2);\n"
        "    let n = b[0];\n"
        "    let m = n;\n"
        "    m |> print dec;\n"
        "    drop b;\n",
    ),
)
probe(
    "r5-copy-shadowing",
    "a copy bind whose name is already live",
    program(
        SCALARS,
        "    let b = bytes(2);\n"
        "    let p = Point { x: 1, y: 2 };\n"
        "    let p = p;\n"
        "    drop b;\n",
    ),
)
probe(
    "r5-copy-self",
    "a copy bind naming itself",
    program(
        "",
        "    let b = bytes(2);\n    let q = q;\n    drop b;\n",
    ),
)
probe(
    "r5-copy-composed-string",
    "a copy bind on the composed string/buffer route",
    "#import std.buffer;\n#import std.string;\n#import std.fmt;\n#import std.io;\n\n"
    "fn main() -> unit effects { io, alloc, mut } {\n"
    "    let b = bytes(2);\n"
    "    b[0] = 104;\n"
    "    b[1] = 105;\n"
    "    let s = borrow_utf8(b, 0, 2);\n"
    "    let t = s;\n"
    "    drop b;\n"
    "    return;\n}\n",
)
probe(
    "r5-copy-moved-record",
    "non-copyability after the whole record is dropped",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    drop f;\n"
        "    let g = f;\n",
    ),
)
probe(
    "r5-copy-loan",
    "a write loan at a copy position",
    program(
        "",
        "    let b = bytes(4);\n"
        "    let l = lend(b, 0, 2);\n"
        "    let m = l;\n"
        "    drop b;\n",
    ),
)
probe(
    "r5-copy-unbound",
    "a name this entry never bound at a copy position",
    program("", "    let q = nope;\n"),
)

# --- the shape collision the census found ----------------------------------
#
# `source.socket.v1`'s creation form `let listener = tcp;` is token for token
# the copy bind's shape, and the census counts 29 of them in the tree. The two
# are separated by **route selection** and by nothing else, so this probe is
# the measurement rather than the argument.
probe(
    "collide-socket-creation",
    "a socket program's `let NAME = tcp;` is untouched",
    "#import std.socket;\n#import std.fmt;\n#import std.io;\n\n"
    "fn main() -> unit effects { io, alloc } {\n"
    "    let listener = tcp;\n"
    "    bind listener;\n"
    "    listen listener;\n"
    "    let client = tcp;\n"
    "    connect client listener;\n"
    "    let server = accept listener;\n"
    "    send client [81];\n"
    "    recv server 1;\n"
    "    close server;\n"
    "    close client;\n"
    "    close listener;\n"
    "    return;\n}\n",
)
probe(
    "collide-buffer-route-transport-word",
    "a transport word at a copy source on the buffer route",
    program(
        "",
        "    let b = bytes(2);\n    let c = tcp;\n    drop b;\n",
    ),
)

# --- the three named boundaries --------------------------------------------
probe(
    "b-view-field",
    "AGGREGATE-VIEW-FIELD",
    program("type Frame { s: string }\n\n", "    let n = 1;\n    n |> print dec;\n"),
)
probe(
    "b-nested-declared",
    "AGGREGATE-NESTED (field naming a declared record type)",
    program(
        "type Inner { x: i32 }\ntype Outer { inner: Inner }\n\n",
        "    let n = 1;\n    n |> print dec;\n",
    ),
)
probe(
    "b-nested-forward",
    "AGGREGATE-NESTED (field naming a type declared after it)",
    program(
        "type Outer { inner: Inner }\ntype Inner { x: i32 }\n\n",
        "    let n = 1;\n    n |> print dec;\n",
    ),
)
probe(
    "b-unknown-field-type",
    "AGGREGATE-FIELD-TYPES (a word this revision models no type for)",
    program("type Frame { x: u8 }\n\n", "    let n = 1;\n    n |> print dec;\n"),
)
probe(
    "b-task-capture",
    "AGGREGATE-TASK-CAPTURE",
    "#import std.concurrency;\n"
    "concurrency checked;\n\n"
    "type Frame { body: bytes }\n\n"
    "fn worker(buffer: own<Buffer> use one) -> own<Buffer> {\n"
    "    return move buffer;\n}\n\n"
    "fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, "
    "may_block } {\n"
    "    let first = qcore_buffer_new(42)?;\n"
    "    thread scope workers {\n"
    "        match spawn workers worker(move first) {\n"
    "            Result::Ok { value: handle } => {\n"
    "                let returned = join move handle?;\n"
    "                let value = qcore_buffer_read(returned);\n"
    "                drop returned;\n"
    "                return value;\n"
    "            }\n"
    "            Result::Err { error: failure } => {\n"
    "                let value = qcore_buffer_read(first);\n"
    "                drop first;\n"
    "                return value;\n"
    "            }\n"
    "        }\n"
    "    }\n}\n",
)
probe(
    "b-task-capture-baseline",
    "AGGREGATE-TASK-CAPTURE (the same task module without the record)",
    "#import std.concurrency;\n"
    "concurrency checked;\n\n"
    "fn worker(buffer: own<Buffer> use one) -> own<Buffer> {\n"
    "    return move buffer;\n}\n\n"
    "fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, "
    "may_block } {\n"
    "    let first = qcore_buffer_new(42)?;\n"
    "    thread scope workers {\n"
    "        match spawn workers worker(move first) {\n"
    "            Result::Ok { value: handle } => {\n"
    "                let returned = join move handle?;\n"
    "                let value = qcore_buffer_read(returned);\n"
    "                drop returned;\n"
    "                return value;\n"
    "            }\n"
    "            Result::Err { error: failure } => {\n"
    "                let value = qcore_buffer_read(first);\n"
    "                drop first;\n"
    "                return value;\n"
    "            }\n"
    "        }\n"
    "    }\n}\n",
)

# --- the IR-only refusals ---------------------------------------------------
probe(
    "ir-aggregate-at-owner-lend",
    "AggregateAtOwnerPosition via a lend of a record",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    let l = lend(f, 0, 2);\n"
        "    drop f;\n",
    ),
)
probe(
    "ir-aggregate-at-owner-index",
    "AggregateAtOwnerPosition via a byte access on a record",
    program(
        OWNING,
        "    let b = bytes(4);\n"
        "    let f = Frame { tag: 7, body: b };\n"
        "    f[0] = 1;\n"
        "    drop f;\n",
    ),
)
probe(
    "ir-not-an-aggregate",
    "NotAnAggregate via a field path on a plain buffer",
    program(
        "",
        "    let b = bytes(4);\n    let x = b.field;\n    drop b;\n",
    ),
)

CODE = re.compile(r"\b(Q\d{3})\b")


def run(root: Path, out: Path, binary: Path) -> list[dict]:
    out.mkdir(parents=True, exist_ok=True)
    rows = []
    for pid, rule, body in PROBES:
        src = out / f"{pid}.qcore"
        src.write_text(body, encoding="utf-8")
        stdout = out / f"{pid}.out"
        stderr = out / f"{pid}.err"
        exitf = out / f"{pid}.exit"
        with stdout.open("wb") as o, stderr.open("wb") as e:
            code = subprocess.call(
                [str(binary), "check", str(src)], cwd=str(root), stdout=o, stderr=e
            )
        exitf.write_text(f"{code}\n", encoding="utf-8")
        text = stdout.read_text(encoding="utf-8", errors="replace") + stderr.read_text(
            encoding="utf-8", errors="replace"
        )
        codes = CODE.findall(text)
        first = codes[0] if codes else None
        rows.append(
            {
                "id": pid,
                "rule": rule,
                "exit": code,
                "code": first,
                "message": first_message(text),
            }
        )
    return rows


def first_message(text: str) -> str:
    for line in text.splitlines():
        line = line.strip()
        if CODE.search(line):
            return line
    return text.strip().splitlines()[0] if text.strip() else ""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("--out", default="measure/reach")
    ap.add_argument("--binary", default="target/debug/quirin")
    ap.add_argument("--expect")
    ap.add_argument("--write-expect")
    args = ap.parse_args()

    root = Path(args.root).resolve()
    binary = (root / args.binary).resolve()
    if not binary.exists():
        print(f"binary not built: {binary}", file=sys.stderr)
        return 2
    out = (root / args.out).resolve()
    rows = run(root, out, binary)
    (out / "reach.json").write_text(
        json.dumps(rows, indent=2) + "\n", encoding="utf-8"
    )

    width = max(len(r["id"]) for r in rows)
    lines = []
    for r in rows:
        verdict = "ADMITTED" if r["exit"] == 0 else f"refused {r['code'] or '?'}"
        lines.append(f"{r['id']:<{width}}  {verdict:<14}  {r['rule']}")
        if r["exit"] != 0:
            lines.append(f"{'':<{width}}  {'':<14}  > {r['message']}")
    table = "\n".join(lines) + "\n"
    (out / "reach.txt").write_text(table, encoding="utf-8")
    print(table)

    if args.write_expect:
        Path(args.write_expect).write_text(
            json.dumps(
                [{"id": r["id"], "exit": r["exit"], "code": r["code"]} for r in rows],
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
    if args.expect:
        want = {e["id"]: e for e in json.loads(Path(args.expect).read_text())}
        bad = []
        for r in rows:
            e = want.get(r["id"])
            if e is None:
                bad.append(f"{r['id']}: no expectation recorded")
            elif e["exit"] != r["exit"] or e["code"] != r["code"]:
                bad.append(
                    f"{r['id']}: expected exit {e['exit']} code {e['code']}, "
                    f"got exit {r['exit']} code {r['code']}"
                )
        for name in want:
            if not any(r["id"] == name for r in rows):
                bad.append(f"{name}: expected probe absent from this run")
        if bad:
            print("REACH MISMATCH", file=sys.stderr)
            for b in bad:
                print("  " + b, file=sys.stderr)
            return 1
        print(f"reach: {len(rows)} probes agree with {args.expect}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
