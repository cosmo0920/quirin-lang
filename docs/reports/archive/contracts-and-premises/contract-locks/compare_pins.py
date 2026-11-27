"""Before/after: the base revision's literal pins against a probe or the locks."""
import json, re, sys, subprocess, pathlib

root = pathlib.Path(sys.argv[1])
base = sys.argv[2]
show = lambda p: subprocess.check_output(["git", "-C", str(root), "show", f"{base}:{p}"], text=True)
src = show("compiler/tests/qcore_contract_corpus.rs")
ec = src[src.index("const EXPECTED_CONTRACTS"):src.index('#[path = "support/contract_corpus.rs"]')]
ec = re.sub(r"//[^\n]*", "", ec)
rows = [(a, b, int(c)) for a, b, c in re.findall(r'\(\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),?\s*\)', ec)]
ei = src[src.index("const EXPECTED_INHERITANCE"):src.index("fn inherited_contracts_are_pinned")]
ei = re.sub(r"//[^\n]*", "", ei)
inh = re.findall(r'\(\s*"([^"]+)",\s*"([^"]+)",\s*"([^"]+)",?\s*\)', ei)
size = int(re.search(r"assert_eq!\(sources.len\(\), (\d+)", src).group(1))
premised = int(re.search(r'"librdkafka/null_deref.qv"\)\s*\.count\(\),\s*(\d+)', src).group(1))
py = show("providers/tests/test_contract_load_path.py")
view_total = int(re.search(r"sum\(len\(view\) for view in views.values\(\)\), (\d+)\)", py).group(1))
libc_view = int(re.search(r'len\(views\["quirin-provider-libc"\]\), (\d+)\)', py).group(1))

locks = {}
for path in sorted(root.glob("providers/*/contract-inventory.lock")):
    lock = json.loads(path.read_text())
    locks[lock["crate"]] = lock
files = {m: r for lock in locks.values() for m, r in lock["files"].items()}
actual = [(m, d["name"], d["clauses"]) for m in sorted(files) for d in files[m]["declarations"]]
actual_inh = [(m, i["child"], i["parent"]) for m in sorted(files) for i in files[m]["inherits"]]

table = [
    ("module count (`sources.len()`)", size, len(files)),
    ("EXPECTED_CONTRACTS declarations", len(rows), len(actual)),
    ("EXPECTED_CONTRACTS clauses (sum)", sum(r[2] for r in rows), sum(r[2] for r in actual)),
    ("EXPECTED_CONTRACTS files with a declaration", len({r[0] for r in rows}), len({r[0] for r in actual})),
    ("EXPECTED_CONTRACTS rows equal, in order", "-", rows == actual),
    ("per-file declaration_count equals rows", "-", all(files[m]["declaration_count"] == len(files[m]["declarations"]) for m in files)),
    ("per-file clause_count equals rows", "-", all(files[m]["clause_count"] == sum(d["clauses"] for d in files[m]["declarations"]) for m in files)),
    ("EXPECTED_INHERITANCE edges", len(inh), len(actual_inh)),
    ("EXPECTED_INHERITANCE rows equal, in order", "-", [tuple(x) for x in inh] == actual_inh),
    ("librdkafka/null_deref.qv premised declarations", premised, len(files["librdkafka/null_deref.qv"]["premises"])),
    ("resource-view modules (sum)", view_total, sum(len(l["resource_view"]) for l in locks.values())),
    ("quirin-provider-libc resource view", libc_view, len(locks["quirin-provider-libc"]["resource_view"])),
]
for name, before, after in table:
    print(f"| {name} | {before} | {after} |")
per_file = {}
for m, n, c in rows:
    d = per_file.setdefault(m, [0, 0])
    d[0] += 1; d[1] += c
mismatch = [m for m in per_file if (files[m]["declaration_count"], files[m]["clause_count"]) != tuple(per_file[m])]
print("per-file mismatches:", mismatch)
print(json.dumps({m: per_file[m] for m in sorted(per_file)}))
