#!/usr/bin/env python3
"""Inventory of every declared, withheld and census-only descriptor cell
(card CELL-RESULT-SET-ALIGNMENT; gateway rules 38c-38f, contract
specification C04.6.3 and C09.4, grammar G05i and G05j).

USER REQUEST 2026-11-04 (verbatim): "We need to write down a prompt to align
the result set for our contracts."
USER DIRECTION 2026-11-03 (standing, verbatim): "The Fluent Bit's caller
tests sometimes check the incorrect assertions so we need to pay attention
for the assertions and actual returning values of Fluent Bit functions."

Usage:
    inventory.py --root REV=PATH [--root REV=PATH ...] --write inventory.json
    inventory.py --root REV=PATH [--root REV=PATH ...] --check inventory.json

A root is a Fluent Bit checkout (its Monkey under ``lib/monkey/``) or, with a
revision label starting ``monkey-``, a standalone Monkey checkout. Every file
is read only.

For every cell named in ``cells.json`` it records:

* the cell's **creators** and, per creator, the **live set**: the values the
  creator puts in the cell on its success class. ``cells.json`` states where
  each set comes from: ``declared`` (a C library ``result_set`` clause),
  ``pin-derived`` (a pinned, derived ``nonnull_result_range`` or a declared
  slot set), or ``MISSING`` (no contract states it). A declared or derived
  set must be found verbatim in the named tracked contract file, and the set
  is parsed from that clause text, never from this script; the contract
  file's SHA-256 is recorded. A MISSING set is never filled in: the reading
  of such a cell is ``undecided (live set MISSING)``, and the census class
  is computed only under the set the ``cells.json`` entry proposes, marked
  ``proposed``;
* every **absent store** its holder writes (the allocation's zero, an
  explicit constant store, ``MK_EVENT_ZERO``'s -1), each from an annotated
  site with its file:line. The **absent set** is their union, computed from
  the annotations of the bodies, never from a constant in this script;
* every **end test** (a comparison of the cell with a constant, or a bare
  ``if (X)``), with its census class (``exact`` / ``redundant`` / ``wrong``,
  CELL-PROTOCOL-PER-END-GUARDS' definitions over V = live | absent) and the
  edge reading of this card's step 4: an edge reading "absent" decides only
  when its values meet the live set nowhere and lie inside the absent set,
  an edge reading "live" only when its values meet the absent set nowhere.

``--check`` recomputes everything and refuses (exit 1) a file that differs,
an unannotated site, an unused annotation, a declared set whose clause is
not in its contract file, or a changed tree digest.

The reader is CELL-PROTOCOL-PER-END-GUARDS' lexical reader (comments and
strings blanked, bases resolved through the enclosing function's
declarations and the headers' record fields), widened to the records below
and to Monkey's headers. It is a census tool, not an analyzer.
"""

import argparse
import bisect
import hashlib
import json
import os
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
ANNOTATIONS = HERE / "annotations.json"
CELLS = HERE / "cells.json"

INT_MIN, INT_MAX = -(2 ** 31), 2 ** 31 - 1

# ---- interval sets over int --------------------------------------------


def merge(intervals):
    out = []
    for lo, hi in sorted(intervals):
        if out and lo <= out[-1][1] + 1:
            out[-1] = (out[-1][0], max(out[-1][1], hi))
        else:
            out.append((lo, hi))
    return out


def inter(a, b):
    out = []
    for lo, hi in a:
        for lo2, hi2 in b:
            l, h = max(lo, lo2), min(hi, hi2)
            if l <= h:
                out.append((l, h))
    return merge(out)


def minus(a, b):
    out = list(a)
    for lo2, hi2 in b:
        nxt = []
        for lo, hi in out:
            if hi < lo2 or lo > hi2:
                nxt.append((lo, hi))
                continue
            if lo < lo2:
                nxt.append((lo, lo2 - 1))
            if hi > hi2:
                nxt.append((hi2 + 1, hi))
        out = nxt
    return merge(out)


