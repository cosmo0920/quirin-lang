#!/usr/bin/env python3
"""census.py CONFIG OUT_DIR [--jobs N] -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE:
every member access to struct flb_processor_instance, and every pointer
conversion into or out of `struct flb_processor_instance *`, read from Clang's
own AST (`-Xclang -ast-dump`, -fsyntax-only, each unit with its own recorded
command) over the units that can reach a processor instance:

  src/flb_processor.c, src/flb_help.c, src/fluent-bit.c,
  src/flb_notification.c, src/flb_metrics_exporter.c, every
  plugins/processor_* unit, and every tests/ or benchmarks/ unit whose source
  text names `processor`.

CONFIG is a configuration name of layout.py (corpus-66910c10, system-046accf6,
pins-046accf6, slice-34155955). A unit recorded twice (static and shared) is
read once, from its first recorded command; layout.py measured both variants
of src/flb_processor.c and they agree.

Member accesses are identified by the FieldDecl a MemberExpr names (its
address), so a same-named member of another record is never counted. Each is
classified from its parent node: write (left operand of `=`), read-write
(compound assignment, ++/--), read (lvalue-to-rvalue), address (`&`,
array decay, base of a further member access), or the parent's kind. The
location is the MemberExpr's begin, as Clang prints it (a macro argument at
its call site); `site` is the nearest preceding location in a .c file in dump
order, which places an access or conversion spelled in a header macro body.

Writes OUT_DIR/<CONFIG>.census.json and prints totals. Exit 0 only when every
unit's dump exited 0 (a unit that never sees the record definition is listed,
not failed)."""
import concurrent.futures
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import layout  # noqa: E402

RECORD = "flb_processor_instance"
PTR = f"struct {RECORD} *"
FIXED = {"src/flb_processor.c", "src/flb_help.c", "src/fluent-bit.c",
         "src/flb_notification.c", "src/flb_metrics_exporter.c"}
LOC = re.compile(r"(<invalid sloc>)|((?:/|<scratch space>|<built-in>|<command line>)[^\s<>,:']*):(\d+):(\d+)"
                 r"|\bline:(\d+):(\d+)|\bcol:(\d+)")
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
NODE = re.compile(r"^([| `]*)[|`]-(\w+) (0x[0-9a-f]+)?")
QUOTED = re.compile(r"'([^']*)'")
CASTS = {"ImplicitCastExpr", "CStyleCastExpr"}


def select(database, source):
    seen, units = set(), []
    for entry in json.load(open(database)):
        rel = os.path.relpath(entry["file"], source)
        if rel in seen or rel.startswith(".."):
            continue
        take = rel in FIXED or rel.startswith("plugins/processor_")
        if not take and (rel.startswith("tests/") or rel.startswith("benchmarks/")):
            try:
                take = "processor" in open(entry["file"], errors="replace").read()
            except OSError:
                take = False
        if take:
            seen.add(rel)
            units.append((rel, entry))
    return sorted(units, key=lambda u: u[0])


def norm(t):
    return re.sub(r"\b(const|volatile|restrict)\b", "", t).replace("  ", " ").strip()


