#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: the STATUS rows this card owns.

Rewrites the head of the `Projected functions` cell (the card's AFTER, from
primary evidence: artifacts/totals-after.txt) and prepends the card's clause to
the `Registered follow-ups` cell. Idempotent (a row already naming the card is
left alone).
"""
import pathlib
import sys

CARD = "CONTRACT-NORETURN-AND-BORROWED-TAILS"
OLD_HEAD = ("| Projected functions | **648 Safe / 9 provisional Unsafe / 4,177 Unknown**, 4,834 total "
            "(W48-BOTTOM-UP-CALLEE-BODIES, ")
NEW_HEAD = (
    "| Projected functions | **649 Safe / 9 provisional Unsafe / 4,176 Unknown**, 4,834 total "
    f"({CARD}, contracts only, no rule, arbiter on main `ca3576dd4` plus the card: `bcc98d2d…` → "
    "`450a9e54…`, **+1 Unknown → Safe**, `cb_debug_chunk_complete` (Monkey; `printf`'s tail now "
    "`resource_variadic_borrowed_tail()`, so its live `buf` is a borrowed read); `rd_kafka_crash` "
    "declared `resource_no_return()` (93 statement path ends, every one resolved `noreturn`, in 66 "
    "functions, none freed alone) and the borrowed tail declared for `printf`, `fprintf`, `snprintf`, "
    "`mk_print` and `flb_log_print` (157 of 197 corpus tail leaves gone); no Safe → non-Safe move; one "
    "leak added inside the already provisional-Unsafe unit test "
    "`ut_sasl_oauthbearer_oidc_assertion_parse_from_file`, a real failure-arm leak by reading; system "
    "and slice no status move ([report](reports/contract-noreturn-and-borrowed-tails/README.md)); "
    "before it 648 / 9 / 4,177, W48-BOTTOM-UP-CALLEE-BODIES, "
)
FOLLOWUPS = "| Registered follow-ups | **696** (wave 48: "
FOLLOWUPS_NEW = (
    "| Registered follow-ups | **695** (wave 48: "
    f"{CARD} retires `CG21-UNDECLARED-NORETURN-CALLEES` and `CG26-BORROWED-TAIL-DECLARATIONS` and "
    "registers `CG26-TAIL-RESIDUAL-ACTUALS`: 696 → 695; "
)


def main() -> int:
    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".") / "docs/STATUS.md"
    text = path.read_text()
    if CARD in text:
        print("already applied")
        return 0
    for old, new in ((OLD_HEAD, NEW_HEAD), (FOLLOWUPS, FOLLOWUPS_NEW)):
        if text.count(old) != 1:
            print(f"anchor not found exactly once: {old[:60]}")
            return 1
        text = text.replace(old, new)
    path.write_text(text)
    print("applied")
    return 0


if __name__ == "__main__":
    sys.exit(main())
