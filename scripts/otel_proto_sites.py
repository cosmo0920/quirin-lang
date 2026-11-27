#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Card OTEL-PROTO-AUDIT, Phase A step 3: the protocol-derived obligation
sites of the OpenTelemetry decoders and encoders, read from Clang's JSON AST.

A *site* is a body position that relies on a protobuf-c field fact:

* ``O1`` a singular sub-message pointer (absent on the wire = NULL) is
  dereferenced, or passed to a callee, directly or through a local copy;
* ``O2`` a repeated field's array is subscripted (``n_x`` bounds it);
* ``O3`` a oneof member is read (only the member the ``*_case``
  discriminant selects is valid);
* ``O4`` a ``bytes`` field's ``data`` is dereferenced or passed on
  (``len = 0`` ⇔ ``data = NULL`` after unpack);
* ``O5`` an enum field's value indexes an array (a decoder keeps values the
  enum does not name).

Each site is classified ``guarded`` (the guard is cited by kind and line),
``candidate`` (no guard found and the function is reachable from an unpack of
wire input in the same unit) or ``not-determinable`` (a named reason). The
reader is syntactic and conservative: a guard it cannot recognise leaves the
site unguarded, never guarded; a candidate is a candidate only, never a
defect claim. Writes into a field (stores) are not sites.

``sites --compile-db DB --file SRC ... --pbc-root DIR`` runs Clang
(``-fsyntax-only -Xclang -ast-dump=json``) on each unit, analyses it and
deletes the dump; ``--path-map OLD=NEW`` rewrites the database's paths.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import otel_proto_audit as audit  # noqa: E402

TRANSPARENT = {"ImplicitCastExpr", "ParenExpr", "CStyleCastExpr", "ConstantExpr"}
EXITS = {"ReturnStmt", "GotoStmt", "BreakStmt", "ContinueStmt"}
NORETURN = {"exit", "abort", "_exit", "__assert_fail"}
LOOPS = {"ForStmt", "WhileStmt", "DoStmt"}
# Callees that release or resize a pointer without reading through it.
RELEASES = {"free", "flb_free", "do_free", "realloc", "flb_realloc", "protobuf_c_message_free_unpacked"}


# --------------------------------------------------------------------------
# Field kinds from the generated code


def field_kinds(pbc_root: Path, exclude: list[str]) -> dict:
    """`(C struct, member) -> kind` for every generated message, plus the
    oneof case constants (`name -> field number`)."""
    sources = {k: v for k, v in audit.read_tree(pbc_root, ".pb-c.c").items() if k not in exclude}
    parsed = audit.parse_pbc(sources)
    cname = {}
    for text in sources.values():
        for m in audit.MESSAGE_DESC.finditer(text):
            cname[m.group(4)] = m.group(2)
    members = {}
    for c, full in cname.items():
        for f in parsed["messages"][full]["fields"]:
            if f["label"] == "repeated":
                members[(c, f["name"])] = ("repeated", f["kind"], f["name"])
                members[(c, "n_" + f["name"])] = ("count", f["kind"], f["name"])
            elif f["oneof"]:
                members[(c, f["name"])] = ("oneof", f["kind"], f["oneof"], f["number"])
                members[(c, f["oneof"] + "_case")] = ("case", "enum", f["oneof"])
            elif f["kind"] == "message":
                members[(c, f["name"])] = ("message", "message", f["name"])
            else:
                members[(c, f["name"])] = ("scalar", f["kind"], f["name"])
                if f["label"] == "optional":
                    members[(c, "has_" + f["name"])] = ("has", "bool", f["name"])
    headers = {k: v for k, v in audit.read_tree(pbc_root, ".pb-c.h").items()
               if k.replace(".pb-c.h", ".pb-c.c") not in exclude}
    constants = {}
    for text in headers.values():
        for body, name in re.findall(r"typedef enum \{(.*?)\}\s*(\w+__\w+Case)\s*;", text, re.S):
            for cname_, value in re.findall(r"(\w+)\s*=\s*(\d+)", body):
                constants[cname_] = int(value)
    return {"members": members, "case_constants": constants}


# --------------------------------------------------------------------------
# AST helpers


def resolve_locations(tree: dict) -> None:
    """Clang elides a location's file and line when unchanged since the last
    printed location; restore them in document order (`_file`, `_line`)."""
    state = {"file": None, "line": None}

    def plain(loc: dict) -> None:
        if "file" in loc:
            state["file"] = loc["file"]
        if "line" in loc:
            state["line"] = loc["line"]
        if loc:
            loc["_file"], loc["_line"] = state["file"], state["line"]

    def visit(obj) -> None:
        if isinstance(obj, dict):
            if "spellingLoc" in obj or "expansionLoc" in obj:
                for k in ("spellingLoc", "expansionLoc"):
                    if k in obj:
                        plain(obj[k])
                exp = obj.get("expansionLoc", {})
                obj["_file"], obj["_line"] = exp.get("_file"), exp.get("_line")
                obj["_offset"] = exp.get("offset")
                return
            if "offset" in obj:
                plain(obj)
                obj["_offset"] = obj["offset"]
                return
            for k, v in obj.items():
                if k != "includedFrom":
                    visit(v)
        elif isinstance(obj, list):
            for x in obj:
                visit(x)

    visit(tree)


def kids(n: dict) -> list:
    return [c for c in n.get("inner", []) or [] if isinstance(c, dict)]


def strip(n: dict | None) -> dict | None:
    while n is not None and n.get("kind") in TRANSPARENT and kids(n):
        n = kids(n)[0]
    return n


def line(n: dict) -> int | None:
    return n.get("range", {}).get("begin", {}).get("_line")


def offset(n: dict) -> int:
    o = n.get("range", {}).get("begin", {}).get("_offset")
    return -1 if o is None else o


def record_of(type_text: str) -> str:
    t = type_text.replace("const ", "").replace("struct ", "").replace("volatile ", "")
    return t.rstrip("* ").strip()


