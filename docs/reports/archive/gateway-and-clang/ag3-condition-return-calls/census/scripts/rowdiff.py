import json, sys
base, tip, fn = sys.argv[1], sys.argv[2], sys.argv[3]
for label, path in (("base", base), ("tip", tip)):
    d = json.load(open(path))
    for u in d["units"]:
        for f in u.get("functions", []):
            if f["function"] == fn:
                print(label, u["file"], f["status"])
                for x in f["diagnostics"]:
                    print("   ", x["line"], x.get("origin"), x["category"], x["description"])
