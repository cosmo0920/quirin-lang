"""Census: how often each aggregate literal changed since 2026-10-01.

For each (file, regex) pin, walk the first-parent-agnostic history of the
file (`git log --since --no-merges -p`) and count commits whose diff adds a
line matching the regex (a changed literal shows as -old/+new)."""
import re, subprocess, sys

repo = sys.argv[1]
since = sys.argv[2] if len(sys.argv) > 2 else "2026-10-01"
PINS = [
    ("compiler/tests/contract_loader.rs", "source_assumptions (production + certificate)", r"source_assumptions\(\)\.len\(\), \d+"),
    ("compiler/tests/contract_loader.rs", "corpus module count", r"modules\.len\(\), \d+"),
    ("compiler/tests/contract_loader.rs", "(selected, v1, rejected)", r"^\+\s*\(\d+, \d+, \d+\)\s*$"),
    ("compiler/tests/contract_loader.rs", "pending_review", r"pending_review\(\)\.len\(\), \d+"),
    ("compiler/tests/contract_loader.rs", "relational", r"relational\(\)\.len\(\), \d+"),
    ("compiler/tests/contract_loader.rs", "retaining", r"retaining\(\)\.len\(\), \d+"),
    ("crates/qcore-audit/tests/provider_selection.rs", "entries/total/source_assumptions", r"(entries\.len\(\)|total|source_assumptions\(\)\.len\(\)|\.1\.len\(\)), \d+\)"),
    ("crates/qcore-c-gateway/tests/neighbours.rs", "source_assumptions", r"source_assumptions\(\)\.len\(\), \d+"),
    ("crates/qcore-c-gateway/tests/prod_contracts.rs", "source_assumptions", r"source_assumptions\(\)\.len\(\), \d+"),
    ("scripts/tests/test_contract_corpus.py", "module counts (150/92/58)", r"assertEqual\((len\(modules\)|sum\(any).*\d+\)$"),
]


def commits(path, pattern):
    out = subprocess.run(["git", "-C", repo, "log", f"--since={since}", "--no-merges",
                          "--format=@@COMMIT %H %ad", "--date=short", "-p", "-U0", "--", path],
                         capture_output=True, text=True, check=True).stdout
    hits = []
    current = None
    rx = re.compile(pattern)
    for line in out.splitlines():
        if line.startswith("@@COMMIT "):
            current = line.split()[1:3]
            continue
        if line.startswith("+") and not line.startswith("+++") and rx.search(line):
            if current and (not hits or hits[-1] != current):
                hits.append(current)
    return hits


total_commits = set()
for path, what, pattern in PINS:
    hits = commits(path, pattern)
    total_commits |= {h[0] for h in hits}
    first = hits[-1][1] if hits else "-"
    last = hits[0][1] if hits else "-"
    print(f"{path}\t{what}\t{len(hits)}\t{first}..{last}")
print("distinct commits changing any pin:", len(total_commits))

for path in sorted({p for p, _, _ in PINS}):
    n = subprocess.run(["git", "-C", repo, "log", f"--since={since}", "--no-merges", "--format=%H", "--", path],
                       capture_output=True, text=True, check=True).stdout.split()
    print(f"commits touching {path}: {len(n)}")