def base_record(member_expr: dict) -> tuple[str | None, dict | None, bool]:
    """The protobuf record a MemberExpr selects from, its base expression and
    whether the selection is through `->`; an anonymous oneof union is looked
    through."""
    base = kids(member_expr)[0] if kids(member_expr) else None
    arrow = bool(member_expr.get("isArrow"))
    sb = strip(base)
    if sb is not None and sb.get("kind") == "MemberExpr" and sb.get("name") == "" and kids(sb):
        arrow = bool(sb.get("isArrow"))
        base = kids(sb)[0]
    if base is None:
        return None, None, arrow
    t = base.get("type", {})
    rec = record_of(t.get("desugaredQualType") or t.get("qualType", ""))
    if arrow is False and rec.endswith("*"):
        rec = rec.rstrip("* ")
    return rec, base, arrow


def key(n: dict | None) -> str | None:
    """A canonical spelling of a side-effect-free lvalue/value expression, or
    None (calls, assignments and other effects have no key)."""
    n = strip(n)
    if n is None:
        return None
    k = n.get("kind")
    if k == "DeclRefExpr":
        return n.get("referencedDecl", {}).get("name")
    if k == "MemberExpr":
        if n.get("name") == "":
            return key(kids(n)[0]) if kids(n) else None
        _rec, base, arrow = base_record(n)
        b = key(base)
        return None if b is None else f"{b}{'->' if arrow else '.'}{n['name']}"
    if k == "ArraySubscriptExpr" and len(kids(n)) == 2:
        b, i = key(kids(n)[0]), key(kids(n)[1])
        return None if b is None or i is None else f"{b}[{i}]"
    if k == "IntegerLiteral":
        return n.get("value")
    if k == "UnaryOperator" and n.get("opcode") in ("*", "&", "-") and kids(n):
        s = key(kids(n)[0])
        return None if s is None else f"{n['opcode']}{s}"
    return None


def is_null(n: dict | None) -> bool:
    n = strip(n)
    return n is not None and n.get("kind") == "IntegerLiteral" and n.get("value") == "0"


def roots(k: str) -> str:
    return re.split(r"->|\.|\[", k, maxsplit=1)[0]


# --------------------------------------------------------------------------
# Facts a condition establishes


def nonnull_facts(cond: dict | None, truth: bool) -> set[str]:
    """Keys known non-null when `cond` evaluates to `truth`."""
    c = strip(cond)
    if c is None:
        return set()
    k = c.get("kind")
    if k == "UnaryOperator" and c.get("opcode") == "!" and kids(c):
        return nonnull_facts(kids(c)[0], not truth)
    if k == "BinaryOperator":
        op = c.get("opcode")
        l, r = (kids(c) + [None, None])[:2]
        if op == "&&":
            return nonnull_facts(l, True) | nonnull_facts(r, True) if truth else set()
        if op == "||":
            return nonnull_facts(l, False) | nonnull_facts(r, False) if not truth else set()
        if op in ("==", "!="):
            other = r if is_null(l) else l if is_null(r) else None
            if other is None:
                return set()
            if (op == "!=") == truth:
                kk = key(other)
                return {kk} if kk else set()
            return set()
        return set()
    if truth:
        kk = key(c)
        if kk and c.get("type", {}).get("qualType", "").rstrip().endswith("*"):
            return {kk}
    return set()


def compare_parts(c: dict) -> tuple[str | None, dict | None, dict | None]:
    if c.get("kind") != "BinaryOperator":
        return None, None, None
    l, r = (kids(c) + [None, None])[:2]
    return c.get("opcode"), l, r


def bound_facts(cond: dict | None, truth: bool) -> set[tuple[str, str]]:
    """(index key, count key) pairs with index < count when `cond` is `truth`;
    (\"#nonzero\", count key) for count ≥ 1."""
    c = strip(cond)
    if c is None:
        return set()
    if c.get("kind") == "UnaryOperator" and c.get("opcode") == "!" and kids(c):
        return bound_facts(kids(c)[0], not truth)
    op, l, r = compare_parts(c)
    if op == "&&":
        return bound_facts(l, True) | bound_facts(r, True) if truth else set()
    if op == "||":
        return bound_facts(l, False) | bound_facts(r, False) if not truth else set()
    if op is None:
        kk = key(c)
        return {("#nonzero", kk)} if truth and kk else set()
    lk, rk = key(l), key(r)
    if lk is None or rk is None:
        return set()
    lt = {"<": "lt", ">": "gt", "<=": "le", ">=": "ge", "==": "eq", "!=": "ne"}.get(op)
    if lt is None:
        return set()
    if not truth:
        lt = {"lt": "ge", "ge": "lt", "gt": "le", "le": "gt", "eq": "ne", "ne": "eq"}[lt]
    out = set()
    if lt == "lt":
        out.add((lk, rk))
    if lt == "gt":
        out.add((rk, lk))
    for cnt, other, rel in ((lk, rk, lt), (rk, lk, {"gt": "lt", "lt": "gt", "ge": "le", "le": "ge"}.get(lt, lt))):
        if other == "0" and rel in ("gt", "ne"):
            out.add(("#nonzero", cnt))
        if other == "1" and rel in ("ge", "eq"):
            out.add(("#nonzero", cnt))
        if other == "0" and rel == "gt":
            out.add(("0", cnt))
    if ("#nonzero", lk) in out or ("#nonzero", rk) in out:
        for cnt in (lk, rk):
            if ("#nonzero", cnt) in out:
                out.add(("0", cnt))
    return out


