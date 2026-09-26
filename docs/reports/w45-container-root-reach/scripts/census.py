#!/usr/bin/env python3
"""W45-CONTAINER-ROOT-REACH census: every `mk_list_add` / `mk_list_del` call
site in the audited units, classified by the container's root shape (a
publication) or by how the unlinked entry was reached (an unlink), with the
enclosing function's status and the leaves at the call's line.

usage: census.py corpus DIAGNOSTICS(.json|.json.gz) OUT.json
       census.py slice  REPORT.json SOURCE_ROOT OUT.json

The classification is lexical (the reader below) and is checked by reading;
`reading` notes in the report record every site whose reading differs.
"""
import gzip
import json
import re
import sys

CORPUS_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/"
CORPUS_FILES = {
    "cio_chunk.c": "lib/chunkio/src/cio_chunk.c",
    "cio_stream.c": "lib/chunkio/src/cio_stream.c",
    "chunkio.c": "lib/chunkio/src/chunkio.c",
    "flb_input_chunk.c": "src/flb_input_chunk.c",
}
for _name in ["mk_rconf.c", "mk_string.c", "mk_event.c"]:
    CORPUS_FILES["mk_core/" + _name] = "lib/monkey/mk_core/" + _name
for _name in ["mk_http.c", "mk_http_thread.c", "mk_vhost.c", "mk_config.c", "mk_scheduler.c",
              "mk_http_parser.c", "mk_lib.c", "mk_plugin.c", "mk_fifo.c", "mk_server.c",
              "mk_mimetype.c"]:
    CORPUS_FILES["mk_server/" + _name] = "lib/monkey/mk_server/" + _name

CALL = re.compile(r"\b(mk_list_add|mk_list_del)\s*\(")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def arguments(text, start):
    depth = 0
    args, current = [], ""
    for ch in text[start:]:
        if ch == "(":
            depth += 1
            if depth == 1:
                continue
        elif ch == ")":
            depth -= 1
            if depth == 0:
                args.append(" ".join(current.split()))
                return args
        elif ch == "," and depth == 1:
            args.append(" ".join(current.split()))
            current = ""
            continue
        if depth >= 1:
            current += ch
    return args


