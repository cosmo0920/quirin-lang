#!/usr/bin/env python3
"""Card ONIGMO-PROVIDER-PLUGIN: find where a C file's conditional groups
close a brace opened before the group, the condition under which
`crates/qcore-audit/src/c_identity.rs` (`structure`, `unbalanced`) runs every
body of the file to its end and resumes its site scan only after that end.

    python3 -I conditional_balance.py FILE...

A rough lexical reproduction of `structure` (comments, strings and
character literals removed; `#if`/`#ifdef`/`#ifndef` open a group, `#elif`/
`#else` restart from the group's opening depth, `#endif` resumes from the end
of the first branch). Prints each file's first offending `}` line, or
`balanced`. Evidence only; the Rust scanner is the authority.
"""
import re
import sys


def strip(text):
    pattern = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\\n])*"|\'(?:\\.|[^\'\\\n])*\'', re.S)
    return pattern.sub(lambda m: re.sub(r"[^\n]", " ", m.group(0)), text)


def logical_lines(text):
    """Physical lines joined at a trailing backslash (a directive's
    continuation is part of the directive), each with its first line's
    number."""
    number, pending, start = 0, "", None
    for physical in text.splitlines():
        number += 1
        if start is None:
            start = number
        if physical.endswith("\\"):
            pending += physical[:-1] + " "
            continue
        yield start, pending + physical
        pending, start = "", None
    if pending:
        yield start, pending


def first_unbalanced(text):
    open_stack, groups = [], []
    for number, line in logical_lines(strip(text)):
        directive = re.match(r"\s*#\s*(\w+)", line)
        if directive:
            name = directive.group(1)
            if name in ("if", "ifdef", "ifndef"):
                groups.append([list(open_stack), None])
            elif name in ("elif", "else", "elifdef", "elifndef") and groups:
                if groups[-1][1] is None:
                    groups[-1][1] = list(open_stack)
                open_stack = list(groups[-1][0])
            elif name == "endif" and groups:
                group = groups.pop()
                if group[1] is not None:
                    open_stack = group[1]
            continue
        for character in line:
            if character == "{":
                open_stack.append(number)
            elif character == "}":
                if any(len(open_stack) <= len(group[0]) for group in groups):
                    return number
                if open_stack:
                    open_stack.pop()
    return None


for path in sys.argv[1:]:
    with open(path, encoding="utf-8", errors="replace") as handle:
        line = first_unbalanced(handle.read())
    print(f"{path}: {'balanced' if line is None else f'first unbalanced close at line {line}'}")
