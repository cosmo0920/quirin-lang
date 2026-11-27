"""Per-component change census of contract_loader.rs's (selected, v1, rejected)
tuple since a date: pairs each commit's removed and added tuple lines."""
import re, subprocess, sys

repo, since = sys.argv[1], (sys.argv[2] if len(sys.argv) > 2 else "2026-10-01")
out = subprocess.run(["git", "-C", repo, "log", f"--since={since}", "--no-merges",
                      "--format=@@COMMIT %H", "-p", "-U0", "--",
                      "compiler/tests/contract_loader.rs"],
                     capture_output=True, text=True, check=True).stdout
rx = re.compile(r"^([+-])\s*\((\d+), (\d+), (\d+)\)\s*$")
changes = [0, 0, 0]
per = {}
commit = None
for line in out.splitlines():
    if line.startswith("@@COMMIT "):
        commit = line.split()[1]
        continue
    m = rx.match(line)
    if m:
        per.setdefault(commit, {})[m.group(1)] = tuple(int(x) for x in m.groups()[1:])
n = 0
for commit, sides in per.items():
    if "-" in sides and "+" in sides:
        n += 1
        for i in range(3):
            if sides["-"][i] != sides["+"][i]:
                changes[i] += 1
print("tuple-changing commits", n, "selected", changes[0], "v1", changes[1], "rejected", changes[2])
