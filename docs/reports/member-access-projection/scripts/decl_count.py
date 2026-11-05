import collections, gzip, json, re, sys

IND = "indirect memory access is outside the resource projection"
rows = [json.loads(l) for l in gzip.open(sys.argv[1], "rt")]
DECL = re.compile(r"^(?:const\s+|unsigned\s+|signed\s+|struct\s+|enum\s+|union\s+)*[A-Za-z_]\w*(?:\s+[A-Za-z_]\w*)?[\s\*]+[A-Za-z_]\w*\s*(\[|,|;)")
hit = [r for r in rows if r["premise"] == "no-evaluated-path" and not r["calls"] and DECL.match(r["origin_text"]) and "=" not in r["origin_text"]]
fns = {(r["file"], r["function"]) for r in hit}
by_fn = collections.defaultdict(list)
for r in rows:
    by_fn[(r["file"], r["function"])].append(r)
only = [k for k in fns if all(x in hit for x in by_fn[k])]
sole = [k for k in only if set(by_fn[k][0]["families"]) <= {IND}]
print("uninitialised declaration leaves", len(hit), "functions", len(fns), "functions whose every IND leaf is one", len(only), "sole IND", len(sole))
for k in sole:
    print("  sole:", k)
for r in hit[:80]:
    print("   ", r["function"][:40], "|", r["origin_text"][:90])
