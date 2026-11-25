#!/usr/bin/env python3
"""product_sites.py SOURCE_ROOT R1124_PICKLE OUT_JSON -- K2: every source line of
the prepared Fluent Bit tree (read-only) holding `( NAME * sizeof (` -- the token
run the total-or-reject declaration reader reads as a declaration candidate --
with its enclosing function (the nearest preceding column-0 definition line) and
that function's 11-24 rows (status and leaf count per unit configuration)."""
import json
import pathlib
import pickle
import re
import sys

root, pkl, out = sys.argv[1:4]
PATTERN = re.compile(r"\(\s*[A-Za-z_]\w*\s*\*\s*sizeof\s*\(")
DEFINITION = re.compile(r"^(?:[A-Za-z_][\w\s\*]*?\b)?([A-Za-z_]\w*)\s*\(")
units = pickle.load(open(pkl, "rb"))
rows = {}
for d in units:
    for f in d["functions"]:
        rows.setdefault((d["file"], f["function"]), []).append([f["status"], len(f["diagnostics"])])
sites = []
for path in sorted(pathlib.Path(root).rglob("*.c")):
    rel = str(path.relative_to(root))
    if not any((rel, fn) in rows for (rel_, fn) in [(rel, None)]) and not any(k[0] == rel for k in rows):
        continue
    lines = path.read_text(errors="replace").splitlines()
    for number, text in enumerate(lines, 1):
        if not PATTERN.search(text):
            continue
        function = None
        for back in range(number - 1, 0, -1):
            candidate = lines[back - 1]
            if candidate[:1].isalpha() or candidate[:1] == "_":
                match = DEFINITION.match(candidate)
                if match and not candidate.rstrip().endswith(";"):
                    function = match.group(1)
                    break
        sites.append({"origin": f"{rel}:{number}", "text": text.strip(), "function": function,
                      "rows": rows.get((rel, function), [])})
json.dump(sites, open(out, "w"), indent=1)
for s in sites:
    print(s["origin"], s["function"], s["rows"], "|", s["text"][:90])
