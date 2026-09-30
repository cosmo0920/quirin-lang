#!/usr/bin/env python3
"""CG10-BOUND-UNTRACKED-ROOTS: the STATUS.md edits, applied once.

usage: update_status.py STATUS_MD

Figures are the card's AFTER on the repaired base (dd4348b4, byte-identical to
f477d6c0's): corpus 3a25c4e9 608 / 10 / 4,216, system f41b88cb 41 / 2 / 191,
tail-pause slice 6949e223 28 / 1 / 240 / 1 Pending (scripts/identity.py,
artifacts/after-repaired-identity.json).
"""

import sys

REPORT = "reports/cg10-bound-untracked-roots/README.md"


def replace_once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def main() -> int:
    path = sys.argv[1]
    text = open(path, encoding="utf-8").read()
    text = replace_once(
        text,
        "| Projected functions | **602 Safe / 10 provisional Unsafe / 4,222 Unknown**, 4,834 total (",
        "| Projected functions | **608 Safe / 10 provisional Unsafe / 4,216 Unknown**, 4,834 total "
        "(CG10-BOUND-UNTRACKED-ROOTS, gateway 11d: a typed-reader chain root the closure does not prove "
        "never bound (a pointer formal or automatic local, never address-taken) read as untracked where no "
        "path reaching the access bound it or passed a refusal, and rule 10k(3)'s pointer actual on such "
        "roots left to that flow reading; **+6 Unknown → Safe** (5 by the premise, among them "
        "`mk_fifo_queue_get` and `mk_plugin_cap`, W47-JOIN-AST-CONSERVATIVE-CLASSES's withdrawals, and "
        "`flb_input_chunk_drop_task_route` through the `flb_task_get_route_data` summary), no correction, no "
        "leak or violation added; of 7,455 root-refusal leaves 7,053 remain, 5,126 of them on a path an "
        "uncontracted call refused; 602 / 10 / 4,222 before it, byte-identical on the repaired base "
        "`4b984bab`; ",
    )
    text = replace_once(
        text,
        "**237 functions: 37 Safe / 2 provisional Unsafe / 195 Unknown / 3 not projected** at "
        "CG08-CASTS-AND-TYPE-QUERIES (",
        "**237 functions: 41 Safe / 2 provisional Unsafe / 191 Unknown / 3 not projected** at "
        "CG10-BOUND-UNTRACKED-ROOTS (gateway 11d: `sb_find_segregated_backlog_by_output_plugin_instance` "
        "and `flb_engine_get_chunk_backpressure_percent`, W47-JOIN-AST-CONSERVATIVE-CLASSES's system "
        "withdrawals, `engine_has_fluentbit_logs_input` and `flb_input_chunk_drop_task_route`: +4; "
        "37 / 2 / 195 before it); at CG08-CASTS-AND-TYPE-QUERIES (",
    )
    text = replace_once(
        text,
        "| Fluent Bit tail-pause slice (measurement, no concurrency verdict) | ",
        "| Fluent Bit tail-pause slice (measurement, no concurrency verdict) | **28 Safe / 1 provisional "
        "Unsafe / 240 Unknown / 1 Pending** functions at CG10-BOUND-UNTRACKED-ROOTS (gateway 11d: "
        "`get_collector` and its caller `flb_input_collector_running`: +2; 26 / 1 / 242 / 1 Pending "
        "before it, [report](" + REPORT + ")); the figures that follow are the older build's: ",
    )
    anchor = "| Partial field links (rule 11c) |"
    at = text.index(anchor)
    end = text.index("\n", at) + 1
    row = (
        "| Bound untracked roots (rule 11d) | a chain root the never-bound closure does not cover is read "
        "as untracked where it is eligible (a pointer formal or automatic local declared once, never "
        "address-taken), its binding absent or known non-null target-free, no place rooted at it bound, "
        "and no path reaching the access refused (`unmodeled`): every custody position on an untracked "
        "object and every uncontracted call refuses there (Lean `QCore.BoundUntracked`); a rule-10k(3) "
        "pointer actual on such roots is read only as untracked. Corpus root-refusal leaves 7,455 on 1,635 "
        "functions → 7,053 on 1,529 (5,126 on a refused path, 1,409 a nullable or tracked binding, 505 "
        "ineligible, 13 rooted); **+6 / +4 / +2** Safe on corpus / system / slice; CG11's identity-free "
        "stores +24 admitted | [CG10-BOUND-UNTRACKED-ROOTS](" + REPORT + ") |\n"
    )
    text = text[:end] + row + text[end:]
    text = replace_once(
        text,
        "| Registered follow-ups | **657** (wave 48: ",
        "| Registered follow-ups | **658** (wave 48: CG10-BOUND-UNTRACKED-ROOTS retires its own entry, "
        "registers `CG10-BOUND-ROOT-PATH-REFUSED` and `CG10-BOUND-ROOT-NULLABLE-OR-TRACKED`, and amends "
        "`CG11-UNRESOLVED-OWNER-POINTER-STORES`, `W44-CONDITION-CALL-INDIRECT-ARGUMENT` and "
        "`W47-JOIN-AST-CONSERVATIVE-CLASSES`: 657 → 658; ",
    )
    open(path, "w", encoding="utf-8").write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