def case_facts(cond: dict | None, truth: bool, constants: dict) -> set[tuple[str, int]]:
    """(case key, field number) pairs known when `cond` is `truth`."""
    c = strip(cond)
    if c is None:
        return set()
    if c.get("kind") == "UnaryOperator" and c.get("opcode") == "!" and kids(c):
        return case_facts(kids(c)[0], not truth, constants)
    op, l, r = compare_parts(c)
    if op == "&&":
        return case_facts(l, True, constants) | case_facts(r, True, constants) if truth else set()
    if op == "||":
        return case_facts(l, False, constants) | case_facts(r, False, constants) if not truth else set()
    if op in ("==", "!=") and (op == "==") == truth:
        for a, b in ((l, r), (r, l)):
            num = constant_number(b, constants)
            if num is not None and key(a):
                return {(key(a), num)}
    return set()


def constant_number(n: dict | None, constants: dict) -> int | None:
    n = strip(n)
    if n is None:
        return None
    if n.get("kind") == "DeclRefExpr" and n.get("referencedDecl", {}).get("kind") == "EnumConstantDecl":
        return constants.get(n["referencedDecl"]["name"])
    if n.get("kind") == "IntegerLiteral":
        return int(n["value"])
    return None


def always_exits(stmt: dict | None) -> bool:
    s = strip(stmt)
    if s is None:
        return False
    k = s.get("kind")
    if k in EXITS:
        return True
    if k == "CompoundStmt":
        return bool(kids(s)) and always_exits(kids(s)[-1])
    if k in ("CaseStmt", "DefaultStmt"):
        return bool(kids(s)) and always_exits(kids(s)[-1])
    if k == "IfStmt" and s.get("hasElse") and len(kids(s)) >= 3:
        return always_exits(kids(s)[1]) and always_exits(kids(s)[2])
    if k == "CallExpr" and kids(s):
        return key(kids(s)[0]) in NORETURN
    return False


# --------------------------------------------------------------------------
# Function analysis


class Function:
    def __init__(self, decl: dict, kinds: dict):
        self.decl = decl
        self.name = decl["name"]
        self.kinds = kinds
        self.body = [c for c in kids(decl) if c.get("kind") == "CompoundStmt"][0]
        self.params = [c.get("name") for c in kids(decl) if c.get("kind") == "ParmVarDecl"]
        self.parent: dict[int, dict] = {}
        self.nodes: list[dict] = []
        stack = [self.body]
        while stack:
            n = stack.pop()
            self.nodes.append(n)
            for c in kids(n):
                self.parent[id(c)] = n
                stack.append(c)
        self.assigns = self._assignments()
        self.calls = [n for n in self.nodes if n.get("kind") == "CallExpr"]

    # ---- structure
    def ancestors(self, n: dict) -> list[dict]:
        out = []
        while id(n) in self.parent:
            n = self.parent[id(n)]
            out.append(n)
        return out

    def use_parent(self, n: dict) -> tuple[dict | None, dict]:
        """The first non-transparent ancestor and the child on the path."""
        child = n
        p = self.parent.get(id(n))
        while p is not None and p.get("kind") in TRANSPARENT:
            child = p
            p = self.parent.get(id(p))
        return p, child

    def _assignments(self) -> list[tuple[str, int, dict | None]]:
        out = []
        for n in self.nodes:
            k = n.get("kind")
            if k == "BinaryOperator" and n.get("opcode") == "=" or k == "CompoundAssignOperator":
                lk = key(kids(n)[0]) if kids(n) else None
                if lk:
                    out.append((lk, offset(n), kids(n)[1] if len(kids(n)) > 1 else None))
            elif k == "UnaryOperator" and n.get("opcode") in ("++", "--") and kids(n):
                lk = key(kids(n)[0])
                if lk:
                    out.append((lk, offset(n), None))
            elif k == "VarDecl":
                init = kids(n)[0] if kids(n) else None
                out.append((n.get("name"), offset(n), init))
            elif k == "UnaryOperator" and n.get("opcode") == "&" and kids(n):
                # an address taken may be written through: treat as an assignment
                lk = key(kids(n)[0])
                if lk:
                    out.append((lk, offset(n), None))
        return out

    def killed(self, k: str, start: int, site: dict, span: tuple[int, int] | None = None) -> bool:
        """An assignment to `k` (or a prefix of it) between `start` and the
        site, or anywhere in a loop around the site that `start` lies
        outside of; assignments inside `span` (a loop's increment) excepted."""
        so = offset(site)
        loops = [a for a in self.ancestors(site) if a.get("kind") in LOOPS]
        for lk, o, _rhs in self.assigns:
            if not (k == lk or k.startswith(lk + "->") or k.startswith(lk + ".") or k.startswith(lk + "[")):
                continue
            if span is not None and span[0] <= o <= span[1]:
                continue
            if start < o < so:
                return True
            for loop in loops:
                lo = offset(loop)
                end = loop.get("range", {}).get("end", {}).get("_offset") or lo
                if lo <= o <= end and not (lo <= start <= end):
                    return True
        return False

    # ---- dominating facts at a site
    def facts_at(self, site: dict, constants: dict) -> list[tuple[str, object, int | None, int]]:
        """(kind, fact, guard line, guard offset) for every fact a guard
        establishes at `site`: ancestors' conditions on the path and earlier
        sibling statements whose branch exits."""
        out = []
        child = site
        grandchild = None
        for a in self.ancestors(site):
            k = a.get("kind")
            ks = kids(a)
            cond = None
            truth = None
            ignore = None
            if k == "IfStmt" and not a.get("hasInit") and not a.get("hasVar") and ks:
                if len(ks) > 1 and child is ks[1]:
                    cond, truth = ks[0], True
                elif len(ks) > 2 and child is ks[2]:
                    cond, truth = ks[0], False
            elif k == "ConditionalOperator" and len(ks) == 3:
                if child is ks[1]:
                    cond, truth = ks[0], True
                elif child is ks[2]:
                    cond, truth = ks[0], False
            elif k == "BinaryOperator" and a.get("opcode") in ("&&", "||") and len(ks) == 2 and child is ks[1]:
                cond, truth = ks[0], a.get("opcode") == "&&"
            elif k == "WhileStmt" and len(ks) == 2 and child is ks[1]:
                cond, truth = ks[0], True
            elif k == "ForStmt" and len(a.get("inner", [])) == 5:
                raw = a["inner"]
                if child is raw[4] or child is raw[3]:
                    cond, truth = raw[2] if raw[2] else None, True
                    # the loop's own increment runs before the condition is
                    # re-tested: it does not kill the condition's facts
                    ignore = raw[3] if raw[3] else None
            if cond is not None:
                self._add(out, cond, truth, constants, site, ignore)
            if k == "CompoundStmt":
                idx = next((i for i, c in enumerate(ks) if c is child), None)
                for prev in ks[: idx or 0]:
                    ps = strip(prev)
                    if ps is not None and ps.get("kind") == "IfStmt" and not ps.get("hasInit") and kids(ps):
                        pk = kids(ps)
                        if len(pk) > 1 and always_exits(pk[1]):
                            self._add(out, pk[0], False, constants, site)
                        elif len(pk) > 2 and always_exits(pk[2]):
                            self._add(out, pk[0], True, constants, site)
            if k == "SwitchStmt" and len(ks) == 2 and grandchild is not None and child is ks[1]:
                out.extend(self._switch_facts(a, grandchild, site, constants))
            grandchild = child
            child = a
        return out

    def _add(self, out, cond, truth, constants, site, ignore=None):
        gl, go = line(cond), offset(cond)
        span = None
        if ignore is not None:
            span = (offset(ignore), ignore.get("range", {}).get("end", {}).get("_offset") or offset(ignore))
        for f in nonnull_facts(cond, truth):
            if not self.killed(f, go, site, span):
                out.append(("nonnull", f, gl, go))
        for f in bound_facts(cond, truth):
            if not any(self.killed(x, go, site, span) for x in f if not x.startswith("#") and not x.isdigit()):
                out.append(("bound", f, gl, go))
        for f in case_facts(cond, truth, constants):
            if not self.killed(f[0], go, site, span):
                out.append(("case", f, gl, go))

    def _switch_facts(self, sw: dict, child: dict, site: dict, constants: dict):
        subject = key(kids(sw)[0])
        body = strip(kids(sw)[1])
        if subject is None or body is None or body.get("kind") != "CompoundStmt":
            return []
        stmts = kids(body)
        idx = next((i for i, c in enumerate(stmts) if c is child), None)
        if idx is None:
            return []
        # Every label from which control can reach the site's statement:
        # walk back until a statement that always exits (no fall-through).
        labels = []
        i = idx
        while i >= 0:
            t = stmts[i]
            while t is not None and t.get("kind") in ("CaseStmt", "DefaultStmt"):
                if t.get("kind") == "DefaultStmt":
                    labels.append(None)
                    t = kids(t)[0] if kids(t) else None
                else:
                    labels.append(constant_number(kids(t)[0], constants) if kids(t) else None)
                    t = kids(t)[1] if len(kids(t)) > 1 else None
            i -= 1
            if i >= 0 and always_exits(stmts[i]):
                break
        if not labels or None in labels:
            return []
        if len(set(labels)) == 1:
            return [("case", (subject, labels[0]), line(sw), offset(sw))]
        return [("case-set", (subject, frozenset(labels)), line(sw), offset(sw))]


