#!/usr/bin/env python3
"""Attribute every difference between a no-build run and the configured run.

Usage: compare.py CONFIGURED_REPORT NO_BUILD_REPORT FLUENT_BIT PREPARED_OUT OUT.json

Units are matched by their checkout-relative `file`. Every unit of either run
lands in exactly one category:

* `no-build-only:not-compiled`: walked by the no-build route, but the
  configured compilation database has no command for it;
* `no-build-only:outside-configured-selection`: compiled by the configured
  build, but outside the profile's library groups the configured run audits;
* `configured-only`: in the configured run only (none is expected);
* `not-prepared:<reason>`: no-build refused preparation (reason code), with
  the configured verdict beside it; a missing include is further attributed
  to the header it names and where that header exists (`source-tree` =
  tracked in the checkout but not on the supplied include path, `untracked` =
  only an untracked or ignored checkout file, `generated` = only in the
  configured build tree, `absent` = none of these);
* `prepared:same-text` / `prepared:different-text`: both prepared, with equal
  or different preprocessed-text digests, and whether the unit verdicts agree.

Function-level rows are compared for every unit both runs analysed. A
*favourable* difference is a no-build `Safe` (unit or function) where the
configured run is not `Safe`; each is listed with its cause.
"""

import collections
import json
import os
import pathlib
import re
import subprocess
import sys

HEADER = re.compile(r"'([^']+)' file not found")


def load(path):
    return json.loads(pathlib.Path(path).read_text())


def functions(row):
    statuses = {entry["function"]: entry["status"] for entry in row.get("functions", [])}
    for name in row.get("not_projected", []):
        statuses.setdefault(name, "NotProjected")
    return statuses


def digest(row):
    identity = row.get("compile_configuration", {}).get("compile_task_identity") or {}
    return identity.get("preprocessed_source_sha256")


def tree_index(root, skip_top=()):
    """Every file path under root, `/`-prefixed, skipping dot-directories and
    the named top-level directories (a checkout's own `build/`)."""
    index = []
    for directory, subdirectories, files in os.walk(root):
        relative = pathlib.Path(directory).relative_to(root)
        subdirectories[:] = [
            name for name in subdirectories
            if not name.startswith(".") and not (relative == pathlib.Path(".") and name in skip_top)
        ]
        index += ["/" + (relative / name).as_posix() for name in files]
    return index


def header_location(header, includer, source_index, tracked, build_index):
    """Where a missing header exists: `source-tree` (tracked in the checkout),
    `untracked` (only as an untracked or ignored file in the checkout, such as
    a header an earlier in-tree configure generated), `generated` (only in the
    configured build tree) or `absent`. A spelling relative to its includer
    (`../config.h`) names one checkout path, resolved against the includer's
    directory; the build tree is searched for its remaining path."""
    if header.startswith(("../", "./")):
        resolved = "/" + os.path.normpath(os.path.join(os.path.dirname(includer), header))
        if resolved in tracked:
            return "source-tree"
        if resolved in source_index:
            return "untracked"
        while header.startswith(("../", "./")):
            header = header.split("/", 1)[1]
        suffix = "/" + header
        return "generated" if any(path.endswith(suffix) for path in build_index) else "absent"
    suffix = "/" + header
    if any(path.endswith(suffix) for path in tracked):
        return "source-tree"
    if any(path.endswith(suffix) for path in source_index):
        return "untracked"
    if any(path.endswith(suffix) for path in build_index):
        return "generated"
    return "absent"


