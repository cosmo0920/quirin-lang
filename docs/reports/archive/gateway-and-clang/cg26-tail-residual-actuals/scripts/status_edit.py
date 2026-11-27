#!/usr/bin/env python3
"""CG26-TAIL-RESIDUAL-ACTUALS: the STATUS rows this card measures (corpus
headline, corpus diagnostics, registered follow-ups). Idempotent."""
import pathlib
import sys

path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "docs/STATUS.md")
text = path.read_text()
MARK = "CG26-TAIL-RESIDUAL-ACTUALS (gateway 12g"
if MARK in text:
    print("already applied")
    raise SystemExit(0)

edits = [
    (
        "| Projected functions | **649 Safe / 9 provisional Unsafe / 4,176 Unknown**, 4,834 total (",
        "| Projected functions | **649 Safe / 9 provisional Unsafe / 4,176 Unknown**, 4,834 total ("
        "CG26-TAIL-RESIDUAL-ACTUALS (gateway 12g, a trailing actual read from its Clang node), arbiter on "
        "main `cb3085365` plus the card: `450a9e54…` → `b864e7ba…`, **no status move**, 25 tail leaves "
        "removed in 15 Unknown functions; before it ",
    ),
    (
        "| Corpus diagnostics | CONTRACT-NORETURN-AND-BORROWED-TAILS (contracts only): ",
        "| Corpus diagnostics | CG26-TAIL-RESIDUAL-ACTUALS (gateway 12g): 25 of the 40 declared-tail "
        "refusals removed (`fprintf` 18, `snprintf` 5, `printf` 2) in 15 Unknown functions, no other leaf "
        "moves; 15 left: 6 casts (CAST-FORMALIZATION's, `CG26-TAIL-RESIDUAL-ACTUALS`), 3 identity "
        "conditionals (`CG26-TAIL-IDENTITY-CONDITIONALS`), 6 statements in two `rdhttp.c` tests rule 11w's "
        "fallback never matches (`CG26-TAIL-UNMATCHED-STATEMENTS`); system: `flb_sched_timer_cb_create`'s 2 "
        "removed, `flb_storage_quarantine_chunk`'s 2 casts left; tail-pause slice byte-identical "
        "([report](reports/cg26-tail-residual-actuals/README.md)). "
        "CONTRACT-NORETURN-AND-BORROWED-TAILS (contracts only): ",
    ),
    (
        "| Registered follow-ups | **696** (wave 48: ",
        "| Registered follow-ups | **698** (wave 48: CG26-TAIL-RESIDUAL-ACTUALS amends "
        "`CG26-TAIL-RESIDUAL-ACTUALS` to its cast residual (CAST-FORMALIZATION's) and registers "
        "`CG26-TAIL-IDENTITY-CONDITIONALS` and `CG26-TAIL-UNMATCHED-STATEMENTS`: 696 → 698; ",
    ),
]
for old, new in edits:
    if text.count(old) != 1:
        raise SystemExit(f"anchor not unique: {old[:60]}")
    text = text.replace(old, new)
path.write_text(text)
print("applied")
