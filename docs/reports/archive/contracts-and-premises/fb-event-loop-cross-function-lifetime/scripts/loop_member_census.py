#!/usr/bin/env python3
"""FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: run-wide candidate sets of the
Fluent Bit event-loop members (read-only, lexical census).

Reads the prepared Fluent Bit exports only (never the user's checkout, never
git): the corpus inputs' 66910c10 source, the system route's 046accf6 export and the tail-pause slice's 90978ac6
export, each over its whole `src/`, `plugins/` and `include/` tree. The
function locator, blanking and actual splitting are the previous card's
(`../../fb-event-loop-retention/scripts/loop_census.py`, imported).

A **loop member** is a record field whose declared type is
`struct mk_event_loop *` (or a `void *` field named `event_loop` / `evl`),
keyed like rule 12n's callback members by the innermost named record that
declares it (`TAG.FIELD`; `coll->instance->config->evl` is `flb_config.evl`),
plus the engine's thread-local slot (`flb_engine_evl_set` / `_get`), keyed
`tls.flb_engine_evl`.

For every member the census lists the stores into it, and for every store how
its right-hand side resolves:

* `alloc:FILE:LINE`  a `mk_event_loop_create` call (one loop-allocation site);
* `member:TAG.FIELD` a read of another loop member (or the TLS slot);
* `formal:FN#I`      a formal of the storing function, expanded over the
                     actuals of every direct call of FN in the tree (FN's
                     address taken anywhere, or no call read, opens it);
* `ret:FN`           a call result, expanded over FN's `return` operands;
* `null`             a `NULL` store (a reset, not a loop);
* `open:WHY`         anything else (an untyped root, a local with an
                     unreadable write, an unknown call ...).

The candidate set of a member is the least fixed point of these edges over the
allocation sites; it is **closed** only when no `open:` origin reaches it. The
census then attributes every loop operation (`mk_event_add`,
`mk_event_timeout_create`, `mk_event_channel_create`; `mk_event_del`,
`mk_event_timeout_destroy`, `mk_event_timeout_disable`,
`mk_event_channel_destroy`; `mk_event_wait`; `mk_event_loop_destroy`) to the
members and allocation sites its loop actual can name, and the held object of
each registration or removal to `TAG.FIELD` of the record whose interior it is.

Lexical: types are read from declarations in the tree, conditional groups are
read over their first arm (the previous card's convention), macros are not
expanded. A match is a site, not a finding. Output:
artifacts/loop-member-census.json.
"""
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(REPORT, "..", "fb-event-loop-retention", "scripts"))
import loop_census as lc  # noqa: E402  (blank, preprocessor_blank, split_top, call_actuals)

