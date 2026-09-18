import json, sys, collections
root = sys.argv[1]
prior = {}
for wave in ("wave36-close", "wave37-close"):
    rec = json.load(open(f"{root}/docs/reports/{wave}/reconciliation.json"))
    for key, v in rec["verdicts"].items():
        prior.setdefault(key, (wave, v["verdict"]))
cards = ["ct-lane-5", "joined-provenance-round-bound", "mutex-share-cross-emission",
         "task-route-tsan", "write-lend-tasks-exercise"]
for card in cards:
    keys = sorted((k for k in prior if k.startswith(card + "#")), key=lambda k: (k.rsplit("#", 1)[0], int(k.rsplit("#", 1)[1])))
    print(card, len(keys), collections.Counter(prior[k][0] for k in keys))
    for k in keys:
        print("  ", k, "->", prior[k])
