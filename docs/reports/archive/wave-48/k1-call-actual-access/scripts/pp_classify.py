#!/usr/bin/env python3
"""pp_classify.py SITES_JSONL PP_DIR OUT_JSON -- K1: read every K1 site's analysed
statement from the reproduced -E -P text (preprocess.py) at the report's `line`, and
record whether it holds a parenthesised name used as an access root
(`( NAME ) ->`, `( NAME ) .`, `& ( NAME ) ->`), the macro-expansion shape of
`rd_kafka_toppar_lock(rktp)` -> `mtx_lock(&(rktp)->rktp_lock)`. Lexical description
only; the gateway decides."""
import collections
import json
import pathlib
import re
import sys

PAREN_ROOT = re.compile(r"\(\s*([A-Za-z_]\w*)\s*\)\s*(->|\.(?![0-9]))")
ADDRESS_PAREN_ROOT = re.compile(r"&\s*\(\s*([A-Za-z_]\w*)\s*\)\s*(->|\.(?![0-9]))")
TYPE_WORDS = {"int", "char", "long", "short", "unsigned", "signed", "void", "float", "double",
              "const", "volatile", "struct", "union", "enum", "size_t", "_Bool"}


def statement(lines, n):
    out, depth = [], 0
    for i in range(n - 1, min(n + 30, len(lines))):
        s = lines[i]
        out.append(s.strip())
        depth += s.count("(") - s.count(")")
        if depth <= 0 and s.rstrip().endswith((";", "{", ")", "}")):
            break
    return " ".join(out)


def main():
    sites, pp, out = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    cache = {}
    counts = collections.Counter()
    rows_by = collections.defaultdict(set)
    freed_rows_by = collections.defaultdict(set)
    examples = {}
    records = []
    for line in open(sites):
        r = json.loads(line)
        path = pp / (r["file"] + ".i")
        if path not in cache:
            cache[path] = path.read_text(errors="replace").splitlines() if path.exists() else []
        text = statement(cache[path], r["line"]) if cache[path] else ""
        roots = [m.group(1) for m in PAREN_ROOT.finditer(text) if m.group(1) not in TYPE_WORDS]
        address_roots = [m.group(1) for m in ADDRESS_PAREN_ROOT.finditer(text) if m.group(1) not in TYPE_WORDS]
        cls = ("no-text" if not text else
               "address-paren-root" if address_roots else
               "paren-root" if roots else "other")
        counts[cls] += 1
        rows_by[cls].add((r["unit"], r["function"]))
        if r["freed_alone"]:
            freed_rows_by[cls].add((r["unit"], r["function"]))
        examples.setdefault(cls, []).append(text[:200]) if len(examples.get(cls, [])) < 12 else None
        records.append({"file": r["file"], "function": r["function"], "line": r["line"],
                        "freed_alone": r["freed_alone"], "class": cls, "paren_roots": roots,
                        "text": text[:400]})
    summary = {
        "sites": sum(counts.values()),
        "classes": {c: {"sites": n, "rows": len(rows_by[c]), "freed_alone_rows": len(freed_rows_by[c])}
                    for c, n in counts.most_common()},
        "examples": examples,
    }
    out.write_text(json.dumps(summary, indent=1) + "\n")
    with open(str(out) + ".rows.jsonl", "w") as handle:
        for rec in records:
            handle.write(json.dumps(rec) + "\n")
    print(json.dumps(summary["classes"], indent=1))


if __name__ == "__main__":
    main()