# --------------------------------------------------------------------------
# Provenance and sites


def classify_member(fn: Function, n: dict) -> tuple | None:
    if n.get("kind") != "MemberExpr" or n.get("name") == "":
        return None
    rec, base, _arrow = base_record(n)
    if rec is None:
        return None
    if rec == "ProtobufCBinaryData":
        return ("bytes-" + n["name"],)
    return fn.kinds["members"].get((rec, n["name"]))


def provenance(fn: Function, expr: dict | None, depth: int = 0) -> tuple | None:
    """(origin kind, origin key, kind tuple) when `expr` reads a protobuf
    pointer field, directly or through a local whose every non-null
    definition does."""
    e = strip(expr)
    if e is None or depth > 3:
        return None
    if e.get("kind") == "CallExpr" and kids(e) and ((key(kids(e)[0]) or "").endswith("__unpack")
                                                    or key(kids(e)[0]) in getattr(fn, "unpackers", ())):
        return ("unpack", key(kids(e)[0]), ("unpack",))
    if e.get("kind") == "MemberExpr":
        kt = classify_member(fn, e)
        if kt and kt[0] in ("message", "repeated") or kt and kt[0] == "oneof" and kt[1] == "message":
            return (kt[0], key(e), kt)
        if kt and kt[0] == "bytes-data":
            return ("bytes", key(e), kt)
        return None
    if e.get("kind") == "ArraySubscriptExpr" and kids(e):
        p = provenance(fn, kids(e)[0], depth + 1)
        if p and p[0] == "repeated":
            return ("element", key(e), p[2])
        return None
    if e.get("kind") == "DeclRefExpr":
        name = key(e)
        defs = [rhs for lk, _o, rhs in fn.assigns if lk == name]
        origins = [provenance(fn, rhs, depth + 1) for rhs in defs if rhs is not None and not is_null(rhs)]
        if origins and all(origins) and len({o[0] for o in origins}) == 1:
            o = origins[0]
            return (o[0], name, o[2], tuple(sorted({x[1] or "" for x in origins})))
    return None


