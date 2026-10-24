#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: BASE/TIP attribution of the pinned corpus arbiter.

usage: compare.py BASE_DIR TIP_DIR BASE_PARITY TIP_PARITY OUT_JSON

BASE_DIR/TIP_DIR hold resource-diagnostics.json (scripts/run_pinned_audit.py).
Rows are keyed (unit file, function, row index within that pair) so repeated
rows of one function stay distinct. Reported:

* custody status counts on both sides and every row whose status changed,
  with its leaves on both sides and its contract dependencies at TIP;
* the leaf-family delta over all rows (closure_r3.family), and the leaves
  naming rd_kafka_log0 / rd_kafka_log_buf as uncontracted callees;
* `unknown_alias` leaves (W47-ALIAS-CALLEE-BORROWED-CLOSURE's leaf kind) in
  total and inside functions calling rd_kafka_log0;
* typed-path parity rows (CG06's `value-connective-*` consultations and the
  variadic-tail statements) naming rd_kafka_log0, by position and outcome.
"""
import collections
import json
import pathlib
import sys

ROOT = next(p for p in pathlib.Path(__file__).resolve().parents if (p / "docs/reports").is_dir())
sys.path.insert(0, str(ROOT / "docs/reports/w48-bottom-up-callee-bodies/scripts"))
import closure_r3  # noqa: E402

LOGGER = ("rd_kafka_log0", "rd_kafka_log_buf")


def rows(path):
    data = closure_r3.load(pathlib.Path(path) / "resource-diagnostics.json")
    out = {}
    seen = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            index = seen[key]
            seen[key] += 1
            out[key + (index,)] = entry
    return out


def leaves(entry):
    return [(d.get("category"), d.get("description"), d.get("line")) for d in entry.get("diagnostics") or []]


def parity(path):
    """Typed-path parity rows of statements whose call is rd_kafka_log0 (the
    callee field, or the statement's first token), by position and outcome."""
    counts = collections.Counter()
    examples = {}
    with open(path) as handle:
        for line in handle:
            if "rd_kafka_log0" not in line:
                continue
            row = json.loads(line)
            texts = row.get("texts") or []
            if row.get("callee") != "rd_kafka_log0" and (not texts or texts[0] != "rd_kafka_log0"):
                continue
            if row.get("new_admitted") or row.get("admitted") is True:
                outcome = "admitted"
            else:
                outcome = (row.get("new_refusal") or row.get("refusal") or row.get("answer")
                           or row.get("old_refusal") or "unlabelled")
            key = (str(row.get("position")), str(outcome)[:120])
            counts[key] += 1
            examples.setdefault(key, {k: row.get(k) for k in ("function", "line", "position")})
    return counts, examples


def main():
    base_dir, tip_dir, base_parity, tip_parity, out = sys.argv[1:6]
    base, tip = rows(base_dir), rows(tip_dir)
    report = {"keys_equal": sorted(base) == sorted(tip)}
    for side, data in (("base", base), ("tip", tip)):
        report[f"{side}_status"] = dict(collections.Counter(e["status"] for e in data.values()))
    changes = []
    for key in sorted(set(base) & set(tip)):
        b, t = base[key], tip[key]
        if b["status"] != t["status"] or leaves(b) != leaves(t):
            changes.append({
                "file": key[0], "function": key[1], "row": key[2],
                "base": b["status"], "tip": t["status"],
                "base_leaves": leaves(b), "tip_leaves": leaves(t),
                "tip_dependencies": t.get("contract_dependencies"),
            })
    report["status_changes"] = [c for c in changes if c["base"] != c["tip"]]
    report["leaf_only_changes"] = len([c for c in changes if c["base"] == c["tip"]])
    removed, added, gained = collections.Counter(), collections.Counter(), []
    for c in changes:
        if c["base"] != c["tip"]:
            continue
        b = collections.Counter((x[0], x[1]) for x in c["base_leaves"])
        t = collections.Counter((x[0], x[1]) for x in c["tip_leaves"])
        for (cat, desc), n in (b - t).items():
            removed[closure_r3.family({"category": cat, "description": desc})] += n
        for (cat, desc), n in (t - b).items():
            added[closure_r3.family({"category": cat, "description": desc})] += n
            gained.append({"function": c["function"], "file": c["file"], "leaf": [cat, desc]})
    report["leaf_trades"] = {"removed": dict(removed.most_common()), "added": dict(added.most_common()),
                             "added_leaves": gained}
    fam = {}
    for side, data in (("base", base), ("tip", tip)):
        counter = collections.Counter()
        for entry in data.values():
            for d in entry.get("diagnostics") or []:
                counter[closure_r3.family(d)] += 1
        fam[side] = counter
    report["family_delta"] = {
        k: [fam["base"][k], fam["tip"][k]]
        for k in sorted(set(fam["base"]) | set(fam["tip"]))
        if fam["base"][k] != fam["tip"][k]
    }
    for side, data in (("base", base), ("tip", tip)):
        named = collections.Counter()
        alias_total = 0
        for entry in data.values():
            for d in entry.get("diagnostics") or []:
                name, kind = closure_r3.callee_of(d)
                if name in LOGGER:
                    named[f"{name}:{kind}"] += 1
                if "unresolved resource identity" in (d.get("description") or ""):
                    alias_total += 1
        report[f"{side}_logger_call_leaves"] = dict(named)
        report[f"{side}_unknown_alias_leaves"] = alias_total
    pb, eb = parity(base_parity)
    pt, et = parity(tip_parity)
    report["parity_rd_kafka_log0"] = {
        "|".join(k): [pb[k], pt[k]] for k in sorted(set(pb) | set(pt))
    }
    report["parity_examples"] = {"|".join(k): v for k, v in sorted(et.items())}
    pathlib.Path(out).write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: report[k] for k in ("keys_equal", "base_status", "tip_status")}, sort_keys=True))
    print("status changes:", len(report["status_changes"]), "leaf-only:", report["leaf_only_changes"])


if __name__ == "__main__":
    main()
