#!/usr/bin/env python3
"""alias_forms.py PPDIR UNIT ALIAS... -- AG1: every typedef declaration of
each ALIAS in one analysed unit (PPDIR/UNIT.i), and whether its struct body
holds a union member; the facts rule 61 decides an alias on."""
import re
import sys


def main() -> int:
    ppdir, unit = sys.argv[1:3]
    text = open(f"{ppdir}/{unit}.i", errors="replace").read()
    for alias in sys.argv[3:]:
        forward = re.findall(r"typedef\s+struct\s+(\w+)\s+" + re.escape(alias) + r"\s*;", text)
        defining = re.findall(r"typedef\s+struct\s+(\w+)\s*\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}\s*" + re.escape(alias) + r"\s*;", text)
        tags = set(forward) | set(defining)
        bodies = []
        for tag in tags:
            for match in re.finditer(r"struct\s+" + re.escape(tag) + r"\s*\{", text):
                depth, i = 0, match.end() - 1
                while i < len(text):
                    if text[i] == "{":
                        depth += 1
                    elif text[i] == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    i += 1
                bodies.append(text[match.end():i])
        unions = any(re.search(r"\bunion\b", body) for body in bodies)
        print(f"{alias}: forward {forward} defining {defining} definitions {len(bodies)} union-member {unions}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
