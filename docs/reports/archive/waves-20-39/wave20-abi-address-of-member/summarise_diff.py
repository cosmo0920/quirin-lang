#!/usr/bin/env python3
"""Print the five-artifact summary of a `diff_artifacts.py` document.

Card ABI-TYPEDEF-ADDRESS-OF-MEMBER. Read-only; the wave-19 card's script,
reused. Constraint 22: the `contracts` section is read in BOTH its normalised
and its raw keying.
"""

import json
import pathlib
import sys


def counts(section: dict) -> dict:
    out = {}
    for key, value in section.items():
        if isinstance(value, dict) and "counts" in value:
            out[key] = value["counts"]
        elif isinstance(value, dict) and "modules" in value:
            out[key] = value["modules"].get("counts")
        elif isinstance(value, dict) and "libraries" in value:
            out[key] = value["libraries"].get("counts")
    return out


def main() -> int:
    document = json.loads(pathlib.Path(sys.argv[1]).read_text())
    print("archived:", document["archived"])
    print("measured:", document["measured"])
    for name, section in document["artifacts"].items():
        print(f"--- {name}")
        envelope = section.get("envelope", {})
        if envelope:
            print("    envelope:", json.dumps(envelope)[:600])
        summary = counts(section)
        if summary:
            print("    counts:  ", json.dumps(summary))
        for key in ("keys", "keying", "raw_keys", "normalised", "raw"):
            if key in section:
                print(f"    {key}:", json.dumps(section[key])[:600])
        for key in ("sources", "manifest", "audit_selection"):
            if key in section and isinstance(section[key], dict):
                for direction in ("added", "removed", "changed"):
                    rows = section[key].get(direction) or (
                        section[key].get("modules", {}).get(direction)
                        if isinstance(section[key].get("modules"), dict)
                        else None
                    )
                    if rows:
                        print(f"    {key}.{direction}:", json.dumps(rows)[:600])
        if not envelope and not summary:
            print("    NO DIFFERENCES" if not section.get("units") else "")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
