#!/usr/bin/env python3
"""accept_as_written.py MODULE -- rewrite a contract module in place as the
human act "accept every held declaration as written" would leave it: every
clause-level `resource_review_pending(CLAUSE, "reason"[, "owner"])` becomes
CLAUSE, every contract-level `requires resource_review_pending(...)` line is
dropped, and every declaration that carried a marker gains
`requires resource_revision(flexible);` so that no pin decides it (the probe
measures the analysis, not the lock). Used only on a scratch copy of the
probe tree (card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL, report §6)."""
import pathlib
import sys

MARK = "resource_review_pending("


def arguments(text: str, start: int):
    """The top-level arguments of the call whose `(` is at `start`, and the
    index just past its `)`."""
    depth, quoted, escape, args, current = 0, False, False, [], []
    for index in range(start, len(text)):
        ch = text[index]
        if quoted:
            current.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                quoted = False
            continue
        if ch == '"':
            quoted = True
            current.append(ch)
        elif ch == "(":
            depth += 1
            if depth > 1:
                current.append(ch)
        elif ch == ")":
            depth -= 1
            if depth == 0:
                args.append("".join(current).strip())
                return args, index + 1
            current.append(ch)
        elif ch == "," and depth == 1:
            args.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    raise SystemExit("unbalanced marker")


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out, marked = [], False
    for line in lines:
        if MARK in line and line.strip().startswith("requires " + MARK):
            marked = True
            continue
        while MARK in line:
            at = line.index(MARK)
            args, end = arguments(line, at + len(MARK) - 1)
            line = line[:at] + args[0] + line[end:]
            marked = True
        if line.strip() == "requires resource_contract_v1;" and marked is not None:
            out.append(line)
            continue
        if line.startswith("};") and marked:
            # close of a declaration that carried a marker: insert the flexible
            # revision just before it
            indent = "    "
            out.append(f"{indent}requires resource_revision(flexible);\n")
            marked = False
        out.append(line)
    path.write_text("".join(out), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
