#!/usr/bin/env python3
"""Card W46-ACCESS-PATH-CENSUS: classify every IND, COND and RET leaf by its
Clang-typed access paths.

usage: access_census.py RUN_DIR DIAGNOSTICS(.json|.json.gz) CONTRACTS.json.gz OUT_DIR [--jobs N]

RUN_DIR is one census run (run_census.py) of the census-feature build. Its
`access-path-units/` holds, per translation-unit analysis, the exact
preprocessed text the analyzer read (`unit-<seq>.c`, whose lines are the
leaves' internal `line` identities) and `units.jsonl` with the dialect and
frontend flags. Each text is given to Clang with exactly the argument vector
`qcore_c_gateway::typed_c_ast::translation_unit_ast` builds
(`--driver-mode=gcc FLAGS -x c DIALECT -fsyntax-only -Xclang -ast-dump=json -`),
so the typed AST classified here is the gateway's own typed-AST transport of
the analysed text. Nothing is read from the original sources.

Join. A unit ordinal is matched to a document unit when the document's
function set is contained in the ordinal's definitions (and exactly one
ordinal qualifies). A leaf `(unit, function, line, family)` is explained by
the constructs of its function at its line of the matching kind: a statement
or `for` clause for `IND`, the controlling expression of the matching
statement kind for `COND`, a `return` for `RET` (an `IND` leaf at a `return f(a->b)`
or a desugared condition binds to that construct: its call is lowered to a
statement at that line). The join refuses to publish (exit 3,
`unmatched.json`) when a leaf has no construct of its own at its line (fewer
constructs than leaves), a construct is bound to two leaf groups, a unit or
function has no unique match, or Clang rejects a unit's text. Several
constructs at one line (a macro expansion) explain their leaf together: the
leaf is expressible only if every path of every one of them is.

Every construct's expression is walked for access paths: a maximal lvalue
built from `.f`, `->f`, `*` and `[i]` steps over a root. Each path carries its
root kind, its steps with Clang's type (typedefs resolved by the unit's own
typedef table), its final value kind and the position it sits in, and is
either expressible or refused with named causes. Output: `census.json`
(schema line first), `leaves.jsonl.gz` (one row per leaf with its constructs
and paths) and `summary.txt`.
"""

import bisect
import collections
import gzip
import json
import multiprocessing
import pathlib
import re
import subprocess
import sys

SCHEMA = "qcore.w46-access-path-census.v1"
CLANG = "/usr/bin/clang"

CONDITION_KIND = {
    "branch condition": "if",
    "loop condition": "while",
    "for-loop condition": "for",
    "do-while condition": "do",
    "switch expression": "switch",
}
FAMILY_IND = "indirect memory access is outside the resource projection"
FAMILY_PREFIX = "unmodeled call, aggregate or indirect access in "

ACCESS = {"MemberExpr", "ArraySubscriptExpr"}
# Clang spells an anonymous record `struct T::(unnamed at <stdin>:L:C)` or
# `union (anonymous union at <stdin>:L:C)`; the census names it `ANON`.
ANONYMOUS = re.compile(r"(?:[A-Za-z_]\w*::)*\((?:unnamed|anonymous)[^)]*\)")
QUALIFIERS = {"const", "volatile", "restrict", "__restrict", "_Atomic", "__restrict__"}
KEYWORDS = {"struct", "union", "enum", "unsigned", "signed", "int", "char", "short", "long",
            "float", "double", "void", "_Bool", "__int128", "_Complex"} | QUALIFIERS


def family(description):
    if description == FAMILY_IND:
        return "IND", None
    if description.startswith(FAMILY_PREFIX):
        rest = description[len(FAMILY_PREFIX):]
        if rest == "return expression":
            return "RET", None
        if rest in CONDITION_KIND:
            return "COND", CONDITION_KIND[rest]
    return None, None


# ---------------------------------------------------------------------------
# Stage A: one unit's typed AST -> constructs with access paths
# ---------------------------------------------------------------------------

