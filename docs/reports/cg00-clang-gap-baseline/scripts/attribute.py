#!/usr/bin/env python3
"""CG00-CLANG-GAP-BASELINE: attribute every unsupported leaf of the corpus
arbiter to at most one Clang gap entry ID (or `unattributed:<reason>`), and
count, per CG card, the Unknown functions it would free.

usage: attribute.py CENSUS_RUN DIAGNOSTICS CONTRACTS OUT_DIR [--jobs N]

CENSUS_RUN   one `run_census.py corpus` run of the `indirect-census` build
             (its `access-path-units/` holds the exact analysed texts)
DIAGNOSTICS  the default build's `resource-diagnostics.json` (the arbiter)
CONTRACTS    that run's `resource-diagnostics.contracts.json`

No new scanner. The typed AST, the construct walk, the access-path
decomposition and the unit join are W46-ACCESS-PATH-CENSUS's
`access_census.py`, imported, not copied. Two thin extensions:

* `FeatureWalker` subclasses its `Walker`: while the census walks a
  construct it also records each node kind, operator, cast kind and a few
  type markers (`_Atomic`, `volatile`, `_Complex`, vectors, `_BitInt`) into
  the construct's `flags`, and marks a cast whose operand is `void *` in the
  path's causes (`cast-from-void`).
* A per-function table of local declarations by line (VLA, static and
  thread-local storage, string and list initialisers, `cleanup`).

Each leaf binds to its function's constructs at its line (the census's own
join: a construct of the leaf's kind first, any construct at the line
otherwise), then `classify` applies one fixed precedence (documented in the
report, §Attribution). Everything is keyed on the leaf's own description
family first; the typed shape decides the entry within it.
"""

import collections
import gzip
import json
import multiprocessing
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w46-access-path-census" / "scripts"))
import access_census as census  # noqa: E402

SCHEMA = "qcore.cg00-clang-gap-attribution.v1"

CARD = {}
for card, entries in {
    "CG01": "E01 E02 E14",
    "CG02": "E03 E04 E05 E08",
    "CG03": "K12 K03",
    "CG04": "E09 E13",
    "CG05": "E10 E11 E12",
    "CG06": "E06 E07",
    "CG07": "E17 E19",
    "CG08": "E15 E16 N11 M18 N08",
    "CG09": "C01 C02 C03",
    "CG10": "M01 I04",
    "CG11": "M02 M03 M04",
    "CG12": "M05",
    "CG13": "M06 M07",
    "CG14": "M08 M09 M10 M11",
    "CG15": "M24 M23",
    "CG16": "M14 M15 M16 M17",
    "CG17": "M12 M13 M19 E18",
    "CG18": "M20 M21 M22 M26 W08",
    "CG19": "W01 W09",
    "CG20": "N01 N02 N03 N04 N05 N06 N07 C04 C05",
    "CG21": "C09 C10 C11 C12 C13 E20 I06",
    "CG22": "K01 K02",
    "CG23": "K05 K11",
    "CG24": "K04 K06",
    "CG25": "K07",
    "CG26": "K08 K09 K10",
    "CG27": "C06 C07 C08",
    "CG28": "W02 W03 W04 W05 W06",
    "CG29": "W07 W10",
    "CG30": "I01 I02 I03 I05 N12 N13 N14",
}.items():
    for entry in entries.split():
        CARD[entry] = card

TYPE_MARKERS = {"_Atomic": "atomic", "volatile": "volatile", "_Complex": "complex",
                "vector_size": "vector", "ext_vector_type": "vector", "_BitInt": "bitint"}


class FeatureWalker(census.Walker):
    """The census walker, also recording what it walks into `flags`."""

    def walk(self, node, ctx):
        kind = node.get("kind")
        if kind:
            self.flags.add("k:" + kind)
            if node.get("opcode"):
                self.flags.add(("post:" if node.get("isPostfix") else "op:") + node["opcode"])
            if node.get("castKind"):
                self.flags.add("cast:" + node["castKind"])
            if kind == "UnaryExprOrTypeTraitExpr":
                self.flags.add("trait:" + str(node.get("name")))
            text = (node.get("type") or {}).get("qualType", "")
            for marker, flag in TYPE_MARKERS.items():
                if marker in text:
                    self.flags.add("type:" + flag)
        return super().walk(node, ctx)

    def decompose(self, top, ctx):
        path = super().decompose(top, ctx)
        extra = set()
        current = top
        for _ in range(64):
            kind = current.get("kind")
            inner = census.children(current)
            if not inner:
                break
            if kind in ("CStyleCastExpr",) or (kind == "ImplicitCastExpr" and current.get(
                    "castKind") not in ("LValueToRValue", "NoOp", "ArrayToPointerDecay")):
                operand = (inner[0].get("type") or {}).get("qualType", "")
                if re.search(r"\bvoid\s*\*", operand):
                    extra.add("cast-from-void")
            if kind in ("DeclRefExpr", "CallExpr"):
                break
            if kind == "ArraySubscriptExpr" and len(inner) == 2:
                first_kind = self.unit.kind_of(self.unit.qual(inner[0]))[0]
                current = inner[0] if first_kind == "pointer" else inner[1]
                continue
            current = inner[0]
        for step in path["steps"]:
            if (step.get("type") or "").rstrip().endswith("[]"):
                extra.add("flexible-array")
        if extra:
            path["causes"] = sorted(set(path["causes"]) | extra)
        return path


