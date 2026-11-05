#!/usr/bin/env python3
"""MEMBER-ACCESS-PROJECTION step 1: the indirect-access census.

usage: classify.py REPORT UNITS_DIR OUT_DIR [--streams RUN_DIR]

REPORT     a run's resource-diagnostics.json
UNITS_DIR  that run's analysed texts (unit-<index>.c + units.jsonl, index =
           the report's unit index; prep_units.py checked each sha256)
--streams  a census-feature run of the same command whose analyzer streams
           (indirect.jsonl, call-arguments.jsonl, parity.jsonl) add the
           routes' own answers

Every `indirect memory access is outside the resource projection` leaf is
bound to the Clang constructs of its function at its line (W46's
access_census.py walker, imported), and each of its access paths is typed.
The leaf gets ONE premise, the first that holds in a fixed order (README).
"""
import collections
import gzip
import json
import multiprocessing
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve()
ROOT = next(p for p in HERE.parents if (p / "docs/reports/w46-access-path-census").is_dir())
sys.path.insert(0, str(ROOT / "docs/reports/w46-access-path-census/scripts"))
import access_census as census  # noqa: E402

IND = "indirect memory access is outside the resource projection"
CHECKOUT = pathlib.Path(
    "/media/Data/GitHub/quirin-lang/build/cycle19/clang/"
    "046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/fluent-bit-clang-source")


def rhs_class(node):
    bare = node
    cast = False
    while bare.get("kind") in ("ParenExpr", "ImplicitCastExpr", "CStyleCastExpr") and census.children(bare):
        if bare.get("kind") == "CStyleCastExpr":
            cast = True
        bare = census.children(bare)[0]
    kind = bare.get("kind")
    if census.is_null_constant(bare):
        label = "null"
    elif kind == "CallExpr":
        label = "call:" + (census.direct_callee(bare) or "<indirect>")
    elif kind == "DeclRefExpr":
        ref = bare.get("referencedDecl", {})
        label = {"ParmVarDecl": "formal", "VarDecl": "variable", "FunctionDecl": "function",
                 "EnumConstantDecl": "constant"}.get(ref.get("kind"), "name")
    elif kind == "StringLiteral":
        label = "string"
    elif kind in ("MemberExpr", "ArraySubscriptExpr") or (kind == "UnaryOperator" and bare.get("opcode") == "*"):
        label = "memory-read"
    elif kind == "UnaryOperator" and bare.get("opcode") == "&":
        label = "address"
    elif kind == "BinaryOperator" and bare.get("opcode") in ("+", "-"):
        label = "arithmetic"
    elif kind in ("ConditionalOperator", "BinaryConditionalOperator"):
        label = "conditional"
    elif kind in ("IntegerLiteral", "FloatingLiteral", "CharacterLiteral"):
        label = "literal"
    else:
        label = "other:" + str(kind)
    return label + ("+cast" if cast else "")


class Walker(census.Walker):
    """W46's walker, plus: a store path's right-hand side class, and a call
    actual path's callee and argument index."""

    def walk(self, node, ctx):
        kind = node.get("kind")
        if kind in ("BinaryOperator",) and node.get("opcode") == "=" and len(census.children(node)) == 2:
            left, right = census.children(node)
            before = len(self.paths)
            null_value = {"null-value"} if census.is_null_constant(right) else set()
            self.walk(left, (ctx - {"read", "address", "decay"}) | {"store"} | null_value)
            for path in self.paths[before:]:
                if path["access"] in ("store", "null-store"):
                    path["rhs"] = rhs_class(right)
            self.walk(right, ctx - {"store"})
            return
        if kind == "CallExpr":
            name = census.direct_callee(node)
            inner = census.children(node)
            self.calls.append(name or "<indirect>")
            base = {c for c in ctx if not c.startswith(("callee=", "arg="))}
            if name is None and inner:
                self.walk(inner[0], base | {"callee"})
            for index, argument in enumerate(inner[1:]):
                self.walk(argument, (base - {"callee"}) | {"call-argument", f"callee={name or '<indirect>'}",
                                                           f"arg={index}"})
            return
        super().walk(node, ctx)

    def decompose(self, top, ctx):
        path = super().decompose(top, ctx)
        for tag in ctx:
            if tag.startswith("callee="):
                path["callee"] = tag[7:]
            elif tag.startswith("arg="):
                path["arg"] = int(tag[4:])
        if "declaration" in ctx:
            path["declaration"] = True
        return path


census.Walker = Walker

# W46's kind_of strips qualifiers by whitespace-separated token, so a member of
# a const-qualified record (`char *const`) kept a trailing `const` and read as a
# scalar. Separate every `*` before classifying (and only then).
_kind_of = census.Unit.kind_of


def _kind_of_spaced(self, text):
    kind, bare, flags = _kind_of(self, re.sub(r"\*", " * ", text or ""))
    return kind, re.sub(r"\s+", " ", bare).replace(" *", "*").replace("* ", "*").replace("*", " *").strip(), flags


