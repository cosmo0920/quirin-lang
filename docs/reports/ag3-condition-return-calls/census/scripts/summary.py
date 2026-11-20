"""Summaries for the report: admission sites, exchanged rows, sole-reason moves."""
import json, re, sys
from collections import Counter, defaultdict

base, tip, cmp_path = sys.argv[1], sys.argv[2], sys.argv[3]
c = json.load(open(cmp_path))
PREFIX = "unmodeled call, aggregate or indirect access in "
NAMED = re.compile(r"call to uncontracted function `([^`]+)` in (branch|loop|for-loop|do-while) condition")
added_lines = defaultdict(list)
for f, fn, cat, d, l, n in c["added"]:
    added_lines[(f, fn, l)].append(d)
adm = []
for f, fn, cat, d, l, n in c["removed"]:
    if d.startswith(PREFIX) and not any(NAMED.fullmatch(x) for x in added_lines[(f, fn, l)]):
        adm.append((f, fn, l, added_lines[(f, fn, l)]))
print("rule 63 (1) admissions (removed generic leaf, no named leaf at its line):")
for a in adm:
    print("  ", a)
rows = {(f, fn) for f, fn, cat, d, l, n in c["added"] if NAMED.fullmatch(d)}
print("rows carrying a named refusal:", len(rows))
callees = Counter(NAMED.fullmatch(d).group(1) for f, fn, cat, d, l, n in c["added"] if NAMED.fullmatch(d))
print("named leaves:", sum(callees.values()), "distinct callees:", len(callees))
print("top callees:", callees.most_common(15))


def sole(path):
    d = json.load(open(path))
    out = {}
    for u in d["units"]:
        for f in u.get("functions", []):
            if f["status"] != "Unknown":
                continue
            kinds = set()
            for x in f["diagnostics"]:
                desc = x["description"]
                m = NAMED.fullmatch(desc)
                if m:
                    kinds.add("named:" + m.group(1))
                elif desc.startswith(PREFIX):
                    kinds.add("generic:" + desc[len(PREFIX):])
                else:
                    kinds.add("other")
            out[(u["file"], f["function"])] = kinds
    return out


b, t = sole(base), sole(tip)
ag3_only_base = {k for k, v in b.items() if v and all(x.startswith("generic:") for x in v)}
ag3_only_tip = {k for k, v in t.items() if v and all(x.startswith("generic:") for x in v)}
named_only_tip = {k for k, v in t.items() if v and all(x.startswith("named:") for x in v)}
one_callee = {k for k in named_only_tip if len(t[k]) == 1}
print("AG3-only (generic) Unknown rows: base", len(ag3_only_base), "tip", len(ag3_only_tip))
print("tip rows blocked only by named condition refusals:", len(named_only_tip), "of which one callee:", len(one_callee))
print("  ", sorted(f"{fn}<-{next(iter(t[(f, fn)]))[6:]}" for f, fn in one_callee)[:40])
