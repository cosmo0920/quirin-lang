#!/usr/bin/env python3
"""Card W47-ALIAS-RESIDUALS-CENSUS: classify every `unknown_alias` leaf by cause.

usage: alias_census.py RUN_DIR DIAGNOSTICS.json CONTRACTS.json OUT_DIR [--jobs N]
                       [--parity PARITY.jsonl] [--partial]

RUN_DIR is one census run of the `indirect-census` build whose
`access-path-units/` holds each analysed unit's exact text (W46's stream). Each
text is given to Clang with the gateway's own typed-AST argument vector
(W46-ACCESS-PATH-CENSUS's `access_census.py`, imported, not copied), and the
unit is joined to the report exactly as that script joins it (a document unit
matches the one analysis whose definitions contain its function set).

A leaf reads `unresolved resource identity `V`` and carries only `V` and an
internal line. Its cause is read from the typed AST of its own function:

* the spelling of `V` (a peer composite `place/peer` is C02's release-alias
  obligation; `__qcore_literal__` is the lowering's literal actual);
* what `V` declares (formal, local, static or global object, enum constant,
  function, or nothing the unit declares) and its Clang type (pointer,
  scalar, record, array);
* for a pointer object, every value the function assigns it (its *origins*:
  a call and its callee, a read through an access path, a copy of another
  name, null, an address, arithmetic, a conditional, a cast), whether a
  controlling expression null-tests it after the last origin before the leaf,
  and whether an unsupported leaf of the same function lies between that
  origin and the leaf (a refusal that poisons the frame, rule 10a);
* how `V` is used at the leaf line (dereferenced root in a statement,
  condition or return, call actual, the owner of a pointer-valued field store);
* whether the typed-path parity stream admits a typed row at the leaf line
  with `V` as a chain root (a P2/P3 typed admission whose root obligation
  this is).

Output: `leaves.jsonl.gz` (schema line first; one row per leaf with its class,
subclass and facts), `summary.json` and `unmatched.json`. Exit 3 when a leaf
cannot be joined to a function (unless `--partial`).
"""

import collections
import gzip
import importlib.util
import json
import multiprocessing
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]
SPEC = importlib.util.spec_from_file_location(
    "access_census", ROOT / "docs/reports/w46-access-path-census/scripts/access_census.py")
AC = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AC)

SCHEMA = "qcore.w47-alias-residuals-census.v1"
ALIAS = re.compile(r"unresolved resource identity `(.*)`$")
UNC = re.compile(r"call to uncontracted function `([^`]*)`")
IDENT = re.compile(r"[A-Za-z_]\w*$")
CHAIN_ROOT = re.compile(r'root: "([^"]*)", dereferenced_root: (true|false), root_never_bound: (true|false)')


def children(node):
    return AC.children(node)


def strip_value(node):
    """Strip parentheses and value-preserving implicit casts; note casts."""
    cast = False
    while True:
        kind = node.get("kind")
        inner = children(node)
        if kind == "ParenExpr" and inner:
            node = inner[0]
            continue
        if kind == "ImplicitCastExpr" and inner:
            if node.get("castKind") not in ("LValueToRValue", "NoOp", "ArrayToPointerDecay",
                                            "FunctionToPointerDecay"):
                cast = True
            node = inner[0]
            continue
        if kind == "CStyleCastExpr" and inner:
            cast = True
            node = inner[0]
            continue
        return node, cast


