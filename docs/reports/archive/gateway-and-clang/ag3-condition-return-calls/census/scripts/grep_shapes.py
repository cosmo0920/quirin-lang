"""Count named lexical sub-shapes over AG3 sites (rows mentioning / sole rows)."""
import json
import re
import sys
from collections import defaultdict

sites = json.load(open(sys.argv[1]))
PATTERNS = {
    "errno": r"\berrno\b",
    "ctype (isspace/isdigit/...)": r"\bis(space|digit|alpha|alnum|upper|lower|xdigit|print|blank|punct|cntrl|graph)\s*\(",
    "tolower/toupper": r"\bto(lower|upper)\s*\(",
    "PB_RETURN_ERROR": r"PB_RETURN_ERROR",
    "rd_assert": r"\brd_assert\s*\(",
    "RD_LIST_FOREACH": r"RD_LIST_FOREACH",
    "rd_list_cnt/rd_list_elem": r"\brd_list_(cnt|elem|empty)\b",
    "TAILQ_/LIST_ macro": r"\b(TAILQ|LIST|STAILQ|SIMPLEQ|CIRCLEQ)_[A-Z_]+\s*\(",
    "mk_list/cfl_list foreach": r"(mk|cfl|flb)_list_foreach",
    "strcmp/strncmp/strcasecmp": r"\bstr(n)?(case)?cmp\s*\(",
    "unlikely/likely": r"\b(un)?likely\s*\(",
    "__builtin_*": r"__builtin_\w+",
    "call through member (->f( / .f()": r"(->|\.)\s*\w+\s*\(",
    "address-of member return": r"^&\s*\w+\s*->",
    "RD_KAFKAP_STR_*": r"RD_KAFKAP_STR_",
    "rd_atomic": r"rd_atomic\d+_",
    "mpack_": r"\bmpack_\w+\s*\(",
    "msgpack_": r"\bmsgpack_\w+\s*\(",
}
mention = defaultdict(set)
sole = defaultdict(set)
sitecount = defaultdict(int)
for s in sites:
    e = s["expr"] or ""
    for name, pat in PATTERNS.items():
        if re.search(pat, e):
            key = (s["function"], s["origin"].rsplit(":", 1)[0] if s["origin"] else "")
            mention[name].add(key)
            sitecount[name] += 1
            if s["sole"]:
                sole[name].add(key)
for name in sorted(PATTERNS, key=lambda n: -len(sole[n])):
    print(f"{name:40s} sites {sitecount[name]:5d} rows {len(mention[name]):5d} sole {len(sole[name]):3d}")
