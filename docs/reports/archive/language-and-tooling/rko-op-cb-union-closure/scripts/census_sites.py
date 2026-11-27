#!/usr/bin/env python3
"""census_sites.py DATABASE OUT.json UNIT... -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION:
a reading aid that locates, in each named librdkafka unit (a path suffix of a
compile-database entry), the stores rule 12n's member-facts collector
(`callback_member_closure::MemberFacts::walk`) reads as opening:

* a whole-object assignment `P = V` of a record whose value V is a call or an
  object of another record (`a whole-object assignment`);
* a `memcpy`-family call into a pointer to a record (`a `memmove` into it`);
* a function-pointer actual of a direct call that is a C-style cast (the
  `a `CStyleCastExpr`` flow of a formal).

For each site it prints the file, line, the target expression's spelling (a
member chain with its record types, or a variable) and whether the chain
passes through a union. Clang's JSON AST of the unit's own recorded command
(`-fsyntax-only -Xclang -ast-dump=json`). Not evidence by itself: every site
it lists is read by hand in the report."""
import json
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


def spell(node):
    """A member chain's spelling with each base's type."""
    node = strip(node)
    kind = node.get("kind")
    if kind == "MemberExpr":
        base = node["inner"][0]
        arrow = "->" if node.get("isArrow") else "."
        return f"{spell(base)}{arrow}{node.get('name')}"
    if kind == "DeclRefExpr":
        decl = node.get("referencedDecl", {})
        return f"{decl.get('name')}<{decl.get('kind')}:{qual(node)}>"
    if kind == "UnaryOperator":
        return f"{node.get('opcode')}({spell(node['inner'][0])})"
    if kind == "ArraySubscriptExpr":
        return f"{spell(node['inner'][0])}[...]"
    return f"<{kind}:{qual(node)}>"


def through_union(node):
    node = strip(node)
    while node.get("kind") in ("MemberExpr", "UnaryOperator", "ArraySubscriptExpr"):
        base = strip(node["inner"][0])
        text = qual(base)
        if node.get("kind") == "MemberExpr" and text.startswith("union"):
            return True
        node = base
    return False


class Walker:
    def __init__(self, main_file):
        self.main = main_file
        self.file = None
        self.line = None
        self.sites = []
        self.function = None

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

    def add(self, what, node, target, value=None):
        self.sites.append({
            "what": what,
            "file": self.file,
            "line": self.line,
            "function": self.function,
            "target": spell(target),
            "target_type": qual(strip(target)),
            "through_union": through_union(target),
            "value": None if value is None else spell(value),
            "value_kind": None if value is None else strip(value).get("kind"),
        })

    def walk(self, node):
        if not isinstance(node, dict):
            return
        self.loc(node)
        kind = node.get("kind")
        if kind == "FunctionDecl" and any(c.get("kind") == "CompoundStmt" for c in node.get("inner", [])):
            self.function = node.get("name")
        if kind == "BinaryOperator" and node.get("opcode") == "=":
            left, right = node["inner"]
            lt = qual(strip(left))
            if (lt.startswith("struct ") or lt.startswith("union ")) and "*" not in lt:
                rs = strip(right)
                if rs.get("kind") == "CallExpr" or qual(rs) != lt:
                    self.add("whole-object assignment", node, left, right)
        if kind == "CallExpr":
            inner = node.get("inner", [])
            callee = strip(inner[0]) if inner else {}
            name = callee.get("referencedDecl", {}).get("name") if callee.get("kind") == "DeclRefExpr" else None
            if name in COPIERS and len(inner) > 1:
                dt = qual(strip(inner[1]))
                if "struct " in dt or "union " in dt:
                    self.add(f"{name} into a record", node, inner[1])
            if name:
                for index, argument in enumerate(inner[1:]):
                    if strip(argument).get("kind") == "CStyleCastExpr" and "(*)" in qual(argument):
                        self.add(f"cast actual {index} of {name}", node, argument, argument)
        for child in node.get("inner", []):
            self.walk(child)


def main():
    database, out, *units = sys.argv[1:]
    entries = json.load(open(database))
    result = {}
    for unit in units:
        entry = next(e for e in entries if e["file"].endswith(unit))
        proc = subprocess.run(command_for(entry), cwd=entry["directory"], capture_output=True)
        if proc.returncode != 0:
            result[unit] = {"error": proc.stderr.decode(errors="replace")[:2000]}
            continue
        ast = json.loads(proc.stdout)
        walker = Walker(entry["file"])
        walker.walk(ast)
        result[unit] = walker.sites
    json.dump(result, open(out, "w"), indent=1)


if __name__ == "__main__":
    main()