class Unit:
    def __init__(self, text):
        self.starts = [0]
        for index, byte in enumerate(text):
            if byte == 10:
                self.starts.append(index + 1)
        self.typedefs = {}
        self.fields = {}
        self.records = {}  # "struct tag" -> complete?
        self.globals = set()
        self.function_kinds = {}

    def line(self, node):
        begin = node.get("range", {}).get("begin", {})
        offset = begin.get("offset")
        if offset is None:
            offset = begin.get("expansionLoc", {}).get("offset")
        if offset is None:
            offset = node.get("loc", {}).get("offset")
        if offset is None:
            return None
        return bisect.bisect_right(self.starts, offset)

    def end_line(self, node):
        end = node.get("range", {}).get("end", {})
        offset = end.get("offset")
        if offset is None:
            offset = end.get("expansionLoc", {}).get("offset")
        if offset is None:
            return None
        return bisect.bisect_right(self.starts, offset)

    # -- types ------------------------------------------------------------
    def qual(self, node):
        t = node.get("type") or {}
        return t.get("desugaredQualType") or t.get("qualType") or ""

    def resolve(self, text, depth=0):
        """Substitute typedef names by their underlying types (textually)."""
        if depth > 12:
            return text
        text = ANONYMOUS.sub("ANON", text)
        out = []
        changed = False
        tokens = re.findall(r"[A-Za-z_][A-Za-z0-9_]*|\S", text)
        previous = None
        for token in tokens:
            if (re.match(r"[A-Za-z_]", token) and token not in KEYWORDS
                    and previous not in ("struct", "union", "enum")
                    and token in self.typedefs):
                out.append(self.typedefs[token])
                changed = True
            else:
                out.append(token)
            previous = token
        if not changed:
            return text
        return self.resolve(" ".join(out), depth + 1)

    def kind_of(self, text):
        resolved = self.resolve(text)
        flags = set()
        if "volatile" in resolved.split():
            flags.add("volatile")
        if "_Atomic" in resolved:
            flags.add("atomic")
        bare = " ".join(t for t in resolved.split() if t not in QUALIFIERS)
        bare = re.sub(r"\(unnamed[^)]*\)|\(anonymous[^)]*\)", "ANON", bare)
        if "(" in bare:
            if re.search(r"\( \* \)", bare) or re.search(r"\(\*\)", bare):
                return "function_pointer", bare, flags
            if bare.rstrip().endswith("*"):
                return "pointer", bare, flags
            return "function", bare, flags
        stripped = bare.rstrip()
        if stripped.endswith("*"):
            return "pointer", bare, flags
        if stripped.endswith("]"):
            return "array", bare, flags
        if bare.startswith("struct ") or bare.startswith("union "):
            return "record", bare, flags
        return "scalar", bare, flags

    def record_complete(self, bare):
        """False only for a record the unit declares and never defines. A
        member selection needs a complete type, so Clang has already refused
        any other incomplete case; an absent tag (a typedef-named anonymous
        record) is complete."""
        words = bare.split()
        if len(words) < 2 or words[1] == "ANON":
            return True
        key = words[0] + " " + words[1]
        return self.records.get(key, True)


def children(node):
    return [c for c in node.get("inner", []) if isinstance(c, dict)]


def strip(node):
    while node.get("kind") in ("ParenExpr", "ImplicitCastExpr") and children(node):
        if node["kind"] == "ImplicitCastExpr" and node.get("castKind") not in (
                "LValueToRValue", "NoOp", "ArrayToPointerDecay", "FunctionToPointerDecay"):
            break
        node = children(node)[0]
    return node


def direct_callee(call):
    callee = strip(children(call)[0]) if children(call) else {}
    if callee.get("kind") == "DeclRefExpr":
        ref = callee.get("referencedDecl", {})
        if ref.get("kind") == "FunctionDecl":
            return ref.get("name")
    return None


def contains_call(node):
    stack = [node]
    while stack:
        current = stack.pop()
        kind = current.get("kind")
        if kind == "CallExpr":
            return True
        if kind in ("UnaryExprOrTypeTraitExpr",):
            continue
        stack.extend(children(current))
    return False


