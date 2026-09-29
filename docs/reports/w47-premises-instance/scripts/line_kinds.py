import collections, gzip, json, pathlib, re, sys
W = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a9952aa2c24a52c19")
ARCH = W / "docs/reports/w47-flb-lib-lifecycle/integration/artifacts"
ROOT = re.compile(r"/home/cosmo/Documents/GitHub/qcore-c/\.claude/worktrees/[A-Za-z0-9_-]+")
for measure, run in (("system", W / "private/pi/arbiters-after/system"),
                     ("slice", pathlib.Path((W / "private/pi/arbiters-after/slice.path").read_text().strip()))):
    manifest = json.loads((ARCH / measure / "manifest.json").read_text())
    entry = manifest["resource-diagnostics.json"]
    theirs = gzip.decompress((ARCH / measure / entry["file"]).read_bytes()).decode()
    ours = (run / "resource-diagnostics.json").read_text()
    a = ROOT.sub("ROOT", ours).splitlines()
    b = ROOT.sub("ROOT", theirs).splitlines()
    kinds = collections.Counter()
    for x, y in zip(a, b):
        if x != y:
            key = x.strip().split(":")[0]
            kinds[key] += 1
    print(measure, dict(kinds))