def main() -> int:
    configured_path, no_build_path, fluent_bit, prepared, out = sys.argv[1:6]
    fluent_bit, prepared = pathlib.Path(fluent_bit), pathlib.Path(prepared)
    source_index = tree_index(fluent_bit, skip_top=("build",))
    build_index = tree_index(prepared / "fluent-bit-clang-production")
    tracked = ["/" + path for path in subprocess.run(
        ["git", "-C", str(fluent_bit), "ls-files", "-z"], check=True, capture_output=True, text=True
    ).stdout.split("\0") if path]
    clone = prepared / "fluent-bit-clang-source"
    compiled = {
        os.path.relpath(entry["file"], clone)
        for entry in load(prepared / "fluent-bit-clang-production/compile_commands.json")
    }
    configured = collections.defaultdict(list)
    for row in load(configured_path)["units"]:
        configured[row["file"]].append(row)
    no_build = {row["file"]: row for row in load(no_build_path)["units"]}
    units, categories = [], collections.Counter()
    headers = collections.Counter()
    header_where = {}
    function_pairs = collections.Counter()
    favourable = []
    first_lines = collections.Counter()
    for file in sorted(set(configured) | set(no_build)):
        rows, row = configured.get(file, []), no_build.get(file)
        record = {"file": file}
        if row is None:
            record["category"] = "configured-only"
            record["configured"] = [r["status"] for r in rows]
        elif not rows:
            record["category"] = "no-build-only:" + (
                "outside-configured-selection" if file in compiled else "not-compiled"
            )
            record["no_build"] = row["status"]
            record["no_build_reason"] = row["assumptions"]["reason"]
            if row["status"] == "Safe":
                favourable.append({"file": file, "level": "unit", "cause": "not compiled by the configured build"})
        else:
            record["configured"] = [r["status"] for r in rows]
            record["no_build"] = row["status"]
            reason = row["assumptions"]["reason"]
            if reason is not None:
                first = row["assumptions"]["first_diagnostic"]
                record["category"] = f"not-prepared:{reason}"
                record["first_diagnostic"] = first
                match = HEADER.search(first or "")
                if reason == "missing-include" and match:
                    header = match.group(1)
                    # The includer as Clang spells it: absolute, or relative
                    # to the unit's own directory (the directory it ran in).
                    spelled = first.split(":", 1)[0]
                    includer = os.path.normpath(
                        os.path.join("/" + os.path.dirname(file), spelled)
                        if not os.path.isabs(spelled)
                        else "/" + os.path.relpath(spelled, fluent_bit)
                    )
                    key = (header, os.path.dirname(includer) if header.startswith(".") else "")
                    if key not in header_where:
                        header_where[key] = header_location(
                            header, includer, source_index, tracked, build_index
                        )
                    record["header"] = header
                    record["header_location"] = header_where[key]
                    headers[(header, header_where[key])] += 1
                    record["category"] += ":" + header_where[key]
                else:
                    first_lines[re.sub(r"^.*?error: ", "", first or "")] += 1
            else:
                same = any(digest(r) == digest(row) for r in rows)
                agree = all(r["status"] == row["status"] for r in rows)
                record["category"] = "prepared:" + ("same-text" if same else "different-text") + (
                    ":same-verdict" if agree else ":different-verdict"
                )
                if row["status"] == "Safe" and any(r["status"] != "Safe" for r in rows):
                    favourable.append({"file": file, "level": "unit", "cause": record["category"],
                                       "configured": record["configured"]})
                mine = functions(row)
                for r in rows:
                    theirs = functions(r)
                    for name in sorted(set(mine) | set(theirs)):
                        a, b = mine.get(name, "absent"), theirs.get(name, "absent")
                        function_pairs["agree" if a == b else "differ"] += 1
                        if a == "Safe" and b != "Safe":
                            favourable.append({"file": file, "level": "function", "function": name,
                                               "no_build": a, "configured": b,
                                               "same_text": same})
                        if a != b:
                            record.setdefault("function_differences", []).append(
                                {"function": name, "no_build": a, "configured": b})
        categories[record["category"]] += 1
        units.append(record)
    summary = {
        "configured_report": configured_path,
        "no_build_report": no_build_path,
        "configured_units": sum(len(rows) for rows in configured.values()),
        "configured_files": len(configured),
        "no_build_units": len(no_build),
        "categories": dict(sorted(categories.items())),
        "missing_headers": [
            {"header": header, "location": where, "units": count}
            for (header, where), count in sorted(headers.items(), key=lambda item: (-item[1], item[0]))
        ],
        "other_first_errors": dict(first_lines.most_common()),
        "function_pairs": dict(function_pairs),
        "favourable": favourable,
        "units": units,
    }
    pathlib.Path(out).write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps({k: summary[k] for k in ("configured_units", "configured_files", "no_build_units",
                                               "categories", "function_pairs")}, indent=1))
    print("favourable:", len(favourable))
    for item in favourable:
        print("  ", item)
    print("missing headers (top 15):")
    for item in summary["missing_headers"][:15]:
        print("  ", item)
    print("other first errors:", summary["other_first_errors"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