class Walker:
    """Collect the access paths and calls of one construct's expressions."""

    def __init__(self, unit, params, locals_):
        self.unit = unit
        self.params = params
        self.locals = locals_
        self.paths = []
        self.calls = []
        self.flags = set()

    def walk(self, node, ctx):
        kind = node.get("kind")
        if kind is None:
            return
        if kind in ("UnaryExprOrTypeTraitExpr", "OffsetOfExpr"):
            return  # unevaluated operand
        if kind in ACCESS or (kind == "UnaryOperator" and node.get("opcode") == "*"):
            self.paths.append(self.decompose(node, ctx))
            return
        if kind == "CallExpr":
            name = direct_callee(node)
            inner = children(node)
            self.calls.append(name or "<indirect>")
            if name is None and inner:
                self.walk(inner[0], ctx | {"callee"})
            for argument in inner[1:]:
                self.walk(argument, (ctx - {"callee"}) | {"call-argument"})
            return
        if kind == "AtomicExpr":
            for child in children(node):
                self.walk(child, ctx | {"atomic"})
            return
        if kind in ("BinaryOperator", "CompoundAssignOperator"):
            op = node.get("opcode")
            inner = children(node)
            if len(inner) == 2 and (op == "=" or kind == "CompoundAssignOperator"):
                null_value = {"null-value"} if op == "=" and is_null_constant(inner[1]) else set()
                self.walk(inner[0], (ctx - {"read", "address", "decay"}) | {"store"} | null_value)
                self.walk(inner[1], ctx - {"store"})
                return
            for child in inner:
                self.walk(child, ctx - {"store"})
            return
        if kind == "UnaryOperator":
            op = node.get("opcode")
            inner = children(node)
            if op == "&":
                for child in inner:
                    self.walk(child, ctx | {"address"})
                return
            if op in ("++", "--"):
                for child in inner:
                    self.walk(child, ctx | {"increment"})
                return
            for child in inner:
                self.walk(child, ctx)
            return
        if kind in ("ConditionalOperator", "BinaryConditionalOperator"):
            inner = children(node)
            for index, child in enumerate(inner):
                self.walk(child, ctx | ({"?:-condition"} if index == 0 else {"?:-arm"}))
            return
        if kind == "ImplicitCastExpr":
            cast = node.get("castKind")
            extra = {"decay"} if cast == "ArrayToPointerDecay" else ({"read"} if cast == "LValueToRValue" else set())
            for child in children(node):
                self.walk(child, (ctx - {"decay", "read"}) | extra)
            return
        if kind == "StmtExpr":
            self.flags.add("statement-expression")
            for child in children(node):
                self.statement_expressions(child, ctx | {"statement-expression"})
            return
        for child in children(node):
            self.walk(child, ctx - {"decay", "read"} if kind not in ("ParenExpr",) else ctx)

    def statement_expressions(self, node, ctx):
        kind = node.get("kind", "")
        if kind.endswith("Stmt") and kind not in ("DeclStmt",):
            for child in children(node):
                self.statement_expressions(child, ctx)
        elif kind == "DeclStmt":
            for decl in children(node):
                for child in children(decl):
                    self.walk(child, ctx)
        else:
            self.walk(node, ctx)

    def decompose(self, top, ctx):
        unit = self.unit
        steps = []
        causes = set()
        hops = 0
        root = None
        root_kind = "other"
        root_type = None
        current = top
        # Positions of index expressions are walked separately as reads.
        while True:
            kind = current.get("kind")
            inner = children(current)
            if kind == "MemberExpr":
                field = unit.fields.get(current.get("referencedMemberDecl"), {})
                kind_, bare, flags = unit.kind_of(unit.qual(current))
                step = {"op": "->" if current.get("isArrow") else ".", "member": current.get("name"),
                        "type": bare, "value": kind_}
                if field.get("union"):
                    causes.add("union")
                if field.get("bitfield"):
                    causes.add("bitfield")
                if flags:
                    causes.add("volatile-atomic")
                base = inner[0] if inner else {}
                probe = base
                while probe.get("kind") == "ParenExpr" or (
                        probe.get("kind") == "ImplicitCastExpr" and probe.get("castKind") == "NoOp"):
                    probe = children(probe)[0] if children(probe) else {}
                through_address = (current.get("isArrow") and probe.get("kind") == "UnaryOperator"
                                   and probe.get("opcode") == "&" and children(probe))
                if through_address:
                    # `(&E)->f` selects E.f: a same-object step, no pointer read.
                    base = children(probe)[0]
                    step["op"] = "."
                    step["via"] = "(&E)->"
                base_kind, base_bare, base_flags = unit.kind_of(unit.qual(base))
                record_text = base_bare
                if current.get("isArrow") and not through_address:
                    hops += 1
                    record_text = base_bare.rstrip().rstrip("*").strip()
                if record_text.startswith("union "):
                    causes.add("union")
                if not unit.record_complete(record_text):
                    causes.add("incomplete-record")
                if base_flags:
                    causes.add("volatile-atomic")
                steps.append(step)
                current = base
                continue
            if kind == "ArraySubscriptExpr" and len(inner) == 2:
                first, second = inner
                first_kind = unit.kind_of(unit.qual(first))[0]
                base, index = (first, second) if first_kind == "pointer" else (second, first)
                if strip(base).get("kind") == "ImplicitCastExpr" or unit.kind_of(unit.qual(strip(base)))[0] == "pointer":
                    pass
                kind_, bare, flags = unit.kind_of(unit.qual(current))
                through_pointer = unit.kind_of(unit.qual(strip_decay(base)))[0] == "pointer"
                if through_pointer:
                    hops += 1
                if flags:
                    causes.add("volatile-atomic")
                if contains_call(index):
                    causes.add("call")
                steps.append({"op": "[]", "member": None, "type": bare, "value": kind_,
                              "through": "pointer" if through_pointer else "array"})
                self.walk(index, (ctx - {"store", "address", "decay", "increment"}) | {"index", "read"})
                current = base
                continue
            if kind == "UnaryOperator" and current.get("opcode") == "*":
                operand = inner[0] if inner else {}
                while operand.get("kind") == "ParenExpr" or (
                        operand.get("kind") == "ImplicitCastExpr" and operand.get("castKind") == "NoOp"):
                    operand = children(operand)[0] if children(operand) else {}
                if operand.get("kind") == "UnaryOperator" and operand.get("opcode") == "&" and children(operand):
                    # `*&E` is E: no step and no pointer read.
                    current = children(operand)[0]
                    continue
                kind_, bare, flags = unit.kind_of(unit.qual(current))
                hops += 1
                if flags:
                    causes.add("volatile-atomic")
                steps.append({"op": "*", "member": None, "type": bare, "value": kind_})
                current = inner[0] if inner else {}
                continue
            if kind == "UnaryOperator" and current.get("opcode") in ("++", "--"):
                causes.add("pointer-step")
                current = inner[0] if inner else {}
                continue
            if kind == "UnaryOperator" and current.get("opcode") == "&":
                causes.add("address-of-base")
                current = inner[0] if inner else {}
                continue
            if kind == "ParenExpr":
                current = inner[0] if inner else {}
                continue
            if kind == "ImplicitCastExpr":
                cast = current.get("castKind")
                if cast not in ("LValueToRValue", "NoOp", "ArrayToPointerDecay"):
                    causes.add("cast")
                current = inner[0] if inner else {}
                continue
            if kind in ("CStyleCastExpr",):
                causes.add("cast")
                current = inner[0] if inner else {}
                continue
            if kind == "BinaryOperator" and current.get("opcode") in ("+", "-") and len(inner) == 2:
                causes.add("pointer-arithmetic")
                left, right = inner
                pointer_left = unit.kind_of(unit.qual(left))[0] in ("pointer", "array")
                pointer, other = (left, right) if pointer_left else (right, left)
                self.walk(other, (ctx - {"store", "address", "decay", "increment"}) | {"index", "read"})
                current = pointer
                continue
            if kind == "DeclRefExpr":
                ref = current.get("referencedDecl", {})
                root = ref.get("name")
                ident = ref.get("id")
                if ref.get("kind") == "ParmVarDecl" or ident in self.params:
                    root_kind = "formal"
                elif ref.get("kind") == "VarDecl":
                    storage = self.locals.get(ident)
                    if storage is None:
                        root_kind = "global"
                    elif storage in ("static", "extern"):
                        root_kind = "global"
                        causes.add("static-local") if storage == "static" else None
                    else:
                        root_kind = "local"
                elif ref.get("kind") == "FunctionDecl":
                    root_kind = "other"
                    causes.add("function-designator")
                else:
                    root_kind = "other"
                    causes.add("root:" + str(ref.get("kind")))
                root_type = unit.kind_of(unit.qual(current))[1]
                if unit.kind_of(unit.qual(current))[2]:
                    causes.add("volatile-atomic")
                break
            if kind == "CallExpr":
                root_kind = "call-result"
                root = direct_callee(current) or "<indirect>"
                causes.add("call")
                self.calls.append(root)
                for argument in inner[1:]:
                    self.walk(argument, (ctx - {"store", "address", "decay", "increment", "callee"}) | {"call-argument"})
                break
            root_kind = "other"
            root = kind
            causes.add("root:" + str(kind))
            for child in inner:
                self.walk(child, (ctx - {"store", "address", "decay", "increment"}) | {"read"})
            break
        steps.reverse()
        final_kind, final_bare, final_flags = unit.kind_of(unit.qual(top))
        if final_flags:
            causes.add("volatile-atomic")
        if "address" in ctx or ("decay" in ctx and final_kind == "array"):
            value = "address"
        elif final_kind == "array":
            value = "address"
        elif final_kind == "record":
            value = "aggregate"
        elif final_kind == "function_pointer" or final_kind == "function":
            value = "function_pointer"
        elif final_kind == "pointer":
            value = "pointer"
        else:
            value = "scalar"
        if "store" in ctx:
            access = "store"
        elif "increment" in ctx:
            access = "increment"
        elif value == "address":
            access = "address"
        else:
            access = "read"
        if access == "store" and "null-value" in ctx and value in ("pointer", "function_pointer"):
            access = "null-store"
        if access == "store" and value in ("pointer", "function_pointer"):
            causes.add("pointer-valued-store")
        if access == "increment" and value == "pointer":
            causes.add("pointer-field-step")
        if value == "aggregate" and access in ("store", "read", "increment"):
            causes.add("aggregate-value")
        if "callee" in ctx:
            causes.add("indirect-call")
        if "atomic" in ctx:
            causes.add("volatile-atomic")
        if "statement-expression" in ctx:
            causes.add("statement-expression")
        return {
            "root": root,
            "root_kind": root_kind,
            "root_type": root_type,
            "steps": steps,
            "hops": hops,
            "root_hop": bool(steps) and (steps[0]["op"] in ("->", "*") or steps[0].get("through") == "pointer"),
            "value": value,
            "access": access,
            "call_argument": "call-argument" in ctx,
            "conditional_operator": bool(ctx & {"?:-condition", "?:-arm"}),
            "in_index": "index" in ctx,
            "causes": sorted(causes),
        }