def subset(a, b):
    return not minus(a, b)


def show(s):
    if not s:
        return "{}"

    def bound(v):
        return "MIN" if v == INT_MIN else "MAX" if v == INT_MAX else str(v)

    return " | ".join(bound(lo) if lo == hi else f"[{bound(lo)}, {bound(hi)}]" for lo, hi in s)


def parse_set(text):
    """G05j's set syntax (`|`, `&`, `^`, `[a, b]`, MIN, MAX), C precedence."""
    tokens = re.findall(r"MIN|MAX|-?\d+|[\[\](),|&^]", text)
    if "".join(tokens) != re.sub(r"\s+", "", text):
        raise ValueError(f"not a set: {text}")
    pos = [0]

    def peek():
        return tokens[pos[0]] if pos[0] < len(tokens) else None

    def take(t=None):
        tok = peek()
        if t is not None and tok != t:
            raise ValueError(f"expected {t} in {text}")
        pos[0] += 1
        return tok

    def bound():
        tok = take()
        return INT_MIN if tok == "MIN" else INT_MAX if tok == "MAX" else int(tok)

    def atom():
        if peek() == "(":
            take("(")
            s = union()
            take(")")
            return s
        if peek() == "[":
            take("[")
            lo = bound()
            take(",")
            hi = bound()
            take("]")
            return [(lo, hi)]
        v = bound()
        return [(v, v)]

    def and_set():
        s = atom()
        while peek() == "&":
            take("&")
            s = inter(s, atom())
        return s

    def xor_set():
        s = and_set()
        while peek() == "^":
            take("^")
            t = and_set()
            s = merge(minus(s, t) + minus(t, s))
        return s

    def union():
        s = xor_set()
        while peek() == "|":
            take("|")
            s = merge(s + xor_set())
        return s

    s = union()
    if peek() is not None:
        raise ValueError(f"trailing tokens in {text}")
    return s


UNIVERSE = [(INT_MIN, INT_MAX)]
# The contrast column (USER DECISION 2026-11-04 "Write [1, MAX]" keeps POSIX's
# set beside the declared one): POSIX's lowest free descriptor may be 0.
POSIX_LIVE = [(0, INT_MAX)]
TEARDOWN = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv"


def declared_sets(record, path):
    """The live_set / absent_set a production per-end clause declares for
    the cell, read from the contract text (None when it states none)."""
    text = (REPO / TEARDOWN).read_text()
    m = re.search(r"quant cell #\w+: " + re.escape(record) + r"\." + re.escape(path)
                  + r" \|-> #fresh\(descriptor\) when [^;]*?(?: live_set\(([^;]*?)\))?(?: absent_set\(([^;]*?)\))?;", text)
    if not m or (m.group(1) is None and m.group(2) is None):
        return None
    return {"live_set": show(parse_set(m.group(1))) if m.group(1) else None,
            "absent_set": show(parse_set(m.group(2))) if m.group(2) else None,
            "contract": TEARDOWN}


def test_set(op, k):
    """The values on which `X OP k` holds."""
    return {
        ">": [(k + 1, INT_MAX)] if k < INT_MAX else [],
        ">=": [(k, INT_MAX)],
        "<": [(INT_MIN, k - 1)] if k > INT_MIN else [],
        "<=": [(INT_MIN, k)],
        "==": [(k, k)],
        "!=": minus(UNIVERSE, [(k, k)]),
    }[op]


# ---- the lexical reader (CELL-PROTOCOL-PER-END-GUARDS', widened) ---------


