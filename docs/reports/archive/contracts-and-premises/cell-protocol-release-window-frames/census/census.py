#!/usr/bin/env python3
"""Release-window call census (card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES;
gateway rule 38g, grammar G05k).

Usage:
    census.py --root REV=PATH [--root REV=PATH ...] --write census.json
    census.py --root REV=PATH [--root REV=PATH ...] --check census.json

For each Fluent Bit tree (read only; a checkout of the named revision) and
each destroyer of a record with declared per-end sentinels, the RELEASE
WINDOW is the text from the destroyer's first close of a declared end
(``mk_event_closesocket(REC->END)``) to the holder's release
(``flb_free(REC)``). Every call in it is listed with its line, its callee
and its statement. Each callee carries exactly one annotation in
``annotations.json``: whether the audited routes contract it, the frame
production declares for it over the record (``checked``, ``declared`` or
``none``) with the body read, and what rule 38g makes of the call.

``--check`` recomputes everything from the trees and refuses (exit 1) a census
file that differs, an unannotated callee, an unused annotation or a changed
file digest. The reader is lexical (comments and strings blanked); it is a
census tool, not an analyzer: the annotations are the reading.
"""

import argparse
import hashlib
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ANNOTATIONS = HERE / "annotations.json"

# Destroyer, its file, its record parameter, the record, and the ends whose
# close opens the window.
DESTROYERS = [
    ("flb_output_instance_destroy", "src/flb_output.c", "ins", "flb_output_instance",
     ["ch_events"]),
    ("flb_input_instance_destroy", "src/flb_input.c", "ins", "flb_input_instance",
     ["ch_events", "ingress_queue_channels"]),
    ("input_thread_instance_destroy", "src/flb_input_thread.c", "thi",
     "flb_input_thread_instance", ["ch_parent_events", "ch_thread_events"]),
]

NOT_CALLS = {"if", "while", "for", "switch", "return", "sizeof", "mk_list_foreach_safe",
             "mk_list_foreach", "mk_list_entry"}


def blank(text):
    """Comments and string/char literals blanked, line structure kept."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if text.startswith("/*", i):
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
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


def function_body(text, name):
    """(start offset, end offset) of `name`'s definition body."""
    m = re.search(r"^[A-Za-z_][\w \*]*\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", text, re.M)
    if not m:
        return None
    depth = 0
    for k in range(m.end() - 1, len(text)):
        if text[k] == "{":
            depth += 1
        elif text[k] == "}":
            depth -= 1
            if depth == 0:
                return m.end() - 1, k + 1
    return None


def statement_of(text, at):
    """The text of the statement holding offset `at` (to its `;`)."""
    start = max(text.rfind(";", 0, at), text.rfind("{", 0, at), text.rfind("}", 0, at)) + 1
    end = text.find(";", at)
    return " ".join(text[start:end + 1].split())


def census(roots):
    result = {"trees": {}, "windows": []}
    for rev, root in roots:
        digests = {}
        for name, path, param, record, ends in DESTROYERS:
            raw = (pathlib.Path(root) / path).read_bytes()
            digests[path] = hashlib.sha256(raw).hexdigest()
            text = blank(raw.decode("utf-8", "replace"))
            span = function_body(text, name)
            if span is None:
                result["windows"].append({"tree": rev, "destroyer": name, "error": "no body"})
                continue
            body_start, body_end = span
            body = text[body_start:body_end]
            closes = [m for m in re.finditer(
                r"mk_event_closesocket\(\s*" + re.escape(param) + r"->(" + "|".join(ends) +
                r")\[\d\]\s*\)", body)]
            free = re.search(r"\bflb_free\(\s*" + re.escape(param) + r"\s*\)", body)
            if not closes or not free:
                result["windows"].append({"tree": rev, "destroyer": name,
                                          "error": "no close or no release"})
                continue
            open_at = closes[0].start()
            calls = []
            for m in re.finditer(r"\b([A-Za-z_]\w*)\s*\(", body[open_at:free.end()]):
                callee = m.group(1)
                if callee in NOT_CALLS:
                    continue
                at = body_start + open_at + m.start()
                calls.append({
                    "line": text.count("\n", 0, at) + 1,
                    "callee": callee,
                    "statement": statement_of(text, at),
                })
            result["windows"].append({
                "tree": rev,
                "destroyer": name,
                "record": record,
                "opens": text.count("\n", 0, body_start + open_at) + 1,
                "release": text.count("\n", 0, body_start + free.start()) + 1,
                "calls": calls,
            })
        result["trees"][rev] = digests
    return result


def annotate(result, annotations):
    problems = []
    used = set()
    for window in result["windows"]:
        for call in window.get("calls", []):
            key = f"{window['record']}:{call['callee']}"
            note = annotations.get(key)
            if note is None:
                problems.append(f"unannotated callee {key} ({window['tree']}:{call['line']})")
                continue
            used.add(key)
            call.update(note)
    for key in sorted(set(annotations) - used):
        problems.append(f"unused annotation {key}")
    return problems


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", action="append", required=True)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write")
    group.add_argument("--check")
    args = parser.parse_args()
    roots = [tuple(r.split("=", 1)) for r in args.root]
    annotations = json.loads(ANNOTATIONS.read_text())
    result = census(roots)
    problems = annotate(result, annotations)
    errors = [w for w in result["windows"] if "error" in w]
    problems += [f"{w['tree']} {w['destroyer']}: {w['error']}" for w in errors]
    text = json.dumps(result, indent=1, sort_keys=True) + "\n"
    if args.write:
        pathlib.Path(args.write).write_text(text)
    else:
        known = pathlib.Path(args.check).read_text()
        if known != text:
            problems.append(f"{args.check} differs from the trees' census")
    windows = [w for w in result["windows"] if "calls" in w]
    print(f"{len(roots)} trees, {len(windows)} windows, "
          f"{sum(len(w['calls']) for w in windows)} calls, {len(problems)} problems")
    for problem in problems:
        print("  " + problem)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