census.Walker = FeatureWalker

VLA = re.compile(r"\[[^\]]*[A-Za-z_(][^\]]*\]")


def declaration_facts(unit, fn):
    """Local declarations by line: VLA, static/thread storage, initialiser."""
    facts = collections.defaultdict(set)
    stack = [fn]
    while stack:
        node = stack.pop()
        inner = census.children(node)
        if node.get("kind") == "VarDecl":
            line = unit.line(node)
            text = (node.get("type") or {}).get("qualType", "")
            if VLA.search(text):
                facts[line].add("vla")
            if node.get("storageClass") in ("static", "extern"):
                facts[line].add("static-storage")
            if node.get("tls"):
                facts[line].add("thread-local")
            if any(c.get("kind") == "CleanupAttr" for c in inner):
                facts[line].add("cleanup")
            if text.rstrip().endswith("]") and any(
                    c.get("kind") == "StringLiteral" for c in inner):
                facts[line].add("string-array-init")
            for c in inner:
                if c.get("kind") == "InitListExpr":
                    facts[line].add("list-init")
        stack.extend(inner)
    return {line: sorted(values) for line, values in facts.items()}


def extract_unit(arguments):
    directory, record = arguments
    text = (pathlib.Path(directory) / record["file"]).read_bytes()
    command = [census.CLANG, "--driver-mode=gcc", *record["frontend_flags"], "-x", "c",
               record["dialect"], "-fsyntax-only", "-Xclang", "-ast-dump=json", "-"]
    completed = subprocess.run(command, input=text, capture_output=True)
    if completed.returncode != 0:
        return record["unit_seq"], {"error": completed.stderr.decode(errors="replace")[:2000]}
    tu = json.loads(completed.stdout)
    unit = census.Unit(text)
    census.collect_tables(unit, tu)
    definitions = {}
    declared = set()
    for decl in census.children(tu):
        if decl.get("kind") != "FunctionDecl":
            continue
        declared.add(decl.get("name"))
        if not any(c.get("kind") == "CompoundStmt" for c in census.children(decl)):
            continue
        definitions[decl.get("name")] = {
            "constructs": census.function_constructs(unit, decl),
            "declarations": declaration_facts(unit, decl),
        }
    return record["unit_seq"], {"definitions": definitions, "declared": sorted(declared)}


def family(description):
    fam, sub = census.family(description)
    if fam:
        return fam, sub
    if description.startswith("call to uncontracted function"):
        return "UNC", None
    return "OTHER", None


def callee_of(description):
    match = re.search(r"`([^`]*)`", description)
    return match.group(1) if match else None


def classify_callee(name, declared, corpus_functions):
    """An uncontracted call: which entry, if any, its callee's shape names."""
    if name in ("volatile", "__asm__", "asm", "goto", "__volatile__"):
        return "C12", "asm-read-as-call"
    if name == "_Generic":
        return "E17", "generic-selection"
    if name.startswith("__builtin_va_") or name in ("va_start", "va_arg", "va_end", "va_copy"):
        return "K09", "va-list-builtin"
    if name.startswith(("__c11_atomic", "__atomic", "__sync")):
        if "compare_exchange" in name or "compare_and_swap" in name:
            return "W03", "atomic-cas"
        if "fence" in name or name == "__sync_synchronize":
            return "W04", "atomic-fence"
        if re.search(r"fetch|exchange|add_and|sub_and|_or_|_and_|test_and_set", name):
            return "W02", "atomic-rmw"
        return "W01", "atomic-load-store"
    if name in ("__builtin_offsetof", "offsetof"):
        return "E18", "offsetof"
    if name in ("setjmp", "_setjmp", "sigsetjmp", "__sigsetjmp", "longjmp", "siglongjmp", "_longjmp"):
        return "C10", "setjmp-longjmp"
    if name.startswith("__builtin_"):
        return "E20", "builtin-as-call"
    if name not in declared:
        return "K01", "indirect-callee"
    if name in corpus_functions:
        return None, "callee-closure"
    return None, "uncontracted-external"


