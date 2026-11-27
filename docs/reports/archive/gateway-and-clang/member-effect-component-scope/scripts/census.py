#!/usr/bin/env python3
"""MEMBER-EFFECT-COMPONENT-SCOPE census: every rule-38i release leaf
(`member-release-through-formal`, `member-address-release-through-formal`)
of one resource-diagnostics report, classified by the declared spelling of
its base and member, and by whether its function has any other leaf.

Not the analyzer. The member and base spellings are read lexically from the
source tree the run analysed (the first `struct TAG {` definition found under
the root, the function's own parameter list), so a class is a reading of the
source, labelled as such; the analyzer decides on the preprocessed unit.

The row scope (`hypothesis_context.scope`) is also tallied: it is the list of
the unit's source functions, not a call-graph component.

usage: census.py REPORT_JSON SOURCE_ROOT [--current PREMISES_JSONL_GZ]
Prints one JSON document on stdout.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

NAMES = ("member-release-through-formal", "member-address-release-through-formal")
LEAF = re.compile(r"release of `&?(\w+)->(\w+)[^`]*`")


class Tree:
    """Lexical declarations of one source tree, read lazily."""

    def __init__(self, root: pathlib.Path):
        self.root = root
        self._files = None
        self._texts = {}
        self._structs = {}
        self._typedefs = None

    def files(self):
        if self._files is None:
            self._files = [
                path
                for path in self.root.rglob("*")
                if path.suffix in (".c", ".h")
                and path.is_file()
                and "build" not in path.relative_to(self.root).parts
            ]
        return self._files

    def text(self, path):
        if path not in self._texts:
            raw = path.read_text(errors="replace")
            raw = re.sub(r"/\*.*?\*/", " ", raw, flags=re.S)
            raw = re.sub(r"//[^\n]*", " ", raw)
            self._texts[path] = raw
        return self._texts[path]

    def find(self, name: str, library_hint: str):
        """The analysed file: `name` is the report's unit file."""
        candidates = [p for p in self.files() if str(p).endswith("/" + name)]
        if len(candidates) > 1 and library_hint:
            narrowed = [p for p in candidates if library_hint in str(p)]
            candidates = narrowed or candidates
        return candidates[0] if candidates else None

    def typedefs(self):
        if self._typedefs is None:
            table = collections.defaultdict(set)
            for path in self.files():
                for match in re.finditer(r"\btypedef\s+([^;{}]*?)\s*\b(\w+)\s*;", self.text(path)):
                    table[match.group(2)].add(" ".join(match.group(1).split()))
            self._typedefs = table
        return self._typedefs

    def struct_body(self, tag: str):
        if tag not in self._structs:
            body = None
            for path in self.files():
                match = re.search(r"\bstruct\s+" + re.escape(tag) + r"\s*\{", self.text(path))
                if match:
                    text = self.text(path)
                    depth, at = 0, match.end() - 1
                    for index in range(at, len(text)):
                        if text[index] == "{":
                            depth += 1
                        elif text[index] == "}":
                            depth -= 1
                            if depth == 0:
                                body = text[at + 1 : index]
                                break
                    break
            self._structs[tag] = body
        return self._structs[tag]


def parameter_spelling(text: str, function: str, base: str):
    """The declaration of `base` in `function`'s parameter list."""
    for match in re.finditer(r"\b" + re.escape(function) + r"\s*\(([^;{]*?)\)\s*\{", text, re.S):
        for parameter in match.group(1).split(","):
            parameter = " ".join(parameter.split())
            if re.search(r"\b" + re.escape(base) + r"$", parameter):
                return parameter
    return None


def base_class(tree: Tree, spelling):
    """('struct', TAG) for `struct TAG *B`; ('typedef', TAG|None) otherwise."""
    if spelling is None:
        return ("unread", None)
    tokens = spelling.replace("*", " * ").split()
    tokens = [t for t in tokens if t not in ("const", "volatile", "restrict", "register")]
    if len(tokens) >= 4 and tokens[0] == "struct" and tokens[2] == "*" and tokens.count("*") == 1:
        return ("struct", tokens[1])
    if len(tokens) == 3 and tokens[1] == "*":
        for target in tree.typedefs().get(tokens[0], ()):
            found = re.fullmatch(r"struct\s+(\w+)", target)
            if found:
                return ("typedef-record", found.group(1))
        return ("typedef-record", None)
    if len(tokens) == 2:
        for target in tree.typedefs().get(tokens[0], ()):
            found = re.fullmatch(r"struct\s+(\w+)\s*\*", target)
            if found:
                return ("typedef-record-pointer", found.group(1))
        return ("typedef-record-pointer", None)
    return ("other", None)