def strip_decay(node):
    while node.get("kind") in ("ImplicitCastExpr", "ParenExpr") and children(node):
        if node.get("kind") == "ImplicitCastExpr" and node.get("castKind") == "ArrayToPointerDecay":
            return children(node)[0]
        if node.get("kind") == "ImplicitCastExpr" and node.get("castKind") not in ("LValueToRValue", "NoOp"):
            return node
        node = children(node)[0]
    return node


def declref_name(node):
    node = strip_all(node)
    if node.get("kind") == "DeclRefExpr":
        ref = node.get("referencedDecl", {})
        if ref.get("kind") in ("VarDecl", "ParmVarDecl"):
            return ref.get("name")
    return None


def strip_all(node):
    while node.get("kind") in ("ParenExpr", "ImplicitCastExpr", "CStyleCastExpr") and children(node):
        node = children(node)[0]
    return node


def is_null_constant(node):
    node = strip_all(node)
    return node.get("kind") == "IntegerLiteral" and node.get("value") == "0"


def null_tested(unit, node, out):
    """Names a controlling expression tests against null: `p`, `!p`,
    `p == NULL`, `p != NULL`, and the operands of `&&` / `||`."""
    bare = strip_all(node)
    kind = bare.get("kind")
    if kind == "BinaryOperator" and bare.get("opcode") in ("&&", "||"):
        for child in children(bare):
            null_tested(unit, child, out)
        return
    if kind == "UnaryOperator" and bare.get("opcode") == "!":
        for child in children(bare):
            null_tested(unit, child, out)
        return
    if kind == "BinaryOperator" and bare.get("opcode") == "=" and children(bare):
        null_tested(unit, children(bare)[0], out)
        return
    if kind == "BinaryOperator" and bare.get("opcode") in ("==", "!=") and len(children(bare)) == 2:
        left, right = children(bare)
        if is_null_constant(right):
            null_tested(unit, left, out)
        elif is_null_constant(left):
            null_tested(unit, right, out)
        return
    name = declref_name(bare)
    if name and unit.kind_of(unit.qual(bare))[0] == "pointer":
        out.add(name)


