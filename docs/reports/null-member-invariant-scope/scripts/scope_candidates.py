#!/usr/bin/env python3
"""scope_candidates.py ROWS.jsonl OUT.qv OUT.json -- NULL-MEMBER-INVARIANT-SCOPE:
choose the functions a `null_member_scope(rd_kafka_broker_s, rkb_rk, rkb)`
entry is declared on, from the stable reading's run (a1), and write their
bounds v1 declarations.

A function is chosen when the run's row has a candidate whose only source is
`rkb->rkb_rk` on its own checks, it is defined in librdkafka with a formal
`rd_kafka_broker_t *rkb` (the first parameter), its definition is
byte-identical at both audited revisions, its address is not taken in the
library (an indirect caller is never checked), and every parameter has a
bounds v1 type. The C text is read at 66910c10 and 046accf6."""
import json
import re
import subprocess
import sys

OLD = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
NEW = "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"
SRC = "lib/librdkafka-2.15.0/src"

SCALARS = {
    "int": "i32", "int32_t": "i32", "int16_t": "i16", "int8_t": "i8", "int64_t": "i64",
    "uint32_t": "u32", "uint16_t": "u16", "uint8_t": "u8", "uint64_t": "u64",
    "size_t": "usize", "ssize_t": "isize", "rd_bool_t": "i32", "rd_ts_t": "i64",
    "unsigned int": "u32", "unsigned": "u32", "long": "i64", "char": "i8",
}


def param_type(text):
    text = re.sub(r"\b(const|volatile|RD_UNUSED|__restrict|restrict)\b", lambda m: m.group(0) if m.group(0) == "const" else "", text).strip()
    if "(" in text or "[" in text or "..." in text:
        return None
    pointer = text.count("*")
    base = text.replace("*", " ").split()
    if not base:
        return None
    name = base[-1]
    words = [w for w in base[:-1] if w != "struct"]
    if pointer == 1:
        return name, "ptr<const, u8>" if words and words[0] == "const" else "ptr<mut, u8>"
    if pointer > 1:
        return None
    words = [w for w in words if w != "const"]
    key = " ".join(words)
    if key in SCALARS:
        return name, SCALARS[key]
    if key.startswith("enum ") or key in enum_typedefs():
        return name, "i32"
    return None


ENUMS = None


def enum_typedefs():
    """Every `typedef enum ... } NAME;` and `typedef enum TAG NAME;` of the
    library's sources (an enumeration passes as an int)."""
    global ENUMS
    if ENUMS is None:
        ENUMS = set()
        import glob
        for path in glob.glob(f"{OLD}/{SRC}/*.h") + glob.glob(f"{OLD}/{SRC}/*.c"):
            text = open(path, errors="replace").read()
            for m in re.finditer(r"typedef\s+enum\b[^;{]*\{[^}]*\}\s*(\w+)\s*;", text):
                ENUMS.add(m.group(1))
            for m in re.finditer(r"typedef\s+enum\s+\w+\s+(\w+)\s*;", text):
                ENUMS.add(m.group(1))
    return ENUMS


def definition(root, name):
    out = subprocess.run(["grep", "-rlE", r"^[a-z].*\b%s\(" % name, f"{root}/{SRC}"], capture_output=True, text=True).stdout.split()
    for path in out:
        if not path.endswith(".c"):
            continue
        text = open(path, errors="replace").read()
        m = re.search(r"^([A-Za-z_][^;{}()]*?)\b%s\s*\(([^;{}]*)\)\s*\{" % re.escape(name), text, re.M)
        if not m:
            continue
        start = m.start()
        depth = 0
        for i in range(m.end() - 1, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    return path[len(root) + 1:], m.group(1), m.group(2), text[start:i + 1]
    return None


def address_taken(name):
    out = subprocess.run(["grep", "-rnwE", name, f"{OLD}/{SRC}"], capture_output=True, text=True).stdout
    for line in out.splitlines():
        code = line.split(":", 2)[2]
        if re.search(r"\b%s\s*\(" % name, code):
            continue
        if re.search(r"^\s*(static\s+)?.*\b%s\b.*;\s*$" % name, code) and "(" in code:
            continue
        return True
    return False


rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
chosen = []
skipped = {}
for row in rows:
    if row["library"] != "librdkafka":
        continue
    checks = (row.get("null") or {}).get("checks") or []
    n = sum(1 for c in checks if c["outcome"] == "candidate" and c.get("sources") == ["memory-field:rkb->rkb_rk"]
            and "cross-unit" not in c["subject"])
    if not n:
        continue
    name = row["function"]
    old = definition(OLD, name)
    new = definition(NEW, name)
    if not old or not new:
        skipped[name] = "no definition found"
        continue
    if old[3] != new[3] or old[0] != new[0]:
        skipped[name] = "definition differs between revisions"
        continue
    result, params = old[1], old[2]
    if "static" in result.split():
        everywhere = subprocess.run(["grep", "-rlE", r"^[a-z].*\b%s\(" % name, f"{OLD}/lib"],
                                    capture_output=True, text=True).stdout.split()
        if len([p for p in everywhere if p.endswith((".c", ".h"))]) != 1:
            skipped[name] = "static and defined (or named) in more than one file"
            continue
    parts = [p.strip() for p in params.split(",")] if params.strip() not in ("", "void") else []
    if not parts or not re.match(r"^rd_kafka_broker_t\s*\*\s*rkb$", parts[0]):
        skipped[name] = "the first parameter is not rd_kafka_broker_t *rkb"
        continue
    typed = [param_type(p) for p in parts]
    if any(t is None for t in typed):
        skipped[name] = "a parameter without a bounds v1 type: " + params
        continue
    if address_taken(name):
        skipped[name] = "address taken (an indirect caller is never checked)"
        continue
    res = " ".join(w for w in result.split() if w not in ("static", "inline", "RD_INLINE", "RD_UNUSED", "RD_WARN_UNUSED_RESULT"))
    if res == "void":
        rtype = "unit"
    elif "*" in res:
        rtype = "ptr<const, u8>" if res.startswith("const") else "ptr<mut, u8>"
    else:
        rt = param_type(res + " x")
        if rt is None:
            skipped[name] = "result without a bounds v1 type: " + res
            continue
        rtype = rt[1]
    chosen.append({"function": name, "file": old[0], "candidates": n,
                   "signature": ", ".join(f"{a}: {b}" for a, b in typed), "result": rtype})

with open(sys.argv[2], "w") as out:
    for c in sorted(chosen, key=lambda c: c["function"]):
        out.write(f"// {c['file']}: {c['candidates']} first read(s) of rkb->rkb_rk on the stable reading.\n")
        out.write(f"extern fn {c['function']}({c['signature']}) -> {c['result']}\ncontract {{\n    requires bounds_contract_v1;\n    requires null_member_scope(rd_kafka_broker_s, rkb_rk, rkb);\n}};\n\n")
json.dump({"chosen": sorted(chosen, key=lambda c: c["function"]), "skipped": dict(sorted(skipped.items()))},
          open(sys.argv[3], "w"), indent=1)
print(len(chosen), "chosen,", sum(c["candidates"] for c in chosen), "candidates;", len(skipped), "skipped")
