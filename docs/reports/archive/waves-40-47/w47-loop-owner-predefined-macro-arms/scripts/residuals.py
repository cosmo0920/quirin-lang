#!/usr/bin/env python3
"""Size this card's registered residuals at the 90978ac6 export. Read-only.

Usage: residuals.py PREDEFINED_FACTS.json

* multi-command owned macros: census files (src, plugins, lib/monkey,
  include; not src/fluent-bit.c) compiled by more than one command whose
  `FLB_`-prefixed `-D` sets differ, and the differing names each file's
  conditional directives test (the configured reading intersects a file's
  commands' sets, so such a name is decided undefined there);
* failed derivations: the facts' failed commands, by directory and first
  reason;
* value comparisons: conditional directives of census files that compare a
  reserved macro's value (`__GNUC__ >= 4`), which the reading leaves
  undecided.
"""

import collections
import json
import pathlib
import re
import shlex
import sys

PREP = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep")
EXPORT = PREP / "fluent-bit-clang-source"
DATABASE = PREP / "fluent-bit-clang-production/compile_commands.json"
PATHS = ("src/", "plugins/", "lib/monkey/", "include/")
CONDITION = re.compile(r"^\s*#\s*(if|elif|ifdef|ifndef)\b(.*)$")
COMPARISON = re.compile(r"\b(__[A-Za-z0-9_]+|_[A-Z][A-Za-z0-9_]*)\s*(>=|<=|==|!=|>|<)")


def census_file(relative):
    return relative.startswith(PATHS) and relative != "src/fluent-bit.c"


def main():
    facts = json.loads(pathlib.Path(sys.argv[1]).read_text())
    by_file = collections.defaultdict(list)
    for entry in json.loads(DATABASE.read_text()):
        relative = str(pathlib.Path(entry["file"]).relative_to(EXPORT)) \
            if entry["file"].startswith(str(EXPORT)) else None
        if relative and census_file(relative):
            words = shlex.split(entry["command"])
            by_file[relative].append({w[2:].split("=")[0] for w in words
                                      if w.startswith("-DFLB_")})
    print("== multi-command owned macros")
    for relative, sets in sorted(by_file.items()):
        if len(sets) < 2:
            continue
        differing = set.union(*sets) - set.intersection(*sets)
        if not differing:
            continue
        text = (EXPORT / relative).read_text(errors="replace")
        tested = sorted(name for name in differing
                        if re.search(rf"^\s*#\s*(if|elif|ifdef|ifndef).*\b{name}\b", text, re.M))
        print(f"{relative}: {len(sets)} commands; differing {sorted(differing)}; tested here {tested}")
    print("== failed derivations")
    failed = [c for c in facts["commands"] + facts["extras"] if c["failure"]]
    for command in failed:
        print(f"{command['key']}: {command['failure'][:160]}")
    print(f"failed: {len(failed)}")
    print("== value comparisons of reserved macros in census files")
    count = 0
    for path in sorted(EXPORT.rglob("*")):
        if path.suffix not in (".c", ".h") or path.is_symlink() or not path.is_file():
            continue
        relative = str(path.relative_to(EXPORT))
        if not census_file(relative) and relative != "tests/runtime/in_tail.c":
            continue
        for number, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
            match = CONDITION.match(line)
            if match and COMPARISON.search(match.group(2)):
                count += 1
    print(f"directives: {count}")


if __name__ == "__main__":
    main()