# Path causes, most specific first: the first one any path of the leaf carries
# names the entry.
CAUSES = [
    ("statement-expression", "E19"),
    ("union", "M06"),
    ("bitfield", "M07"),
    ("flexible-array", "M13"),
    ("pointer-valued-store", None),  # M02 / M03 / M04 by shape, below
    ("pointer-field-step", "M02"),
    ("aggregate-value", "M08"),
    ("cast-from-void", "M05"),
    ("cast", "E15"),
    ("incomplete-record", "I03"),
    ("indirect-call", "K01"),
    ("function-designator", "K01"),
    ("call", "E11"),
    ("pointer-arithmetic", "M15"),
    ("pointer-step", "E05"),
    ("static-local", "M24"),
    ("address-of-base", "M01"),
]
ROOTS = {"CompoundLiteralExpr": "M10", "StmtExpr": "E19", "ConditionalOperator": "E07",
         "BinaryConditionalOperator": "E07", "GenericSelectionExpr": "E17",
         "PredefinedExpr": "M11", "StringLiteral": "M11", "VAArgExpr": "K09"}


def store_entry(path):
    steps = path["steps"]
    if len(steps) > 1 or any(s["op"] == "[]" for s in steps) or path["value"] == "function_pointer":
        return "M04"
    if path["root_kind"] in ("formal", "global"):
        return "M03"
    return "M02"


def path_entry(paths):
    causes = set()
    for path in paths:
        causes |= set(path["causes"])
    if "volatile-atomic" in causes:
        return None, causes  # split by the construct's type markers
    for cause, entry in CAUSES:
        if cause in causes:
            if entry is None:
                stores = [p for p in paths if "pointer-valued-store" in p["causes"]]
                return store_entry(stores[0]), causes
            return entry, causes
    for cause in causes:
        if cause.startswith("root:"):
            return ROOTS.get(cause[5:], "unattributed:root-" + cause[5:]), causes
    return None, causes


def feature_entry(flags, kind):
    """Construct features, for a leaf no path cause explains."""
    if "type:complex" in flags:
        return "N12"
    if "type:vector" in flags:
        return "N13"
    if "type:bitint" in flags:
        return "N14"
    if "k:StmtExpr" in flags:
        return "E19"
    if "k:GenericSelectionExpr" in flags:
        return "E17"
    if "k:OffsetOfExpr" in flags:
        return "E18"
    if "k:VAArgExpr" in flags:
        return "K09"
    if "k:AtomicExpr" in flags:
        return "W01"
    if "k:CompoundLiteralExpr" in flags:
        return "M10"
    if "k:InitListExpr" in flags:
        return "M09"
    if "k:ConditionalOperator" in flags or "k:BinaryConditionalOperator" in flags:
        return "E07"
    if "op:," in flags:
        return "E08"
    if kind != "condition" and ("op:&&" in flags or "op:||" in flags):
        return "E06"
    if "post:++" in flags or "post:--" in flags or "op:++" in flags or "op:--" in flags:
        return "E05"
    if any(f.startswith("trait:") for f in flags):
        return "E16"
    if any(f in flags for f in ("cast:PointerToIntegral", "cast:IntegralToPointer")):
        return "M18"
    if any(f in flags for f in ("cast:IntegralToFloating", "cast:FloatingToIntegral",
                                "cast:FloatingCast", "cast:FloatingToBoolean")):
        return "N08"
    return None


