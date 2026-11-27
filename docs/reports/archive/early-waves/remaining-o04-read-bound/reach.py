#!/usr/bin/env python3
"""Why card O04-READ-BOUND's corpus reach is zero, measured rather than argued.

Read-only. Two measurements over the pinned inputs:

1. **Contract side.** Of the contracted functions the pinned corpus manifest
   puts in force, how many declare a `Borrowed` parameter whose declared C type
   is a *pointer*? Only such a position can ever receive an admitted bound read:
   a scalar member is refused by the member premise, and every custody-bearing
   position is refused by `O04-READ-BOUND-CUSTODY`.

2. **Source side.** For each such (function, parameter index), how many call
   sites in the pinned clone pass a single `->` selection as that whole
   argument? That is a strict over-count of what could fire — it ignores the
   base premises, the member type, the field link and the liveness gate — so a
   small number here bounds the reach from above.

    python3 -B docs/reports/remaining-o04-read-bound/reach.py \\
        --contracts build/cycle19/reports-before/resource-diagnostics.contracts.json \\
        --clone <pinned clone>
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

DECLARATION = re.compile(
    r"extern fn (\w+)\s*\(([^)]*)\)\s*->\s*[^\n]*\ncontract \{(.*?)\n\};", re.S
)


def borrowed_pointer_positions(sources: dict[str, str]) -> dict[str, list[int]]:
    """Function -> the indices of its `Borrowed` pointer-typed parameters."""
    out: dict[str, list[int]] = {}
    total = 0
    for text in sources.values():
        for name, parameters, body in DECLARATION.findall(text):
            total += 1
            names = []
            types = {}
            for parameter in (p.strip() for p in parameters.split(",")):
                if not parameter:
                    continue
                key, _, value = parameter.partition(":")
                names.append(key.strip())
                types[key.strip()] = value.strip()
            borrowed = set(re.findall(r"resource_borrowed\((\w+)\)", body))
            indices = [
                index
                for index, key in enumerate(names)
                if key in borrowed and types.get(key, "").startswith("ptr<")
            ]
            if indices:
                out[name] = indices
    print(f"contracted functions in force: {total}")
    print(f"with at least one Borrowed pointer parameter: {len(out)}")
    for name, indices in sorted(out.items()):
        print(f"  {name}: parameter index/indices {indices}")
    return out


SELECTION = re.compile(r"^\s*[A-Za-z_]\w*\s*->\s*[A-Za-z_]\w*\s*$")


def split_arguments(text: str) -> list[str] | None:
    """Split one call's argument list, or `None` if it is unbalanced here."""
    depth = 0
    current = ""
    arguments = []
    for character in text:
        if character in "([":
            depth += 1
        elif character in ")]":
            if depth == 0:
                arguments.append(current)
                return arguments
            depth -= 1
        elif character == "," and depth == 0:
            arguments.append(current)
            current = ""
            continue
        current += character
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contracts", required=True, type=pathlib.Path)
    parser.add_argument("--clone", required=True, type=pathlib.Path)
    parser.add_argument(
        "--diagnostics",
        type=pathlib.Path,
        help="restrict the source scan to the file names the 185-unit audit actually covers",
    )
    arguments = parser.parse_args()

    report = json.loads(arguments.contracts.read_text())
    positions = borrowed_pointer_positions(report["sources"])

    audited: set[str] | None = None
    if arguments.diagnostics:
        units = json.loads(arguments.diagnostics.read_text())["units"]
        audited = {unit["file"] for unit in units}
        print(f"audited unit file names: {len(audited)}")

    hits = 0
    scanned = 0
    by_call = {True: 0, False: 0}
    for path in sorted(arguments.clone.rglob("*.c")):
        if audited is not None and path.name not in audited:
            continue
        try:
            text = path.read_text(errors="replace")
        except OSError:
            continue
        scanned += 1
        for name, indices in positions.items():
            for match in re.finditer(rf"\b{name}\s*\(", text):
                call = split_arguments(text[match.end() :])
                if call is None:
                    continue
                for index in indices:
                    if index < len(call) and SELECTION.match(call[index]):
                        hits += 1
                        base = call[index].split("->")[0].strip()
                        # The base premise this card inherits from O04-B08:
                        # the aggregate must be bound by a *call*, in this same
                        # file. A base read out of another object, or a formal,
                        # is refused here and belongs to card C02-P's route.
                        acquired = re.search(
                            rf"\b{re.escape(base)}\s*=\s*[^;=]*\w\s*\(", text
                        )
                        by_call[bool(acquired)] += 1
                        print(
                            f"  CANDIDATE {path.relative_to(arguments.clone)}: "
                            f"{name} argument {index} = {call[index].strip()!r} "
                            f"[base bound by a call in this file: {bool(acquired)}]"
                        )
    print(f"C files scanned: {scanned}")
    print(f"over-counted candidate bound reads in the pinned clone: {hits}")
    print(
        f"  of which the base is never bound by a call in its file "
        f"(card C02-P's route): {by_call[False]}"
    )
    print(f"  of which the base may be call-bound (this card's route): {by_call[True]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