def read_unit(args):
    rel, entry, source = args
    command = layout.base_command(entry) + ["-fsyntax-only", "-fno-color-diagnostics",
                                            "-Xclang", "-ast-dump", entry["file"]]
    proc = subprocess.Popen(command, cwd=entry["directory"], stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, text=True, errors="replace")
    state = {"file": None, "line": None, "site": None}
    fields = {}            # FieldDecl address -> member name
    record_depth = None    # depth of the RecordDecl being read
    stack = []             # [depth, kind, line_text, children, begin, index in parent]
    accesses, casts = [], []
    pending_cast = {}      # id(stack entry) -> cast info awaiting first child

    def scan(text):
        begin = None
        for m in LOC.finditer(STRING.sub('""', text)):
            if m.group(1):
                loc = ("<invalid>", 0, 0)
            elif m.group(2):
                state["file"], state["line"] = m.group(2), int(m.group(3))
                loc = (state["file"], state["line"], int(m.group(4)))
            elif m.group(5):
                state["line"] = int(m.group(5))
                loc = (state["file"], state["line"], int(m.group(6)))
            else:
                loc = (state["file"], state["line"], int(m.group(7)))
            if begin is None:
                begin = loc
            if loc[0] and loc[0].endswith(".c"):
                state["site"] = loc
        return begin

    for raw in proc.stdout:
        line = raw.rstrip("\n")
        m = NODE.match(line)
        if not m:
            scan(line)
            continue
        depth = (len(m.group(1)) + 2) // 2
        kind, addr = m.group(2), m.group(3)
        begin = scan(line[m.end():])
        while stack and stack[-1][0] >= depth:
            stack.pop()
        parent = stack[-1] if stack else None
        index = 0
        if parent is not None:
            index = parent[3]
            parent[3] += 1
        if record_depth is not None and depth <= record_depth:
            record_depth = None
        if kind == "RecordDecl" and line.rstrip().endswith(f"struct {RECORD} definition"):
            record_depth = depth
        elif record_depth is not None and depth == record_depth + 1 and kind == "FieldDecl":
            name = re.search(r" (\w+) '", line[m.end():])
            fields[addr] = name.group(1) if name else "?"
        node = [depth, kind, line, 0, begin, index]
        if parent is not None and parent[1] in CASTS and index == 0 and id(parent) in pending_cast:
            info = pending_cast.pop(id(parent))
            types = QUOTED.findall(line[m.end():])
            child_type = norm(types[0]) if types else "?"
            if child_type != info["to"] and (child_type == PTR or info["to"] == PTR):
                info["from"] = child_type
                casts.append(info)
        if kind in CASTS:
            types = QUOTED.findall(line[m.end():])
            ck = re.search(r"<(\w+)>", line[m.end():].split("'")[-1]) if types else None
            pending_cast[id(node)] = {"kind": kind, "to": norm(types[0]) if types else "?",
                                      "cast": ck.group(1) if ck else "?",
                                      "file": begin[0] if begin else None,
                                      "line": begin[1] if begin else None,
                                      "site": state["site"]}
        if kind == "MemberExpr":
            target = line.rstrip().split(" ")[-1]
            if target in fields:
                op = re.search(r" (->|\.)(\w+) 0x[0-9a-f]+$", line.rstrip())
                accesses.append({"member": fields[target], "op": op.group(1) if op else "?",
                                 "file": begin[0] if begin else None,
                                 "line": begin[1] if begin else None,
                                 "col": begin[2] if begin else None,
                                 "site": state["site"],
                                 "_chain": list(stack), "_index": index})
        stack.append(node)
    code = proc.wait()
    for a in accesses:
        chain, index = a.pop("_chain"), a.pop("_index")
        parent = chain[-1] if chain else None
        # Parentheses do not change the use: classify by the first other ancestor.
        while parent is not None and parent[1] == "ParenExpr" and len(chain) > 1:
            index = parent[5]
            chain = chain[:-1]
            parent = chain[-1]
        pk = parent[1] if parent else "?"
        pl = parent[2] if parent else ""
        if pk == "BinaryOperator" and "'='" in pl and index == 0:
            use = "write"
        elif pk == "CompoundAssignOperator" and index == 0:
            use = "read-write"
        elif pk == "UnaryOperator" and ("'++'" in pl or "'--'" in pl):
            use = "read-write"
        elif pk == "UnaryOperator" and "'&'" in pl:
            use = "address"
        elif pk == "ImplicitCastExpr" and "<LValueToRValue>" in pl:
            use = "read"
        elif pk == "ImplicitCastExpr" and "<ArrayToPointerDecay>" in pl:
            use = "address"
        elif pk == "MemberExpr":
            use = "address"
        elif pk == "UnaryExprOrTypeTraitExpr":
            use = "unevaluated"
        else:
            use = pk
        a["use"] = use
    for item in accesses + casts:
        if item["file"] and item["file"].startswith(source + "/"):
            item["file"] = item["file"][len(source) + 1:]
        site = item.get("site")
        if site:
            item["site"] = (site[0][len(source) + 1:] if site[0].startswith(source + "/") else site[0]) + f":{site[1]}"
    return {"unit": rel, "exit": code, "record_found": bool(fields), "fields": sorted(set(fields.values())),
            "accesses": accesses, "conversions": casts}


def main():
    config, out = sys.argv[1], os.path.abspath(sys.argv[2])
    jobs = int(sys.argv[sys.argv.index("--jobs") + 1]) if "--jobs" in sys.argv else 4
    root = dict(layout.CONFIGS)[config]
    database = os.path.join(root, "fluent-bit-clang-production/compile_commands.json")
    source = os.path.join(root, "fluent-bit-clang-source")
    units = select(database, source)
    with concurrent.futures.ThreadPoolExecutor(jobs) as pool:
        rows = list(pool.map(read_unit, [(rel, entry, source) for rel, entry in units]))
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, f"{config}.census.json"), "w") as handle:
        json.dump({"config": config, "units": rows}, handle, indent=1)
        handle.write("\n")
    total = sum(len(r["accesses"]) for r in rows)
    conv = sum(len(r["conversions"]) for r in rows)
    bad = [r["unit"] for r in rows if r["exit"] != 0]
    absent = [r["unit"] for r in rows if not r["record_found"]]
    print(f"{config}: {len(rows)} units, {total} member accesses, {conv} conversions; "
          f"failed: {bad}; record not in the unit: {len(absent)} {absent}")
    return 0 if not bad else 1


if __name__ == "__main__":
    raise SystemExit(main())
