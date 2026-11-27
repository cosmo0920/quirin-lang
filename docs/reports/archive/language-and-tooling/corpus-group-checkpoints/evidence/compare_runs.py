#!/usr/bin/env python3
"""Byte comparison of two pinned corpus run directories (card
CORPUS-GROUP-CHECKPOINTS evidence). Every published artifact is compared
byte for byte; the console is compared after replacing the run's own output
directory (the last line names the report path). Exit 0 when all equal."""
import hashlib
import pathlib
import sys

ARTIFACTS = [
    "resource-diagnostics.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.source-effects.json",
    "resource-diagnostics.components.json",
    "monkey-source.json",
    "corpus-null.jsonl",
    "corpus-null.summary.json",
    "corpus-outer.json",
]


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else "absent"


def main() -> int:
    left, right = (pathlib.Path(arg).resolve() for arg in sys.argv[1:3])
    equal = True
    names = sorted({p.name for p in left.iterdir()} | {p.name for p in right.iterdir()})
    extra = [n for n in names if n not in ARTIFACTS and n not in (
        "console.log", "stderr.log", "slot.json", "slot-wait.json", "time.txt", "exit.txt")]
    for name in ARTIFACTS:
        a, b = digest(left / name), digest(right / name)
        same = a == b and a != "absent"
        equal &= same
        print(f"{'same' if same else 'DIFF'} {name} {a[:16]} {b[:16]}")
    consoles = []
    for side in (left, right):
        text = (side / "console.log").read_text()
        consoles.append(text.replace(str(side), "<OUT>"))
    same = consoles[0] == consoles[1]
    equal &= same
    print(f"{'same' if same else 'DIFF'} console.log (output directory masked) "
          f"{hashlib.sha256(consoles[0].encode()).hexdigest()[:16]} "
          f"{hashlib.sha256(consoles[1].encode()).hexdigest()[:16]}")
    if extra:
        print(f"other files: {extra}")
    print("ALL EQUAL" if equal else "DIFFERENCES")
    return 0 if equal else 1


if __name__ == "__main__":
    raise SystemExit(main())