def blank(text):
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append(c + re.sub(r"[^\n]", " ", text[i + 1:j - 1]) + c)
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def struct_fields(headers):
    table = {}
    for code in headers:
        for m in re.finditer(r"\bstruct\s+(\w+)\s*\{", code):
            depth, j = 1, m.end()
            while j < len(code) and depth:
                depth += {"{": 1, "}": -1}.get(code[j], 0)
                j += 1
            body = code[m.end():j - 1]
            fields = table.setdefault(m.group(1), {})
            for f in re.finditer(r"\bstruct\s+(\w+)\s*\**\s*(\w+)\s*(?:\[[^\]]*\])?\s*;", body):
                fields.setdefault(f.group(2), f.group(1))
    return table


HEADER = re.compile(r"^[A-Za-z_][^;\n]*?\b(\w+)\s*\(", re.M)


def functions(code):
    spans = []
    for m in re.finditer(r"^\{", code, re.M):
        head = code[max(0, m.start() - 600):m.start()]
        headers = list(HEADER.finditer(head))
        if not headers:
            continue
        name = headers[-1].group(1)
        if name in ("if", "while", "for", "switch", "return", "sizeof"):
            continue
        close = re.compile(r"^\}", re.M).search(code, m.end())
        end = close.end() if close else len(code)
        start = m.start() - len(head) + headers[-1].start()
        spans.append((name, start, end))
    return spans


def declared_types(text, typedefs):
    names = {}
    for m in re.finditer(r"\bstruct\s+(\w+)\s*(\**)\s*(\w+)\s*[,;=)\[]", text):
        names.setdefault(m.group(3), m.group(1))
    for alias, record in typedefs.items():
        for m in re.finditer(r"\b" + alias + r"\s*\*\s*(\w+)", text):
            names.setdefault(m.group(1), record)
    return names


def resolve(base, scope, fields):
    parts = re.split(r"\s*(?:->|\.)\s*", base.strip())
    record = scope.get(parts[0])
    for part in parts[1:]:
        if record is None:
            return None
        record = fields.get(record, {}).get(part)
    return record


def base_before(code, pos):
    j = pos
    while True:
        k = j
        while k > 0 and code[k - 1].isspace():
            k -= 1
        m = re.search(r"[A-Za-z_]\w*$", code[max(0, k - 80):k])
        if not m:
            return None, False
        start = k - len(m.group(0))
        k2 = start
        while k2 > 0 and code[k2 - 1].isspace():
            k2 -= 1
        if code[k2 - 2:k2] == "->":
            j = k2 - 2
            continue
        if k2 > 0 and code[k2 - 1] == "." and not code[k2 - 2:k2 - 1].isdigit():
            j = k2 - 1
            continue
        base = re.sub(r"\s+", "", code[start:pos])
        amp = code[k2 - 1:k2] == "&" and code[k2 - 2:k2] != "&&"
        return base, amp


def callee_before(code, pos):
    depth = 0
    j = pos - 1
    while j >= 0:
        c = code[j]
        if c == ")":
            depth += 1
        elif c == "(":
            if depth == 0:
                m = re.search(r"(\w+)\s*$", code[:j])
                return m.group(1) if m else None
            depth -= 1
        elif c in ";{}" and depth == 0:
            return None
        j -= 1
    return None


def statement_value(code, pos):
    end = code.find(";", pos)
    return " ".join(code[pos:end].split()) if end >= 0 else ""


COMPARISON = re.compile(r"\s*(>=|<=|==|!=|>|<)\s*(-?\s*\d+)\b")
REVERSED = re.compile(r"\b(-?\s*\d+)\s*(>=|<=|==|!=|>|<)\s*$")
FLIP = {">": "<", "<": ">", ">=": "<=", "<=": ">=", "==": "==", "!=": "!="}


