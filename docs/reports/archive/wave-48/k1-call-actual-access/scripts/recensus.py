#!/usr/bin/env python3
"""Card K1 (round 34): re-census bundle K1 against the user's 2026-11-24 `current` report.

usage: recensus.py REPORT_JSON OUT_JSON

Joins every IND/COND/RET leaf group of REPORT_JSON to the CURRENT-ACCESS-ALIAS-CENSUS
(base 7b726587b, 2026-11-07) leaf groups by (file, function, preprocessed line, family,
sub) and every unknown_alias leaf by (file, function, line). The Fluent Bit revision and
the preprocessed text are the same in both runs, so the key is stable. A leaf group the
11-07 census has no row for carries the tag `new` and is never counted as removable.
Rows are unit configurations; a row is freed by K1 when every item it has carries only
K1 tags (statement, condition and return `call-arg`).
"""
import collections
import gzip
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CENSUS = ROOT / "docs/reports/current-access-alias-census"
FAMILY_IND = "indirect memory access is outside the resource projection"
FAMILY_PREFIX = "unmodeled call, aggregate or indirect access in "
CONDITION_KIND = {"branch condition": "if", "loop condition": "while", "for-loop condition": "for",
                  "do-while condition": "do", "switch expression": "switch"}
UNC = re.compile(r"call to uncontracted function `([^`]*)`")
K1 = {"expr:statement:call-arg", "expr:cond:call-arg", "expr:return:call-arg"}


def family(leaf, defined):
    category = leaf["category"]
    if category == "unknown_alias":
        return "ALIAS", None
    if category != "unsupported":
        return category.upper(), None
    d = leaf["description"]
    if d == FAMILY_IND:
        return "IND", None
    if d.startswith(FAMILY_PREFIX):
        rest = d[len(FAMILY_PREFIX):]
        if rest == "return expression":
            return "RET", None
        if rest in CONDITION_KIND:
            return "COND", CONDITION_KIND[rest]
    m = UNC.match(d)
    if m:
        return ("UNC-RUN" if m.group(1) in defined else "UNC-EXT"), m.group(1)
    for key, name in (("callback", "CALLBACK"), ("slot", "SLOT"), ("out-parameter", "OUTPARAM"),
                      ("retention", "RETAIN"), ("retain", "RETAIN"), ("reference", "REFCOUNT"),
                      ("nullable", "NULLABLE"), ("va_", "VA")):
        if key in d:
            return name, None
    return "SYNTAX", None


def coarse(tag):
    if not tag.startswith("expr:"):
        return tag
    _, construct, position, _roots = tag.split(":", 3)
    kind = "cond" if construct.startswith("cond") else construct
    return f"expr:{kind}:{'call-arg' if 'call-arg' in position else 'direct'}"


def jsonl_gz(path):
    with gzip.open(path, "rt") as handle:
        handle.readline()
        return [json.loads(line) for line in handle]


