"""K2: classify each 11-24 K2 leaf by its source text into a concrete C shape (lexical, census only).

usage: classify.py K2_SITES_JSON OUT_JSON
"""
import collections, json, re, sys

sites = json.load(open(sys.argv[1]))

RULES = [
    # (shape id, family set, regex on the leaf's source text, owner / reason)
    ("container-of", None, r"\b(mk_list_entry|cfl_list_entry|container_of)\s*\(", "pointer arithmetic deriving an enclosing record (`(T *)((char *)p - offsetof)`): an access derivation, not a non-access"),
    ("cast-address-actual", "IND", r"\(\s*(const\s+)?(char|unsigned char)\s*\*\*\s*\)\s*&", "a cast of an address actual (`(char **)&end`), CG08"),
    ("multi-declarator", "IND", r"^(const\s+)?[A-Za-z_][\w ]*\*\s*\w+\s*=[^;]*,\s*\*\s*\w+", "an initialized multi-pointer declarator"),
    ("sizeof-deref", None, r"sizeof\s*\(\s*\*", "`sizeof(*p)`: an unevaluated operand"),
    ("builtin-bitcount", None, r"__builtin_c[lt]z|\brd_clz\s*\(", "a `__builtin_clz`/`__builtin_ctz` bit count"),
    ("sizeof-type-condition", "COND", r"sizeof\s*\(\s*[A-Za-z_]\w*\s*\)\s*==", "`sizeof(T) == K` beside a scalar test"),
    ("indirect-call", "COND", r"(?<![\w>.])(cmp|cb|match|filter|may_assign)\s*\(", "a call through a formal or local function pointer (open set, rule 11p)"),
    ("assignment-in-condition", "COND", r"\(\s*\(?\s*(int\)\s*\()?\s*\w+\s*=[^=]", "an assignment in a condition (W44-CONDITION-CALL-ASSIGNMENT, rule 10x)"),
    ("buf-tags-macro", "COND", r"rd_kafka_buf_(skip|read)_tags", "the `rd_kafka_buf_*_tags` macro's loop (a goto-bearing `do`/`while`)"),
    ("unit-test-macro", None, r"RD_UT_ASSERT|verifyNonRackAwareAssignment|RD_UT_SAY", "a unit-test macro expansion"),
    ("pointer-compare", "COND", r"\b(pIn|ip|p)\s*<=?\s*\w+\s*-", "a pointer comparison against pointer arithmetic"),
    ("negated-call-condition", "COND", r"^if\s*\(\s*!\s*[A-Za-z_]\w*\s*\(", "a negated direct call `!f(...)` (a same-unit or cross-unit callee)"),
    ("openssl-stack", "COND", r"sk_X509\w*_num", "an OpenSSL `sk_*_num` stack macro (inline cast call)"),
    ("call-condition", "COND", r"\b[A-Za-z_]\w*\s*\(", "a direct call in a condition (other)"),
    ("call-in-scalar-return", "RET", r"^return\b.*\b[A-Za-z_]\w*\s*\(.*(%|&&|\|\||!=|==|\?|\^|\+|\*|>>)", "a call nested in a returned scalar expression"),
    ("call-in-return-other", "RET", r"^return\b.*\b[A-Za-z_]\w*\s*\(", "a whole-call or pointer-valued return"),
    ("call-actual-scalar-expr", "IND", r"\(\s*[^;]*\b\w+\s*(==|\?)", "a call actual that is a scalar comparison or `?:`"),
    ("pointer-decl-null", "IND", r"\*\s*\w+\s*=\s*NULL", "a pointer declarator initialized to NULL (other)"),
]

out = []
counts = collections.Counter()
rows_by = collections.defaultdict(set)
freed_by = collections.defaultdict(set)
for s in sites:
    text = s["text"]
    shape = "other"
    for sid, fams, rx, why in RULES:
        if fams and s["fam"] != fams:
            continue
        if re.search(rx, text):
            shape = sid
            break
    s["shape"] = shape
    counts[shape] += 1
    rows_by[shape].add((s["function"], s["unit"]))
    out.append(s)
# Rows freed alone by one shape: every K2 leaf of the row has that shape and nothing else blocks it.
by_row = collections.defaultdict(list)
for s in out:
    by_row[(s["function"], s["unit"])].append(s)
for key, ss in by_row.items():
    if ss[0]["others"]:
        continue
    shapes = {x["shape"] for x in ss}
    freed_by["+".join(sorted(shapes))].add(key)
why = {sid: w for sid, _, _, w in RULES}
table = [{"shape": k, "leaves": v, "rows": len(rows_by[k]), "why": why.get(k, "")} for k, v in counts.most_common()]
json.dump({"table": table, "freed_alone_by_shape": {k: sorted(map(list, v)) for k, v in freed_by.items()},
           "sites": out}, open(sys.argv[2], "w"), indent=1)
for t in table:
    print(f"{t['leaves']:4d} {t['rows']:4d}  {t['shape']}")
print()
for k, v in sorted(freed_by.items()):
    print(len(v), k, sorted(f for f, _ in v))
for s in out:
    if s["shape"] == "other":
        print("OTHER", s["fam"], s["origin"], s["text"][:120])