def end_test(code, start, end):
    m = COMPARISON.match(code, end)
    if m:
        return {"op": m.group(1), "value": int(m.group(2).replace(" ", ""))}
    m = REVERSED.search(code[max(0, start - 40):start])
    if m:
        return {"op": FLIP[m.group(2)], "value": int(m.group(1).replace(" ", ""))}
    before = code[max(0, start - 40):start]
    after = code[end:end + 2]
    if re.search(r"\bif\s*\(\s*$", before) and after.lstrip().startswith(")"):
        return {"op": "!=", "value": 0}
    if re.search(r"\bif\s*\(\s*!\s*$", before) and after.lstrip().startswith(")"):
        return {"op": "==", "value": 0}
    return None


# ---- configuration (cells.json) ----------------------------------------


def load_cells():
    spec = json.loads(CELLS.read_text())
    return spec


def scan_tree(revision, root, spec):
    monkey = revision.startswith("monkey-")
    tree = spec["trees"]["monkey" if monkey else "fluent-bit"]
    records = spec["records"]
    members = sorted({m for ms in records.values() for m in ms}, key=len, reverse=True)
    struct_members = set(spec["struct_members"])
    specific = set(spec["specific"])
    arrays = set(spec["arrays"])
    typedefs = spec.get("typedefs", {})
    needle = re.compile(r"\b(" + "|".join(sorted(set(members) | set(records)
                                                  | {l["name"] for l in spec["locals"]})) + r")\b")
    member_re = re.compile(r"(->|\.)\s*(" + "|".join(members) + r")\b")
    rest_re = re.compile(r"(?:\s*\[\s*[^\]]*\])?(?:\s*\.\s*\w+)?")
    root = pathlib.Path(root)
    files = []
    for top in tree["scan"]:
        for dirpath, dirnames, filenames in os.walk(root / top):
            dirnames.sort()
            for f in sorted(filenames):
                if f.endswith((".c", ".h")):
                    files.append(pathlib.Path(dirpath) / f)
    files.sort()
    skip = re.compile(tree["skip"]) if tree.get("skip") else None
    digests, texts = {}, {}
    for path in files:
        rel = path.relative_to(root).as_posix()
        if skip and skip.search(rel):
            continue
        data = path.read_bytes()
        digests[rel] = hashlib.sha256(data).hexdigest()
        raw = data.decode("utf-8", errors="replace")
        if rel.endswith(".h") or needle.search(raw):
            texts[rel] = blank(raw)
    fields = struct_fields(t for r, t in texts.items() if r.endswith(".h"))
    first_event = {}
    for record in records:
        for code in (t for r, t in texts.items() if r.endswith(".h")):
            if re.search(r"\bstruct\s+" + record + r"\s*\{\s*struct\s+mk_event\s+event\s*;", code):
                first_event[record] = True
    sites = []
    for rel, code in texts.items():
        if not needle.search(code):
            continue
        spans = functions(code)
        newlines = [i for i, c in enumerate(code) if c == "\n"]
        lines = code.splitlines()
        line_of = lambda pos: bisect.bisect_left(newlines, pos) + 1
        scopes = {}

        def scope_at(pos):
            for name, start, end in spans:
                if start <= pos < end:
                    if start not in scopes:
                        scopes[start] = declared_types(code[start:end], typedefs)
                    return name, scopes[start]
            return "<file>", {}

        file_scope = declared_types(code, typedefs)
        for m in member_re.finditer(code):
            member = m.group(2)
            base, amp = base_before(code, m.start())
            if not base:
                continue
            function, scope = scope_at(m.start())
            merged = dict(file_scope)
            merged.update(scope)
            record = resolve(base, merged, fields)
            if record not in records and not (record is None and member in specific):
                continue
            if record in records and member not in records[record]:
                continue
            rest = rest_re.match(code, m.end()).group(0)
            path = member + re.sub(r"\s+", "", rest)
            # A struct member (an mk_event, the window) is a cell only as
            # `.fd`; the bare member is read only as an address handed on.
            if member in struct_members and not path.endswith(".fd") and not amp:
                continue
            if member in struct_members and amp and path != member:
                continue
            end = m.end() + len(rest)
            after = code[end:end + 16]
            if amp:
                kind, detail = "address", callee_before(code, m.start())
            elif re.match(r"\s*(=[^=]|\+=|-=|\|=|&=|\+\+|--)", after):
                kind, detail = "store", statement_value(code, end)
            elif re.match(r"\s*[,)]", after) and not rest and member in arrays:
                kind, detail = "array", callee_before(code, m.start())
            else:
                kind, detail = "read", None
            line = line_of(m.start())
            site = {"file": rel, "line": line, "function": function, "record": record,
                    "base": base, "path": path, "kind": kind, "detail": detail,
                    "text": " ".join(lines[line - 1].split())}
            if kind == "read":
                test = end_test(code, m.start() - len(base), end)
                if test:
                    site["test"] = test
            sites.append(site)
        # Plain locals that hold a creator's result (Monkey's `timeout_fd`).
        for local in spec["locals"]:
            for name, start, end in spans:
                if name != local["function"] or not rel.endswith(local["file"]):
                    continue
                for m in re.finditer(r"(?<![\w.>])" + local["name"] + r"\b", code[start:end]):
                    pos = start + m.start()
                    stop = pos + len(local["name"])
                    after = code[stop:stop + 16]
                    if re.match(r"\s*(=[^=]|\+=|-=|\+\+|--)", after):
                        kind, detail = "store", statement_value(code, stop)
                    elif re.match(r"\s*;", after) and re.search(r"\bint\s*$", code[max(0, pos - 20):pos]):
                        kind, detail = "declaration", None
                    else:
                        kind, detail = "read", None
                    line = line_of(pos)
                    site = {"file": rel, "line": line, "function": name, "record": "<local>",
                            "base": "", "path": local["name"], "kind": kind, "detail": detail,
                            "text": " ".join(lines[line - 1].split())}
                    if kind == "read":
                        test = end_test(code, pos, stop)
                        if test:
                            site["test"] = test
                    sites.append(site)
        for m in re.finditer(r"\b((?:mk_event|MK_EVENT)_\w+)\s*\(", code):
            depth, j = 1, m.end()
            while j < len(code) and depth:
                depth += {"(": 1, ")": -1}.get(code[j], 0)
                j += 1
            function, scope = scope_at(m.start())
            merged = dict(file_scope)
            merged.update(scope)
            for arg in re.split(r",", code[m.end():j - 1]):
                name = re.sub(r"^\s*\((?:struct\s+\w+|void)\s*\*\s*\)\s*", "", arg).strip()
                record = merged.get(name) if re.fullmatch(r"[A-Za-z_]\w*", name) else None
                if record in records and first_event.get(record) and "event" in records[record]:
                    line = line_of(m.start())
                    sites.append({"file": rel, "line": line, "function": function, "record": record,
                                  "base": name, "path": "event", "kind": "address",
                                  "detail": m.group(1) + " (the record as its first-member event)",
                                  "text": " ".join(lines[line - 1].split())})
        for record in records:
            for m in re.finditer(r"sizeof\s*\(\s*struct\s+" + record + r"\s*\)", code):
                function, _ = scope_at(m.start())
                line = line_of(m.start())
                sites.append({"file": rel, "line": line, "function": function, "record": record,
                              "base": "", "path": "<record>", "kind": "alloc", "detail": "sizeof",
                              "text": " ".join(lines[line - 1].split())})
        for m in re.finditer(r"\b(sizeof\s*\(\s*\*\s*(\w+)\s*\)|mem(?:set|cpy)\s*\(\s*(\w+)\s*,)", code):
            function, scope = scope_at(m.start())
            merged = dict(file_scope)
            merged.update(scope)
            var = m.group(2) or m.group(3)
            record = merged.get(var)
            if record in records:
                line = line_of(m.start())
                sites.append({"file": rel, "line": line, "function": function, "record": record,
                              "base": var, "path": "<record>", "kind": "alloc",
                              "detail": m.group(1).split("(")[0].strip(),
                              "text": " ".join(lines[line - 1].split())})
    for record in records:
        if record not in fields and any(s["record"] == record for s in sites):
            raise SystemExit(f"{revision}: struct {record} not found in the headers")
    digest = hashlib.sha256("".join(f"{r} {d}\n" for r, d in sorted(digests.items())).encode()).hexdigest()
    held = sorted({s["file"] for s in sites})
    return {"revision": revision, "scanned_files": len(digests), "tree_sha256": digest,
            "file_sha256": {f: digests[f] for f in held}, "sites": sites}


