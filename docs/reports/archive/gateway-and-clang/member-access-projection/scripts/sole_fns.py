import collections, gzip, json, sys

IND = "indirect memory access is outside the resource projection"
rows = [json.loads(l) for l in gzip.open(sys.argv[1], "rt")]
want = sys.argv[2] if len(sys.argv) > 2 else None
by = collections.defaultdict(list)
for r in rows:
    if set(r["families"]) <= {IND} and r["status"] == "Unknown":
        by[(r["file"], r["function"])].append(r)
for key, leaves in sorted(by.items(), key=lambda kv: kv[0][1]):
    premises = sorted({l["premise"] for l in leaves})
    if want and premises != [want]:
        continue
    print(f"## {key[1]} ({key[0]}) {premises}")
    for l in leaves:
        print("   ", l["origin"].rsplit('/', 1)[-1], "|", l["origin_text"][:90])