def deref_uses(fn: Function, n: dict) -> str | None:
    """How the value of `n` is used: deref, subscript, call-arg, test, store,
    alias or other."""
    p, child = fn.use_parent(n)
    if p is None:
        return "other"
    k = p.get("kind")
    if k == "MemberExpr" and p.get("isArrow") and kids(p) and kids(p)[0] is child:
        return "deref"
    if k == "MemberExpr" and p.get("name") == "" and kids(p) and kids(p)[0] is child:
        return "deref" if p.get("isArrow") else "other"
    if k == "UnaryOperator" and p.get("opcode") == "*":
        return "deref"
    if k == "ArraySubscriptExpr" and kids(p) and kids(p)[0] is child:
        return "subscript"
    if k == "CallExpr" and kids(p) and kids(p)[0] is not child:
        return "call-arg"
    if k == "BinaryOperator" and p.get("opcode") in ("==", "!=", "&&", "||"):
        return "test"
    if k == "UnaryOperator" and p.get("opcode") == "!":
        return "test"
    if k in ("IfStmt", "WhileStmt", "ConditionalOperator", "ForStmt") and kids(p) and kids(p)[0] is child:
        return "test"
    if k == "BinaryOperator" and p.get("opcode") == "=" and kids(p) and kids(p)[0] is child:
        return "store"
    if k == "BinaryOperator" and p.get("opcode") == "=" or k == "VarDecl":
        return "alias"
    if k == "ReturnStmt":
        return "return"
    if k == "MemberExpr":
        return "member-read"
    return "other"


