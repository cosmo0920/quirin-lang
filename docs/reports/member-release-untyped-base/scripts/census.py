#!/usr/bin/env python3
"""MEMBER-RELEASE-UNTYPED-BASE census: every rule-38i release leaf whose base
is a formal spelled through a typedef, with the base's spelling, whether the
lexical declaration reader leaves that base untyped (its
`Refusal::UntypedBase` at a release actual `B->F`) or resolves it, the
member's spelling, the release callee's mode, and every other blocker of the
function.

Not the analyzer. Spellings are read lexically from the source tree the run
analysed with MEMBER-EFFECT-COMPONENT-SCOPE's reader (`census.py` there),
refined here for a typedef of an untagged record (`typedef struct { … } T;`)
and a typedef of a record pointer. The callee's release mode is read from the
provider contracts (`resource_released(P, nullable|nonnull)`, following
`contract inherit`); a callee no provider declares is labelled `unit`.

usage:
  census.py report ROUTE REPORT_JSON SOURCE_ROOT PROVIDERS_DIR
  census.py current PREMISES_JSONL_GZ SOURCE_ROOT PROVIDERS_DIR
Prints one JSON document on stdout.
"""

import collections
import gzip
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPORTS = HERE.parent.parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


parent = load("census", REPORTS / "member-effect-component-scope/scripts/census.py")
estimate = load("estimate_step1", REPORTS / "member-effect-summary/scripts/estimate_step1.py")

NAMES = parent.NAMES


def release_modes(root: pathlib.Path):
    """`callee -> mode` for every provider declaration that releases."""
    direct, inherits = {}, {}
    for path in root.glob("*/contracts/**/*.qv"):
        text = re.sub(r"//[^\n]*", "", path.read_text(errors="replace"))
        for match in re.finditer(
            r"extern\s+fn\s+(\w+)\s*\([^)]*\)[^\n]*?contract\s+inherit\s+(?:\w+::)?(\w+)\s*;", text
        ):
            inherits[match.group(1)] = match.group(2)
        # A declaration runs to the next `extern fn` (a contract body may hold
        # `};` inside a quantified proposition).
        starts = [m for m in re.finditer(r"extern\s+fn\s+(\w+)", text)]
        for at, match in enumerate(starts):
            end = starts[at + 1].start() if at + 1 < len(starts) else len(text)
            modes = re.findall(r"resource_released\(\s*\w+\s*,\s*(\w+)", text[match.start():end])
            if modes:
                direct.setdefault(match.group(1), set()).update(modes)
    direct = {name: "/".join(sorted(modes)) for name, modes in direct.items()}
    def mode(name, depth=0):
        if name in direct:
            return direct[name]
        if name in inherits and depth < 8:
            return mode(inherits[name], depth + 1)
        return None
    return mode


class Tree(parent.Tree):
    def raw(self, path):
        """The file's text with its line numbering kept."""
        return path.read_text(errors="replace") if path else ""

    def untagged_typedefs(self):
        """Typedef names of a record defined in the typedef itself
        (`typedef struct [TAG] { … } T;`): name -> {(tag used, body)}."""
        if getattr(self, "_untagged", None) is None:
            table = collections.defaultdict(set)
            for path in self.files():
                text = self.text(path)
                for match in re.finditer(r"\btypedef\s+(struct|union)\s*(?:\w+\s*)?\{", text):
                    depth, at = 0, match.end() - 1
                    for index in range(at, len(text)):
                        if text[index] == "{":
                            depth += 1
                        elif text[index] == "}":
                            depth -= 1
                            if depth == 0:
                                tail = re.match(r"\s*(\w+)\s*;", text[index + 1:])
                                if tail:
                                    table[tail.group(1)].add((match.group(1), text[at + 1:index]))
                                break
            self._untagged = table
        return self._untagged


def base_spelling(tree: Tree, spelling):
    """(class, tag, body): the base's declared spelling, read lexically."""
    if spelling is None:
        return ("unread", None, None)
    tokens = spelling.replace("*", " * ").split()
    qualifiers = [t for t in tokens if t in ("volatile", "_Atomic")]
    tokens = [t for t in tokens if t not in ("const", "volatile", "restrict", "register", "_Atomic")]
    suffix = "-qualified" if qualifiers else ""
    if len(tokens) >= 4 and tokens[0] in ("struct", "union") and tokens[2] == "*" and tokens.count("*") == 1:
        return (f"{tokens[0]} T *{suffix}", tokens[1], None)
    if len(tokens) == 3 and tokens[1] == "*":
        name = tokens[0]
        for target in tree.typedefs().get(name, ()):
            found = re.fullmatch(r"(struct|union)\s+(\w+)", target)
            if found:
                return (f"typedef of `{found.group(1)} TAG`, `T *B`{suffix}", found.group(2), None)
        for tag_used, body in tree.untagged_typedefs().get(name, ()):
            return (f"typedef of a `{tag_used} [TAG] {{…}}` definition, `T *B`{suffix}", None, body)
        return (f"unresolved typedef, `T *B`{suffix}", None, None)
    if len(tokens) == 2:
        for target in tree.typedefs().get(tokens[0], ()):
            found = re.fullmatch(r"(struct|union)\s+(\w+)\s*\*", target)
            if found:
                return (f"typedef of `{found.group(1)} TAG *`, `TP B`{suffix}", found.group(2), None)
        return (f"unresolved typedef, `TP B`{suffix}", None, None)
    return ("other", None, None)