def origin_of(rhs):
    """(kind, detail) of one value assigned to a pointer object."""
    node, cast = strip_value(rhs)
    kind = node.get("kind")
    inner = children(node)
    if kind == "CallExpr":
        return "call", AC.direct_callee(node) or "<indirect>"
    if kind in ("MemberExpr", "ArraySubscriptExpr") or (kind == "UnaryOperator" and node.get("opcode") == "*"):
        return "path", None
    if kind == "DeclRefExpr":
        ref = node.get("referencedDecl", {})
        if ref.get("kind") in ("VarDecl", "ParmVarDecl"):
            return "name", ref.get("name")
        if ref.get("kind") == "EnumConstantDecl":
            return "constant", ref.get("name")
        if ref.get("kind") == "FunctionDecl":
            return "function", ref.get("name")
        return "other", ref.get("kind")
    if kind == "IntegerLiteral":
        return ("null", None) if node.get("value") == "0" else ("constant", None)
    if kind in ("StringLiteral", "CharacterLiteral", "FloatingLiteral"):
        return "constant", None
    if kind == "UnaryOperator" and node.get("opcode") == "&":
        return "address", None
    if kind == "BinaryOperator" and node.get("opcode") in ("+", "-"):
        return "arithmetic", None
    if kind == "BinaryOperator" and node.get("opcode") == "=" and len(inner) == 2:
        return origin_of(inner[1])
    if kind == "BinaryOperator" and node.get("opcode") == ",":
        return origin_of(inner[-1])
    if kind in ("ConditionalOperator", "BinaryConditionalOperator"):
        return "conditional", None
    if kind == "StmtExpr":
        return "statement-expression", None
    if kind in ("UnaryOperator",) and node.get("opcode") in ("++", "--"):
        return "arithmetic", None
    return ("cast-" if cast else "") + "other", kind


def declared_name(node):
    node, _ = strip_value(node)
    if node.get("kind") == "DeclRefExpr":
        ref = node.get("referencedDecl", {})
        if ref.get("kind") in ("VarDecl", "ParmVarDecl"):
            return ref.get("name")
    return None


def function_facts(unit, fn):
    """Variables (formal/local with type kind), origins per name, uses."""
    variables = {}
    for child in children(fn):
        if child.get("kind") == "ParmVarDecl" and child.get("name"):
            variables[child["name"]] = {"decl": "formal", "type": unit.kind_of(unit.qual(child))[0],
                                        "line": unit.line(child)}
    origins = collections.defaultdict(list)
    stores = collections.defaultdict(list)   # owner name -> lines of pointer-valued field stores
    mentions = collections.defaultdict(set)  # name -> callees whose actuals mention it
    stack = [fn]
    while stack:
        node = stack.pop()
        kind = node.get("kind")
        inner = children(node)
        if kind == "VarDecl" and node.get("name"):
            storage = node.get("storageClass", "auto")
            decl = "local" if storage not in ("static", "extern") else storage
            variables.setdefault(node["name"], {"decl": decl, "type": unit.kind_of(unit.qual(node))[0],
                                                "line": unit.line(node)})
            if inner and node.get("init"):
                origins[node["name"]].append((unit.line(node),) + origin_of(inner[0]))
            elif inner and any(c.get("kind", "").endswith(("Expr", "Literal", "Operator")) for c in inner):
                origins[node["name"]].append((unit.line(node),) + origin_of(inner[-1]))
        if kind in ("BinaryOperator", "CompoundAssignOperator") and len(inner) == 2 and (
                node.get("opcode") == "=" or kind == "CompoundAssignOperator"):
            name = declared_name(inner[0])
            if name:
                if kind == "CompoundAssignOperator":
                    origins[name].append((unit.line(node), "arithmetic", None))
                else:
                    origins[name].append((unit.line(node),) + origin_of(inner[1]))
            else:
                target, _ = strip_value(inner[0])
                if target.get("kind") == "MemberExpr" and unit.kind_of(unit.qual(target))[0] == "pointer":
                    base, _ = strip_value(children(target)[0]) if children(target) else ({}, False)
                    owner = declared_name(base) if base else None
                    if owner:
                        stores[owner].append(unit.line(node))
        if kind == "UnaryOperator" and node.get("opcode") in ("++", "--") and inner:
            name = declared_name(inner[0])
            if name:
                origins[name].append((unit.line(node), "arithmetic", None))
        if kind == "UnaryOperator" and node.get("opcode") == "&" and inner:
            name = declared_name(inner[0])
            if name:
                origins[name].append((unit.line(node), "address-taken", None))
        if kind == "CallExpr" and inner:
            # Every name mentioned anywhere inside an actual (`f(&(p)->x)`
            # mentions `p`): the lexical closure reads call arguments as
            # token spans, so a mention is what it can see.
            callee = AC.direct_callee(node) or "<indirect>"
            for argument in inner[1:]:
                probe = [argument]
                while probe:
                    current = probe.pop()
                    if current.get("kind") == "DeclRefExpr":
                        ref = current.get("referencedDecl", {})
                        if ref.get("kind") in ("VarDecl", "ParmVarDecl") and ref.get("name"):
                            mentions[ref["name"]].add(callee)
                    probe.extend(children(current))
        stack.extend(inner)
    for name in origins:
        origins[name].sort(key=lambda item: (item[0] or 0, item[1], str(item[2])))
    return variables, dict(origins), dict(stores), {name: sorted(v) for name, v in mentions.items()}


