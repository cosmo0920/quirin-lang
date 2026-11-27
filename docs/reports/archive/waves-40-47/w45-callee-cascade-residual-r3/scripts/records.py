#!/usr/bin/env python3
"""W45-CALLEE-CASCADE-RESIDUAL round 3: the increment paragraph (newest first in
docs/development/increments/wave-48-part6.md) and the STATUS rows, from the
card's primary evidence (README sections 1-6). Idempotent."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = "reports/w45-callee-cascade-residual-r3/README.md"
MARK = "W45-CALLEE-CASCADE-RESIDUAL round 3"

PARAGRAPH = (
    "**`W45-CALLEE-CASCADE-RESIDUAL` round 3 (2026-10-11, base `ef6b32192`; prediction "
    "`3eba9abbc`).** Contracts for the callees that each free two or more, on the user's "
    "request \"Fill the gap of absence of contracts\" ([report](../../" + REPORT + ")); no "
    "analyzer code, gateway rule or Lean. The BEFORE reproduces (corpus `ded14632…` 915 / 9 / "
    "3,910; population 506 blocked by uncontracted calls alone, 352 by one callee alone). "
    "Fourteen pinned declarations, each body read at `66910c10` and `046accf6`: five Admin-API "
    "destructors and `mk_fifo_queue_destroy` (released), `unit_test_telemetry` (a synchronous "
    "callback invocation), `cfl_list_size`, `flb_hash_table_del`, and five 1-tier callees whose "
    "home body is Safe and validates them (`rd_bytes_hash`, `rd_kafka_mem_calloc`, "
    "`cio_file_native_get_size`, `cio_stream_size_chunks_up`, `cmt_atomic_initialize`). Corpus "
    "**+18 Unknown → Safe** (`cdb0267b…` 933 / 9 / 3,892), system +2 (93 / 4 / 351), slice +2 "
    "(52 / 1 / 347 / 4 Pending); no Safe → non-Safe move, no leak or violation, null property "
    "unchanged; Safe rows resting only on validated declarations 101 → 108. Two declarations "
    "were measured and withdrawn: `cio_file_hash` (its own row Safe → Unknown on a W4 summary "
    "conflict) and cJSON's static `print` (the compile route reads `print (` as a `.qr` print "
    "call). Remaining 481 / 329; the 26 callees still deciding 2 or more are decisions or "
    "reader extensions. Pins: loader v1 379 → 393, retaining 44 → 45, fourteen clause rows. "
    "Follow-ups 760 → 763 (amended `W45-CALLEE-CASCADE-RESIDUAL`, "
    "`W44-CALLEE-DESTRUCTOR-SUMMARY`, `W48-CALLEE-DECISION-TABLE`, "
    "`W43-LIBC-ABI-READER-SHAPES`; registered `QR-PRINT-ROUTE-MATCHES-DECLARATION`, "
    "`CALLBACK-STORED-IN-FRAME-MAP`, `BORROWED-RESULT-SUMMARY-CONFLICT`)."
)

STATUS = {
    "| Projected functions | ": (
        "**933 Safe / 9 provisional Unsafe / 3,892 Unknown**, 4,834 total (" + MARK + ", "
        "fourteen pinned declarations, arbiter on main `ef6b32192` plus the card: `ded14632…` → "
        "`cdb0267b…`, **+18 Unknown → Safe**, no Safe → non-Safe move, no leak or violation "
        "added, null property unchanged; system 91 / 4 / 353 → **93 / 4 / 351**, slice 50 / 1 / "
        "349 / 4 Pending → **52 / 1 / 347 / 4 Pending** ([report](" + REPORT + ")); before it "
        "915 / 9 / 3,910: "
    ),
    "| Contract dependencies per verdict (rules 13f–13k) | ": (
        "Since " + MARK + " (corpus run a3, binary `551fa31b…`): **933 Safe rows: 397 rest on no "
        "declaration, 108 only on validated declarations, 24 on a partly-validated one, 24 only "
        "on a null premise, 380 on a declared assumption**; 359 declarations consulted (63 validated, 37 partly validated, "
        "258 declared assumptions, 1 held) ([roll-up](" + REPORT.replace("README.md", "artifacts/rollup-a3-corpus.json") + ")). Earlier: "
    ),
    "| Uncontracted-call refusals | ": (
        "After " + MARK + " (corpus on main `ef6b32192` plus the card): **481** Unknown "
        "functions are blocked by uncontracted calls alone, 329 by one callee alone (506 / 352 "
        "before it; system 27 / 19 → 26 / 18, slice 42 / 28 → 40 / 26); the 26 callees still "
        "deciding 2 or more corpus functions alone (57 functions) are decision classes or "
        "reader extensions ([ranking](" + REPORT.replace("README.md", "artifacts/rank-a3-corpus.json") + ")). "
    ),
    "| Contract corpus | ": (
        "**393** resource-v1 declarations by the loader pin since " + MARK + " (fourteen "
        "declarations appended to selected modules of seven crates; 379 → 393, retaining 44 → "
        "45, selection 61 and rejected 314 unchanged, pinned at `66910c10` and `046accf6`; "
        "[report](" + REPORT + ")). Before it: "
    ),
    "| Registered follow-ups | ": (
        "**763** (wave 48: " + MARK + " amends `W45-CALLEE-CASCADE-RESIDUAL`, "
        "`W44-CALLEE-DESTRUCTOR-SUMMARY`, `W48-CALLEE-DECISION-TABLE` and "
        "`W43-LIBC-ABI-READER-SHAPES`, and registers `QR-PRINT-ROUTE-MATCHES-DECLARATION`, "
        "`CALLBACK-STORED-IN-FRAME-MAP` and `BORROWED-RESULT-SUMMARY-CONFLICT`; before it 760). "
        "Earlier: "
    ),
}


def main():
    inc = REPO / "docs/development/increments/wave-48-part6.md"
    text = inc.read_text()
    if MARK not in text:
        anchor = "it is not extended further.\n\n"
        assert text.count(anchor) == 1
        text = text.replace(anchor, anchor + PARAGRAPH + "\n\n", 1)
        inc.write_text(text)
    status = REPO / "docs/STATUS.md"
    lines = status.read_text().split("\n")
    for prefix, insert in STATUS.items():
        matches = [i for i, line in enumerate(lines) if line.startswith(prefix)]
        assert len(matches) == 1, prefix
        i = matches[0]
        if MARK in lines[i]:
            continue
        lines[i] = prefix + insert + lines[i][len(prefix):]
    status.write_text("\n".join(lines))
    print("ok")


if __name__ == "__main__":
    main()
