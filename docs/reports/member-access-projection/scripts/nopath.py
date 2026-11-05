import collections, gzip, json, re, sys

IND = "indirect memory access is outside the resource projection"
rows = [json.loads(l) for l in gzip.open(sys.argv[1], "rt")]
rows = [r for r in rows if r["premise"] == "no-evaluated-path"]
DECL = re.compile(r"^(?:const\s+|static\s+|unsigned\s+|signed\s+|struct\s+|enum\s+|union\s+)*[A-Za-z_]\w*(?:\s+[A-Za-z_]\w*)?[\s\*]+[A-Za-z_]\w*\s*(\[|,|=|;)")
kinds = collections.Counter()
examples = collections.defaultdict(list)
for r in rows:
    text = r["origin_text"]
    if not r["calls"] and DECL.match(text):
        if "," in text:
            k = "declaration:multi-declarator"
        elif "[" in text:
            k = "declaration:array"
        else:
            k = "declaration:other"
    elif r["calls"]:
        k = "call-statement"
    else:
        k = "other"
    kinds[k] += 1
    examples[k].append((r["function"], text[:80], set(r["families"]) <= {IND}))
print(kinds)
for k, v in examples.items():
    print("==", k, "sole:", sum(1 for e in v if e[2]))
    for e in v[:12]:
        print("   ", e)