def expression_facts(unit, node, condition):
    call_args = []
    assigned = set()
    stack = [node]
    while stack:
        current = stack.pop()
        kind = current.get("kind")
        if kind in ("UnaryExprOrTypeTraitExpr", "OffsetOfExpr"):
            continue
        inner = children(current)
        if kind == "CallExpr":
            call_args.append([direct_callee(current) or "<indirect>",
                              [declref_name(argument) for argument in inner[1:]]])
        if kind in ("BinaryOperator", "CompoundAssignOperator") and inner and (
                current.get("opcode") == "=" or kind == "CompoundAssignOperator"):
            name = declref_name(inner[0])
            if name:
                assigned.add(name)
        if kind == "UnaryOperator" and current.get("opcode") in ("++", "--") and inner:
            name = declref_name(inner[0])
            if name:
                assigned.add(name)
        if kind == "VarDecl" and current.get("name"):
            assigned.add(current["name"])
        stack.extend(inner)
    rhs_call = False
    bare = strip_all(node)
    if bare.get("kind") == "BinaryOperator" and bare.get("opcode") == "=" and len(children(bare)) == 2:
        rhs_call = strip_all(children(bare)[1]).get("kind") == "CallExpr"
    tested = set()
    if condition:
        null_tested(unit, node, tested)
    return {"call_args": call_args, "assigned": sorted(assigned), "rhs_call": rhs_call,
            "null_tested": sorted(tested)}


