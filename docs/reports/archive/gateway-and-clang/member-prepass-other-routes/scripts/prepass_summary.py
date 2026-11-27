#!/usr/bin/env python3
"""prepass_summary.py NAME [OUT.json] -- MEMBER-PREPASS-OTHER-ROUTES: the
pre-pass-alone sections (prepass.sh NAME) per route: units, read, unread,
rounds, tokens, wall time, and each scope entry held or dropped (with why).
Prints it, and writes OUT.json when given."""
import json
import pathlib
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/mpor"


def main():
    name = sys.argv[1]
    out = {}
    for route in ("corpus", "system", "slice"):
        path = PRIVATE / f"{name}-prepass-{route}.json"
        if not path.exists():
            out[route] = {"missing": str(path)}
            continue
        section = json.loads(path.read_text())
        out[route] = {
            "exit": (PRIVATE / f"{name}-prepass-{route}.exit").read_text().strip(),
            "units": section["units"], "read": section["read"],
            "unread": section["unread"], "rounds": section["rounds"],
            "tokens": len(section["tokens"]), "seconds": round(section["seconds"], 1),
            "scope": {k: v for k, v in section["scope"].items() if k != "meaning"},
            "outside_program": len(section["outside_program"]),
        }
    text = json.dumps(out, indent=1) + "\n"
    if len(sys.argv) > 2:
        pathlib.Path(sys.argv[2]).write_text(text)
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
