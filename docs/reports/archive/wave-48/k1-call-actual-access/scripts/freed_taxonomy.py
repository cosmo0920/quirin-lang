#!/usr/bin/env python3
"""freed_taxonomy.py PP_CLASSES_ROWS OUT_JSON -- K1: a lexical taxonomy of the rows
K1 frees alone (pp_classify.py rows with freed_alone), by the analysed statement of
each of their K1 sites. A row's class set is the union over its sites; its primary
class is the first of PRIORITY present. Lexical description only (the gateway's
streams and the unit reproduction decide)."""
import collections
import json
import re
import sys

PATTERNS = [
    ("custody-position", re.compile(
        r"\b(\w*_destroy\w*|\w*_free|free|close|cfl_sds_destroy|rd_free|mk_mem_free|cfl_kvlist_destroy)\s*\(")),
    ("nested-op-err-destroy", re.compile(r"\brd_kafka_op_err_destroy\s*\(")),
    ("call-through-formal", re.compile(r"(^|[^\w>.])(cmp|compare)\s*\(")),
    ("function-pointer-actual", re.compile(
        r"[(,]\s*(\(void \*\))?\s*(\w+_cb|\w+_free|rd_free|\w+_cmp\w*|\w+_hash|\w+_opaque|\w+_cmp_str)\s*[,)]"
        r"|msg_order_cmp")),
    ("log-or-format", re.compile(r"\b(rd_kafka_log0|flb_log_print|fprintf|printf|snprintf)\s*\(")),
    ("parenthesised-member-address", re.compile(r"&\s*\(\s*[A-Za-z_]\w*\s*\)\s*->")),
    ("conditional-actual", re.compile(r"\?")),
    ("cast-actual", re.compile(r"[(,]\s*\((const |unsigned )*\w+\s*\*+\)")),
    ("sizeof-actual", re.compile(r"\bsizeof\b")),
    ("nested-call-actual", re.compile(r"\w+\s*\([^()]*\b\w+\s*\(")),
]
PRIORITY = [name for name, _ in PATTERNS] + ["other"]


def main():
    rows_path, out = sys.argv[1:3]
    rows = collections.defaultdict(set)
    texts = collections.defaultdict(list)
    for line in open(rows_path):
        r = json.loads(line)
        if not r["freed_alone"]:
            continue
        key = (r["file"], r["function"])
        found = {name for name, rx in PATTERNS if rx.search(r["text"])} or {"other"}
        rows[key] |= found
        texts[key].append(r["text"][:160])
    primary = collections.Counter()
    members = collections.defaultdict(list)
    for key, found in rows.items():
        p = next(name for name in PRIORITY if name in found)
        primary[p] += 1
        members[p].append(key[1])
    summary = {
        "functions": len(rows),
        "primary": dict(primary.most_common()),
        "any": dict(collections.Counter(c for found in rows.values() for c in found).most_common()),
        "members": {k: sorted(v) for k, v in members.items()},
    }
    open(out, "w").write(json.dumps(summary, indent=1) + "\n")
    print(json.dumps({k: summary[k] for k in ("functions", "primary", "any")}, indent=1))


if __name__ == "__main__":
    main()