def key(site):
    return "|".join([site["record"] or "?", site["file"], site["function"], site["kind"],
                     site["path"], site["text"]])


# ---- the live sets: from the contracts, never from this script ------------


def creator_sets(spec, problems):
    """Per creator: its live set as read from the named contract clause, or
    MISSING. A declared or derived clause must be present verbatim."""
    out = {}
    for name, entry in spec["creators"].items():
        status = entry["status"]
        record = {"status": status, "why": entry["why"]}
        if status in ("declared", "pin-derived"):
            path = REPO / entry["contract"]["file"]
            text = path.read_text()
            clause = entry["contract"]["clause"]
            if clause not in text:
                problems.append(f"creator {name}: clause not in {entry['contract']['file']}: {clause}")
                continue
            m = re.search(r"\bresult_set\((.*)\)|nonnull_result_range\((-?\d+),\s*(-?\d+|MAX)\)|written_set\(\w+(?:\[\d\])?,\s*(.*)\) when", clause)
            if m.group(1) is not None:
                whole = parse_set(m.group(1))
            elif m.group(4) is not None:
                whole = parse_set(m.group(4))
            else:
                whole = parse_set(f"[{m.group(2)}, {m.group(3)}]")
            success = parse_set(entry["success_class"]) if entry.get("success_class") else UNIVERSE
            record.update(contract=entry["contract"], contract_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                          live=show(inter(whole, success)))
        else:
            record["proposed"] = entry["proposed"]
        out[name] = record
    return out