def member_class(tree: Tree, tag, member: str):
    if tag is None:
        return "untagged"
    body = tree.struct_body(tag)
    if body is None:
        return "no-definition"
    depth, start, declarations = 0, 0, []
    for index, char in enumerate(body):
        if char in "{(":
            depth += 1
        elif char in "})":
            depth -= 1
        elif char == ";" and depth == 0:
            declarations.append(" ".join(body[start:index].split()))
            start = index + 1
    for declaration in declarations:
        if not re.search(r"\b" + re.escape(member) + r"\b\s*(\[[^\]]*\]\s*)*(:\s*\w+\s*)?(,|$)", declaration):
            continue
        if "(" in declaration:
            return "function-pointer"
        head = declaration.split(",")[0]
        if "[" in head:
            return "array"
        if "*" in declaration:
            return "star-spelled pointer"
        words = [w for w in head.split() if w not in ("const", "volatile", "unsigned", "signed")]
        if len(words) == 2:
            targets = tree.typedefs().get(words[0], set())
            if any(t.endswith("*") for t in targets):
                return "pointer typedef"
            if targets:
                return "other typedef"
            return "unresolved name"
        return "other"
    return "member not found"


def census(report_path, root):
    report = json.loads(pathlib.Path(report_path).read_text())
    tree = Tree(pathlib.Path(root))
    leaves = []
    scopes = collections.Counter()
    seen = set()
    for unit in report["units"]:
        # A report may list one unit more than once (the system route
        # analyses some files twice); an identical row is counted once.
        path = None
        for function in unit["functions"]:
            key = (unit["file"], function["function"], function["status"],
                   json.dumps(function["diagnostics"], sort_keys=True))
            if key in seen:
                continue
            seen.add(key)
            scope = json.loads(function["hypothesis_context"]["scope"])
            scopes["one function" if len(scope) == 1 else "several functions"] += 1
            named = [
                d for d in function["diagnostics"]
                if any(n in d.get("description", "") for n in NAMES)
            ]
            if not named:
                continue
            if path is None:
                path = tree.find(unit["file"], unit.get("library", ""))
            text = tree.text(path) if path else ""
            sole = len(named) == len(function["diagnostics"])
            for diagnostic in named:
                found = LEAF.search(diagnostic["description"])
                base, member = found.groups() if found else (None, None)
                b_kind, tag = base_class(tree, parameter_spelling(text, function["function"], base))
                leaves.append({
                    "file": unit["file"],
                    "function": function["function"],
                    "status": function["status"],
                    "origin": diagnostic.get("origin"),
                    "name": next(n for n in NAMES if n in diagnostic["description"]),
                    "base": b_kind,
                    "member": member_class(tree, tag, member) if member else "unparsed",
                    "sole": sole,
                })
    return leaves, scopes


def table(leaves):
    rows = collections.defaultdict(lambda: [0, set(), set()])
    for leaf in leaves:
        key = (leaf["base"], leaf["member"])
        rows[key][0] += 1
        rows[key][1].add((leaf["file"], leaf["function"]))
        if leaf["sole"]:
            rows[key][2].add((leaf["file"], leaf["function"]))
    return [
        {
            "base": base,
            "member": member,
            "leaves": count,
            "functions": len(functions),
            "sole_blocker_functions": sorted("::".join(f) for f in sole),
        }
        for (base, member), (count, functions, sole) in sorted(rows.items())
    ]


def main(argv):
    report, root = argv[1], argv[2]
    leaves, scopes = census(report, root)
    print(json.dumps({
        "report": report,
        "source_root": root,
        "row_scopes": dict(scopes),
        "leaves": len(leaves),
        "functions": len({(l["file"], l["function"]) for l in leaves}),
        "table": table(leaves),
        "rows": leaves,
    }, indent=1))


if __name__ == "__main__":
    main(sys.argv)