class Unit:
    def __init__(self, tree: dict, source: str, kinds: dict):
        self.kinds = kinds
        self.source = source
        self.functions: dict[str, Function] = {}
        for d in tree.get("inner", []):
            if d.get("kind") != "FunctionDecl" or not any(c.get("kind") == "CompoundStmt" for c in kids(d)):
                continue
            if d.get("loc", {}).get("_file") != source:
                continue
            self.functions[d["name"]] = Function(d, kinds)
        self.wire = self._wire_reachable()
        self._param_cache: dict = {}

    def callees(self, fn: Function) -> set[str]:
        out = set()
        for c in fn.calls:
            k = key(kids(c)[0]) if kids(c) else None
            if k:
                out.add(k)
        return out

    def _wire_reachable(self) -> set[str]:
        """Functions of the unit reachable from an unpack of wire bytes: an
        unpacker (a function calling a generated `*__unpack`, or returning
        such a function's result), its callers, and everything they call."""
        unpackers = {name for name, fn in self.functions.items()
                     if any(c.endswith("__unpack") for c in self.callees(fn))}
        changed = True
        while changed:
            changed = False
            for name, fn in self.functions.items():
                if name in unpackers:
                    continue
                for n in fn.nodes:
                    if n.get("kind") == "ReturnStmt" and kids(n):
                        r = strip(kids(n)[0])
                        if r is not None and r.get("kind") == "CallExpr" and kids(r) \
                                and key(kids(r)[0]) in unpackers:
                            unpackers.add(name)
                            changed = True
                            break
        for fn in self.functions.values():
            fn.unpackers = unpackers
        roots_ = set(unpackers) | {name for name, fn in self.functions.items()
                                   if self.callees(fn) & unpackers}
        seen, stack = set(), list(roots_)
        while stack:
            name = stack.pop()
            if name in seen:
                continue
            seen.add(name)
            stack.extend(c for c in self.callees(self.functions[name]) if c in self.functions)
        return seen

    # ---- does callee `name` guard its parameter `index` before every deref?
    def param_guarded(self, name: str, index: int, need: str, depth: int = 0) -> tuple[str, str]:
        ck = (name, index, need)
        if ck in self._param_cache:
            return self._param_cache[ck]
        fn = self.functions.get(name)
        if fn is None:
            return ("not-determinable", "callee-outside-unit")
        if index >= len(fn.params) or depth > 3:
            return ("not-determinable", "callee-variadic-or-deep")
        self._param_cache[ck] = ("not-determinable", "callee-recursive")
        pname = fn.params[index]
        verdicts = []
        for n in fn.nodes:
            if n.get("kind") == "DeclRefExpr" and key(n) == pname:
                use = deref_uses(fn, n)
                if use in ("deref", "subscript"):
                    facts = fn.facts_at(n, self.kinds["case_constants"])
                    ok = any(f[0] == "nonnull" and f[1] == pname for f in facts)
                    verdicts.append(("guarded", f"callee {name} tests `{pname}`") if ok
                                    else ("unguarded", f"callee {name} dereferences `{pname}`"))
                elif use == "call-arg":
                    p, child = fn.use_parent(n)
                    facts = fn.facts_at(n, self.kinds["case_constants"])
                    if any(f[0] == "nonnull" and f[1] == pname for f in facts):
                        verdicts.append(("guarded", f"callee {name} tests `{pname}`"))
                        continue
                    callee = key(kids(p)[0])
                    pos = [i for i, c in enumerate(kids(p)[1:]) if c is child]
                    if callee in self.functions and pos:
                        verdicts.append(self.param_guarded(callee, pos[0], need, depth + 1))
                    else:
                        verdicts.append(("not-determinable", "callee-outside-unit"))
                elif use == "alias":
                    verdicts.append(("not-determinable", "callee-copies-parameter"))
        if not verdicts:
            result = ("guarded", f"callee {name} never dereferences `{pname}`")
        elif any(v[0] == "unguarded" for v in verdicts):
            result = next(v for v in verdicts if v[0] == "unguarded")
        elif all(v[0] == "guarded" for v in verdicts):
            result = verdicts[0]
        else:
            result = next(v for v in verdicts if v[0] == "not-determinable")
        self._param_cache[ck] = result
        return result

    def param_bounded(self, name: str, arr_index: int, cnt_index: int, depth: int = 0) -> tuple[str, str]:
        fn = self.functions.get(name)
        if fn is None:
            return ("not-determinable", "callee-outside-unit")
        if max(arr_index, cnt_index) >= len(fn.params) or depth > 3:
            return ("not-determinable", "callee-variadic-or-deep")
        arr, cnt = fn.params[arr_index], fn.params[cnt_index]
        found = False
        for n in fn.nodes:
            if n.get("kind") == "ArraySubscriptExpr" and kids(n) and key(kids(n)[0]) == arr:
                found = True
                idx = key(kids(n)[1])
                facts = fn.facts_at(n, self.kinds["case_constants"])
                if not any(f[0] == "bound" and (f[1] == (idx, cnt) or idx == "0" and f[1] == ("#nonzero", cnt))
                           for f in facts):
                    return ("unguarded", f"callee {name} subscripts `{arr}` without `< {cnt}`")
            elif n.get("kind") == "DeclRefExpr" and key(n) == arr and deref_uses(fn, n) not in ("subscript", "test"):
                p, child = fn.use_parent(n)
                if deref_uses(fn, n) == "call-arg":
                    args = kids(p)[1:]
                    callee = key(kids(p)[0])
                    a_pos = [i for i, c in enumerate(args) if c is child]
                    c_pos = [i for i, c in enumerate(args) if key(c) == cnt]
                    if callee in self.functions and a_pos and c_pos:
                        status = self.param_bounded(callee, a_pos[0], c_pos[0], depth + 1)
                        if status[0] != "guarded":
                            return status
                        found = True
                        continue
                return ("not-determinable", f"callee {name} passes `{arr}` on")
        return ("guarded", f"callee {name} bounds `{arr}` by `{cnt}`") if found else \
            ("guarded", f"callee {name} never subscripts `{arr}`")

    # ---- sites
    def sites(self) -> list[dict]:
        out = []
        consts = self.kinds["case_constants"]
        for fn in self.functions.values():
            wire = fn.name in self.wire
            for n in fn.nodes:
                k = n.get("kind")
                kt = classify_member(fn, n) if k == "MemberExpr" else None
                if kt is not None:
                    use = deref_uses(fn, n)
                    if kt[0] == "oneof" and use != "store":
                        out.append(self._oneof_site(fn, n, kt, use, wire, consts))
                    if kt[0] == "scalar" and kt[1] == "enum":
                        p, child = fn.use_parent(n)
                        if p is not None and p.get("kind") == "ArraySubscriptExpr" and kids(p)[1] is child:
                            out.append(self._site(fn, n, "O5", "enum-index", key(n), wire,
                                                  "candidate" if wire else "not-determinable",
                                                  None if wire else "encoder-constructed-value",
                                                  "an enum value used as an index"))
                        elif wire and p is not None and p.get("kind") == "CallExpr" and kids(p)[0] is not child:
                            out.append(self._enum_arg(fn, n, p, child))
                if k in ("MemberExpr", "UnaryOperator", "ArraySubscriptExpr", "CallExpr"):
                    out.extend(self._pointer_sites(fn, n, wire, consts))
        return [s for s in out if s is not None]

    def _enum_arg(self, fn, n, call, child):
        """A decoded enum value (possibly one the enum does not name) passed
        to a callee: guarded only when the unit's callee never indexes by
        that parameter."""
        callee = key(kids(call)[0])
        pos = [i for i, c in enumerate(kids(call)[1:]) if c is child]
        target = self.functions.get(callee)
        if target is None or not pos or pos[0] >= len(target.params):
            return self._site(fn, n, "O5", "call-arg", key(n), True, "not-determinable",
                              "enum-to-callee-outside-unit", f"to `{callee}`")
        pname = target.params[pos[0]]
        for m in target.nodes:
            if m.get("kind") == "ArraySubscriptExpr" and len(kids(m)) > 1 and key(kids(m)[1]) == pname:
                return self._site(fn, n, "O5", "call-arg", key(n), True, "candidate", None,
                                  f"callee {callee} indexes by `{pname}`")
        return self._site(fn, n, "O5", "call-arg", key(n), True, "guarded", None,
                          f"callee {callee} never indexes by `{pname}`")

    def _site(self, fn, n, ob, form, subject, wire, cls, reason, guard):
        if reason is not None and " " in reason:
            # a callee's sentence becomes its stable code; the sentence stays
            # as the site's detail
            guard = guard or reason
            reason = ("callee-passes-parameter-on" if " passes " in reason
                      else "callee-copies-parameter" if "copies" in reason
                      else "callee-reason-unclassified")
        return {"file": self.source, "function": fn.name, "line": line(n), "obligation": ob,
                "form": form, "subject": subject, "wire_reachable": wire,
                "class": cls, "reason": reason, "detail": guard}

    def _oneof_site(self, fn, n, kt, use, wire, consts):
        rec, base, arrow = base_record(n)
        case_key = f"{key(base)}{'->' if arrow else '.'}{kt[2]}_case" if key(base) else None
        facts = fn.facts_at(n, consts)
        for f in facts:
            if f[0] == "case" and f[1] == (case_key, kt[3]):
                return self._site(fn, n, "O3", use, key(n), wire, "guarded", None,
                                  f"case `{case_key}` = {kt[3]} (line {f[2]})")
        for f in facts:
            if f[0] == "case-set" and f[1][0] == case_key:
                return self._site(fn, n, "O3", use, key(n), wire, "candidate" if wire else "not-determinable",
                                  None if wire else "encoder-constructed-value",
                                  f"reached from several case labels of `{case_key}`: {sorted(f[1][1])}")
        return self._site(fn, n, "O3", use, key(n), wire, "candidate" if wire else "not-determinable",
                          None if wire else "encoder-constructed-value", f"no test of `{case_key}`")

    def _pointer_sites(self, fn, n, wire, consts):
        """O1/O2/O4 sites at a dereference, a subscript or a call argument."""
        k = n.get("kind")
        out = []
        if k == "MemberExpr" and n.get("isArrow") and kids(n):
            subj = kids(n)[0]
            sx = strip(subj)
            if sx is not None and sx.get("kind") == "MemberExpr" and sx.get("name") == "":
                return out
            out.append(self._deref(fn, n, subj, "deref", wire, consts))
        elif k == "MemberExpr" and n.get("name") == "" and n.get("isArrow") and kids(n):
            out.append(self._deref(fn, n, kids(n)[0], "deref", wire, consts))
        elif k == "UnaryOperator" and n.get("opcode") == "*" and kids(n):
            out.append(self._deref(fn, n, kids(n)[0], "deref", wire, consts))
        elif k == "ArraySubscriptExpr" and kids(n):
            out.append(self._subscript(fn, n, wire, consts))
        elif k == "CallExpr" and kids(n):
            callee = key(kids(n)[0])
            args = kids(n)[1:]
            for i, a in enumerate(args):
                out.append(self._call_arg(fn, n, callee, i, a, args, wire, consts))
        return [s for s in out if s is not None]

    def _deref(self, fn, n, subj, form, wire, consts):
        p = provenance(fn, subj)
        if p is None or p[0] not in ("message", "oneof", "bytes", "unpack"):
            return None
        sk = key(subj)
        facts = fn.facts_at(n, consts)
        ob = {"message": "O1", "oneof": "O1", "bytes": "O4", "unpack": "O0"}[p[0]]
        for f in facts:
            if f[0] == "nonnull" and f[1] in (sk, p[1]):
                return self._site(fn, n, ob, form, sk, wire, "guarded", None, f"`{f[1]}` != NULL (line {f[2]})")
        if p[0] == "oneof":
            return self._oneof_pointer(fn, n, subj, sk, p, form, wire, facts)
        if p[0] == "bytes":
            base = p[1][: -len(".data")] if p[1] and p[1].endswith(".data") else None
            for f in facts:
                if f[0] == "bound" and base and f[1] in (("#nonzero", base + ".len"), ("0", base + ".len")):
                    return self._site(fn, n, ob, form, sk, wire, "guarded", None,
                                      f"`{base}.len` > 0 (line {f[2]})")
        if len(p) > 3 and len(p[3]) > 1:
            return self._site(fn, n, ob, form, sk, wire, "not-determinable", "alias-several-origins", None)
        return self._site(fn, n, ob, form, sk, wire, "candidate" if wire else "not-determinable",
                          None if wire else "encoder-constructed-value", None)

    def _oneof_pointer(self, fn, n, subj, sk, p, form, wire, facts):
        # a oneof message member is non-null when its case is selected
        kt = p[2]
        origin = strip(subj)
        if origin is not None and origin.get("kind") == "MemberExpr":
            rec, base, arrow = base_record(origin)
            case_key = f"{key(base)}{'->' if arrow else '.'}{kt[2]}_case"
            for f in facts:
                if f[0] == "case" and f[1] == (case_key, kt[3]):
                    return self._site(fn, n, "O1", form, sk, wire, "guarded", None,
                                      f"case `{case_key}` = {kt[3]} selects a non-null member (line {f[2]})")
        return self._site(fn, n, "O1", form, sk, wire, "candidate" if wire else "not-determinable",
                          None if wire else "encoder-constructed-value", "oneof message member without its case")

    def _subscript(self, fn, n, wire, consts):
        base, idx = kids(n)[0], kids(n)[1] if len(kids(n)) > 1 else None
        p = provenance(fn, base)
        if p is None or p[0] not in ("repeated", "bytes"):
            return None
        if fn.use_parent(n)[0] is not None and deref_uses(fn, n) == "store" and p[0] == "repeated" and not wire:
            pass
        ik = key(idx)
        facts = fn.facts_at(n, consts)
        if p[0] == "bytes":
            base_k = p[1][: -len(".data")] if p[1] and p[1].endswith(".data") else None
            for f in facts:
                if f[0] == "bound" and base_k and f[1] == (ik, base_k + ".len"):
                    return self._site(fn, n, "O4", "subscript", key(base), wire, "guarded", None,
                                      f"`{ik}` < `{base_k}.len` (line {f[2]})")
            return self._site(fn, n, "O4", "subscript", key(base), wire, "candidate" if wire else "not-determinable",
                              None if wire else "encoder-constructed-value", None)
        arr_key = p[1]
        origin_keys = {arr_key} | set(p[3] if len(p) > 3 else ())
        counts = set()
        for ok in origin_keys:
            if ok and (ok.endswith("->" + p[2][2]) or ok.endswith("." + p[2][2])):
                counts.add(ok[: -len(p[2][2])] + "n_" + p[2][2])
        # a local copy of a count, or the local a count was set from
        for lk, _o, rhs in fn.assigns:
            if rhs is not None and key(rhs) in counts:
                counts.add(lk)
        for lk, _o, rhs in fn.assigns:
            if lk in counts and rhs is not None and key(rhs) and strip(rhs).get("kind") == "DeclRefExpr":
                counts.add(key(rhs))
        for f in facts:
            if f[0] == "bound" and (f[1][0] == ik and f[1][1] in counts
                                    or ik == "0" and f[1][0] in ("#nonzero", "0") and f[1][1] in counts):
                return self._site(fn, n, "O2", "subscript", key(base), wire, "guarded", None,
                                  f"`{ik}` < `{f[1][1]}` (line {f[2]})")
        if not counts:
            return self._site(fn, n, "O2", "subscript", key(base), wire, "not-determinable",
                              "array-count-not-in-scope", None)
        return self._site(fn, n, "O2", "subscript", key(base), wire, "candidate" if wire else "not-determinable",
                          None if wire else "encoder-constructed-value", None)

    def _call_arg(self, fn, call, callee, i, arg, args, wire, consts):
        p = provenance(fn, arg)
        if p is None or p[0] == "element":
            return None
        ak = key(arg)
        facts = fn.facts_at(call, consts)
        if callee in RELEASES or (callee or "").endswith("__free_unpacked"):
            return None  # releasing or resizing a field's storage reads no element
        if p[0] == "repeated":
            counts = {p[1][: -len(p[2][2])] + "n_" + p[2][2]} if p[1] else set()
            cnt_pos = [j for j, a in enumerate(args) if key(a) in counts]
            if not cnt_pos:
                for f in facts:
                    if f[0] == "bound" and f[1][0] in ("#nonzero", "0") and f[1][1] in counts:
                        pass
                return self._site(fn, call, "O2", "call-arg", ak, wire, "not-determinable",
                                  "array-passed-without-its-count", f"to `{callee}`")
            status, why = self.param_bounded(callee, i, cnt_pos[0])
            cls = {"guarded": "guarded", "unguarded": "candidate" if wire else "not-determinable"}.get(status, status)
            reason = None if cls in ("guarded", "candidate") else (why if status != "unguarded" else "encoder-constructed-value")
            return self._site(fn, call, "O2", "call-arg", ak, wire, cls, reason, why)
        ob = {"message": "O1", "oneof": "O1", "bytes": "O4", "unpack": "O0"}[p[0]]
        for f in facts:
            if f[0] == "nonnull" and f[1] in (ak, p[1]):
                return self._site(fn, call, ob, "call-arg", ak, wire, "guarded", None,
                                  f"`{f[1]}` != NULL (line {f[2]})")
        if p[0] == "oneof":
            origin = strip(arg)
            if origin is not None and origin.get("kind") == "MemberExpr":
                rec, base, arrow = base_record(origin)
                case_key = f"{key(base)}{'->' if arrow else '.'}{p[2][2]}_case"
                for f in facts:
                    if f[0] == "case" and f[1] == (case_key, p[2][3]):
                        return self._site(fn, call, ob, "call-arg", ak, wire, "guarded", None,
                                          f"case `{case_key}` = {p[2][3]} selects a non-null member (line {f[2]})")
        if p[0] == "bytes":
            base = p[1][: -len(".data")] if p[1] and p[1].endswith(".data") else None
            for f in facts:
                if f[0] == "bound" and base and f[1] in (("#nonzero", base + ".len"), ("0", base + ".len")):
                    return self._site(fn, call, ob, "call-arg", ak, wire, "guarded", None,
                                      f"`{base}.len` > 0 (line {f[2]})")
            if any(key(a) == (base or "") + ".len" for a in args):
                return self._site(fn, call, ob, "call-arg", ak, wire, "not-determinable",
                                  "bytes-passed-with-length-to-callee" if callee not in self.functions
                                  else "bytes-passed-with-length-to-unit-callee", f"to `{callee}`")
            return self._site(fn, call, ob, "call-arg", ak, wire, "not-determinable",
                              "bytes-passed-without-length", f"to `{callee}`")
        status, why = self.param_guarded(callee, i, "nonnull") if callee else ("not-determinable", "indirect-call")
        if status == "guarded":
            return self._site(fn, call, ob, "call-arg", ak, wire, "guarded", None, why)
        if status == "unguarded":
            return self._site(fn, call, ob, "call-arg", ak, wire, "candidate" if wire else "not-determinable",
                              None if wire else "encoder-constructed-value", why)
        return self._site(fn, call, ob, "call-arg", ak, wire, "not-determinable", why, f"to `{callee}`")


