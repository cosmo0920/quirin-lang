"""Read-only: every row whose contract dependencies name an in-place callee.

Usage: inplace_rows.py <resource-diagnostics.json> [<label>]
Prints function, status, artifact and the in-place callees it depends on.
"""
import json
import sys

IN_PLACE = {"regcomp", "regexec", "regerror", "regfree", "pthread_key_create", "pthread_key_delete"}
data = json.load(open(sys.argv[1]))
label = sys.argv[2] if len(sys.argv) > 2 else ""
count = 0
for unit in data.get("units", []):
    for function in unit.get("functions", []):
        declarations = set(function.get("contract_dependencies", {}).get("declarations", []))
        hit = sorted(declarations & IN_PLACE)
        if not hit:
            continue
        count += 1
        artifact = function.get("hypothesis_context", {}).get("artifact", "")
        reasons = sorted({
            (d.get("kind", {}) if isinstance(d.get("kind"), str) else json.dumps(d.get("kind")))[:90]
            for d in function.get("diagnostics", [])
        })
        print(f"{label}\t{function.get('function')}\t{function.get('status')}\t{artifact}\t{','.join(hit)}\t{len(reasons)} diag")
print(f"{label}\ttotal\t{count}", file=sys.stderr)
