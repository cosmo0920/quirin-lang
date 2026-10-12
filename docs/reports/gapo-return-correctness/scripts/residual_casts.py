#!/usr/bin/env python3
"""residual_casts.py PARITY.jsonl -- GAPO-RETURN-CORRECTNESS: the size of the
registered residual GAPO-UNCAST-ACTUALS-AFTER-LATER-LOWERINGS. Counts the
statements (function, line) whose W44 call-actual consultation was admitted
(`position: "call-argument"`, `new_admitted: true`) and whose spelling holds
a cast-closing `* )`: a call W44's re-classification lowered, which rule
12h's hook H-U0 is not asked of. An upper bound on the casts that hide a
place there (the `* )` test is lexical), and a lower bound on the residual
(10u, 10y, 11w widenings and elaborated pieces are not counted)."""
import json
import sys

statements = set()
functions = set()
with open(sys.argv[1], encoding="utf-8") as stream:
    for line in stream:
        if '"call-argument"' not in line:
            continue
        row = json.loads(line)
        if row.get("position") != "call-argument" or not row.get("new_admitted"):
            continue
        texts = row.get("statement_texts") or row.get("texts") or []
        if any(a == "*" and b == ")" for a, b in zip(texts, texts[1:])):
            statements.add((row.get("function"), row.get("line")))
            functions.add(row.get("function"))
print(f"W44-admitted statements with `* )`: {len(statements)} in {len(functions)} functions")