TREES = {
    "corpus-66910c10": "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-source",
    "system-046accf6":("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/"
                        "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
    "slice-90978ac6": ("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/"
                       "fluent-bit-clang-source"),
}
ROUTE_FILES = {
    "system-046accf6": lc.SYSTEM_FILES,
    "slice-90978ac6": lc.SLICE_FILES,
}
REGISTER = {"mk_event_add": (0, 4), "mk_event_timeout_create": (0, 3),
            "mk_event_channel_create": (0, 3)}
REMOVE = {"mk_event_del": (0, 1), "mk_event_timeout_destroy": (0, 1),
          "mk_event_timeout_disable": (0, 1), "mk_event_channel_destroy": (0, 3)}
WAIT = {"mk_event_wait": (0, None), "mk_event_wait_2": (0, None),
        "flb_event_priority_live_foreach": (2, None), "mk_event_foreach": (1, None)}
DESTROY = {"mk_event_loop_destroy": (0, None)}
OPERATIONS = {**{k: ("register",) + v for k, v in REGISTER.items()},
              **{k: ("remove",) + v for k, v in REMOVE.items()},
              **{k: ("wait",) + v for k, v in WAIT.items()},
              **{k: ("destroy",) + v for k, v in DESTROY.items()}}
LOOP_FIELD_NAMES = {"evl", "event_loop", "ch_evl"}
TLS_KEY = "tls.flb_engine_evl"


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def tree_files(root, exts=(".c", ".h")):
    out = []
    for sub in ("src", "plugins", "include"):
        for d, _, names in os.walk(os.path.join(root, sub)):
            for name in names:
                if name.endswith(exts):
                    out.append(os.path.relpath(os.path.join(d, name), root))
    return sorted(out)


def match_brace(text, i):
    depth = 0
    for j in range(i, len(text)):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                return j
    return len(text) - 1


# ---------------------------------------------------------------- types

DECL = re.compile(r"^(?:(?:const|volatile|static|extern|register|unsigned|signed)\s+)*"
                  r"(?:(struct|union|enum)\s+(\w+)|(\w+))\s*([\s\*]*?)\s*(\w+)\s*(\[[^\]]*\])*\s*(?::\s*\d+)?$")


def parse_decl(text):
    """`struct T *name` -> (base, pointer_depth, name) or None. `base` is the
    struct/union tag or the type word."""
    t = " ".join(text.split())
    t = re.sub(r"\b(?:const|volatile)\b\s*", "", t).strip()
    fp = re.match(r"^.*\(\s*\*\s*(\w+)\s*\)\s*\(.*\)$", t)
    if fp:
        return ("fnptr", 1, fp.group(1))
    m = DECL.match(t)
    if not m:
        return None
    base = m.group(2) or m.group(3)
    return (base, m.group(4).count("*"), m.group(5))


class Types:
    """Record fields and typedefs read from every file of the tree."""

    def __init__(self):
        self.fields = {}    # tag -> {field: (base, depth)}
        self.typedefs = {}  # name -> tag
        self.returns = {}   # function -> (base, depth)

    def tag(self, base):
        seen = set()
        while base in self.typedefs and base not in seen:
            seen.add(base)
            base = self.typedefs[base]
        return base

    def read(self, clean):
        for m in re.finditer(r"\btypedef\s+(?:struct|union)\s+(\w+)\s+(\w+)\s*;", clean):
            self.typedefs[m.group(2)] = m.group(1)
        for m in re.finditer(r"\b(?:(typedef)\s+)?(struct|union)\s+(\w+)?\s*\{", clean):
            start = m.end() - 1
            end = match_brace(clean, start)
            body = clean[start + 1:end]
            tail = re.match(r"\s*(\w+)\s*;", clean[end + 1:end + 80])
            tag = m.group(3)
            if m.group(1) and tail:
                if tag:
                    self.typedefs[tail.group(1)] = tag
                else:
                    tag = tail.group(1)
            if not tag:
                continue
            fields = self.fields.setdefault(tag, {})
            self._fields(body, fields)

    def _fields(self, body, fields):
        # flatten nested anonymous records into the enclosing one
        i, depth, start = 0, 0, 0
        while i < len(body):
            c = body[i]
            if c == "{":
                end = match_brace(body, i)
                inner = body[i + 1:end]
                after = re.match(r"\s*(\w*)\s*;", body[end + 1:end + 80])
                if after and not after.group(1):
                    self._fields(inner, fields)
                start = end + 1 + (after.end() if after else 0)
                i = start
                continue
            if c == ";" and depth == 0:
                decl = body[start:i]
                for k, part in enumerate(lc.split_top(decl)):
                    if k == 0:
                        parsed = parse_decl(part)
                        first = parsed
                    else:
                        # `struct T *a, *b;`: the later declarators share the base
                        mm = re.match(r"^\s*(\**)\s*(\w+)", part)
                        parsed = (first[0], mm.group(1).count("*"), mm.group(2)) if (first and mm) else None
                    if parsed:
                        fields[parsed[2]] = (parsed[0], parsed[1])
                start = i + 1
            i += 1


def is_loop_type(base, depth, name=None):
    if base == "mk_event_loop" and depth == 1:
        return True
    return base == "void" and depth == 1 and name in LOOP_FIELD_NAMES


# ---------------------------------------------------------------- functions

FUNC_HEAD = re.compile(r"((?:[\w\*]+\s+)*?[\w\*]+[\s\*]+)(\w+)\s*\(([^;{}]*)\)\s*(?:__attribute__\s*\(\(.*?\)\)\s*)*$", re.S)


class Function:
    def __init__(self, rel, name, ret, params_text, start, end, clean, raw_lines):
        self.rel, self.name, self.start, self.end = rel, name, start, end
        self.body = clean[start:end]
        self.params = []
        self.vars = {}
        for part in lc.split_top(params_text):
            p = parse_decl(part.strip())
            if p and p[2] not in ("void",):
                self.params.append(p[2])
                self.vars[p[2]] = (p[0], p[1])
        for m in re.finditer(r"(?:^|[;{}(])\s*((?:(?:const|static|volatile|unsigned)\s+)*"
                             r"(?:struct\s+\w+|union\s+\w+|\w+)[\s\*]+\w+(?:\s*,\s*\**\s*\w+)*)\s*(?==|;|,|\[)",
                             self.body):
            decl = m.group(1)
            parts = lc.split_top(decl)
            first = parse_decl(parts[0])
            if not first or first[0] in ("return", "goto", "case", "else", "sizeof"):
                continue
            self.vars.setdefault(first[2], (first[0], first[1]))
            for part in parts[1:]:
                mm = re.match(r"^\s*(\**)\s*(\w+)", part)
                if mm:
                    self.vars.setdefault(mm.group(2), (first[0], mm.group(1).count("*")))
        self.ret = parse_decl((ret or "").strip() + " x")
        self.clean = clean

    def line(self, pos):
        return self.clean.count("\n", 0, pos) + 1


def locate_functions(rel, clean_first_arm, clean_all):
    out = []
    depth = 0
    buffer_start = 0
    i = 0
    text = clean_first_arm
    while i < len(text):
        c = text[i]
        if c == "{":
            if depth == 0:
                head = text[buffer_start:i].strip()
                m = FUNC_HEAD.search(head)
                if m and m.group(2) not in ("if", "while", "for", "switch") and "=" not in head.split("(")[0] \
                        and not re.search(r"\b(struct|union|enum)\s+\w*\s*$", head):
                    end = match_brace(text, i)
                    out.append(Function(rel, m.group(2), m.group(1), m.group(3), i, end, clean_all, None))
                    i = end + 1
                    buffer_start = i
                    continue
            depth += 1
        elif c == "}":
            depth = max(0, depth - 1)
            if depth == 0:
                buffer_start = i + 1
        elif c == ";" and depth == 0:
            buffer_start = i + 1
        i += 1
    return out


# ---------------------------------------------------------------- resolution

PATH = re.compile(r"^\(*\s*(?:\(\s*(?:struct\s+)?(\w+)\s*(\**)\s*\)\s*)?(&\s*)?(\w+)((?:\s*(?:->|\.)\s*\w+)*)\s*\)*$")


class Census:
    def __init__(self, label, root):
        self.label, self.root = label, root
        self.types = Types()
        self.funcs = []
        self.by_name = {}
        self.files = {}
        self.digests = {}
        self.calls = {}   # name -> [(Function caller, actuals, rel, line)]
        self.address_taken = {}
        for rel in tree_files(root):
            path = os.path.join(root, rel)
            with open(path, encoding="utf-8", errors="replace") as f:
                raw = f.read()
            blanked = lc.blank(raw)
            clean = "\n".join(" " * len(line) if line.lstrip().startswith("#") else line
                              for line in blanked.split("\n"))
            self.files[rel] = clean
            self.types.read(lc.preprocessor_blank(blanked))
            if rel.endswith(".c"):
                self.digests[rel] = sha256(path)
                fns = locate_functions(rel, lc.preprocessor_blank(blanked), clean)
                self.funcs.extend(fns)
                for fn in fns:
                    self.by_name.setdefault(fn.name, []).append(fn)
        for fn in self.funcs:
            if fn.ret and fn.ret[0] == "mk_event_loop":
                self.types.returns[fn.name] = fn.ret
        self._index_calls()

    def enclosing(self, rel, pos):
        for fn in self.funcs:
            if fn.rel == rel and fn.start < pos < fn.end:
                return fn
        return None

    def _index_calls(self):
        names = set(self.by_name)
        word = re.compile(r"\b(\w+)\b(\s*\()?")
        for rel, clean in self.files.items():
            if not rel.endswith(".c"):
                continue
            for m in word.finditer(clean):
                name = m.group(1)
                if name not in names:
                    continue
                fn = self.enclosing(rel, m.start())
                if fn is None:
                    continue  # a definition head or a file-scope initializer
                if m.group(2):
                    actuals = lc.call_actuals(clean, clean.index("(", m.start()))
                    self.calls.setdefault(name, []).append((fn, actuals, rel, fn.line(m.start())))
                else:
                    self.address_taken.setdefault(name, []).append(f"{rel}:{fn.line(m.start())}")
        # an address taken at file scope (an initializer) also opens the formal
        for rel, clean in self.files.items():
            if not rel.endswith(".c"):
                continue
            for m in word.finditer(clean):
                if m.group(1) in names and not m.group(2) and self.enclosing(rel, m.start()) is None:
                    prev = clean[max(0, m.start() - 2):m.start()]
                    if re.search(r"[=,{&]\s*$", clean[max(0, m.start() - 40):m.start()]):
                        self.address_taken.setdefault(m.group(1), []).append(
                            f"{rel}:{clean.count(chr(10), 0, m.start()) + 1} (file scope)")
                    del prev

    # -- types of paths
    def path_type(self, fn, root, fields, cast=None):
        """(owner_tag of the last field, (base, depth) of the whole path)."""
        if cast:
            cur = (self.types.tag(cast[0]), cast[1])
        elif root in fn.vars:
            b, d = fn.vars[root]
            cur = (self.types.tag(b), d)
        else:
            return None, None
        owner = None
        for f in fields:
            tag = cur[0]
            owner = tag
            entry = self.types.fields.get(tag, {}).get(f)
            if entry is None:
                return owner, None
            cur = (self.types.tag(entry[0]), entry[1])
        return owner, cur

    def member_key(self, fn, expr):
        """`TAG.FIELD` when expr names a loop member, else None."""
        m = PATH.match(expr.strip())
        if not m or m.group(3):
            return None
        cast = (m.group(1), len(m.group(2) or "")) if m.group(1) else None
        root = m.group(4)
        fields = [f for f in re.split(r"\s*(?:->|\.)\s*", m.group(5).strip()) if f]
        if not fields or fields[-1] not in LOOP_FIELD_NAMES | self.loop_field_names():
            return None
        owner, typ = self.path_type(fn, root, fields, cast)
        if owner is None:
            return "untyped." + fields[-1]
        if typ and is_loop_type(typ[0], typ[1], fields[-1]):
            return f"{owner}.{fields[-1]}"
        if typ is None:
            return f"{owner}?.{fields[-1]}"
        return None

    _lfn = None

    def loop_field_names(self):
        if self._lfn is None:
            self._lfn = {f for tag, fs in self.types.fields.items() for f, (b, d) in fs.items()
                         if is_loop_type(self.types.tag(b), d, f)}
        return self._lfn

    def loop_members(self):
        out = {}
        for tag, fs in self.types.fields.items():
            for f, (b, d) in fs.items():
                if is_loop_type(self.types.tag(b), d, f):
                    out[f"{tag}.{f}"] = f"{b}{' *' * d}"
        return out

    def resolve(self, fn, expr, depth=0):
        """First-level origins of a loop-typed expression in fn."""
        e = " ".join(expr.split()).strip()
        while e.startswith("(") and e.endswith(")") and lc.call_actuals(e, 0) is not None \
                and len(lc.call_actuals(e, 0)) == 1 and lc.call_actuals("(" + e[1:-1] + ")", 0):
            inner = e[1:-1].strip()
            if inner.count("(") != inner.count(")"):
                break
            e = inner
        if e in ("NULL", "0"):
            return {"null"}
        call = re.match(r"^(\w+)\s*\((.*)\)$", e, re.S)
        if call:
            name = call.group(1)
            if name == "mk_event_loop_create":
                return {f"alloc:{fn.rel}:{{line}}"}
            if name == "flb_engine_evl_get":
                return {f"member:{TLS_KEY}"}
            if name in self.types.returns or name in self.by_name:
                return {f"ret:{name}"}
            return {f"open:call {name}"}
        key = self.member_key(fn, e)
        if key:
            return {f"member:{key}"}
        m = PATH.match(e)
        if m and not m.group(5) and not m.group(3):
            name = m.group(4)
            if name in fn.params:
                return {f"formal:{fn.name}#{fn.params.index(name)}"}
            if name in fn.vars:
                return self.local_origins(fn, name, depth)
            return {f"open:global {name}"}
        return {f"open:{e[:60]}"}

    def local_origins(self, fn, name, depth):
        if depth > 4:
            return {"open:local depth"}
        out = set()
        decl = re.search(r"\b" + re.escape(name) + r"\s*=(?!=)\s*([^;,]*)[;,]", fn.body)
        writes = []
        for m in re.finditer(r"(?<![\w>.])" + re.escape(name) + r"\s*=(?!=)\s*([^;]*);", fn.body):
            writes.append((m.group(1), fn.start + m.start(1)))
        if not writes and not decl:
            return {f"open:local {name} never written"}
        if re.search(r"&\s*" + re.escape(name) + r"\b", fn.body):
            out.add(f"open:local {name} address taken")
        for rhs, pos in writes:
            for o in self.resolve(fn, rhs, depth + 1):
                out.add(o.replace("{line}", str(fn.line(pos))))
        return out

    # -- stores into loop members
    def stores(self):
        out = []
        assign = re.compile(r"((?:\(\s*(?:struct\s+)?\w+\s*\*+\s*\)\s*)?\w+(?:\s*(?:->|\.)\s*\w+)+)\s*=(?!=)\s*([^;]*);")
        for fn in self.funcs:
            for m in assign.finditer(fn.body):
                key = self.member_key(fn, m.group(1))
                if not key:
                    continue
                pos = fn.start + m.start()
                origins = {o.replace("{line}", str(fn.line(fn.start + m.start(2))))
                           for o in self.resolve(fn, m.group(2))}
                out.append({"member": key, "site": f"{fn.rel}:{fn.line(pos)}", "function": fn.name,
                            "store": " ".join(m.group(0).split())[:120], "origins": sorted(origins)})
            for m in re.finditer(r"\bflb_engine_evl_set\s*\(", fn.body):
                actuals = lc.call_actuals(fn.body, m.end() - 1)
                pos = fn.start + m.start()
                origins = {o.replace("{line}", str(fn.line(pos))) for o in self.resolve(fn, actuals[0])}
                out.append({"member": TLS_KEY, "site": f"{fn.rel}:{fn.line(pos)}", "function": fn.name,
                            "store": f"flb_engine_evl_set({actuals[0]})", "origins": sorted(origins)})
        return out

    # -- closure
    def closure(self, stores):
        members = {}
        for s in stores:
            members.setdefault(s["member"], []).append(s)
        memo = {}

        def expand(origin, stack):
            if origin.startswith("alloc:") or origin == "null":
                return {origin}, set()
            if origin.startswith("open:"):
                return set(), {origin[5:]}
            if origin in stack:
                return set(), set()
            if origin in memo:
                return memo[origin]
            stack = stack | {origin}
            sites, opens = set(), set()
            if origin.startswith("member:"):
                key = origin[7:]
                if key not in members:
                    opens.add(f"{key}: no store read")
                for s in members.get(key, []):
                    for o in s["origins"]:
                        a, b = expand(o, stack)
                        sites |= a
                        opens |= b
            elif origin.startswith("formal:"):
                name, index = origin[7:].rsplit("#", 1)
                index = int(index)
                if self.address_taken.get(name):
                    opens.add(f"{name}: address taken at {self.address_taken[name][0]}")
                calls = self.calls.get(name, [])
                if not calls:
                    opens.add(f"{name}: no direct call read")
                for caller, actuals, rel, line in calls:
                    if not actuals or len(actuals) <= index:
                        opens.add(f"{name}: call at {rel}:{line} unreadable")
                        continue
                    for o in self.resolve(caller, actuals[index]):
                        a, b = expand(o.replace("{line}", str(line)), stack)
                        sites |= a
                        opens |= b
            elif origin.startswith("ret:"):
                name = origin[4:]
                for fn in self.by_name.get(name, []):
                    for m in re.finditer(r"\breturn\s+([^;]*);", fn.body):
                        for o in self.resolve(fn, m.group(1)):
                            a, b = expand(o.replace("{line}", str(fn.line(fn.start + m.start()))), stack)
                            sites |= a
                            opens |= b
                if name not in self.by_name:
                    opens.add(f"{name}: no definition read")
            memo[origin] = (sites, opens)
            return sites, opens

        out = {}
        for key in sorted(members):
            sites, opens = expand("member:" + key, frozenset())
            out[key] = {"candidates": sorted(s for s in sites if s != "null"),
                        "null_store": "null" in sites,
                        "closed": not opens, "open_reasons": sorted(opens)}
        return out, expand

    # -- operations
    def operations(self, expand):
        out = []
        pattern = re.compile(r"\b(" + "|".join(map(re.escape, OPERATIONS)) + r")\s*\(")
        for fn in self.funcs:
            for m in pattern.finditer(fn.body):
                callee = m.group(1)
                kind, loop_at, held_at = OPERATIONS[callee]
                actuals = lc.call_actuals(fn.body, m.end() - 1) or []
                pos = fn.start + m.start()
                entry = {"operation": kind, "callee": callee, "site": f"{fn.rel}:{fn.line(pos)}",
                         "function": fn.name, "actuals": actuals}
                if loop_at is not None and len(actuals) > loop_at:
                    first = {o.replace("{line}", str(fn.line(pos))) for o in self.resolve(fn, actuals[loop_at])}
                    entry["loop_origins"] = sorted(first)
                    sites, opens = set(), set()
                    for o in first:
                        a, b = expand(o, frozenset())
                        sites |= a
                        opens |= b
                    entry["loop_candidates"] = sorted(s for s in sites if s != "null")
                    entry["loop_open"] = sorted(opens)
                if held_at is not None and len(actuals) > held_at:
                    entry["held"] = self.held_key(fn, actuals[held_at])
                out.append(entry)
        return out

    def held_key(self, fn, expr):
        e = " ".join(expr.split())
        m = PATH.match(e)
        if not m:
            return {"spelled": e, "key": None}
        cast = (m.group(1), len(m.group(2) or "")) if m.group(1) else None
        root = m.group(4)
        fields = [f for f in re.split(r"\s*(?:->|\.)\s*", m.group(5).strip()) if f]
        if m.group(3) and fields:
            owner, _ = self.path_type(fn, root, fields, cast)
            return {"spelled": e, "key": f"{owner}.{fields[-1]}" if owner else None, "form": "interior"}
        if m.group(3):
            return {"spelled": e, "key": f"&{root}", "form": "frame object"}
        if root in fn.params:
            return {"spelled": e, "key": f"formal {root}", "form": "value"}
        writes = [w.group(1).strip() for w in re.finditer(r"(?<![\w>.])" + re.escape(root) + r"\s*=(?!=)\s*([^;]*);", fn.body)]
        keys = sorted({self.held_key(fn, w)["key"] or "?" for w in writes if w != e}) if writes else []
        one = keys[0] if len(keys) == 1 and keys[0] != "?" and "." in keys[0] else None
        return {"spelled": e, "key": one, "form": "local copy" if one else "local",
                "writes": writes[:4], "write_keys": keys}


def build(label, root):
    c = Census(label, root)
    stores = c.stores()
    closure, expand = c.closure(stores)
    ops = c.operations(expand)
    members = c.loop_members()
    per_member = {}
    for key in sorted(set(members) | {s["member"] for s in stores} | {TLS_KEY}):
        per_member[key] = {
            "declared_type": members.get(key, "thread-local struct mk_event_loop *" if key == TLS_KEY else None),
            "stores": [s for s in stores if s["member"] == key],
            **closure.get(key, {"candidates": [], "null_store": False, "closed": False,
                                "open_reasons": ["no store read"]}),
            "operations": {},
        }
    for op in ops:
        for o in op.get("loop_origins", []):
            if o.startswith("member:"):
                key = o[7:]
                slot = per_member.setdefault(key, {"declared_type": None, "stores": [], "candidates": [],
                                                   "null_store": False, "closed": False,
                                                   "open_reasons": ["no store read"], "operations": {}})
                slot["operations"].setdefault(op["operation"], []).append(
                    {"site": op["site"], "function": op["function"], "callee": op["callee"],
                     "held": (op.get("held") or {}).get("key") or (op.get("held") or {}).get("spelled")})
    # Held keys: for every interior `TAG.FIELD` registered or removed, the
    # loop origins (first level) of its registrations and of its removals.
    held = {}
    for op in ops:
        if op["operation"] not in ("register", "remove"):
            continue
        key = (op.get("held") or {}).get("key")
        if not key or "." not in key or key.startswith("None"):
            continue
        slot = held.setdefault(key, {"register": {}, "remove": {}})
        for o in op.get("loop_origins", ["none"]):
            slot[op["operation"]].setdefault(o, []).append(f"{op['function']} ({op['site']})")
    for key, slot in held.items():
        reg = set(slot["register"])
        rem = set(slot["remove"])
        slot["removed_only_at_other_origins"] = sorted(rem - reg)
        slot["registered_never_removed_at_same_origin"] = sorted(reg - rem)
    allocs = {}
    for key, m in per_member.items():
        for a in m["candidates"]:
            allocs.setdefault(a, []).append(key)
    summary = {
        "members": len(per_member),
        "members_with_store": sum(1 for m in per_member.values() if m["stores"]),
        "closed": sorted(k for k, m in per_member.items() if m["closed"] and m["stores"]),
        "open": sorted(k for k, m in per_member.items() if not m["closed"]),
        "allocation_sites": len(allocs),
        "operations": {k: sum(1 for o in ops if o["operation"] == k) for k in ("register", "remove", "wait", "destroy")},
        "operations_by_first_origin": {},
    }
    for o in ops:
        cls = ",".join(sorted({x.split(":", 1)[0] for x in o.get("loop_origins", [])})) or "none"
        summary["operations_by_first_origin"].setdefault(o["operation"], {}).setdefault(cls, 0)
        summary["operations_by_first_origin"][o["operation"]][cls] += 1
    return {"label": label, "root": root, "summary": summary, "members": per_member,
            "allocation_sites": {a: sorted(ms) for a, ms in sorted(allocs.items())},
            "held_keys": dict(sorted(held.items())),
            "operations": ops, "file_sha256": c.digests,
            "route_files": ROUTE_FILES.get(label, [])}


def main():
    out = {"card": "FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME",
           "kind": "read-only lexical census of loop-member candidate sets; sites, not findings",
           "trees": {}}
    for label, root in TREES.items():
        out["trees"][label] = build(label, root)
    dest = os.path.join(REPORT, "artifacts", "loop-member-census.json")
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, ensure_ascii=False, sort_keys=False)
        f.write("\n")
    for label, t in out["trees"].items():
        print("==", label, json.dumps(t["summary"], indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