def collect_tables(unit, tu):
    for decl in children(tu):
        kind = decl.get("kind")
        if kind == "VarDecl":
            unit.globals.add(decl.get("id"))
    stack = [tu]
    while stack:
        node = stack.pop()
        kind = node.get("kind")
        if kind == "TypedefDecl" and node.get("name"):
            t = node.get("type", {})
            unit.typedefs[node["name"]] = t.get("qualType", "")
        elif kind == "RecordDecl":
            tag = node.get("tagUsed", "struct")
            name = node.get("name")
            if name:
                key = tag + " " + name
                unit.records[key] = unit.records.get(key, False) or bool(node.get("completeDefinition"))
            for field in children(node):
                if field.get("kind") == "FieldDecl":
                    unit.fields[field.get("id")] = {
                        "union": tag == "union",
                        "bitfield": bool(field.get("isBitfield")),
                    }
        if kind in ("FunctionDecl",):
            continue_into = True
        stack.extend(children(node))


def function_constructs(unit, fn):
    params = set()
    body = None
    for child in children(fn):
        if child.get("kind") == "ParmVarDecl":
            params.add(child.get("id"))
        if child.get("kind") == "CompoundStmt":
            body = child
    locals_ = {}
    stack = [body]
    while stack:
        node = stack.pop()
        if node.get("kind") == "VarDecl":
            locals_[node.get("id")] = node.get("storageClass", "auto")
        stack.extend(children(node))
    constructs = []

    def expression_construct(kind, clause, node, lines):
        walker = Walker(unit, params, locals_)
        if node.get("kind") == "DeclStmt":
            for decl in children(node):
                for child in children(decl):
                    walker.walk(child, {"declaration"} if decl.get("kind") == "VarDecl" else set())
        else:
            walker.walk(node, set())
        facts = expression_facts(unit, node, kind == "condition")
        constructs.append({
            "kind": kind,
            "clause": clause,
            "lines": sorted({l for l in lines if l}),
            "paths": walker.paths,
            "calls": walker.calls,
            "flags": sorted(walker.flags),
            "block": list(block[-1]) if block else None,
            **facts,
        })

    block = []

    def visit(node):
        kind = node.get("kind")
        if kind is None:
            return
        inner = node.get("inner", [])
        line = unit.line(node)
        if kind == "CompoundStmt":
            for index, child in enumerate(children(node)):
                block.append((node.get("id"), index))
                visit(child)
                block.pop()
            return
        if kind == "IfStmt":
            items = [c for c in inner if isinstance(c, dict)]
            index = 0
            if node.get("hasInit"):
                visit(items[index]); index += 1
            if node.get("hasVar"):
                expression_construct("statement", "condition-variable", items[index], [unit.line(items[index])]); index += 1
            cond = items[index]; index += 1
            expression_construct("condition", "if", cond, [line, unit.line(cond)])
            for rest in items[index:]:
                visit(rest)
            return
        if kind == "WhileStmt":
            items = [c for c in inner if isinstance(c, dict)]
            index = 0
            if node.get("hasVar"):
                expression_construct("statement", "condition-variable", items[index], [unit.line(items[index])]); index += 1
            cond = items[index]
            expression_construct("condition", "while", cond, [line, unit.line(cond)])
            for rest in items[index + 1:]:
                visit(rest)
            return
        if kind == "DoStmt":
            items = [c for c in inner if isinstance(c, dict)]
            body_, cond = items[0], items[1]
            visit(body_)
            expression_construct("condition", "do", cond, [unit.line(cond), unit.end_line(node), line])
            return
        if kind == "ForStmt":
            items = list(inner) + [{}] * (5 - len(inner))
            init, var, cond, inc, body_ = items[:5]
            if init.get("kind"):
                expression_construct("statement", "for-init", init, [line, unit.line(init)])
            if var.get("kind"):
                expression_construct("statement", "condition-variable", var, [line])
            if cond.get("kind"):
                expression_construct("condition", "for", cond, [line, unit.line(cond)])
            if inc.get("kind"):
                expression_construct("statement", "for-step", inc, [line, unit.line(inc)])
            visit(body_)
            return
        if kind == "SwitchStmt":
            items = [c for c in inner if isinstance(c, dict)]
            index = 0
            if node.get("hasInit"):
                visit(items[index]); index += 1
            if node.get("hasVar"):
                index += 1
            cond = items[index]
            expression_construct("condition", "switch", cond, [line, unit.line(cond)])
            for rest in items[index + 1:]:
                visit(rest)
            return
        if kind in ("CaseStmt",):
            items = [c for c in inner if isinstance(c, dict)]
            if items:
                visit(items[-1])
            return
        if kind in ("DefaultStmt", "LabelStmt", "AttributedStmt"):
            for child in children(node):
                if child.get("kind", "").endswith("Stmt") or child.get("kind", "").endswith("Expr") or child.get("kind", "").endswith("Operator"):
                    visit(child)
            return
        if kind == "ReturnStmt":
            items = [c for c in inner if isinstance(c, dict)]
            if items:
                expression_construct("return", "return", items[0], [line])
            else:
                constructs.append({"kind": "return", "clause": "return-void", "lines": [line], "paths": [],
                                   "calls": [], "flags": [], "block": list(block[-1]) if block else None,
                                   "call_args": [], "assigned": [], "rhs_call": False, "null_tested": []})
            return
        if kind in ("BreakStmt", "ContinueStmt", "GotoStmt", "NullStmt", "IndirectGotoStmt", "GCCAsmStmt"):
            return
        # A simple statement: an expression or a declaration.
        expression_construct("statement", "statement", node, [line])

    if body is not None:
        visit(body)
    return constructs


