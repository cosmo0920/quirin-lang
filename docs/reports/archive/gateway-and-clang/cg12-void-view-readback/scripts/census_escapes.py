#!/usr/bin/env python3
"""CG12-VOID-VIEW-READBACK: the loop-owner census's `void *` escapes
(`W47-FIELDS-BY-MEMBER-VOID-ESCAPES`), classified by what CG12 can follow.

usage: census_escapes.py [--json OUT]

Reads the escape labels of W47-PREMISES-INSTANCE's archived premised census
(`fixed-premised.json.gz`; the census code and its typed facts are unchanged
since, and share no reader with the gateway) and each site's line in the
census's export (`private/w47-tail/attempt-1/prep/fluent-bit-clang-source`,
read-only). Classes:

* `discard`: `conv:void|…`, a `(void) x;` statement (Clang `ToVoid`): no
  pointer value exists after it, so there is nothing to follow;
* `member-store`: a `void *` value stored into a member (`O->F = x;`);
* `local`: a `void *` value assigned to or initialising a local;
* `argument`: a `void *` actual of a call.

A `member-store` is followed by rule 11j only when, in the same function,
`O` is a local bound by one call (an admitted store base) and `O->F` is read
back there; the script checks both textually and reports each.
"""

import gzip
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ARCHIVE = HERE.parents[1] / "w47-premises-instance/artifacts/census/fixed-premised.json.gz"
EXPORT = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source")


def find(value, key):
    if isinstance(value, dict):
        if key in value:
            return value[key]
        for inner in value.values():
            found = find(inner, key)
            if found is not None:
                return found
    elif isinstance(value, list):
        for inner in value:
            found = find(inner, key)
            if found is not None:
                return found
    return None


def function_text(lines, line, name):
    """The body of `name` around `line` (from its signature to the next line
    that is a lone `}`)."""
    start = line - 1
    while start > 0 and not re.match(rf"^\S.*\b{re.escape(name)}\s*\(", lines[start]):
        start -= 1
    end = line - 1
    while end < len(lines) and lines[end] != "}":
        end += 1
    return lines[start:end + 1]


def main() -> int:
    census = json.loads(gzip.decompress(ARCHIVE.read_bytes()))
    fields = find(census, "fields_by_member")
    rows = []
    for label in fields["escapes"]:
        _, _, rest = label.partition("struct flb_input_instance ")
        site, function, kind = rest.split(" ")
        path, line = site.rsplit(":", 1)
        line = int(line)
        lines = (EXPORT / path).read_text(errors="replace").split("\n")
        text = lines[line - 1].strip()
        destination, _, context = kind.removeprefix("conv:").partition("|")
        row = {"site": site, "function": function, "kind": kind, "line": text}
        if destination == "void":
            row["class"] = "discard"
        elif context.startswith("arg:"):
            row["class"] = "argument"
        else:
            store = re.match(
                r"^([A-Za-z_]\w*(?:(?:->|\.)\w+)*)(?:->|\.)(\w+)\s*=[^=]", text)
            if store:
                owner, member = store.group(1), store.group(2)
                body = function_text(lines, line, function)
                acquired = bool(re.match(r"^[A-Za-z_]\w*$", owner)) and any(
                    re.search(rf"\b{re.escape(owner)}\s*=\s*\w+\s*\(", other) for other in body)
                reads = sum(
                    1 for other in body
                    if re.search(rf"\b{re.escape(owner)}\s*->\s*{re.escape(member)}\b(?!\s*=[^=])", other))
                row.update({"class": "member-store", "owner": owner, "member": member,
                            "owner_is_a_local_bound_by_a_call": acquired,
                            "reads_back_in_the_function": reads,
                            "followed_by_11j": acquired and reads > 0})
            else:
                row["class"] = "local"
        rows.append(row)
    counts = {}
    for row in rows:
        counts[row["class"]] = counts.get(row["class"], 0) + 1
    followed = sum(1 for row in rows if row.get("followed_by_11j"))
    record = {
        "escapes": len(rows),
        "other_types": fields["other_types"],
        "classes": counts,
        "followed_by_11j": followed,
        "rows": rows,
    }
    print(json.dumps({k: v for k, v in record.items() if k != "rows"}, indent=1))
    for row in rows:
        if row["class"] != "discard":
            print(json.dumps(row))
    if len(sys.argv) > 2 and sys.argv[1] == "--json":
        pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