# ---- classes and readings ----------------------------------------------


def classify(op, k, live, absent):
    """CELL-PROTOCOL-PER-END-GUARDS' classes over V = live | absent: the edge
    that uses the end (`>`, `>=`, `!=`: true; `<`, `<=`, `==`: false) reads
    it live."""
    true_set = test_set(op, k)
    present = true_set if op in (">", ">=", "!=") else minus(UNIVERSE, true_set)
    shared = inter(live, absent)
    if shared:
        return "wrong", f"absent value {show(shared)} is also a live value: no test of this end is exact"
    live_read_absent = minus(live, present)
    absent_read_live = inter(absent, present)
    if live_read_absent or absent_read_live:
        why = []
        if live_read_absent:
            why.append(f"live {show(live_read_absent)} read as absent")
        if absent_read_live:
            why.append(f"absent {show(absent_read_live)} read as live")
        return "wrong", "; ".join(why)
    if present == live:
        return "exact", "present exactly on the live set"
    return "redundant", "correct on live | absent, differs outside it"


def reading(op, k, live, absent):
    """Step 4's reading of each edge: `absent` decided only when the edge's
    values meet the live set nowhere and lie inside the absent set; `live`
    decided only when they meet the absent set nowhere."""
    true_set = test_set(op, k)
    false_set = minus(UNIVERSE, true_set)
    live_edge, dead_edge = (true_set, false_set) if op in (">", ">=", "!=") else (false_set, true_set)
    # Over the cell's values (live | absent), as rule 38f (6) reads it.
    dead = "decided" if (not inter(dead_edge, live)
                         and subset(inter(dead_edge, merge(live + absent)), absent)) else "NOT decided"
    alive = "decided" if not inter(live_edge, absent) else "NOT decided"
    return {"dead_edge": show(dead_edge), "dead_edge_reads_absent": dead,
            "live_edge": show(live_edge), "live_edge_reads_live": alive}


