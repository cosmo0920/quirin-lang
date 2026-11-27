#!/usr/bin/env python3
"""Classify each external callee of a ranking by owner.

Usage: classify_owners.py RANK_JSON SOURCE_ROOT OUT_JSON

RANK_JSON is `rank_external.py --json` output. SOURCE_ROOT is the pinned Fluent
Bit source (read only). Classes, first match wins:

* `libc-posix`: the name is in `providers/quirin-provider-libc/libc-names.txt`
  (every function a libc6-dev/linux-libc-dev header declares at file scope in
  the pinned units);
* `builtin`: a `__builtin_*`, `__atomic_*` or `__sync_*` compiler builtin, or a
  name the corpus reaches only through a macro expansion of one;
* `bundled-library:<lib>`: declared or defined under `lib/<lib>/` of the source;
* `fluent-bit-internal`: declared or defined under `src/`, `include/` or
  `plugins/` of the source, but in no corpus unit;
* `unresolvable`: no declaration found by the textual search below.

The search is textual: a line where the name is followed by `(` and preceded by
a type-ish token, not ending in `;` for a call. It records the first file that
matches as evidence; it proves ownership only together with a reading.
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    rows = json.loads(pathlib.Path(sys.argv[1]).read_text())
    source = pathlib.Path(sys.argv[2])
    libc = set(
        line.strip()
        for line in (ROOT / "providers/quirin-provider-libc/libc-names.txt").read_text().splitlines()
        if line.strip() and not line.startswith("#")
    )
    owners = {}
    pending = []
    for row in rows:
        name = row["callee"]
        if name in libc:
            owners[name] = {"owner": "libc-posix", "evidence": "providers/quirin-provider-libc/libc-names.txt"}
        elif re.match(r"__(builtin|atomic|sync)_", name):
            owners[name] = {"owner": "builtin", "evidence": "compiler builtin spelling"}
        else:
            pending.append(name)
    found = {name: [] for name in pending}
    if pending:
        alternation = "|".join(sorted((re.escape(n) for n in pending), key=len, reverse=True))
        decl = re.compile(
            rf"^(?:[A-Za-z_][A-Za-z0-9_ \t\*]*[ \t\*])?({alternation})[ \t]*\((?!.*;[ \t]*$)|"
            rf"^[A-Za-z_][A-Za-z0-9_ \t\*]*[ \t\*]({alternation})[ \t]*\(.*\)[^;]*;|"
            rf"^#[ \t]*define[ \t]+({alternation})\b",
            re.M,
        )
        for path in sorted(source.rglob("*")):
            if path.suffix not in (".c", ".h") or not path.is_file():
                continue
            text = path.read_text(errors="replace")
            for match in decl.finditer(text):
                name = match.group(1) or match.group(2) or match.group(3)
                found[name].append(str(path.relative_to(source)))
    for name in pending:
        rel = sorted(set(found[name]))
        if not rel:
            owners[name] = {"owner": "unresolvable", "evidence": "no declaration found"}
            continue
        lib = [r for r in rel if r.startswith("lib/")]
        if lib:
            parts = lib[0].split("/")
            owners[name] = {"owner": f"bundled-library:{parts[1]}", "evidence": lib[0]}
        else:
            owners[name] = {"owner": "fluent-bit-internal", "evidence": rel[0]}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(owners, indent=1, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
