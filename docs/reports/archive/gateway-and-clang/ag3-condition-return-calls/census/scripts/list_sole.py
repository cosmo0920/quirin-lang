import json, sys
rows = json.load(open(sys.argv[1]))
want = sys.argv[2] if len(sys.argv) > 2 else None
for r in rows:
    if not r["sole"]:
        continue
    for a in r["ag3"]:
        if want and a["position"] != want:
            continue
        print(f'{r["library"]:10s} {r["function"][:40]:40s} {a["position"][:6]} {a["origin"]} | {(a["text"] or "")[:150]}')
