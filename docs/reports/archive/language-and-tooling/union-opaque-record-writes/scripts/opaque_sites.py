#!/usr/bin/env python3
"""opaque_sites.py DATABASE OUT.json RECORDS UNIT... -- UNION-OPAQUE-RECORD-WRITES-BY-TYPE
(adapted from RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION's census_sites.py): a
reading aid that locates, in each named unit (a path suffix of a compile-database
entry, or `ALL:<substring>` for every entry whose path contains it), every
opaque write rule 12n's member-facts collector
(`callback_member_closure::MemberFacts::walk`) reads as opening a record by
type (`MemberFacts::opened`), restricted to the records named in RECORDS (a
comma list of tags, `*` for all):

* a whole-object assignment `P = V` of a record whose value V is a call or an
  object of another record (`a whole-object assignment`);
* a `memcpy`-family call into a pointer to a record (`a `memmove` into it`).

For each site: file, line, enclosing function, the target's spelling (a member
chain with each variable's kind and type), the record written, whether the
chain passes through a union, and the target's root: `local` / `global` /
`parameter` (an object by value), `arrow <record>` (a member path below a
pointer to a record), or `untargeted` (a dereference, an array element, a
pointer handed to a copier). Clang's JSON AST of the unit's own recorded
command (`-fsyntax-only -Xclang -ast-dump=json`). Not evidence by itself:
every site it lists is read by hand in the report."""
import json
import re
import shlex
import subprocess
import sys

COPIERS = {"memcpy", "memmove", "bcopy", "__builtin_memcpy", "__builtin_memmove",
           "__builtin___memcpy_chk"}


def command_for(entry):
    words = shlex.split(entry["command"]) if "command" in entry else list(entry["arguments"])
    out, skip = [], False
    for word in words:
        if skip:
            skip = False
            continue
        if word == "-o":
            skip = True
            continue
        if word == "-c":
            continue
        out.append(word)
    return out + ["-fsyntax-only", "-Xclang", "-ast-dump=json"]


def qual(node):
    t = node.get("type", {})
    return t.get("desugaredQualType") or t.get("qualType") or ""


def strip(node):
    while node.get("kind") in ("ParenExpr", "ImplicitCastExpr") and node.get("inner"):
        node = node["inner"][0]
    return node


TYPEDEFS = {}


def record_of(text, pointee=False):
    text = text.strip()
    for q in ("const ", "volatile "):
        text = text.replace(q, "")
    if pointee:
        if not text.endswith("*"):
            return None
        text = text[:-1].strip()
    if "*" in text or "[" in text or "(" in text and "unnamed" not in text:
        return None
    match = re.match(r"(struct|union) ([A-Za-z_][A-Za-z0-9_]*)", text)
    if match:
        return match.group(2)
    return TYPEDEFS.get(text.strip())


def spell(node):
    node = strip(node)
    kind = node.get("kind")
    if kind == "MemberExpr":
        base = node["inner"][0]
        arrow = "->" if node.get("isArrow") else "."
        return f"{spell(base)}{arrow}{node.get('name')}"
    if kind == "DeclRefExpr":
        decl = node.get("referencedDecl", {})
        return f"{decl.get('name')}<{decl.get('kind')}>"
    if kind == "UnaryOperator":
        return f"{node.get('opcode')}({spell(node['inner'][0])})"
    if kind == "ArraySubscriptExpr":
        return f"{spell(node['inner'][0])}[...]"
    if kind == "CStyleCastExpr":
        return f"({qual(node)})({spell(node['inner'][0])})"
    return f"<{kind}:{qual(node)}>"


def chain(node):
    """The member path from the target down to its root, and the root."""
    names, through_union = [], False
    node = strip(node)
    while node.get("kind") == "MemberExpr":
        names.append(node.get("name"))
        base = strip(node["inner"][0])
        if not node.get("isArrow") and qual(base).startswith("union"):
            through_union = True
        if node.get("isArrow"):
            return list(reversed(names)), through_union, ("arrow", record_of(qual(base), True))
        node = base
    if node.get("kind") == "DeclRefExpr":
        decl = node.get("referencedDecl", {})
        kinds = {"VarDecl": "variable", "ParmVarDecl": "parameter"}
        return list(reversed(names)), through_union, (kinds.get(decl.get("kind"), decl.get("kind")), decl.get("name"))
    return list(reversed(names)), through_union, ("untargeted", node.get("kind"))


