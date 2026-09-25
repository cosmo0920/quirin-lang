#!/usr/bin/env python3
"""Explain the tail refusals left after T6, line by line.

Usage: tail_residual.py DIAGNOSTICS(.json|.json.gz) UNIT_TEXTS_DIR OUT.json

For every remaining `arguments beyond the declared parameters of `X`` leaf,
the analysed line's calls of X are lexed as `tail_nested_lines.py` lexes them
and split into literal-only tails and tails that carry a name or an
expression. On one line the k-th leaf of X is **consistent with a name
refusal** when the line carries at least k calls of X whose tail is not
literal-only: every remaining leaf so explained is refused on a name
(premise ii, or premise i for a name), which T6 deliberately leaves alone. The
rest are listed.
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import tail_nested_lines as lexer  # noqa: E402


def main() -> int:
    data = lexer.load(sys.argv[1])
    texts = pathlib.Path(sys.argv[2])
    pairing = json.loads((texts / "pairing.json").read_text())
    cache = {}
    rows = []
    for unit in data["units"]:
        key = f"{unit['library']}/{unit['file']}"
        name = pairing.get(key, {}).get("text")
        if name and name not in cache:
            cache[name] = (texts / name).read_text(errors="replace").split("\n")
        lines = cache.get(name)
        for entry in unit["functions"]:
            seen = collections.Counter()
            for diagnostic in entry.get("diagnostics") or []:
                match = lexer.TAIL.match(diagnostic["description"])
                if not match:
                    continue
                callee, line = match.group(1), diagnostic["line"]
                seen[(callee, line)] += 1
                tokens = lexer.TOKEN.findall(lines[line - 1]) if lines else []
                named = literal = 0
                for _, _, arguments in lexer.calls(tokens, callee):
                    rest = arguments[lexer.NAMED.get(callee, len(arguments)):]
                    if not rest or not any(rest):
                        continue
                    if all(a and all(lexer.is_literal(t) for t in a) for a in rest):
                        literal += 1
                    else:
                        named += 1
                rows.append({
                    "unit": key, "function": entry["function"], "status": entry["status"],
                    "callee": callee, "line": line, "origin": diagnostic.get("origin"),
                    "named_tail_calls_on_line": named, "literal_tail_calls_on_line": literal,
                    "consistent_with_name_refusal": seen[(callee, line)] <= named,
                    "rdut_line": bool(lines and "RDUT:" in lines[line - 1]),
                })
    summary = {
        "leaves": len(rows),
        "by_callee": dict(collections.Counter(r["callee"] for r in rows)),
        "consistent_with_name_refusal": sum(r["consistent_with_name_refusal"] for r in rows),
        "not_explained": [r for r in rows if not r["consistent_with_name_refusal"]],
        "rdut_line_leaves": sum(r["rdut_line"] for r in rows),
        "functions": len({(r["unit"], r["function"]) for r in rows}),
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps({"summary": summary, "rows": rows}, indent=1) + "\n")
    brief = dict(summary)
    brief["not_explained"] = len(summary["not_explained"])
    print(json.dumps(brief, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