# --------------------------------------------------------------------------
# Driver


def ast_of(entry: dict, source: str, path_map: list[tuple[str, str]], clang: str) -> dict:
    def mapped(s: str) -> str:
        for old, new in path_map:
            s = s.replace(old, new)
        return s

    args = shlex.split(mapped(entry["command"])) if "command" in entry else [mapped(a) for a in entry["arguments"]]
    out, skip = [], False
    for a in args[1:]:
        if skip:
            skip = False
            continue
        if a in ("-o", "-MF", "-MT", "-MQ"):
            skip = True
            continue
        if a in ("-c", "-MD", "-MMD") or a == source or a == mapped(entry["file"]):
            continue
        out.append(a)
    with tempfile.TemporaryFile("w+") as fh:
        r = subprocess.run([clang] + out + ["-fsyntax-only", "-w", "-Xclang", "-ast-dump=json", source],
                           cwd=mapped(entry["directory"]), stdout=fh, stderr=subprocess.PIPE, text=True)
        if r.returncode != 0:
            raise audit.Refused(f"{source}: clang exit {r.returncode}: {r.stderr[-400:]}")
        fh.seek(0)
        return json.load(fh)


def analyse(tree: dict, source: str, kinds: dict) -> dict:
    resolve_locations(tree)
    unit = Unit(tree, source, kinds)
    sites = unit.sites()
    return {
        "functions": {
            name: {
                "line": fn.decl.get("loc", {}).get("_line"),
                "static": fn.decl.get("storageClass") == "static",
                "params": [[c.get("name"), c.get("type", {}).get("qualType")]
                           for c in kids(fn.decl) if c.get("kind") == "ParmVarDecl"],
            }
            for name, fn in sorted(unit.functions.items())
        },
        "wire_reachable": sorted(unit.wire),
        "sites": sites,
    }


