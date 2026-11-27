"""census.py REPORT SOURCE_ROOT PREFIX -- OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY:
every leaf of one opt-in `current` report whose description starts with
PREFIX, grouped by row, with the source line at its origin and its shape
(`star`: the line spells `*NAME = …`; `other`: anything else)."""
import collections
import json
import re
import sys


def main() -> int:
    report, root, prefix = sys.argv[1], sys.argv[2].rstrip("/") + "/", sys.argv[3]
    data = json.load(open(report))
    cache = {}

    def text(origin):
        path, number = origin.rsplit(":", 1)
        if path not in cache:
            try:
                cache[path] = open(root + path, errors="replace").read().split("\n")
            except OSError:
                cache[path] = []
        lines = cache[path]
        index = int(number) - 1
        return lines[index].strip() if 0 <= index < len(lines) else "?"

    rows = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit["functions"]:
            for leaf in function["diagnostics"]:
                if leaf["description"].startswith(prefix):
                    rows[(unit["file"], function["function"], function["status"])].append(
                        (leaf["origin"], leaf["description"], text(leaf["origin"]))
                    )
    shapes = collections.Counter()
    for (path, name, status), leaves in sorted(rows.items()):
        print(f"{path} {name} {status}")
        for origin, description, line in leaves:
            shape = "star" if re.match(r"^\*\s*[A-Za-z_]\w*\s*=", line) else "other"
            shapes[shape] += 1
            print(f"    {shape} {origin} | {description} | {line[:120]}")
    print(f"{len(rows)} rows, {sum(len(v) for v in rows.values())} leaves, {dict(shapes)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
