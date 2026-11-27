#!/usr/bin/env python3
"""Card K1: lexical description of every K1 site (sites.py output), full statement text.

usage: lexical.py SITES_JSONL SOURCE_ROOT OUT_JSON

Reads each site's statement from the prepared tree at its origin (up to the
statement's end by parenthesis balance) and records lexical features. The
features describe the source; they decide nothing (the gateway's streams do).
"""
import collections
import json
import pathlib
import re
import sys

CUSTODY = re.compile(r"\b(\w*(destroy|_free|free|close|release|unref|decref|_drop)\w*|RD_IF_FREE)\s*\(")
LOG = re.compile(r"\b(rd_kafka_dbg|rd_kafka_log|rd_rkb_dbg|rd_rkb_log|rd_kafka_dbg0|flb_(debug|trace|info|warn|error|plg_\w+)|"
                 r"cio_log_\w+|cmt_errno|ctr_errno|mk_(warn|err|info|debug)|fprintf|printf|snprintf|rd_snprintf)\s*\(")


def statement(lines, n):
    out, depth = [], 0
    for i in range(n - 1, min(n + 12, len(lines))):
        s = re.sub(r"//.*$", "", lines[i])
        out.append(s.strip())
        depth += s.count("(") - s.count(")")
        if depth <= 0 and s.rstrip().endswith((";", "{", ")")):
            break
    return re.sub(r"/\*.*?\*/", "", " ".join(out))


def main():
    sites, root, out = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    cache = {}
    features = collections.Counter()
    features_freed = collections.Counter()
    primary = collections.Counter()
    primary_freed = collections.Counter()
    primary_rows = collections.defaultdict(set)
    rows = []
    for line in open(sites):
        r = json.loads(line)
        origin = r["origin"] or ""
        file, _, number = origin.rpartition(":")
        path = root / file
        if path not in cache:
            try:
                cache[path] = path.read_text(errors="replace").splitlines()
            except OSError:
                cache[path] = []
        text = statement(cache[path], int(number)) if number.isdigit() and cache[path] else ""
        position = ("return" if text.startswith("return") else
                    "condition" if re.match(r"(if|while|for|do|switch|\}\s*while)\b", text) else "statement")
        f = {
            "custody-like-callee": bool(CUSTODY.search(text)),
            "log-or-format": bool(LOG.search(text)),
            "sizeof": "sizeof" in text,
            "conditional": "?" in text,
            "address": bool(re.search(r"[(,]\s*&", text)),
            "cast-in-actual": bool(re.search(r"[(,]\s*\(\s*(const\s+)?(unsigned\s+)?\w+\s*\*+\s*\)", text)),
            "macro-upper": bool(re.match(r"[A-Z_][A-Z0-9_]{3,}\s*\(", text)),
            "nested-call": len(re.findall(r"\b\w+\s*\(", re.sub(r"^(if|while|for|return|switch)\b", "", text))) > 1,
            "deref-call": bool(re.search(r"\b\d+\s*\*\s*\w+\s*\(", text)),
            "empty-text": not text,
        }
        for k, v in f.items():
            if v:
                features[k] += 1
                if r["freed_alone"]:
                    features_freed[k] += 1
        p = next((k for k in ("empty-text", "custody-like-callee", "log-or-format", "macro-upper", "sizeof",
                              "cast-in-actual", "conditional", "nested-call", "address", "deref-call") if f[k]),
                 "plain")
        key = f"{position}:{p}"
        primary[key] += 1
        primary_rows[key].add((r["unit"], r["function"]))
        if r["freed_alone"]:
            primary_freed[key] += 1
        rows.append({**{k: r[k] for k in ("file", "function", "line", "origin", "freed_alone")},
                     "position": position, "primary": p, "statement": text[:400]})
    summary = {
        "sites": len(rows),
        "features": dict(features.most_common()),
        "features_freed_alone": dict(features_freed.most_common()),
        "primary": {k: {"sites": v, "rows": len(primary_rows[k]), "freed_alone_sites": primary_freed[k]}
                    for k, v in primary.most_common()},
    }
    out.write_text(json.dumps(summary, indent=1) + "\n")
    with open(str(out) + ".rows.jsonl", "w") as handle:
        for r in rows:
            handle.write(json.dumps(r) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