def main():
    report = json.load(open(sys.argv[1]))
    out = pathlib.Path(sys.argv[2])
    old_leaves = {}
    for r in jsonl_gz(CENSUS / "artifacts/shape-leaves.jsonl.gz"):
        old_leaves[(r["file"], r["function"], r["line"], r["family"], r["sub"])] = \
            (sorted({coarse(t) for t in r["tags"]}), r.get("facts"), r["tags"])
    old_alias = {}
    for r in jsonl_gz(CENSUS / "artifacts/alias-leaves.jsonl.gz"):
        old_alias.setdefault((r["file"], r["function"], r["line"]), r["group"])
    frozen = [entry.split("::", 2) for entry in
              re.search(r"## Frozen candidate list.*?```text\n(.*?)```",
                        (CENSUS / "predictions/K1.md").read_text(), re.S).group(1).split()]

    defined = {f["function"] for u in report["units"] for f in u.get("functions", [])}
    rows = []
    k1_leaf_groups = collections.Counter()
    k1_leaves = collections.Counter()
    k1_fine = collections.Counter()
    joined = collections.Counter()
    for index, unit in enumerate(report["units"]):
        for fn in unit.get("functions", []):
            items = []
            groups = collections.defaultdict(list)
            for leaf in fn["diagnostics"]:
                fam, sub = family(leaf, defined)
                if fam in ("IND", "COND", "RET"):
                    groups[(fam, sub, leaf["line"])].append(leaf)
                elif fam == "ALIAS":
                    g = old_alias.get((unit["file"], fn["function"], leaf["line"]))
                    joined["alias-joined" if g else "alias-new"] += 1
                    items.append(("ALIAS", ["alias:" + g] if g else ["new"], leaf["line"]))
                elif fam in ("UNC-RUN", "UNC-EXT"):
                    items.append((fam, [fam + ":" + sub], leaf["line"]))
                else:
                    items.append((fam, ["fix:" + fam], leaf["line"]))
            for (fam, sub, line), group in groups.items():
                hit = old_leaves.get((unit["file"], fn["function"], line, fam, sub))
                joined["group-joined" if hit else "group-new"] += 1
                tags = hit[0] if hit else ["new"]
                for _ in group:
                    items.append((fam, tags, line))
                if set(tags) & K1:
                    for t in set(tags) & K1:
                        k1_leaf_groups[t] += 1
                        k1_leaves[t] += len(group)
                    for t in hit[2]:
                        k1_fine[t] += len(group)
            rows.append({"i": index, "file": unit["file"], "library": unit.get("library"),
                         "function": fn["function"], "status": fn["status"], "items": items})

    def freed_by(row, tags):
        return bool(row["items"]) and all(set(t) <= tags for _, t, _ in row["items"])

    unknown = [r for r in rows if r["status"] == "Unknown"]
    touched = [r for r in unknown if any(set(t) & K1 for _, t, _ in r["items"])]
    freed = [r for r in unknown if freed_by(r, K1)]
    per_tag_alone = {t: sum(1 for r in unknown if freed_by(r, {t})) for t in sorted(K1)}
    nxt = collections.Counter()
    for r in touched:
        rest = sorted({f for f, t, _ in r["items"] if not set(t) <= K1})
        nxt["none" if not rest else "+".join(rest[:4])] += 1
    by_key = collections.defaultdict(list)
    for r in rows:
        by_key[(r["file"], r["function"])].append(r)
    frozen_status = []
    for lib, file, func in frozen:
        now = by_key.get((file, func), [])
        statuses = sorted({r["status"] for r in now})
        k1_now = sum(1 for r in now for _, t, _ in r["items"] if set(t) & K1)
        freed_now = any(freed_by(r, K1) for r in now if r["status"] == "Unknown")
        blockers = collections.Counter()
        for r in now:
            for f, t, _ in r["items"]:
                if not set(t) <= K1:
                    blockers[f + ":" + ",".join(t)] += 1
        frozen_status.append({"function": f"{lib}::{file}::{func}", "statuses": statuses,
                              "k1_leaves_now": k1_now, "freed_alone_now": freed_now,
                              "other_blockers": dict(blockers.most_common(6))})
    summary = {
        "report": sys.argv[1],
        "rows": len(rows), "unknown_rows": len(unknown),
        "safe_rows": sum(1 for r in rows if r["status"] == "Safe"),
        "unsafe_rows": sum(1 for r in rows if r["status"] == "Unsafe"),
        "join": dict(joined),
        "k1_leaf_groups": dict(k1_leaf_groups), "k1_leaves": dict(k1_leaves),
        "k1_leaves_total": sum(k1_leaves.values()),
        "k1_fine_tags": dict(k1_fine.most_common()),
        "k1_rows_touched": len(touched),
        "k1_rows_freed_alone": len(freed),
        "k1_distinct_freed_alone": len({(r["file"], r["function"]) for r in freed}),
        "per_tag_freed_alone": per_tag_alone,
        "next_blocker_of_touched": dict(nxt.most_common(25)),
        "frozen_list": {
            "size": len(frozen),
            "safe_now": sum(1 for f in frozen_status if f["statuses"] == ["Safe"]),
            "absent_now": sum(1 for f in frozen_status if not f["statuses"]),
            "still_freed_alone": sum(1 for f in frozen_status if f["freed_alone_now"]),
            "unknown_with_no_k1_leaf": sum(1 for f in frozen_status
                                           if "Unknown" in f["statuses"] and f["k1_leaves_now"] == 0),
            "rows": frozen_status,
        },
        "freed_alone_now": sorted(f"{r.get('library')}::{r['file']}::{r['function']}" for r in freed),
    }
    out.write_text(json.dumps(summary, indent=1) + "\n")
    with gzip.open(str(out) + ".rows.jsonl.gz", "wt") as handle:
        for r in rows:
            handle.write(json.dumps(r) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k not in ("frozen_list", "freed_alone_now")}, indent=1))
    print(json.dumps({k: v for k, v in summary["frozen_list"].items() if k != "rows"}))


if __name__ == "__main__":
    main()