def classify(leaf, constructs, declarations, declared, corpus_functions, contracted):
    description = leaf["description"]
    fam, sub = family(description)
    line = leaf["line"]
    decl = set(declarations.get(line, ()))
    own = [c for c in constructs if census.construct_matches(c, fam, sub)] if fam in ("IND", "COND", "RET") else []
    bound = own or constructs
    flags = set()
    paths = []
    calls = set()
    for construct in bound:
        flags |= set(construct["flags"])
        paths.extend(construct["paths"])
        calls |= {c for c in construct["calls"] if c}
    kind = bound[0]["kind"] if bound else None

    if fam == "UNC":
        entry, reason = classify_callee(callee_of(description), declared, corpus_functions)
        return entry or "unattributed:" + reason, reason

    if fam in ("IND", "COND", "RET"):
        if "vla" in decl:
            return "M12", "vla-declaration"
        entry, causes = path_entry(paths)
        if "volatile-atomic" in causes:
            return ("W01" if "type:atomic" in flags else "M23"), "volatile-atomic"
        if entry:
            return entry, "path:" + ",".join(sorted(causes))
        feature = feature_entry(flags, kind)
        if feature:
            return feature, "construct-feature"
        uncontracted = sorted(c for c in calls if c != "<indirect>" and c not in contracted
                              and c not in corpus_functions)
        if "<indirect>" in calls:
            return "K01", "indirect-call"
        if fam == "COND" and calls:
            if uncontracted:
                return "unattributed:uncontracted-callee-in-condition", ",".join(uncontracted)
            if any(c.get("rhs_call") for c in bound):
                return "E09", "call-result-assigned-in-condition"
            return "E13", "call-in-condition-leaf"
        if fam == "RET" and calls:
            if uncontracted:
                return "unattributed:uncontracted-callee-in-return", ",".join(uncontracted)
            return "K12", "contracted-call-in-return"
        if paths:
            same_line = sum(1 for c in constructs if c["kind"] in ("statement", "condition", "return"))
            if same_line > 1:
                return "I04", "expressible-path-on-a-shared-line"
            return "M01", "expressible-path-premise"
        if fam == "IND" and calls:
            if uncontracted:
                return "unattributed:uncontracted-callee", ",".join(uncontracted)
            return "E11", "call-in-statement"
        return "unattributed:no-typed-shape", kind or "no-construct"

    # OTHER descriptions: the family names the refusal; the shape the entry.
    d = description
    if d == "braced initializer, expression, or declaration is not modeled":
        if "vla" in decl:
            return "M12", "vla-declaration"
        if "string-array-init" in decl:
            return "M11", "string-array-initializer"
        if "static-storage" in decl or "thread-local" in decl:
            return "M24", "static-storage-declaration"
        if "cleanup" in decl:
            return "C11", "cleanup-attribute"
        feature = feature_entry(flags, kind)
        if feature:
            return feature, "construct-feature"
        if "list-init" in decl:
            return "M09", "list-initializer"
        entry, causes = path_entry(paths)
        if entry:
            return entry, "path:" + ",".join(sorted(causes))
        return "unattributed:initializer-other", ",".join(sorted(f for f in flags if f.startswith("k:"))[:6])
    if d == "unsupported assignment expression":
        feature = feature_entry(flags, kind)
        if feature:
            return feature, "construct-feature"
        entry, causes = path_entry(paths)
        if entry:
            return entry, "path:" + ",".join(sorted(causes))
        if "k:CompoundAssignOperator" in flags:
            return "E03", "compound-assignment"
        if calls:
            return "E11", "call-in-assignment"
        if "cast:BitCast" in flags or "k:CStyleCastExpr" in flags:
            return "E15", "cast-in-assignment"
        return "unattributed:assignment-other", ",".join(sorted(f for f in flags if f.startswith(("k:", "op:")))[:8])
    if d == "unrecognized assignment destination":
        if "string-array-init" in decl:
            return "M11", "string-array-initializer"
        entry, causes = path_entry(paths)
        if entry:
            return entry, "path:" + ",".join(sorted(causes))
        if "k:CompoundAssignOperator" in flags:
            return "E03", "compound-assignment"
        feature = feature_entry(flags, kind)
        if feature:
            return feature, "construct-feature"
        if paths:
            return "M01", "unrecognized-place"
        return "unattributed:destination-other", ",".join(sorted(f for f in flags if f.startswith(("k:", "op:")))[:8])
    if d.startswith("C operator `sizeof`"):
        return "E16", "sizeof-operand"
    if d == "unmodeled increment or decrement expression":
        return "E05", "value-producing-increment"
    if d.startswith("store through `"):
        return "M02", "store-through-slot"
    if d.startswith("arguments beyond the declared parameters"):
        return "K08", "variadic-tail"
    if d.startswith("nested call `"):
        if "op:&&" in flags or "op:||" in flags:
            return "E06", "connective-around-nested-call"
        if "k:ConditionalOperator" in flags:
            return "E07", "conditional-around-nested-call"
        return "E10", "nested-call"
    if d == "unmodeled expression may mutate a formal resource identity":
        if "post:++" in flags or "post:--" in flags or "op:++" in flags or "op:--" in flags:
            return "E05", "increment-in-expression"
        return "E04", "assignment-in-expression"
    if d == "local declaration may shadow a formal resource identity":
        if "vla" in decl:
            return "M12", "vla-declaration"
        return "I04", "name-identity"
    if d.startswith("discarded owned result of resource API"):
        if any("pointer-valued-store" in p["causes"] for p in paths) or any(
                p["access"] == "store" for p in paths):
            return "M02", "owned-result-stored-into-place"
        if kind == "condition":
            return "E09", "owned-result-in-condition"
        return "unattributed:discarded-owned-result", kind or ""
    if d == "resource call with a formal embedded in an unmodeled expression":
        feature = feature_entry(flags, kind)
        return (feature or "E11"), "formal-in-unmodeled-expression"
    if d == "call has an unmodeled formal address or borrowed expression":
        return "K03", "formal-address-actual"
    if d.startswith("arguments do not match resource contract"):
        return "K12", "contract-arity-or-shape"
    if d.startswith("for-loop clause with a call or grouping"):
        return ("E08" if "op:," in flags else "E13"), "for-clause"
    if "abi-unreadable-declaration" in d:
        return "I03", "abi-unreadable-declaration"
    if d.startswith("alias set exceeded"):
        return "C08", "alias-budget"
    if d.startswith("nullable resource `"):
        return "unattributed:nullable-obligation", "undischarged-null-obligation"
    if d.startswith(("unlink by", "publication by", "transfer of")):
        return "unattributed:container-protocol", "container-protocol"
    if "thread-synchronization obligation" in d or "literal-argument premise" in d:
        return "unattributed:declared-premise", "declared-premise"
    if d.startswith("tracked descriptor"):
        return "unattributed:descriptor-scalar-read", "descriptor"
    if d.startswith("publication of a tracked resource into the static-storage object"):
        return "M24", "static-storage-publication"
    return "unattributed:other", d[:80]