def unit_tables(tu):
    globals_, enums, functions = set(), set(), set()
    for decl in children(tu):
        kind = decl.get("kind")
        if kind == "VarDecl" and decl.get("name"):
            globals_.add(decl["name"])
        if kind == "FunctionDecl" and decl.get("name"):
            functions.add(decl["name"])
    stack = [tu]
    while stack:
        node = stack.pop()
        if node.get("kind") == "EnumConstantDecl" and node.get("name"):
            enums.add(node["name"])
        stack.extend(children(node))
    return globals_, enums, functions


def extract_unit(arguments):
    directory, record = arguments
    text = (pathlib.Path(directory) / record["file"]).read_bytes()
    command = [AC.CLANG, "--driver-mode=gcc", *record["frontend_flags"], "-x", "c", record["dialect"],
               "-fsyntax-only", "-Xclang", "-ast-dump=json", "-"]
    completed = subprocess.run(command, input=text, capture_output=True)
    if completed.returncode != 0:
        return record["unit_seq"], {"error": completed.stderr.decode(errors="replace")[:2000]}
    tu = json.loads(completed.stdout)
    unit = AC.Unit(text)
    AC.collect_tables(unit, tu)
    globals_, enums, functions = unit_tables(tu)
    global_types = {}
    for decl in children(tu):
        if decl.get("kind") == "VarDecl" and decl.get("name"):
            global_types[decl["name"]] = unit.kind_of(unit.qual(decl))[0]
    definitions = {}
    for decl in children(tu):
        if decl.get("kind") != "FunctionDecl":
            continue
        if not any(c.get("kind") == "CompoundStmt" for c in children(decl)):
            continue
        variables, origins, stores, mentions = function_facts(unit, decl)
        constructs = AC.function_constructs(unit, decl)
        definitions[decl.get("name")] = {
            "storage": decl.get("storageClass", "extern"),
            "line": unit.line(decl),
            "end": unit.end_line(decl),
            "variables": variables,
            "origins": origins,
            "stores": stores,
            "mentions": mentions,
            "constructs": [{k: c[k] for k in ("kind", "clause", "lines", "call_args", "assigned",
                                              "null_tested", "calls")}
                           | {"paths": [{k: p[k] for k in ("root", "root_kind", "root_hop", "value", "access",
                                                           "call_argument", "causes")}
                                        for p in c["paths"]]}
                           for c in constructs],
        }
    return record["unit_seq"], {"definitions": definitions, "globals": global_types,
                                "enums": sorted(enums), "functions": sorted(functions)}


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def contracted_names(path):
    data = load(path)
    names = set()
    # The corpus sidecar keys module texts as `sources`, the system slice's as
    # `contracts`.
    for source in (data.get("sources") or data.get("contracts") or {}).values():
        for block in re.split(r"(?=\bextern\s+fn\s)", source):
            match = re.match(r"extern\s+fn\s+([A-Za-z_]\w*)", block)
            if match:
                names.add(match.group(1))
    for module in data["contract_inclusion"]["modules"].values():
        names.update(module.get("inherited", {}))
    return names


def parity_roots(path):
    """(function, line) -> {root: (admitted, never_bound)} from typed rows."""
    roots = collections.defaultdict(dict)
    if path is None:
        return roots
    with open(path) as handle:
        for line in handle:
            row = json.loads(line)
            if not row.get("new_admitted"):
                continue
            for root, _deref, never in CHAIN_ROOT.findall(row.get("new_chains") or ""):
                roots[(row["function"], row["line"])][root] = never == "true"
    return roots


