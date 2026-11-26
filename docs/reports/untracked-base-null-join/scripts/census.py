#!/usr/bin/env python3
"""census.py REPORT.json CHECKOUT -- card UNTRACKED-BASE-NULL-JOIN-DISCHARGE.

The card can move only a row whose verdict rests on a rule-14n premise
`null-deref:F:LINE:VAR` (the only reading that admits the joined literal
null, see the report section 1). For every row of an opt-in `current` report
that lists such a premise, this reads the function's own definition in the
checkout and asks whether VAR is ever written the null constant there
(`VAR = NULL`, `VAR = 0`, `VAR = (void *) 0`, `T *VAR = NULL`, ...), and
whether VAR is also written something else (the join the card is about needs
both). A row whose premise variable is never written a null constant cannot
move. The scan is lexical, over the original source, and over-approximates:
a null write that every path overwrites before the read is still counted.

Prints one JSON line per premise row and a summary line.
"""
import json
import re
import sys
from pathlib import Path

NULL = r"(?:NULL|0|\(\s*void\s*\*\s*\)\s*0|\(\s*\(\s*void\s*\*\s*\)\s*0\s*\))"


def body(text: str, function: str) -> str | None:
    """The brace-balanced body of the first definition of `function`."""
    for match in re.finditer(r"\b" + re.escape(function) + r"\s*\(", text):
        depth, index = 0, match.end() - 1
        # Skip the parameter list.
        while index < len(text):
            if text[index] == "(":
                depth += 1
            elif text[index] == ")":
                depth -= 1
                if depth == 0:
                    break
            index += 1
        rest = text[index + 1 :]
        stripped = rest.lstrip()
        if not stripped.startswith("{"):
            continue
        start = index + 1 + (len(rest) - len(stripped))
        depth = 0
        for end in range(start, len(text)):
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
                if depth == 0:
                    return text[start : end + 1]
    return None


def main() -> int:
    report = json.load(open(sys.argv[1]))
    checkout = Path(sys.argv[2])
    rows = 0
    candidates = []
    for unit in report["units"]:
        path = checkout / unit["file"]
        text = path.read_text(errors="replace") if path.exists() else ""
        for row in unit.get("functions", []):
            premises = row.get("contract_dependencies", {}).get("null_premises", [])
            if not premises:
                continue
            rows += 1
            own = []
            for premise in premises:
                _, function, line, variable = premise.split(":", 3)
                if function != row["function"]:
                    own.append({"premise": premise, "own": False})
                    continue
                code = body(text, function) or ""
                null_write = re.search(
                    r"(?<![=!<>])\b" + re.escape(variable) + r"\s*=\s*" + NULL + r"\s*[;,)]",
                    code,
                )
                writes = re.findall(
                    r"(?<![=!<>.>])\b" + re.escape(variable) + r"\s*=(?!=)\s*([^;]*);", code
                )
                own.append(
                    {
                        "premise": premise,
                        "own": True,
                        "null_written": bool(null_write),
                        "other_writes": [w.strip()[:60] for w in writes
                                         if not re.fullmatch(NULL, w.strip())],
                        "body_found": bool(code),
                    }
                )
            record = {
                "file": unit["file"],
                "function": row["function"],
                "status": row["status"],
                "premises": own,
            }
            print(json.dumps(record))
            if any(p.get("null_written") for p in own):
                candidates.append(record)
    safe = [c for c in candidates if c["status"] == "Safe"]
    print(json.dumps({
        "premise_rows": rows,
        "null_written_rows": len(candidates),
        "null_written_safe_rows": len(safe),
        "safe_candidates": [f"{c['file']}::{c['function']}" for c in safe],
    }))
    return 0


if __name__ == "__main__":
    sys.exit(main())