def functions(lines):
    """(name, header_start, body_start, body_end) of every column-0 body."""
    found = []
    index = 0
    while index < len(lines):
        same_line = (not lines[index].startswith((" ", "\t", "#", "/", "*", "}"))
                     and re.search(r"\)\s*\{\s*$", lines[index]) is not None
                     and re.search(r"\b(if|for|while|switch|else)\b", lines[index]) is None)
        if lines[index].startswith("{") or same_line:
            start = index
            depth = 0
            end = None
            for at in range(index, len(lines)):
                depth += lines[at].count("{") - lines[at].count("}")
                if depth == 0:
                    end = at
                    break
            header = None
            name = None
            for back in range(start - (0 if same_line else 1), max(start - 10, -1), -1):
                match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)\s*\(", lines[back])
                if match and not lines[back].startswith((" ", "\t")):
                    name, header = match.group(1), back
                    break
            if name and end is not None:
                found.append((name, header, start, end))
            index = (end or index) + 1
        else:
            index += 1
    return found


def formals(lines, header, body_start):
    text = " ".join(lines[header:body_start + 1])
    if "{" in text:
        text = text[:text.rfind("{")]
    open_at = text.find("(")
    close_at = text.rfind(")")
    params = text[open_at + 1:close_at]
    names = []
    for part in params.split(","):
        idents = IDENT.findall(part.split("[")[0])
        if idents and idents[-1] not in ("void",):
            names.append(idents[-1])
    return names


def rebound(body, name):
    return bool(re.search(r"(?<![>.\w])" + re.escape(name) + r"\s*(=[^=]|\+\+|--|[-+*/|&^]=)", body)) or \
        bool(re.search(r"&\s*" + re.escape(name) + r"\b(?!\s*(->|\.|\[))", body))


def root_of(spelling):
    spelling = spelling.strip()
    while spelling.startswith("(") and spelling.endswith(")"):
        spelling = spelling[1:-1].strip()
    address = spelling.startswith("&")
    spelling = spelling.lstrip("&").strip().lstrip("(").strip()
    match = IDENT.match(spelling)
    return (match.group(0) if match else None), address, spelling


def declaration(body, name):
    """The local declaration line of `name` in `body`, if any."""
    pattern = re.compile(r"^\s*(?:const\s+|static\s+)?(?:struct\s+|enum\s+|union\s+)?[A-Za-z_][A-Za-z0-9_]*[\s\*]+" +
                         re.escape(name) + r"\s*(=|;|\[|,)", re.M)
    match = pattern.search(body)
    return match


def local_origin(body, name):
    """How a local pointer name gets its value: the right-hand sides of its writes."""
    values = []
    for match in re.finditer(r"(?<![>.\w])" + re.escape(name) + r"\s*=\s*([^;=][^;]*);", body):
        values.append(" ".join(match.group(1).split()))
    return values


def classify_root(root, address, spelling, body, params, file_text, published_names, returned_names):
    if root is None:
        return "other"
    if root in params:
        return "rebound-formal" if rebound(body, root) else "formal"
    decl = declaration(body, root)
    if decl is None:
        if re.search(r"^\S.*\b" + re.escape(root) + r"\b[^(]*[;=]", file_text, re.M):
            return "global"
        return "other"
    values = local_origin(body, root)
    if not values:
        # A declared local with no write: a local object (`struct mk_list L;`)
        # when its address is taken, otherwise uninitialised.
        return "local-object" if address and not spelling.startswith(root + " ->") \
            and "->" not in spelling.split(root, 1)[1][:4] else "local-unwritten"
    kinds = set()
    for value in values:
        head = IDENT.match(value.lstrip("(&*").strip())
        first = head.group(0) if head else ""
        if re.match(r"^\(?\s*\{", value) or "mk_list_entry" in value or "container_of" in value \
                or "__mptr" in value:
            kinds.add("element")
        elif first in params and not rebound(body, first):
            kinds.add("derived-formal")
        elif re.match(r"^[A-Za-z_][A-Za-z0-9_]*\s*\(", value) or re.match(r"^\(.*\)\s*[A-Za-z_]\w*\s*\(", value):
            callee = re.findall(r"([A-Za-z_]\w*)\s*\(", value)
            callee = [c for c in callee if c not in ("struct",)]
            name = callee[0] if callee else "?"
            if any(word in name for word in ("alloc", "calloc", "malloc", "create", "new", "dup")):
                kinds.add("body-acquired")
            else:
                kinds.add("callee-result")
        elif value in ("NULL", "0"):
            kinds.add("null")
        else:
            kinds.add("other-local")
    kinds.discard("null")
    if kinds == {"body-acquired"}:
        if root in published_names:
            return "owner-published"
        if root in returned_names:
            return "owner-returned"
        return "owner-local"
    if len(kinds) == 1:
        return kinds.pop()
    return "mixed:" + ",".join(sorted(kinds))


def entry_reach(entry, body, params, file_text):
    root, _, spelling = root_of(entry)
    if root is None:
        return "other", None
    if spelling.startswith(root + " [") or re.search(re.escape(root) + r"\s*=\s*&", body):
        pass
    if root in params:
        return ("rebound-formal" if rebound(body, root) else "formal-entry"), None
    values = local_origin(body, root)
    decl = declaration(body, root)
    if decl is None:
        return "global-entry", None
    reach = set()
    heads = []
    for value in values:
        if value in ("NULL", "0"):
            continue
        entry_match = re.match(r"mk_list_entry\s*\(\s*(\w+)\s*,", value)
        first_match = re.match(r"mk_list_entry_(first|last)\s*\((.*),[^,]*,[^,]*\)$", value)
        if entry_match:
            cursor = entry_match.group(1)
            loop = re.search(r"mk_list_foreach(_safe)?\s*\(\s*" + re.escape(cursor) + r"\s*,(?:\s*\w+\s*,)?\s*([^)]*(?:\([^)]*\))?[^)]*)\)", body)
            if loop:
                reach.add("iteration-safe" if loop.group(1) else "iteration-plain")
                heads.append(" ".join(loop.group(2).split()))
            else:
                reach.add("container_of-other")
        elif first_match:
            reach.add("lookup-first")
            heads.append(" ".join(first_match.group(2).split()))
        elif value.startswith("&"):
            reach.add("embedded")
        elif re.match(r"^\(?[\w\s\*]*\)?\s*[A-Za-z_]\w*\s*\(", value):
            reach.add("callee-result")
        else:
            reach.add("other-local")
    if not reach:
        return "unwritten-local", None
    return ",".join(sorted(reach)), heads


def census(unit_files, source_root, report, out):
    status, leaves = {}, {}
    for unit in report["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            status.setdefault(key, set()).add(function["status"])
            for leaf in function["diagnostics"]:
                leaves.setdefault((unit["file"], function["function"], leaf["origin"]), set()).add(
                    f"{leaf['category']}: {leaf['description']}")
    sites = []
    for unit, path in unit_files.items():
        file_text = open(source_root + path, errors="replace").read()
        lines = file_text.split("\n")
        for name, header, start, end in functions(lines):
            body = "\n".join(lines[start:end + 1])
            params = formals(lines, header, start)
            published = set()
            returned = set()
            for match in CALL.finditer(body):
                args = arguments(body, match.end() - 1)
                if match.group(1) == "mk_list_add" and args:
                    root, _, _ = root_of(args[0])
                    if root:
                        published.add(root)
            for match in re.finditer(r"return\s+([A-Za-z_]\w*)\s*;", body):
                returned.add(match.group(1))
            for match in CALL.finditer(body):
                line = start + body.count("\n", 0, match.start()) + 1
                stripped = lines[line - 1].lstrip()
                if stripped.startswith(("*", "//", "#")):
                    continue
                args = arguments(body, match.end() - 1)
                origin = f"{unit}:{line}"
                site = {
                    "unit": unit,
                    "line": line,
                    "function": name,
                    "call": match.group(1),
                    "arguments": args,
                    "status": sorted(status.get((unit, name), {"not reported"})),
                    "leaves": sorted(leaves.get((unit, name, origin), set())),
                }
                if match.group(1) == "mk_list_add" and len(args) == 2:
                    root, address, spelling = root_of(args[1])
                    site["container_root"] = root
                    site["shape"] = classify_root(root, address, spelling, body, params, file_text,
                                                  published, returned)
                    entry_root, _, _ = root_of(args[0])
                    site["entry_reach"], _ = entry_reach(args[0], body, params, file_text)
                elif match.group(1) == "mk_list_del" and args:
                    reach, heads = entry_reach(args[0], body, params, file_text)
                    site["entry_reach"] = reach
                    if heads:
                        site["heads"] = heads
                        shapes = []
                        for head in heads:
                            root, address, spelling = root_of(head)
                            shapes.append(classify_root(root, address, spelling, body, params,
                                                        file_text, published, returned))
                        site["head_shape"] = shapes
                sites.append(site)
    json.dump(sites, open(out, "w"), indent=1)
    for site in sites:
        extra = site.get("shape") or site.get("head_shape") or ""
        print(f"{site['unit']}:{site['line']} {site['function']} {site['status']} "
              f"{site['call']}({', '.join(site['arguments'])}) :: {site.get('entry_reach')} {extra}")
        for leaf in site["leaves"]:
            print("      ", leaf)


def main():
    mode = sys.argv[1]
    if mode == "corpus":
        census(CORPUS_FILES, CORPUS_ROOT, load(sys.argv[2]), sys.argv[3])
    else:
        report = load(sys.argv[2])
        files = {unit["file"]: unit["file"] for unit in report["units"]}
        census(files, sys.argv[3].rstrip("/") + "/", report, sys.argv[4])


main()