class Callees:
    """Where a callee's summary would come from (W46 predict.py's vocabulary)."""

    def __init__(self, diagnostics, definitions, contracted):
        self.contracted = contracted
        self.status = {}
        self.defined_in = collections.defaultdict(list)
        for unit in diagnostics["units"]:
            for function in unit["functions"]:
                self.status[(unit["file"], function["function"])] = function["status"]
        self.storage = definitions
        for file, names in definitions.items():
            for name, storage in names.items():
                if (file, name) in self.status:
                    self.defined_in[name].append((file, storage))

    def kind(self, file, name):
        if name == "<indirect>":
            return "indirect", None
        if name in self.contracted:
            return "contracted", None
        if (file, name) in self.status:
            return "same-unit", self.status[(file, name)]
        if name in self.storage.get(file, {}):
            return "same-unit-header", None
        external = [(f, s) for f, s in self.defined_in.get(name, []) if s != "static"]
        if len(external) == 1:
            return "cross-unit", self.status.get((external[0][0], name))
        if len(external) > 1:
            return "cross-unit-ambiguous", None
        return "external", None


NON_POINTER = {"literal-actual", "scalar-name", "aggregate-name", "array-name", "other-type-name",
               "non-object-name", "place-spelled"}
UNCONTRACTED_RESULT = {"call-result-external", "call-result-corpus", "call-result-mixed"}
DERIVED_LOCAL = {"copy-of-name", "mixed-origin", "local-unbound", "other-origin", "derived-address",
                 "derived-arithmetic", "derived-conditional", "derived-constant", "derived-function"}


def discharge_group(cls, cause, facts=None):
    """The group of leaves one implementation card would discharge together.

    The origin decides first where the origin alone needs a model the
    projection lacks (a name that is not a pointer object, a peer release, a
    pointer-valued field store, an address-taken pointer written through
    `&V`); otherwise the cause decides (a poisoned binding is freed by the
    refusal that poisoned it, whatever its origin); an unguarded binding is
    grouped by where its value came from.
    """
    if cls == "unclassified":
        return "unclassified"
    if cls in NON_POINTER:
        return "non-pointer-name"
    if cls == "global-pointer":
        return "global-pointer"
    if cls == "peer-release-alias":
        return "peer-release"
    if cls == "field-store-owner":
        return "field-store-owner"
    if cls == "address-taken-pointer":
        return "address-taken"
    if cause == "poisoned":
        return "poisoned-by-refusal"
    if cause == "guarded-clean":
        return "guarded-other"
    if cls == "formal-custody":
        closure = (facts or {}).get("closure", "not-handed")
        if closure == "not-handed" and not any(use.startswith(("deref-", "member-"))
                                               for use in (facts or {}).get("uses", [])):
            # `T *x = formal;` (typically a `void *` callback context): the
            # copy of a formal no statement bound, outside the traversal
            # exemption, reports the source.
            return "formal-copied"
        return "formal-" + closure
    if cls == "call-result-contracted":
        return "contracted-result-unguarded"
    if cls in UNCONTRACTED_RESULT:
        return "uncontracted-result-unguarded"
    if cls == "borrowed-path-read":
        return "borrowed-path-unguarded"
    if cls == "derived-statement-expression":
        return "statement-expression-unguarded"
    if cls in DERIVED_LOCAL:
        return "derived-local-unguarded"
    return "unclassified"


def refusal_family(description):
    """The STATUS-row family of one unsupported leaf, with a short tail."""
    if description == "indirect memory access is outside the resource projection":
        return "IND"
    if description.startswith("call to uncontracted function"):
        return "UNC"
    if description == "unmodeled call, aggregate or indirect access in return expression":
        return "RET"
    if description.startswith("unmodeled call, aggregate or indirect access in"):
        return "COND"
    return "OTHER: " + description.split("`")[0].strip()[:60]


