#!/usr/bin/env python3
"""Store census of the declared cell-protocol members (card
CELL-PROTOCOL-FB-DECLARATIONS; gateway rule 38c, contract specification
§C04.6.3, grammar G05i).

Usage:
    census.py --root REV=PATH [--root REV=PATH ...] --write census.json
    census.py --root REV=PATH [--root REV=PATH ...] --check census.json

Reads every C source and header of each Fluent Bit tree (read only; the tree
is a checkout of the named revision) and lists every access to a member of a
record the card considers for a protocol, with the member path, the base
expression, the record the base resolves to, and the kind of access:

* ``store``    ``BASE->PATH = V`` (also compound assignments, ``++``, ``--``);
* ``address``  ``&BASE->PATH`` handed to a call (the callee may store);
* ``array``    the bare array member handed to a call (it decays to a pointer);
* ``read``     anything else (tests and value reads);
* ``alloc``    ``sizeof(struct R)`` / ``sizeof(*X)`` with X a ``struct R *``
  (the record's initial bytes), and ``memset``/``memcpy`` of a record.

Every non-read site of a considered record, and every non-read site of a
specific member whose base the reader cannot resolve, must carry exactly one
annotation in ``annotations.json`` (keyed by revision-independent text, so
one annotation covers a site that only moved lines), stating whether the site
keeps the record's protocol. ``--check`` recomputes the census from the trees
and refuses (exit 1) a census file that differs, an unannotated site, an
unused annotation, or a tree digest that changed. The census binds itself to
the sources through the SHA-256 of every file it read, folded into one tree
digest per revision, and lists the digest of every file holding a site.

The reader is lexical: comments and string literals are blanked, a base is
resolved through the enclosing function's declarations (``struct T *name``,
``struct T name``, ``T *name`` for the known typedefs) and the record field
table parsed from the headers. What it cannot resolve it reports, never
drops. It is a census tool, not an analyzer; the annotations are the reading.
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
ANNOTATIONS = HERE / "annotations.json"

# The records the card considers (PROPOSAL.md section 1 of
# MK-EVENT-CELL-CROSS-FUNCTION) and, per record, the members a protocol of
# it would name: the cells, their `#same` event, and the guard.
RECORDS = {
    "flb_sched": ["ch_events", "event"],
    "flb_input_thread_instance": [
        "notification_channels_initialized", "notification_channels", "notification_event",
        "ch_parent_events", "ch_thread_events", "event", "event_local",
    ],
    "flb_config": [
        "notification_channels_initialized", "notification_channels", "notification_event",
        "ch_manager", "ch_event", "ch_notif", "ch_self_events", "event_thread_init",
    ],
    "flb_input_instance": ["ch_events", "event"],
    "flb_output_instance": ["ch_events", "event"],
}
# Members whose name is specific enough that an unresolved base is reported.
SPECIFIC = {
    "ch_events", "notification_channels_initialized", "notification_channels",
    "notification_event", "ch_parent_events", "ch_thread_events", "ch_manager", "ch_notif",
    "ch_self_events", "event_thread_init",
}
TYPEDEFS = {"flb_ctx_t": "flb_lib_ctx"}
MEMBERS = sorted({m for ms in RECORDS.values() for m in ms}, key=len, reverse=True)
FIRST_EVENT = {}
SCAN_DIRS = ("src", "include", "plugins")
NEEDLE = re.compile(r"\b(" + "|".join(sorted(set(MEMBERS) | set(RECORDS))) + r")\b")
SUFFIXES = (".c", ".h")


def blank(text):
    """Comments and string/char literals to spaces, newlines kept."""
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
    """struct name -> {field: pointee struct} for `struct T *f;` / `struct T f;`."""
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
    """(name, start, end) of every function body, Fluent Bit style: a body
    opens with `{` in column 0 after a header line that starts in column 0,
    and closes with the next `}` in column 0. Brace counting would be misled
    by preprocessor arms that each open a block."""
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


def declared_types(text):
    names = {}
    for m in re.finditer(r"\bstruct\s+(\w+)\s*(\**)\s*(\w+)\s*[,;=)\[]", text):
        names.setdefault(m.group(3), m.group(1))
    for alias, record in TYPEDEFS.items():
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


MEMBER = re.compile(r"(->|\.)\s*(" + "|".join(MEMBERS) + r")\b")
REST = re.compile(r"(?:\s*\[\s*[^\]]*\])?(?:\s*\.\s*\w+)?")


def base_before(code, pos):
    """The access chain ending at `pos` (`a->b.c`), and whether `&` precedes it."""
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


def scan_file(rel, code, fields, sites):
    spans = functions(code)
    newlines = [i for i, c in enumerate(code) if c == "\n"]
    lines = code.splitlines()
    line_of = lambda pos: bisect.bisect_left(newlines, pos) + 1
    scopes = {}

    def scope_at(pos):
        for name, start, end in spans:
            if start <= pos < end:
                if start not in scopes:
                    scopes[start] = declared_types(code[start:end])
                return name, scopes[start]
        return "<file>", {}

    file_scope = declared_types(code)
    for m in MEMBER.finditer(code):
        member = m.group(2)
        base, amp = base_before(code, m.start())
        if not base:
            continue
        function, scope = scope_at(m.start())
        merged = dict(file_scope)
        merged.update(scope)
        record = resolve(base, merged, fields)
        if record not in RECORDS and not (record is None and member in SPECIFIC):
            continue
        if record in RECORDS and member not in RECORDS[record]:
            continue
        rest = REST.match(code, m.end()).group(0)
        path = member + re.sub(r"\s+", "", rest)
        end = m.end() + len(rest)
        after = code[end:end + 3]
        if amp:
            kind, detail = "address", callee_before(code, m.start())
        elif re.match(r"\s*(=[^=]|\+=|-=|\|=|&=|\+\+|--)", after):
            kind, detail = "store", statement_value(code, end)
        elif re.match(r"\s*[,)]", after) and not rest:
            kind, detail = "array", callee_before(code, m.start())
        else:
            kind, detail = "read", None
        line = line_of(m.start())
        text = " ".join(lines[line - 1].split())
        sites.append({
            "file": rel, "line": line, "function": function, "record": record,
            "base": base, "path": path, "kind": kind, "detail": detail, "text": text,
        })
    # A record whose first member is its `struct mk_event` is its own event:
    # the record pointer handed to an event-loop call stores `event.fd`.
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
            if record in RECORDS and FIRST_EVENT.get(record):
                line = line_of(m.start())
                sites.append({"file": rel, "line": line, "function": function, "record": record,
                              "base": name, "path": "event", "kind": "address",
                              "detail": m.group(1) + " (the record as its first-member event)",
                              "text": " ".join(lines[line - 1].split())})
    for record in RECORDS:
        for m in re.finditer(r"sizeof\s*\(\s*struct\s+" + record + r"\s*\)", code):
            function, _ = scope_at(m.start())
            line = line_of(m.start())
            sites.append({"file": rel, "line": line, "function": function, "record": record,
                          "base": "", "path": "<record>", "kind": "alloc",
                          "detail": "sizeof", "text": " ".join(code.splitlines()[line - 1].split())})
    for m in re.finditer(r"\b(sizeof\s*\(\s*\*\s*(\w+)\s*\)|mem(?:set|cpy)\s*\(\s*(\w+)\s*,)", code):
        function, scope = scope_at(m.start())
        merged = dict(file_scope)
        merged.update(scope)
        var = m.group(2) or m.group(3)
        record = merged.get(var)
        if record in RECORDS:
            line = line_of(m.start())
            sites.append({"file": rel, "line": line, "function": function, "record": record,
                          "base": var, "path": "<record>", "kind": "alloc",
                          "detail": m.group(1).split("(")[0].strip(),
                          "text": " ".join(code.splitlines()[line - 1].split())})


def census(revision, root):
    root = pathlib.Path(root)
    files = []
    for top in SCAN_DIRS:
        for dirpath, dirnames, filenames in os.walk(root / top):
            dirnames.sort()
            for f in sorted(filenames):
                if f.endswith(SUFFIXES):
                    files.append(pathlib.Path(dirpath) / f)
    files.sort()
    digests = {}
    texts = {}
    for path in files:
        data = path.read_bytes()
        rel = path.relative_to(root).as_posix()
        digests[rel] = hashlib.sha256(data).hexdigest()
        raw = data.decode("utf-8", errors="replace")
        if rel.startswith("include/") or NEEDLE.search(raw):
            texts[rel] = blank(raw)
    fields = struct_fields(t for r, t in texts.items() if r.startswith("include/"))
    FIRST_EVENT.clear()
    for record in RECORDS:
        for code in (t for r, t in texts.items() if r.startswith("include/")):
            m = re.search(r"\bstruct\s+" + record + r"\s*\{\s*struct\s+mk_event\s+event\s*;", code)
            if m:
                FIRST_EVENT[record] = True
    for record in RECORDS:
        if record not in fields:
            raise SystemExit(f"{revision}: struct {record} not found in include/")
    sites = []
    for rel, code in texts.items():
        if NEEDLE.search(code):
            scan_file(rel, code, fields, sites)
    tree = hashlib.sha256("".join(f"{r} {d}\n" for r, d in sorted(digests.items())).encode()).hexdigest()
    held = sorted({s["file"] for s in sites})
    return {
        "revision": revision,
        "scanned_files": len(digests),
        "tree_sha256": tree,
        "file_sha256": {f: digests[f] for f in held},
        "sites": sites,
    }


def key(site):
    return "|".join([site["record"] or "?", site["file"], site["function"], site["kind"],
                     site["path"], site["text"]])


def annotate(result, annotations):
    """Attach each non-read site's annotation; return the problems."""
    problems = []
    used = set()
    by_key = {}
    for entry in annotations["sites"]:
        if entry["key"] in by_key:
            problems.append(f"duplicate annotation {entry['key']}")
        by_key[entry["key"]] = entry
    for rev in result["revisions"]:
        for site in rev["sites"]:
            if site["kind"] == "read":
                continue
            k = key(site)
            entry = by_key.get(k)
            if entry is None:
                problems.append(f"{rev['revision']}: unannotated {k} ({site['file']}:{site['line']})")
                continue
            used.add(k)
            site["keeps"] = entry["keeps"]
    for k in by_key:
        if k not in used:
            problems.append(f"unused annotation {k}")
    return problems


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", action="append", required=True, metavar="REV=PATH")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write", type=pathlib.Path)
    group.add_argument("--check", type=pathlib.Path)
    args = parser.parse_args()
    revisions = []
    for spec in args.root:
        revision, _, path = spec.partition("=")
        revisions.append(census(revision, path))
    result = {
        "schema": "cell-protocol-store-census/1",
        "card": "CELL-PROTOCOL-FB-DECLARATIONS",
        "records": RECORDS,
        "revisions": revisions,
    }
    annotations = json.loads(ANNOTATIONS.read_text()) if ANNOTATIONS.exists() else {"sites": []}
    problems = annotate(result, annotations)
    rendered = json.dumps(result, indent=1, sort_keys=True) + "\n"
    if args.write:
        args.write.write_text(rendered)
    else:
        if args.check.read_text() != rendered:
            problems.append(f"{args.check} differs from the recomputed census")
    for p in problems:
        print(p, file=sys.stderr)
    counts = {r["revision"]: (r["scanned_files"], len(r["sites"])) for r in revisions}
    print(f"census: {counts}; {len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