def extract_unit(arguments):
    directory, record = arguments
    path = pathlib.Path(directory) / record["file"]
    text = path.read_bytes()
    command = [CLANG, "--driver-mode=gcc", *record["frontend_flags"], "-x", "c", record["dialect"],
               "-fsyntax-only", "-Xclang", "-ast-dump=json", "-"]
    completed = subprocess.run(command, input=text, capture_output=True)
    if completed.returncode != 0:
        return record["unit_seq"], {"error": completed.stderr.decode(errors="replace")[:2000]}
    tu = json.loads(completed.stdout)
    unit = Unit(text)
    collect_tables(unit, tu)
    definitions = {}
    for decl in children(tu):
        if decl.get("kind") != "FunctionDecl":
            continue
        if not any(c.get("kind") == "CompoundStmt" for c in children(decl)):
            continue
        name = decl.get("name")
        definitions[name] = {
            "storage": decl.get("storageClass", "extern"),
            "line": unit.line(decl),
            "end": unit.end_line(decl),
            "constructs": function_constructs(unit, decl),
        }
    return record["unit_seq"], {"definitions": definitions, "command": command[1:]}


# ---------------------------------------------------------------------------
# Stage B/C: join
# ---------------------------------------------------------------------------

def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def construct_matches(construct, fam, sub):
    if fam == "IND":
        return construct["kind"] == "statement"
    if fam == "RET":
        return construct["kind"] == "return"
    if fam == "COND":
        return construct["kind"] == "condition" and construct["clause"] == sub
    return False


