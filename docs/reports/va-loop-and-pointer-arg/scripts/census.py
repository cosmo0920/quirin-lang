#!/usr/bin/env python3
"""VA-LOOP-AND-POINTER-ARG census over one `resource-diagnostics.json`.

Usage: census.py REPORT SOURCE-ROOT [SOURCE-ROOT ...] OUT.json

Every function of REPORT with a leaf whose original source line (its `origin`,
resolved under the source roots by path suffix) spells `va_arg`, or whose
description is one of the variadic families, is listed with its status and
every leaf. Each `va_arg` leaf is classified by its description and its source
line:

  condition-assign   a condition refusal on a line `while|if|for (... (NAME = va_arg(...)) ...`
                     whose whole controlling expression is `(NAME = va_arg(L, T))`
  condition-other    a condition refusal on a line spelling `va_arg` in another shape
  pointer-store      `va_list operation refused: the va_arg value stored into NAME ...`
  other-expression   `call to uncontracted function __builtin_va_arg` (va_arg inside
                     another expression: a member or element store, a cast, an actual)
  protocol           `va_list protocol refused: ...`

A function is a *candidate* when every leaf it has is condition-assign or
pointer-store (the shapes this card lowers) and it has at least one; whether it
moves still depends on the protocol and on what the lowered statements meet
downstream, which the AFTER measures. Nothing is piped; the JSON is the record.
"""

import collections
import json
import pathlib
import re
import sys

CONDITION = re.compile(
    r"^unmodeled call, aggregate or indirect access in (loop|branch|for-loop|do-while) condition$"
)
POINTER = re.compile(r"^va_list operation refused: the `va_arg` value stored into `([^`]+)`")
VA_ARG_LEAF = "call to uncontracted function `__builtin_va_arg`"
PROTOCOL = "va_list protocol refused:"
ASSIGN = re.compile(
    r"\b(while|if)\s*\(\s*\(\s*([A-Za-z_]\w*)\s*=\s*va_arg\s*\([^()]*\)\s*\)\s*\)"
)


class Sources:
    def __init__(self, roots):
        self.roots = [pathlib.Path(root) for root in roots]
        self.cache = {}

    def line(self, origin):
        if not origin or ":" not in origin:
            return None
        path, _, number = origin.rpartition(":")
        try:
            number = int(number)
        except ValueError:
            return None
        lines = self.cache.get(path)
        if lines is None:
            lines = []
            for root in self.roots:
                candidate = root / path
                if candidate.is_file():
                    lines = candidate.read_text(errors="replace").splitlines()
                    break
            if not lines:
                matches = []
                for root in self.roots:
                    matches.extend(root.rglob(pathlib.Path(path).name))
                matches = sorted({m.resolve() for m in matches if str(m).endswith(path)})
                if len(matches) == 1:
                    lines = matches[0].read_text(errors="replace").splitlines()
            self.cache[path] = lines
        if 0 < number <= len(lines):
            return lines[number - 1]
        return None


def classify(description, text):
    if CONDITION.match(description) and text and "va_arg" in text:
        return "condition-assign" if ASSIGN.search(text) else "condition-other"
    if POINTER.match(description):
        return "pointer-store"
    if description == VA_ARG_LEAF:
        return "other-expression"
    if description.startswith(PROTOCOL):
        return "protocol"
    return None


def main():
    report_path, *roots, out = sys.argv[1:]
    report = json.loads(pathlib.Path(report_path).read_text())
    sources = Sources(roots)
    rows = []
    classes = collections.Counter()
    class_functions = collections.defaultdict(set)
    for unit in report["units"]:
        for entry in unit.get("functions") or []:
            key = f"{unit.get('library', '')}/{unit['file']}/{entry['function']}"
            leaves = []
            relevant = False
            for diagnostic in entry.get("diagnostics") or []:
                description = diagnostic.get("description", "")
                origin = diagnostic.get("origin")
                text = sources.line(origin)
                kind = classify(description, text)
                if kind is None and text and "va_arg" in text:
                    kind = "va_arg-line-other"
                if kind:
                    relevant = True
                    classes[kind] += 1
                    class_functions[kind].add(key)
                leaves.append(
                    {
                        "line": diagnostic.get("line"),
                        "origin": origin,
                        "description": description,
                        "class": kind,
                        "source": text.strip() if (kind and text) else None,
                    }
                )
            if relevant:
                kinds = [leaf["class"] for leaf in leaves]
                rows.append(
                    {
                        "function": key,
                        "status": entry["status"],
                        "leaves": leaves,
                        "candidate": all(k in ("condition-assign", "pointer-store") for k in kinds)
                        and bool(kinds),
                    }
                )
    statuses = collections.Counter(
        entry["status"] for unit in report["units"] for entry in unit.get("functions") or []
    )
    result = {
        "report": report_path,
        "statuses": dict(statuses),
        "leaves_by_class": dict(classes),
        "functions_by_class": {k: len(v) for k, v in class_functions.items()},
        "candidates": [row["function"] for row in rows if row["candidate"]],
        "functions": rows,
    }
    pathlib.Path(out).write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: result[k] for k in ("statuses", "leaves_by_class", "functions_by_class", "candidates")}, indent=1))


if __name__ == "__main__":
    main()
