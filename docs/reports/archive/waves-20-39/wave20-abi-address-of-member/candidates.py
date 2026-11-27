#!/usr/bin/env python3
"""Card ABI-TYPEDEF-ADDRESS-OF-MEMBER — pre-registration of the candidate set.

Enumerates, over the pinned corpus sources, every call statement of a
*contracted* callee that carries an `& BASE -> FIELD` actual. That is a strict
over-approximation of what this card's widening can newly admit: the widening
only ever moves such an actual out of the lexical refusal, and only at a
declared `Borrowed` position whose member type this reader newly commits to.

Read-only. Run before the corpus measurement, so that the measurement is
allowed to contradict the prediction.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]


def contracted_callees() -> list[str]:
    manifest = json.loads((ROOT / "contract" / "cycle19-resource-contracts.json").read_text())
    names: list[str] = []
    for module in manifest["modules"]:
        text = (ROOT / "contract" / module).read_text()
        names.extend(re.findall(r"^extern fn ([A-Za-z_][A-Za-z0-9_]*)", text, re.M))
    return sorted(set(names))


def main() -> int:
    roots = [pathlib.Path(argument) for argument in sys.argv[1:]]
    if not roots:
        print("usage: candidates.py <source-root> [<source-root> …]")
        return 2
    callees = contracted_callees()
    print(f"contracted callees: {len(callees)}")
    pattern = re.compile(
        r"\b(" + "|".join(re.escape(name) for name in callees) + r")\s*\(([^;]{0,400}?)\)\s*;"
    )
    member_address = re.compile(r"&\s*[A-Za-z_][A-Za-z0-9_]*\s*->\s*[A-Za-z_][A-Za-z0-9_]*")
    hits: dict[str, list[tuple[str, int, str]]] = {}
    for root in roots:
        for path in sorted(root.rglob("*.c")):
            try:
                text = path.read_text(errors="replace")
            except OSError:
                continue
            for line_number, line in enumerate(text.splitlines(), 1):
                for match in pattern.finditer(line):
                    if member_address.search(match.group(2)):
                        hits.setdefault(match.group(1), []).append(
                            (str(path.relative_to(root)), line_number, line.strip())
                        )
    total = 0
    for callee in sorted(hits):
        rows = hits[callee]
        total += len(rows)
        print(f"\n{callee}: {len(rows)} call statement(s) with an `&BASE->FIELD` actual")
        for unit, line_number, text in rows:
            print(f"  {unit}:{line_number}  {text}")
    print(f"\ntotal candidate statements: {total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