def reader(tree: Tree, spelling):
    """What the lexical declaration reader (`typed_field_access.rs`,
    `read_typedefs`/`classify_specifiers`) answers for the base, read over the
    whole tree (the analyzer reads the preprocessed unit, so a disagreement
    found here may be absent there): `resolved` for `struct TAG *B` and for
    `T *B` with exactly one `typedef struct TAG T;` and no typedef defining
    `T`; otherwise the cause it leaves the base untyped (`Refusal::UntypedBase`)."""
    if spelling is None:
        return "unread"
    tokens = spelling.replace("*", " * ").split()
    tokens = [t for t in tokens if t not in ("const", "volatile", "restrict", "register", "_Atomic")]
    if len(tokens) >= 4 and tokens[0] in ("struct", "union") and tokens[2] == "*":
        return "resolved (tag spelling)"
    if len(tokens) == 2:
        return "untyped: a typedef of a record pointer (`*` in the typedef)"
    if len(tokens) != 3 or tokens[1] != "*":
        return "other"
    name = tokens[0]
    forward = tree.typedefs().get(name, set())
    defined = tree.untagged_typedefs().get(name, set())
    if forward and defined:
        return "untyped: disagreeing typedefs (a forward one and a defining one)"
    if defined:
        return "untyped: the typedef defines the record"
    if len(forward) == 1 and re.fullmatch(r"(struct|union)\s+\w+", next(iter(forward))):
        return "resolved (`typedef struct TAG T;`)"
    if len(forward) > 1:
        return "untyped: disagreeing typedefs"
    if forward:
        return "untyped or resolved: a typedef of another name (chain not followed)"
    return "untyped: typedef not found"


def member_spelling(tree: Tree, tag, body, member):
    if body is not None:
        saved = tree._structs.get("\0untagged")
        tree._structs["\0untagged"] = body
        try:
            return parent.member_class(tree, "\0untagged", member)
        finally:
            tree._structs["\0untagged"] = saved
    return parent.member_class(tree, tag, member)


def callee_of(text: str, origin: str, base: str, member: str):
    line = int(origin.rsplit(":", 1)[1])
    lines = text.splitlines()
    window = " ".join(lines[max(0, line - 2):line + 1])
    found = re.search(r"(?<![\w.>])(?!(?:if|while|return|sizeof)\b)(\w+)\s*\(\s*&?\s*\(?\s*" + re.escape(base) + r"\s*->\s*" + re.escape(member) + r"\b",
                      window)
    return found.group(1) if found else None


def leaf_of(tree, text, raw, function, base, member, origin, mode):
    spelling = parent.parameter_spelling(text, function, base)
    b_class, tag, body = base_spelling(tree, spelling)
    callee = callee_of(raw, origin, base, member)
    return {
        "base": base,
        "member": member,
        "base_spelling": spelling,
        "base_class": b_class,
        "reader": reader(tree, spelling),
        "member_class": member_spelling(tree, tag, body, member),
        "callee": callee,
        "release_mode": (mode(callee) or "unit") if callee else "unread",
    }


def short(description: str) -> str:
    found = re.search(r"\(([a-z][a-z0-9-]+)\)\s*$", description)
    if found:
        return found.group(1)
    return description.split(":")[0][:90]