def classify(variable, definition, unit_facts, leaf_line, unsupported_lines, typed, callees, file):
    """(class, subclass, facts) of one leaf."""
    facts = {}
    if "/" in variable:
        place, peer = variable.split("/", 1)
        facts["place"], facts["peer"] = place, peer
        return "peer-release-alias", "released-peer", facts
    if variable == "__qcore_literal__":
        return "literal-actual", "literal", facts
    if not IDENT.match(variable):
        facts["spelling"] = variable
        return "place-spelled", "place", facts
    info = definition["variables"].get(variable)
    if info is None:
        if variable in unit_facts["globals"]:
            info = {"decl": "global", "type": unit_facts["globals"][variable]}
        elif variable in unit_facts["enums"]:
            return "non-object-name", "enum-constant", facts
        elif variable in unit_facts["functions"]:
            return "non-object-name", "function", facts
        else:
            return "non-object-name", "undeclared", facts
    facts["decl"], facts["type"] = info["decl"], info["type"]

    # Use of V at the leaf line.
    uses = set()
    for construct in definition["constructs"]:
        if leaf_line not in construct["lines"]:
            continue
        for path in construct["paths"]:
            if path["root"] == variable:
                if path["root_hop"]:
                    uses.add("deref-" + construct["kind"])
                else:
                    uses.add("member-" + construct["kind"])
        for callee, arguments in construct["call_args"]:
            if variable in arguments:
                uses.add("actual:" + callees.kind(file, callee)[0])
        if variable in construct["assigned"]:
            uses.add("assigned")
        if variable in construct["null_tested"]:
            uses.add("null-tested")
    if leaf_line in definition["stores"].get(variable, []):
        uses.add("pointer-field-store-owner")
    facts["uses"] = sorted(uses)
    # Why the custody closure (`UntrackedBases::never_bound`, rule 5b's
    # premise) may not prove V never bound, read from the whole function: V
    # handed as a direct actual to an uncontracted callee (corpus-defined or
    # external) or to a contracted one (its mode decides), V written (by
    # origin) or address-taken. An unbound root the closure does not prove
    # never bound is refused by every dereference obligation.
    handed = set()
    uncontracted = set()
    for callee in definition["mentions"].get(variable, []):
        kind = callees.kind(file, callee)[0]
        handed.add("contracted" if kind == "contracted" else
                   "corpus" if kind.startswith(("same-unit", "cross-unit")) else
                   "indirect" if kind == "indirect" else "external")
        if kind != "contracted":
            uncontracted.add(callee)
    facts["handed_to"] = sorted(handed)
    facts["uncontracted_callees"] = sorted(uncontracted)
    facts["typed_row"] = variable in typed
    if variable in typed:
        facts["typed_root_never_bound"] = typed[variable]

    if info["type"] != "pointer":
        kind = {"scalar": "scalar-name", "record": "aggregate-name", "array": "array-name"}.get(
            info["type"], "other-type-name")
        return kind, info["decl"], facts
    if info["decl"] in ("global", "extern", "static"):
        return "global-pointer", info["decl"], facts

    origins = definition["origins"].get(variable, [])
    before = [o for o in origins if (o[0] or 0) <= leaf_line]
    reaching = before or origins  # a loop can carry a later assignment around
    facts["origins"] = sorted({o[1] if o[1] != "call" else "call:" + str(o[2]) for o in reaching})
    kinds = {o[1] for o in reaching} - {"null"}
    last = max([o[0] or 0 for o in before] or [0])
    guards = [min(construct["lines"]) for construct in definition["constructs"]
              if construct["kind"] == "condition" and construct["lines"]
              and last <= min(construct["lines"]) < leaf_line and variable in construct["null_tested"]]
    facts["guarded"] = bool(guards)
    # The cause, read lexically (control flow is not followed): no dominating
    # null test since the last value reaching the leaf (`unguarded`); a null
    # test, then an unsupported statement of this function strictly between
    # the last such test and the leaf, whose refusal may poison the binding
    # (rule 10a: `poisoned`); or a null test with no refusal between
    # (`guarded-clean`: the unresolved state has another cause, e.g. a
    # release, an escape, an ambiguous join or an unresolved callee effect).
    if not guards:
        cause = "unguarded"
    else:
        guard_line = max(guards)
        between = [(line, text) for line, text in unsupported_lines if guard_line < line < leaf_line]
        facts["refusal_after_guard"] = bool(between)
        if between:
            # The first refusal after the guard, named by its family.
            facts["first_refusal"] = refusal_family(between[0][1])
            facts["first_refusal_line"] = between[0][0]
        cause = "poisoned" if facts["refusal_after_guard"] else "guarded-clean"
    facts["cause"] = cause
    if "pointer-field-store-owner" in uses:
        # A pointer value stored into a field of V: the owned-field model (P5).
        return "field-store-owner", cause, facts
    if not kinds:
        if info["decl"] == "formal":
            handed = set(facts["handed_to"])
            facts["closure"] = ("handed-uncontracted" if handed & {"corpus", "external", "indirect"} else
                                "handed-contracted" if handed else "not-handed")
            return "formal-custody", cause, facts
        return "local-unbound", cause, facts
    if "address-taken" in kinds:
        return "address-taken-pointer", cause, facts
    if len(kinds) > 1:
        facts["mixed"] = "+".join(sorted(kinds))
        return "mixed-origin", cause, facts
    kind = next(iter(kinds))
    if kind == "call":
        callee_kinds = set()
        for origin in reaching:
            if origin[1] == "call":
                callee_kind, status = callees.kind(file, origin[2])
                callee_kinds.add(callee_kind + (":" + status if status else ""))
        facts["callee_kinds"] = sorted(callee_kinds)
        if len(callee_kinds) == 1:
            only = next(iter(callee_kinds))
            if only == "contracted":
                return "call-result-contracted", cause, facts
            if only.startswith(("same-unit", "cross-unit")):
                return "call-result-corpus", cause, facts
            return "call-result-external", cause, facts
        return "call-result-mixed", cause, facts
    if kind == "path":
        return "borrowed-path-read", cause, facts
    if kind == "name":
        return "copy-of-name", cause, facts
    if kind in ("arithmetic", "conditional", "statement-expression", "constant", "function", "address"):
        return "derived-" + kind, cause, facts
    return "other-origin", cause, facts