def summarise(results: dict) -> dict:
    out = {}
    for src, r in results.items():
        c = Counter((s["obligation"], s["class"]) for s in r["sites"])
        out[src] = {f"{o}:{k}": v for (o, k), v in sorted(c.items())}
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--compile-db", type=Path, required=True)
    ap.add_argument("--file", action="append", required=True, help="a unit's source path (after --path-map)")
    ap.add_argument("--pbc-root", type=Path, required=True)
    ap.add_argument("--exclude", action="append", default=[])
    ap.add_argument("--path-map", action="append", default=[], help="OLD=NEW rewrite of database paths")
    ap.add_argument("--clang", default="/usr/lib/llvm-21/bin/clang")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args(argv)
    path_map = [tuple(p.split("=", 1)) for p in args.path_map]
    try:
        kinds = field_kinds(args.pbc_root, args.exclude)
        db = json.loads(args.compile_db.read_text())
        results = {}
        for src in args.file:
            entries = [e for e in db if _mapped(e["file"], path_map) == src]
            if not entries:
                raise audit.Refused(f"{src}: not in the compile database")
            tree = ast_of(entries[0], src, path_map, args.clang)
            results[src] = analyse(tree, src, kinds)
            del tree
    except audit.Refused as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    doc = {"summary": summarise(results), "units": results}
    text = json.dumps(doc, indent=1, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(text)
    else:
        sys.stdout.write(text)
    return 0


def _mapped(s: str, path_map) -> str:
    for old, new in path_map:
        s = s.replace(old, new)
    return s


if __name__ == "__main__":
    sys.exit(main())