def addressed(node):
    """The place `&P` (or a decayed array `P`) names, else None."""
    while True:
        kind = node.get("kind")
        if kind == "ParenExpr":
            node = node["inner"][0]
        elif kind == "ImplicitCastExpr" and node.get("castKind") == "ArrayToPointerDecay":
            return node["inner"][0]
        elif kind in ("ImplicitCastExpr", "CStyleCastExpr"):
            node = node["inner"][0]
        elif kind == "UnaryOperator" and node.get("opcode") == "&":
            return node["inner"][0]
        else:
            return None


class Walker:
    def __init__(self, records):
        self.records = records
        self.file = None
        self.line = None
        self.sites = []
        self.function = None
        self.storage = {}

    def loc(self, node):
        for key in ("loc", "range"):
            loc = node.get(key, {})
            if key == "range":
                loc = loc.get("begin", {})
            loc = loc.get("expansionLoc", loc)
            if "file" in loc:
                self.file = loc["file"]
            if "line" in loc:
                self.line = loc["line"]

    def wanted(self, record):
        return record is not None and ("*" in self.records or record in self.records)

    def add(self, what, record, target, value=None):
        names, through_union, root = chain(target) if target is not None else ([], False, ("untargeted", None))
        if root[0] == "variable":
            root = ("local" if self.function else "global", root[1])
        self.sites.append({
            "what": what,
            "record": record,
            "file": self.file,
            "line": self.line,
            "function": self.function,
            "target": spell(target) if target is not None else None,
            "path": names,
            "through_union": through_union,
            "root": list(root),
            "value": None if value is None else spell(value),
            "value_kind": None if value is None else strip(value).get("kind"),
        })

    def walk(self, node):
        if not isinstance(node, dict):
            return
        self.loc(node)
        kind = node.get("kind")
        if kind == "TypedefDecl" and node.get("name"):
            target = record_of(qual(node))
            if target:
                TYPEDEFS[node["name"]] = target
        if kind == "FunctionDecl" and any(c.get("kind") == "CompoundStmt" for c in node.get("inner", [])):
            saved = self.function
            self.function = node.get("name")
            for child in node.get("inner", []):
                self.walk(child)
            self.function = saved
            return
        if kind == "BinaryOperator" and node.get("opcode") == "=":
            left, right = node["inner"]
            place = strip(left)
            record = record_of(qual(place)) or record_of(place.get("type", {}).get("qualType", ""))
            if record and "(unnamed" not in qual(place):
                source = strip(right)
                if source.get("kind") == "CallExpr" or record_of(qual(source)) != record:
                    if self.wanted(record):
                        self.add("whole-object assignment", record, left, right)
        if kind == "CallExpr":
            inner = node.get("inner", [])
            callee = strip(inner[0]) if inner else {}
            name = callee.get("referencedDecl", {}).get("name") if callee.get("kind") == "DeclRefExpr" else None
            if name in COPIERS and len(inner) > 1:
                record = record_of(qual(strip(inner[1])), True)
                if self.wanted(record):
                    place = addressed(inner[1])
                    self.add(f"`{name}` into it", record, place if place is not None else None, None)
                    if place is None:
                        self.sites[-1]["target"] = spell(inner[1])
        for child in node.get("inner", []):
            self.walk(child)


def main():
    database, out, records, *units = sys.argv[1:]
    records = set(records.split(","))
    entries = json.load(open(database))
    chosen = []
    for unit in units:
        if unit.startswith("ALL:"):
            chosen += [e for e in entries if unit[4:] in e["file"] and e["file"].endswith(".c")]
        else:
            chosen.append(next(e for e in entries if e["file"].endswith(unit)))
    result = {}
    for entry in chosen:
        proc = subprocess.run(command_for(entry), cwd=entry["directory"], capture_output=True)
        if proc.returncode != 0:
            result[entry["file"]] = {"error": proc.stderr.decode(errors="replace")[:2000]}
            continue
        ast = json.loads(proc.stdout)
        TYPEDEFS.clear()
        walker = Walker(records)
        walker.walk(ast)
        result[entry["file"]] = walker.sites
    json.dump(result, open(out, "w"), indent=1)
    for unit, sites in sorted(result.items()):
        if isinstance(sites, dict):
            print("ERROR", unit, sites["error"][:200])
            continue
        for site in sites:
            print(f"{site['file'].split('/')[-1] if site['file'] else unit.split('/')[-1]}:{site['line']}"
                  f" {site['function']} {site['what']} {site['record']} target={site['target']}"
                  f" root={site['root']} union={site['through_union']}")


if __name__ == "__main__":
    main()
