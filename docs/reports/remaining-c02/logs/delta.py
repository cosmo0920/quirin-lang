import json, sys

SP = "/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-qcore-execution-audit-plan-24520f/73582972-2836-40ca-bca8-03209bc93942/scratchpad/c02"


def load(label):
    return json.load(open("%s/cmetrics-audit-%s.json" % (SP, label)))


a = load(sys.argv[1] if len(sys.argv) > 1 else "baseline")
b = load(sys.argv[2] if len(sys.argv) > 2 else "c02")


def walk(o, path=""):
    if isinstance(o, dict):
        for k, v in o.items():
            yield from walk(v, path + "/" + k)
    elif isinstance(o, list):
        for i, v in enumerate(o):
            yield from walk(v, path + "/%d" % i)
    else:
        yield path, o


da = dict(walk(a))
db = dict(walk(b))
keys = sorted(set(da) | set(db))
for k in keys:
    if da.get(k) != db.get(k):
        print("CHANGED", k)
        print("   before:", repr(da.get(k))[:400])
        print("   after :", repr(db.get(k))[:400])