def main():
    run = pathlib.Path(sys.argv[1])
    diagnostics = load(sys.argv[2])
    out = pathlib.Path(sys.argv[4])
    jobs = 12
    if "--jobs" in sys.argv:
        jobs = int(sys.argv[sys.argv.index("--jobs") + 1])
    out.mkdir(parents=True, exist_ok=True)
    directory = run / "access-path-units"
    records = [json.loads(line) for line in (directory / "units.jsonl").read_text().splitlines()]
    with multiprocessing.Pool(jobs) as pool:
        extracted = dict(pool.map(extract_unit, [(str(directory), r) for r in records], chunksize=1))
    errors = {seq: value["error"] for seq, value in extracted.items() if "error" in value}

    # Unit matching.
    unmatched = {"units": [], "leaves": [], "constructs": [], "functions": [], "clang_errors": errors}
    unit_of = {}
    for document in diagnostics["units"]:
        names = {f["function"] for f in document["functions"]}
        if not names:
            continue  # a data-only unit: no function, no leaf to bind
        candidates = [seq for seq, value in extracted.items()
                      if "definitions" in value and names <= set(value["definitions"])]
        if len(candidates) != 1:
            # Several analyses can carry the same function set (a unit analysed
            # twice); take the one whose leaf lines all land on constructs.
            unmatched["units"].append({"file": document["file"], "candidates": candidates})
            continue
        unit_of[document["file"]] = candidates[0]

    for seq, count in collections.Counter(unit_of.values()).items():
        if count != 1:
            unmatched["units"].append({"unit_seq": seq, "documents": count})

    rows = []
    for document in diagnostics["units"]:
        seq = unit_of.get(document["file"])
        if seq is None:
            continue
        definitions = extracted[seq]["definitions"]
        for function in document["functions"]:
            name = function["function"]
            definition = definitions.get(name)
            if definition is None:
                unmatched["functions"].append({"file": document["file"], "function": name})
                continue
            groups = collections.defaultdict(list)
            for leaf in function["diagnostics"]:
                if leaf["category"] != "unsupported":
                    continue
                fam, sub = family(leaf["description"])
                if fam is None:
                    continue
                groups[(fam, sub, leaf["line"])].append(leaf)
            for (fam, sub, line), leaves in sorted(groups.items(), key=lambda item: item[0][2]):
                at_line = [c for c in definition["constructs"] if line in c["lines"]]
                constructs = [c for c in at_line if construct_matches(c, fam, sub)]
                if not constructs and fam == "IND":
                    # `return f(a->b)` and a desugared call-bearing condition
                    # lower their call to a statement: the refusal is the
                    # call statement's, at the return's or condition's line.
                    constructs = [c for c in at_line if c["kind"] == "return" and c["paths"]]
                    if not constructs:
                        constructs = [c for c in at_line if c["kind"] == "condition" and c["paths"]]
                    if not constructs:
                        # A return whose only lexical access is unevaluated
                        # (`sizeof(*p)`): bound, and classified no-typed-access.
                        constructs = [c for c in at_line if c["kind"] == "return"]
                if len(constructs) < len(leaves):
                    # Fewer constructs than leaves: some leaf has none of its own.
                    unmatched["leaves"].append({"file": document["file"], "function": name, "line": line,
                                                "family": fam, "sub": sub, "origin": leaves[0].get("origin"),
                                                "count": len(leaves), "constructs": len(constructs)})
                    continue
                rows.append({
                    "library": document.get("library"),
                    "file": document["file"],
                    "unit_seq": seq,
                    "function": name,
                    "status": function["status"],
                    "line": line,
                    "origin": leaves[0].get("origin"),
                    "family": fam,
                    "condition": sub,
                    "leaves": len(leaves),
                    "constructs": constructs,
                })
    # Construct side: every construct bound to a leaf group must be bound to
    # exactly one group (a construct cannot explain two families' leaves).
    seen = collections.Counter()
    for row in rows:
        for construct in row["constructs"]:
            seen[(row["file"], row["function"], id(construct))] += 1
    for (file, function, _), count in seen.items():
        if count != 1:
            unmatched["constructs"].append({"file": file, "function": function, "bound": count})

    (out / "unmatched.json").write_text(json.dumps(unmatched, indent=1) + "\n")
    with gzip.open(out / "leaves.jsonl.gz", "wt") as handle:
        handle.write(json.dumps({"schema": SCHEMA, "kind": "leaf-rows"}) + "\n")
        for row in rows:
            handle.write(json.dumps(row) + "\n")
    (out / "unit-map.json").write_text(json.dumps({"schema": SCHEMA, "unit_of": unit_of,
        "clang_command": extracted[next(iter(extracted))].get("command"),
        "definitions": {file: {name: value["storage"] for name, value in extracted[seq]["definitions"].items()}
                        for file, seq in unit_of.items()}}) + "\n")
    # Every construct of every analysed function with a leaf-free line is
    # kept too (compactly), for the guard and release facts the prediction
    # reads beside the leaf rows.
    with gzip.open(out / "functions.jsonl.gz", "wt") as handle:
        handle.write(json.dumps({"schema": SCHEMA, "kind": "function-constructs"}) + "\n")
        for document in diagnostics["units"]:
            seq = unit_of.get(document["file"])
            if seq is None:
                continue
            for function in document["functions"]:
                definition = extracted[seq]["definitions"].get(function["function"])
                if definition is None:
                    continue
                handle.write(json.dumps({
                    "file": document["file"], "function": function["function"],
                    "constructs": [{k: c[k] for k in ("kind", "clause", "lines", "block", "call_args",
                                                       "assigned", "null_tested", "calls")}
                                   for c in definition["constructs"]],
                }) + "\n")
    bad = sum(len(v) for k, v in unmatched.items() if k != "clang_errors") + len(errors)
    print(f"units matched {len(unit_of)}/{len(diagnostics['units'])}; leaf groups {len(rows)}; "
          f"unmatched: units {len(unmatched['units'])}, functions {len(unmatched['functions'])}, "
          f"leaves {len(unmatched['leaves'])}, constructs {len(unmatched['constructs'])}, clang errors {len(errors)}")
    return 3 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