def build(roots, write):
    spec = load_cells()
    problems = []
    sets = creator_sets(spec, problems)
    annotations = json.loads(ANNOTATIONS.read_text())
    by_key = {}
    for entry in annotations["sites"]:
        if entry["key"] in by_key:
            problems.append(f"duplicate annotation {entry['key']}")
        by_key[entry["key"]] = entry
    used = set()
    witnesses = {w["key"]: w["why"] for w in spec.get("witness_tests", [])}
    used_witnesses = set()
    revisions = []
    for revision, path in roots:
        rev = scan_tree(revision, path, spec)
        for site in rev["sites"]:
            if site["kind"] == "read":
                continue
            entry = by_key.get(key(site))
            if entry is None:
                problems.append(f"{revision}: unannotated {key(site)} ({site['file']}:{site['line']})")
                continue
            used.add(key(site))
            site["role"] = entry["role"]
            for f in ("value", "cells", "creator", "superseded"):
                if f in entry:
                    site[f] = entry[f]
        cells = []
        for cell in spec["cells"]:
            if revision.startswith("monkey-") != (cell.get("tree") == "monkey") and cell.get("tree") != "both":
                continue
            cell_out = {k: cell[k] for k in ("record", "path", "protocol") if k in cell}
            names = {cell["path"]} | set(cell.get("aliases", []))
            mine = [s for s in rev["sites"] if s["record"] == cell["record"]
                    and (s["path"] in names or s["path"] == "<record>"
                         or cell["path"] in s.get("cells", []))]
            # The creators and their live sets.
            creations = [s for s in mine if s.get("role") == "creation"
                         and (s["path"] in names or cell["path"] in s.get("cells", []))]
            creators = sorted({s["creator"] for s in creations})
            cell_out["creators"] = [{"creator": c, "sites": [f"{s['file']}:{s['line']}" for s in creations
                                                               if s["creator"] == c],
                                     **sets.get(c, {"status": "MISSING", "why": "no entry"})} for c in creators]
            if not creators:
                problems.append(f"{revision}: cell {cell['record']}.{cell['path']} has no creation site")
            declared = [sets[c] for c in creators if c in sets and sets[c]["status"] != "MISSING"]
            missing = [c for c in creators if c not in sets or sets[c]["status"] == "MISSING"]
            if missing:
                live = parse_set(cell["proposed_live"])
                cell_out["live_set"] = {"status": "MISSING", "missing": missing,
                                        "proposed": show(live)}
            else:
                live = merge([iv for d in declared for iv in parse_set(d["live"])])
                cell_out["live_set"] = {"status": "declared", "set": show(live)}
            # The absent stores.
            stores = []
            for s in mine:
                if s.get("role") == "alloc" and cell["path"] in s.get("superseded", []):
                    continue
                if s.get("role") in ("alloc", "absent") and s.get("value") is not None:
                    if s["path"] == "<record>" or s["path"] in names or cell["path"] in s.get("cells", []):
                        stores.append({"site": f"{s['file']}:{s['line']}", "function": s["function"],
                                       "value": s["value"], "text": s["text"]})
            absent = merge([(v, v) for v in {st["value"] for st in stores}])
            cell_out["absent_stores"] = stores
            cell_out["absent_set"] = show(absent)
            # The production declaration's sets, checked against the census:
            # every absent store inside the declared absent set, every
            # creator's live set inside the declared live set.
            sets_declared = declared_sets(cell["record"], cell["path"])
            if sets_declared:
                cell_out["declared"] = sets_declared
                if sets_declared["absent_set"] and not subset(absent, parse_set(sets_declared["absent_set"])):
                    problems.append(f"{revision}: {cell['record']}.{cell['path']}: absent stores {show(absent)} outside the declared {sets_declared['absent_set']}")
                if sets_declared["live_set"] and not subset(live, parse_set(sets_declared["live_set"])):
                    problems.append(f"{revision}: {cell['record']}.{cell['path']}: creators' live set {show(live)} outside the declared {sets_declared['live_set']}")
            tests = []
            for s in rev["sites"]:
                if "test" not in s or s["record"] != cell["record"] or s["path"] not in names:
                    continue
                op, k = s["test"]["op"], s["test"]["value"]
                wkey = "|".join([s["record"], s["file"], s["function"], s["path"], s["text"]])
                if wkey in witnesses:
                    used_witnesses.add(wkey)
                    tests.append({"site": f"{s['file']}:{s['line']}", "function": s["function"],
                                  "test": f"{s['path']} {op} {k}", "text": s["text"],
                                  "class": "witness", "why": witnesses[wkey]})
                    continue
                cls, why = classify(op, k, live, absent)
                posix_cls, posix_why = classify(op, k, POSIX_LIVE, absent)
                t = {"site": f"{s['file']}:{s['line']}", "function": s["function"],
                     "test": f"{s['path']} {op} {k}", "text": s["text"], "class": cls, "why": why,
                     "class_posix": posix_cls, "why_posix": posix_why}
                if missing:
                    t["class_basis"] = "proposed live set (MISSING)"
                    t["reading"] = {"dead_edge_reads_absent": "NOT decided (live set MISSING)",
                                    "live_edge_reads_live": "NOT decided (live set MISSING)",
                                    "under_proposed": reading(op, k, live, absent)}
                else:
                    t["reading"] = reading(op, k, live, absent)
                tests.append(t)
            cell_out["end_tests"] = tests
            cells.append(cell_out)
        rev["cells"] = cells
        revisions.append(rev)
    for k in witnesses:
        if k not in used_witnesses:
            problems.append(f"unused witness test {k}")
    for k in by_key:
        if k not in used:
            problems.append(f"unused annotation {k}")
    return {"schema": "cell-result-set-inventory/1", "card": "CELL-RESULT-SET-ALIGNMENT",
            "creators": sets, "revisions": revisions}, problems


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", action="append", required=True, metavar="REV=PATH")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write", type=pathlib.Path)
    group.add_argument("--check", type=pathlib.Path)
    parser.add_argument("--declarations", default=None,
                        help="the per-end declarations to check (default: the production teardown_destroy.qv; the planned patch module for a patched tree)")
    parser.add_argument("--annotations", default=None, help="the site readings (default: annotations.json)")
    args = parser.parse_args()
    global TEARDOWN, ANNOTATIONS, CELLS
    if args.declarations:
        TEARDOWN = args.declarations
    if args.annotations:
        ANNOTATIONS = pathlib.Path(args.annotations)
    roots = [tuple(r.split("=", 1)) for r in args.root]
    result, problems = build(roots, args.write)
    rendered = json.dumps(result, indent=1, sort_keys=True) + "\n"
    if args.write:
        args.write.write_text(rendered)
    elif args.check.read_text() != rendered:
        problems.append(f"{args.check} differs from the recomputed inventory")
    for p in problems:
        print(p, file=sys.stderr)
    for rev in result["revisions"]:
        tally, posix = {}, {}
        for cell in rev["cells"]:
            for t in cell["end_tests"]:
                tally[t["class"]] = tally.get(t["class"], 0) + 1
                if "class_posix" in t:
                    posix[t["class_posix"]] = posix.get(t["class_posix"], 0) + 1
        missing = sum(1 for c in rev["cells"] if c["live_set"]["status"] == "MISSING")
        print(f"{rev['revision']}: {rev['scanned_files']} files, {len(rev['sites'])} sites, "
              f"{len(rev['cells'])} cells ({missing} with a MISSING live set); end tests {tally}; against POSIX's [0, MAX] {posix}")
    print(f"{len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
