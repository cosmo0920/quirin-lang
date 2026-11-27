#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: one-off update of the records after
the provider gate found the stale providers/tests pin
(PROVIDER-LOAD-PATH-VIEW-PIN-STALE registered; ledger 696 -> 697). Each
replacement must match exactly once.
"""
import pathlib
import sys

EDITS = {
    "docs/STATUS.md": [
        ("| Registered follow-ups | **696** (wave 48: CONTRACT-NORETURN-AND-BORROWED-TAILS retires "
         "`CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` and registers "
         "`CG26-TAIL-RESIDUAL-ACTUALS` and `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`: 696 → 696; ",
         "| Registered follow-ups | **697** (wave 48: CONTRACT-NORETURN-AND-BORROWED-TAILS retires "
         "`CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` and registers "
         "`CG26-TAIL-RESIDUAL-ACTUALS`, `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS` and "
         "`PROVIDER-LOAD-PATH-VIEW-PIN-STALE`: 696 → 697; "),
    ],
    "docs/development/increments/wave-48-part4.md": [
        ("Ledger 696 → 696: `CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` "
         "retired, `CG26-TAIL-RESIDUAL-ACTUALS` and `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS` registered.",
         "Ledger 696 → 697: `CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` "
         "retired, `CG26-TAIL-RESIDUAL-ACTUALS`, `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS` and "
         "`PROVIDER-LOAD-PATH-VIEW-PIN-STALE` (the provider gate's `providers/tests` step fails at the "
         "base: a resource-view count pinned at 58, measured 60 since two earlier cards) registered."),
    ],
    "docs/reports/contract-noreturn-and-borrowed-tails/README.md": [
        ("Ledger 696 → 696: both follow-ups are retired, and two residuals are\nregistered.",
         "Ledger 696 → 697: both follow-ups are retired, and three follow-ups are\nregistered."),
        ("**Retired** (`scripts/update_ledger.py`; 696 → 696 with the two\nregistrations):",
         "**Retired** (`scripts/update_ledger.py`; 696 → 697 with the three\nregistrations):"),
        ("- `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`: the 34 crash statements in 26\n"
         "  functions that keep their continuation (§3).",
         "- `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`: the 34 crash statements in 26\n"
         "  functions that keep their continuation (§3).\n"
         "- `PROVIDER-LOAD-PATH-VIEW-PIN-STALE`: the provider gate's\n"
         "  `providers/tests` failure, which is pre-existing at the base (§6)."),
    ],
}


def main() -> int:
    repo = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    for name, edits in EDITS.items():
        path = repo / name
        text = path.read_text()
        for old, new in edits:
            if text.count(old) != 1:
                print(f"{name}: anchor not found exactly once: {old[:70]}")
                return 1
            text = text.replace(old, new)
        path.write_text(text)
    print("applied")
    return 0


if __name__ == "__main__":
    sys.exit(main())
