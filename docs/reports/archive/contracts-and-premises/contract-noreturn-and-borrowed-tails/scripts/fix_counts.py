#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: one-off correction of the records'
path-end counts after scripts/path_ends.py (92 statement path ends in 45
bodies; 34 further crash statements in 26 functions admitted with their
continuation) and the second registered residual (ledger 696 -> 696).
Each replacement must match exactly once.
"""
import pathlib
import sys

EDITS = {
    "docs/STATUS.md": [
        ("declared `resource_no_return()` (93 statement path ends, every one resolved `noreturn`, in 66 "
         "functions, none freed alone)",
         "declared `resource_no_return()` (92 statement path ends, every one resolved `noreturn`, none "
         "freeing a function; 34 more crash statements in 26 functions admitted with their continuation, "
         "`CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`)"),
        ("| Registered follow-ups | **695** (wave 48: CONTRACT-NORETURN-AND-BORROWED-TAILS retires "
         "`CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` and registers "
         "`CG26-TAIL-RESIDUAL-ACTUALS`: 696 → 695; ",
         "| Registered follow-ups | **696** (wave 48: CONTRACT-NORETURN-AND-BORROWED-TAILS retires "
         "`CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` and registers "
         "`CG26-TAIL-RESIDUAL-ACTUALS` and `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`: 696 → 696; "),
        ("leaves of crash statements whose member-chain `rk` actual is now read through the contract in "
         "force, and 7 unresolved identities removed, 21 unresolved identities added on those chain roots "
         "(66 functions)",
         "leaves of crash statements whose member-chain `rk` actual is now read through the contract in "
         "force (admitted with their continuation, not as path ends), and 7 unresolved identities removed, "
         "21 unresolved identities added on those chain roots (66 functions)"),
    ],
    "docs/development/increments/wave-48-part4.md": [
        ("93 `rd_kafka_crash` statements end their paths (all resolved `noreturn`) in 66 functions, none "
         "freed (the census had seen 40: 26 more refused the call earlier at its `rkb->rkb_rk` actual)",
         "92 `rd_kafka_crash` statements end their paths (all resolved `noreturn`), freeing no function; "
         "34 more, in 26 functions, used to be refused at their member-chain `rk` actual and are now "
         "admitted through rule 10k(3)'s typed call actual as ordinary calls with their continuation "
         "(sound, not a path end: `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`)"),
        ("Ledger 696 → 695: `CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` "
         "retired, `CG26-TAIL-RESIDUAL-ACTUALS` registered.",
         "Ledger 696 → 696: `CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` "
         "retired, `CG26-TAIL-RESIDUAL-ACTUALS` and `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS` registered."),
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
