#!/usr/bin/env python3
"""Write the no-build measurement configurations for Fluent Bit.

Usage: selection.py FLUENT_BIT PROFILE_DIR PREPARED_OUT LOAD_PATH RUN_DIR

Writes two `quirin-audit contracts` JSON configurations into RUN_DIR:

* ``bare.json``: the Cycle 19 profile's library groups (``audit.json``) as
  no-build sources, with no ``-I``, ``-D`` or ``--std``;
* ``src-includes.json``: the same sources with the include directories a user
  can name from the source tree alone: every ``-I``/``-isystem`` directory of
  the selected configured compile records (in first-seen order) that lies in
  the source clone, rebased onto FLUENT_BIT. Directories in the build tree
  (generated headers) are left out, because they do not exist without a
  build; they are listed in ``selection.json``. No definition is supplied.

Selection follows the profile's source roots: ``top_level`` names each
``*.c`` file directly in the root, ``recursive`` names the root (the route
walks it), ``exact`` names the file. Nothing is read from FLUENT_BIT except
directory listings.
"""

import json
import pathlib
import shlex
import sys


def main() -> int:
    fluent_bit, profile_dir, prepared, load_path, run_dir = map(pathlib.Path, sys.argv[1:6])
    profile = json.loads((profile_dir / "profile.json").read_text())
    audit = json.loads((profile_dir / "audit.json").read_text())
    sources = []
    for group in audit["libraries"]:
        library = profile["libraries"][group["library"]]
        for root in library["source_roots"]:
            selection = root["selection"]
            if selection == "top_level":
                directory = fluent_bit / root["root"]
                sources += sorted(
                    str(path.relative_to(fluent_bit))
                    for path in directory.iterdir()
                    if path.is_file() and path.suffix == ".c"
                )
            elif selection == "recursive":
                sources.append(root["root"])
            else:
                sources.append(selection["exact"]["file"])
    clone = prepared / "fluent-bit-clang-source"
    build = prepared / "fluent-bit-clang-production"
    database = json.loads((build / "compile_commands.json").read_text())
    selected_prefixes = [str(clone / source) for source in sources]
    includes, build_tree = [], []
    for entry in database:
        file = entry["file"]
        if not any(file == prefix or file.startswith(prefix + "/") for prefix in selected_prefixes):
            continue
        words = entry["arguments"] if "arguments" in entry else shlex.split(entry["command"])
        index = 0
        while index < len(words):
            word = words[index]
            value = None
            for flag in ("-isystem", "-I"):
                if word == flag:
                    index += 1
                    value = words[index]
                elif word.startswith(flag) and len(word) > len(flag):
                    value = word[len(flag):]
                if value is not None:
                    break
            index += 1
            if value is None:
                continue
            path = pathlib.Path(value)
            if not path.is_absolute():
                path = pathlib.Path(entry["directory"]) / path
            if path.is_relative_to(clone):
                rebased = fluent_bit / path.relative_to(clone)
                if rebased.is_dir() and str(rebased) not in includes:
                    includes.append(str(rebased))
            elif str(path) not in build_tree:
                build_tree.append(str(path))
    run_dir.mkdir(parents=True, exist_ok=True)
    base = {
        "schema_version": 1,
        "mode": "contracts-only",
        "build": "none",
        "source_root": str(fluent_bit),
        "sources": sources,
        "load_path": str(load_path),
        "jobs": 16,
    }
    for name, extra in (("bare", {}), ("src-includes", {"include_dirs": includes})):
        config = dict(base, **extra, report=str(run_dir / name / "report.json"))
        (run_dir / f"{name}.json").write_text(json.dumps(config, indent=1) + "\n")
    (run_dir / "selection.json").write_text(
        json.dumps(
            {"sources": sources, "source_include_dirs": includes, "build_tree_include_dirs": build_tree},
            indent=1,
        )
        + "\n"
    )
    print(f"{len(sources)} source arguments, {len(includes)} source-tree include dirs, "
          f"{len(build_tree)} build-tree include dirs left out")
    return 0


if __name__ == "__main__":
    sys.exit(main())
