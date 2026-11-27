#!/usr/bin/env python3
"""Run the task_route_tsan test binary N times, M copies at once, and count
failing runs and failing rows.

usage: load.py BINARY N M OUT_JSON [libtest args...]
"""
import collections, concurrent.futures, json, os, re, subprocess, sys, time

binary, n, m, out = os.path.abspath(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
extra = sys.argv[5:]
cwd = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "compiler")

def one(i):
    r = subprocess.run([binary, *extra], cwd=cwd, capture_output=True, text=True)
    failed = re.findall(r"^test (\S+) \.\.\. FAILED$", r.stdout, re.M)
    summary = re.findall(r"^test result: .*$", r.stdout, re.M)
    return {"rc": r.returncode, "failed": failed, "summary": summary[-1] if summary else None,
            "panic": re.findall(r"panicked at [^\n]*\n[^\n]*", r.stdout)}

t0 = time.time()
with concurrent.futures.ThreadPoolExecutor(m) as pool:
    results = list(pool.map(one, range(n)))
rows = collections.Counter(name for r in results for name in r["failed"])
record = {
    "binary": os.path.basename(binary), "runs": n, "concurrent_copies": m, "args": extra,
    "failing_runs": sum(r["rc"] != 0 for r in results), "failing_rows": dict(rows),
    "panics": sorted({p for r in results for p in r["panic"]}),
    "seconds": round(time.time() - t0, 1),
}
json.dump(record, open(out, "w"), indent=1)
print(json.dumps({k: record[k] for k in ("runs", "concurrent_copies", "args", "failing_runs", "failing_rows", "seconds")}))