def main():
    run, diagnostics_path, contracts_path, out = (pathlib.Path(a) for a in sys.argv[1:5])
    jobs = int(sys.argv[sys.argv.index("--jobs") + 1]) if "--jobs" in sys.argv else 8
    out.mkdir(parents=True, exist_ok=True)
    diagnostics = census.load(diagnostics_path)
    contracts = census.load(contracts_path)
    contracted = set()
    for text in contracts.get("sources", {}).values():
        contracted |= set(re.findall(r"extern\s+fn\s+([A-Za-z_]\w*)\s*\(", text))
    directory = run / "access-path-units"
    records = [json.loads(l) for l in (directory / "units.jsonl").read_text().splitlines()]
    with multiprocessing.Pool(jobs) as pool:
        extracted = dict(pool.map(extract_unit, [(str(directory), r) for r in records], chunksize=1))
    errors = {seq: v["error"] for seq, v in extracted.items() if "error" in v}

    corpus_functions = {f["function"] for u in diagnostics["units"] for f in u["functions"]}
    unit_of, unmatched = {}, []
    for document in diagnostics["units"]:
        names = {f["function"] for f in document["functions"]}
        if not names:
            continue
        candidates = [seq for seq, v in extracted.items()
                      if "definitions" in v and names <= set(v["definitions"])]
        if len(candidates) != 1:
            unmatched.append({"file": document["file"], "candidates": candidates})
            continue
        unit_of[document["file"]] = candidates[0]

    rows = []
    for document in diagnostics["units"]:
        seq = unit_of.get(document["file"])
        for function in document["functions"]:
            definition = extracted[seq]["definitions"].get(function["function"]) if seq is not None else None
            declared = set(extracted[seq]["declared"]) if seq is not None else set()
            for leaf in function["diagnostics"]:
                if leaf["category"] != "unsupported":
                    continue
                if definition is None:
                    entry, reason = "unattributed:unmatched-function", ""
                else:
                    constructs = [c for c in definition["constructs"] if leaf["line"] in c["lines"]]
                    entry, reason = classify(leaf, constructs, definition["declarations"], declared,
                                             corpus_functions, contracted)
                rows.append({
                    "file": document["file"], "function": function["function"],
                    "status": function["status"], "line": leaf["line"], "origin": leaf.get("origin"),
                    "family": family(leaf["description"])[0],
                    "description": re.sub(r"`[^`]*`", "`X`", leaf["description"]),
                    "entry": entry, "card": CARD.get(entry, "unattributed"), "reason": reason,
                })
    with gzip.open(out / "leaves.jsonl.gz", "wt") as handle:
        handle.write(json.dumps({"schema": SCHEMA, "kind": "leaf-attribution"}) + "\n")
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")
    (out / "unmatched.json").write_text(json.dumps({"units": unmatched, "clang_errors": errors}, indent=1) + "\n")
    print(f"leaves {len(rows)}; units matched {len(unit_of)}; unmatched units {len(unmatched)}; "
          f"clang errors {len(errors)}")
    return 3 if unmatched or errors else 0


if __name__ == "__main__":
    sys.exit(main())