def from_report(route, report_path, root, providers):
    report = json.loads(pathlib.Path(report_path).read_text())
    tree = Tree(pathlib.Path(root))
    mode = release_modes(pathlib.Path(providers))
    functions, seen = [], set()
    for unit in report["units"]:
        path = None
        for function in unit["functions"]:
            key = (unit["file"], function["function"], function["status"],
                   json.dumps(function["diagnostics"], sort_keys=True))
            if key in seen:
                continue
            seen.add(key)
            named = [d for d in function["diagnostics"] if any(n in d.get("description", "") for n in NAMES)]
            if not named:
                continue
            if path is None:
                path = tree.find(unit["file"], unit.get("library", ""))
            text = tree.text(path) if path else ""
            raw = tree.raw(path)
            leaves, others = [], collections.Counter()
            for diagnostic in function["diagnostics"]:
                found = parent.LEAF.search(diagnostic.get("description", ""))
                if diagnostic in named and found:
                    leaf = leaf_of(tree, text, raw, function["function"], *found.groups(),
                                   diagnostic.get("origin", ":0"), mode)
                    leaf["origin"] = diagnostic.get("origin")
                    leaves.append(leaf)
                else:
                    others[short(diagnostic.get("description", ""))] += 1
            functions.append(finish(route, unit["file"], function["function"], function["status"],
                                    leaves, others))
    return [f for f in functions if f["typedef_base_leaves"]]


def from_current(premises, root, providers):
    rows = [json.loads(line) for line in gzip.open(premises, "rt")]
    releases, owned = estimate.contracts(pathlib.Path(providers))
    mode = release_modes(pathlib.Path(providers))
    tree = Tree(pathlib.Path(root))
    by_function = collections.defaultdict(list)
    for row in rows:
        by_function[(row["file"], row["function"])].append(row)
    functions = []
    for (file, function), function_rows in sorted(by_function.items()):
        names = [estimate.name_of(row, releases, owned) for row in function_rows]
        if not any(name in NAMES for name in names):
            continue
        path = tree.root / file
        text = tree.text(path) if path.is_file() else ""
        raw = tree.raw(path) if path.is_file() else ""
        leaves, others = [], collections.Counter()
        for row, name in zip(function_rows, names):
            if name in NAMES:
                step = row["paths"][0]
                leaf = leaf_of(tree, text, raw, function, step["root"], step["steps"][0]["member"],
                               row["origin"], mode)
                leaf["origin"] = row["origin"]
                leaf["clang_root_type"] = step.get("root_type")
                leaves.append(leaf)
            else:
                others[name or row["premise"]] += row["leaves"]
        for family, count in function_rows[0]["families"].items():
            if family != estimate.IND:
                others[family] += count
        functions.append(finish("current", file, function, function_rows[0]["status"], leaves, others))
    return [f for f in functions if f["typedef_base_leaves"]]


def typedef_spelled(leaf):
    return not (leaf["base_class"].startswith("struct T *") or leaf["base_class"].startswith("union T *"))


def finish(route, file, function, status, leaves, others):
    base_leaves = [leaf for leaf in leaves if typedef_spelled(leaf)]
    other_release = [leaf for leaf in leaves if not typedef_spelled(leaf)]
    if other_release:
        others["rule-38i release leaf over a `struct T *` base"] += len(other_release)
    return {
        "route": route,
        "file": file,
        "function": function,
        "status": status,
        "typedef_base_leaves": base_leaves,
        "other_blockers": dict(sorted(others.items())),
        "sole": not others,
    }


def table(functions):
    rows = collections.defaultdict(lambda: [0, set(), set()])
    for function in functions:
        for leaf in function["typedef_base_leaves"]:
            key = (function["route"], leaf["reader"], leaf["base_class"], leaf["member_class"],
                   leaf["release_mode"])
            rows[key][0] += 1
            rows[key][1].add(function["function"])
            if function["sole"]:
                rows[key][2].add(function["function"])
    return [
        {"route": k[0], "reader": k[1], "base": k[2], "member": k[3], "release_mode": k[4], "leaves": v[0],
         "functions": len(v[1]), "sole_blocker_functions": sorted(v[2])}
        for k, v in sorted(rows.items())
    ]


def main(argv):
    if argv[1] == "report":
        functions = from_report(argv[2], argv[3], argv[4], argv[5])
        inputs = {"route": argv[2], "report": argv[3], "source_root": argv[4], "providers": argv[5]}
    elif argv[1] == "current":
        functions = from_current(argv[2], argv[3], argv[4])
        inputs = {"route": "current", "premises": argv[2], "source_root": argv[3], "providers": argv[4]}
    else:
        raise SystemExit(__doc__)
    print(json.dumps({
        "inputs": inputs,
        "leaves": sum(len(f["typedef_base_leaves"]) for f in functions),
        "functions": len(functions),
        "reader_untyped_leaves": sum(1 for f in functions for leaf in f["typedef_base_leaves"]
                                     if leaf["reader"].startswith("untyped:")),
        "reader_untyped_functions": sum(1 for f in functions if any(
            leaf["reader"].startswith("untyped:") for leaf in f["typedef_base_leaves"])),
        "sole_blocker_functions": sorted(f"{f['file']}::{f['function']}" for f in functions if f["sole"]),
        "table": table(functions),
        "rows": functions,
    }, indent=1))


if __name__ == "__main__":
    main(sys.argv)