def main():
    arguments = sys.argv[1:]
    options = {}
    for flag in ("--jobs", "--parity"):
        if flag in arguments:
            index = arguments.index(flag)
            options[flag] = arguments[index + 1]
            del arguments[index:index + 2]
    partial = "--partial" in arguments
    arguments = [a for a in arguments if a != "--partial"]
    run, diagnostics_path, contracts_path, out = arguments
    run, out = pathlib.Path(run), pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    diagnostics = load(diagnostics_path)
    contracted = contracted_names(contracts_path)
    typed_rows = parity_roots(options.get("--parity"))
    directory = run / "access-path-units"
    records = [json.loads(line) for line in (directory / "units.jsonl").read_text().splitlines()]
    with multiprocessing.Pool(int(options.get("--jobs", 6))) as pool:
        extracted = dict(pool.map(extract_unit, [(str(directory), r) for r in records], chunksize=1))
    errors = {seq: value["error"] for seq, value in extracted.items() if "error" in value}

    unmatched = {"units": [], "functions": [], "clang_errors": errors}
    # A document unit matches the analyses whose definitions contain its
    # function set. A file compiled in two configurations (the system slice)
    # has two documents and two such analyses: they are paired in order (the
    # analyses run in document order), and every leaf line must then fall
    # inside its function's extent, which is checked below.
    # The current audit analyses each system unit more than once (the same
    # text again for its later phase), so several analyses can qualify. A
    # candidate is *valid* only when every diagnostic line of every function
    # of the document falls inside that function's extent in the candidate's
    # text; the first valid unused candidate is taken, else the first valid
    # one (an identical re-analysis). No valid candidate refuses the unit.
    unit_of = {}
    used = set()

    def valid(seq, document):
        definitions = extracted[seq]["definitions"]
        for function in document["functions"]:
            definition = definitions[function["function"]]
            low, high = definition["line"] or 0, definition["end"] or 0
            if any(not low <= leaf["line"] <= high for leaf in function["diagnostics"] if leaf["line"]):
                return False
        return True

    for index, document in enumerate(diagnostics["units"]):
        names = frozenset(f["function"] for f in document["functions"])
        if not names:
            continue
        candidates = sorted(seq for seq, value in extracted.items()
                            if "definitions" in value and names <= set(value["definitions"]))
        good = [seq for seq in candidates if valid(seq, document)]
        choice = next((seq for seq in good if seq not in used), good[0] if good else None)
        if choice is None:
            unmatched["units"].append({"file": document["file"], "candidates": candidates})
            continue
        unit_of[index] = choice
        used.add(choice)
    definitions_by_file = {}
    for index, seq in unit_of.items():
        file = diagnostics["units"][index]["file"]
        definitions_by_file.setdefault(file, {}).update(
            {name: value["storage"] for name, value in extracted[seq]["definitions"].items()})
    callees = Callees(diagnostics, definitions_by_file, contracted)

    rows = []
    for index, document in enumerate(diagnostics["units"]):
        seq = unit_of.get(index)
        for function in document["functions"]:
            leaves = [leaf for leaf in function["diagnostics"] if leaf["category"] == "unknown_alias"]
            if not leaves:
                continue
            definition = None if seq is None else extracted[seq]["definitions"].get(function["function"])
            if definition is None:
                unmatched["functions"].append({"file": document["file"], "function": function["function"],
                                               "leaves": len(leaves)})
                for leaf in leaves:
                    rows.append({"library": document.get("library"), "file": document["file"],
                                 "function": function["function"], "status": function["status"],
                                 "line": leaf["line"], "origin": leaf.get("origin"),
                                 "variable": ALIAS.match(leaf["description"]).group(1),
                                 "class": "unclassified", "subclass": "no-typed-function", "facts": {}})
                continue
            extent = (definition["line"] or 0, definition["end"] or 0)
            outside = [leaf["line"] for leaf in leaves if not extent[0] <= leaf["line"] <= extent[1]]
            if outside:
                unmatched["functions"].append({"file": document["file"], "function": function["function"],
                                               "outside_extent": outside})
            unsupported_lines = sorted((leaf["line"], leaf["description"]) for leaf in function["diagnostics"]
                                       if leaf["category"] == "unsupported")
            for leaf in leaves:
                match = ALIAS.match(leaf["description"])
                variable = match.group(1) if match else leaf["description"]
                typed = typed_rows.get((function["function"], leaf["line"]), {})
                cls, sub, facts = classify(variable, definition, extracted[seq], leaf["line"],
                                           unsupported_lines, typed, callees, document["file"])
                rows.append({"library": document.get("library"), "file": document["file"],
                             "function": function["function"], "status": function["status"],
                             "unit_seq": seq, "line": leaf["line"], "origin": leaf.get("origin"), "variable": variable,
                             "class": cls, "subclass": sub, "facts": facts})

    for row in rows:
        row["group"] = discharge_group(row["class"], row["facts"].get("cause"), row["facts"])
    # A binding left unresolved at one line stays unresolved at every later use
    # of the same name: the earliest leaf of a (function, name) pair is its
    # `first` leaf, every later one a `repeat` of the same obligation.
    earliest = {}
    for row in rows:
        key = (row["file"], row["function"], row["variable"])
        earliest[key] = min(earliest.get(key, row["line"]), row["line"])
    for row in rows:
        row["first"] = row["line"] == earliest[(row["file"], row["function"], row["variable"])]

    with gzip.open(out / "leaves.jsonl.gz", "wt") as handle:
        handle.write(json.dumps({"schema": SCHEMA, "kind": "alias-leaf-rows"}) + "\n")
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")
    (out / "unit-map.json").write_text(json.dumps({"schema": SCHEMA,
                                                   "unit_of": {f"{diagnostics['units'][i]['file']}#{i}": seq
                                                               for i, seq in unit_of.items()},
                                                   "definitions": definitions_by_file}) + "\n")
    (out / "unmatched.json").write_text(json.dumps(unmatched, indent=1) + "\n")
    classes = collections.Counter(row["class"] for row in rows)
    subclasses = collections.Counter((row["class"], row["subclass"]) for row in rows)
    summary = {
        "schema": SCHEMA,
        "leaves": len(rows),
        "report_unknown_alias": diagnostics["diagnostic_counts"].get("unknown_alias"),
        "classes": dict(classes.most_common()),
        "subclasses": {f"{c}/{s}": n for (c, s), n in subclasses.most_common()},
        "groups": dict(collections.Counter(row["group"] for row in rows).most_common()),
        "causes": dict(collections.Counter(row["facts"].get("cause", "n/a") for row in rows).most_common()),
        "first_leaves": sum(1 for row in rows if row["first"]),
        "first_leaves_per_class": dict(collections.Counter(row["class"] for row in rows if row["first"]).most_common()),
        "poisoning_refusal": dict(collections.Counter(
            row["facts"]["first_refusal"] for row in rows if row["facts"].get("first_refusal")).most_common()),
        "units_matched": len(unit_of),
        "unmatched": {k: len(v) for k, v in unmatched.items()},
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps(summary, indent=1))
    bad = len(unmatched["units"]) + len(unmatched["functions"]) + len(errors)
    return 3 if bad and not partial else 0


if __name__ == "__main__":
    sys.exit(main())