census.Unit.kind_of = _kind_of_spaced


def extract(arguments):
    directory, record = arguments
    seq, value = census.extract_unit((directory, record))
    return seq, value


def pointer_free(unit, record_text, seen=None):
    """A record with no pointer member, transitively (rule 11f's custody-free
    record). None when the record's fields are not all known."""
    seen = seen or set()
    if record_text in seen:
        return True
    seen.add(record_text)
    fields = unit.record_fields.get(record_text)
    if fields is None:
        return None
    for field_type in fields:
        kind, bare, _ = unit.kind_of(field_type)
        if kind in ("pointer", "function_pointer"):
            return False
        if kind == "record":
            inner = pointer_free(unit, bare, seen)
            if inner is not True:
                return inner
        if kind == "array":
            element = re.sub(r"\s*\[[^\]]*\]$", "", bare)
            ekind, ebare, _ = unit.kind_of(element)
            if ekind in ("pointer", "function_pointer"):
                return False
            if ekind == "record":
                inner = pointer_free(unit, ebare, seen)
                if inner is not True:
                    return inner
    return True


def path_class(path):
    """Shape label of one typed path."""
    causes = set(path["causes"])
    hard = sorted(causes - {"call"})
    if hard:
        return "cause:" + hard[0]
    if path["root_kind"] == "call-result":
        return "root:call-result"
    if path["root_kind"] in ("global", "other"):
        return "root:" + path["root_kind"]
    depth = path["hops"]
    if depth > 2 or (depth == 2 and path["steps"] and path["steps"][-1]["op"] in ("[]", "*")):
        return "deep"
    return "direct"


def value_class(path):
    value = path["value"]
    if value in ("pointer", "function_pointer"):
        return "pointer"
    if value == "aggregate":
        return "aggregate"
    if value == "address":
        return "address"
    return "scalar"


def main():
    report = json.load(open(sys.argv[1]))
    units_dir = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    records = [json.loads(line) for line in (units_dir / "units.jsonl").read_text().splitlines()]
    by_seq = {r["unit_seq"]: r for r in records}
    with multiprocessing.Pool(8) as pool:
        extracted = dict(pool.map(extract, [(str(units_dir), r) for r in records], chunksize=1))

    rows = []
    unbound = []
    for index, document in enumerate(report["units"]):
        definitions = extracted.get(index, {}).get("definitions", {})
        source_lines = None
        for function in document["functions"]:
            groups = collections.defaultdict(list)
            for leaf in function["diagnostics"]:
                if leaf["category"] == "unsupported" and leaf["description"] == IND:
                    groups[leaf["line"]].append(leaf)
            if not groups:
                continue
            definition = definitions.get(function["function"])
            families = collections.Counter()
            for leaf in function["diagnostics"]:
                if leaf["category"] == "unsupported":
                    families[leaf["description"]] += 1
                else:
                    families[leaf["category"]] += 1
            for line, leaves in sorted(groups.items()):
                constructs = []
                if definition is not None:
                    at_line = [c for c in definition["constructs"] if line in c["lines"]]
                    constructs = [c for c in at_line if c["kind"] == "statement"] or at_line
                origin = leaves[0].get("origin") or ""
                origin_text = ""
                if ":" in origin:
                    file, number = origin.rsplit(":", 1)
                    try:
                        text = (CHECKOUT / file).read_text(errors="replace").splitlines()
                        origin_text = text[int(number) - 1]
                    except (OSError, IndexError, ValueError):
                        origin_text = ""
                paths = [p for c in constructs for p in c["paths"]]
                calls = [name for c in constructs for name in c["calls"]]
                if not constructs:
                    unbound.append({"file": document["file"], "function": function["function"],
                                    "line": line, "origin": origin})
                rows.append({
                    "unit": index,
                    "file": document["file"],
                    "library": document.get("library"),
                    "function": function["function"],
                    "status": function["status"],
                    "line": line,
                    "leaves": len(leaves),
                    "origin": origin,
                    "origin_text": origin_text.strip(),
                    "macro": not re.search(r"->|\[|\w\s*\.\s*\w|\*\s*\(?\s*\w", origin_text),
                    "construct_kinds": sorted({c["kind"] + ":" + c["clause"] for c in constructs}),
                    "calls": calls,
                    "paths": paths,
                    "families": dict(families),
                })
    with gzip.open(out / "ind-leaves.jsonl.gz", "wt") as handle:
        for row in rows:
            handle.write(json.dumps(row) + "\n")
    (out / "unbound.json").write_text(json.dumps(unbound, indent=1) + "\n")
    print(f"leaf groups {len(rows)}; leaves {sum(r['leaves'] for r in rows)}; unbound {len(unbound)}")


if __name__ == "__main__":
    main()
