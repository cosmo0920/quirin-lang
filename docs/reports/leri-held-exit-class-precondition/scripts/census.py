#!/usr/bin/env python3
"""census.py TREE -- FB-RETRY-OR6-ELEMENT-IDENTITY: every function in TREE's
src/ and plugins/ that hands one of its own formals, as written, to an
element destroyer naming a link member (the bodies gateway rule 91 can
summarise). A lexical reading: a top-level function definition's parameter
names, then each `DESTROYER(NAME)` call in its body whose NAME is one."""
import json
import pathlib
import re
import sys

DESTROYERS = [
    "flb_input_chunk_destroy", "flb_input_coro_destroy", "flb_input_instance_destroy",
    "flb_output_flush_destroy", "flb_output_instance_destroy", "flb_sched_request_destroy",
    "flb_sched_timer_destroy", "flb_tail_file_remove", "flb_task_destroy",
    "flb_task_retry_destroy", "mk_http_thread_destroy",
]
CALL = re.compile(r"\b(" + "|".join(DESTROYERS) + r")\s*\(\s*([A-Za-z_]\w*)\s*[,)]")
DEF = re.compile(r"^[A-Za-z_][\w \*]*?\b([A-Za-z_]\w*)\s*\(([^;{)]*)\)\s*\{", re.M)


def functions(text):
    for match in DEF.finditer(text):
        name, params = match.group(1), match.group(2)
        if name in ("if", "for", "while", "switch"):
            continue
        start = match.end() - 1
        depth, at = 0, start
        while at < len(text):
            if text[at] == "{":
                depth += 1
            elif text[at] == "}":
                depth -= 1
                if depth == 0:
                    break
            at += 1
        names = [re.findall(r"([A-Za-z_]\w*)\s*(?:\[[^\]]*\])?\s*$", p.strip())
                 for p in params.split(",")]
        yield name, [n[0] for n in names if n], text[start:at + 1], text.count("\n", 0, start) + 1


def main() -> int:
    tree = pathlib.Path(sys.argv[1])
    rows = []
    for path in sorted(list(tree.glob("src/**/*.c")) + list(tree.glob("plugins/**/*.c"))):
        text = path.read_text(errors="replace")
        for name, params, body, line in functions(text):
            for call in CALL.finditer(body):
                if call.group(2) in params:
                    rows.append({"file": str(path.relative_to(tree)), "function": name,
                                 "line": line, "destroyer": call.group(1),
                                 "formal": call.group(2)})
    print(json.dumps(rows, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
