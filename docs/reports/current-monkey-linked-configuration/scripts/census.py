#!/usr/bin/env python3
"""Census of the two Monkey configurations the `current` route can read.

CURRENT-MONKEY-LINKED-CONFIGURATION, scope item 1. Reads, never writes, the
user's preparation:

* the standalone Monkey database (`monkey/monkey-build`, the forced-Libevent
  copy the route read before this card), and
* the Fluent Bit database (`fluent-bit-clang-production`), whose
  `lib/monkey` entries are the configuration linked into `bin/fluent-bit`.

A unit is a Monkey unit by the route's own rule (`is_monkey_unit`): a `.c`
file under `mk_core/` (not `mk_core/deps/`), `mk_server/` or `tls/`. For every
such file in either database the census records which databases compile it,
how the two commands differ (macro definitions, include directories, other
words) and, by preprocessing each command with Clang (`-E`, output under
`--scratch`), whether the two configurations compile the same text: per
unit, the files it includes from outside the C library and, per top-level
function definition, whether its preprocessed tokens are equal, differ, or
exist in only one configuration.

Paths are compared after rewriting each tree's roots to placeholders, and the
user's `/home/cosmo/Documents/GitHub/qcore-c` spelling is rewritten to the
repository's own path before any command runs.
"""

import argparse
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys
from pathlib import Path

USER_PREFIX = "/home/cosmo/Documents/GitHub/qcore-c"
REPO = "/media/Data/GitHub/quirin-lang"


def rewrite(text):
    return text.replace(USER_PREFIX, REPO)


def monkey_relative(file, root):
    try:
        relative = Path(file).relative_to(root).as_posix()
    except ValueError:
        return None
    if not relative.endswith(".c"):
        return None
    if relative.startswith("mk_core/") and not relative.startswith("mk_core/deps/"):
        return relative
    if relative.startswith("mk_server/") or relative.startswith("tls/"):
        return relative
    return None


def words(entry):
    if "arguments" in entry:
        return [rewrite(word) for word in entry["arguments"]]
    return [rewrite(word) for word in shlex.split(entry["command"])]


def classify(argv):
    defines, includes, other = [], [], []
    index = 1
    while index < len(argv):
        word = argv[index]
        if word in ("-o", "-c"):
            index += 2
            continue
        if word.startswith("-D"):
            defines.append(word[2:] if len(word) > 2 else argv[index + 1])
            index += 1 if len(word) > 2 else 2
            continue
        if word.startswith("-I"):
            includes.append(word[2:] if len(word) > 2 else argv[index + 1])
            index += 1 if len(word) > 2 else 2
            continue
        other.append(word)
        index += 1
    return defines, includes, other


def placeholder(text, roots):
    for name, root in roots:
        text = text.replace(root, "<" + name + ">")
    return text


TOKEN = re.compile(
    r"[A-Za-z_][A-Za-z_0-9]*|\d[\w.]*|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|->|\+\+|--|<<=|>>=|[<>=!&|+\-*/%^]=|&&|\|\||<<|>>|\.\.\.|\S"
)


def preprocess(argv, directory, output):
    command = []
    skip = False
    for index, word in enumerate(argv):
        if skip:
            skip = False
            continue
        if word == "-o":
            skip = True
            continue
        if word == "-c":
            continue
        command.append(word)
    command += ["-E", "-o", str(output)]
    result = subprocess.run(command, cwd=directory, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"preprocessing failed in {directory}: {result.stderr[:2000]}")
    return output.read_text(errors="replace")


LINE_MARKER = re.compile(r'^# \d+ "([^"]+)"')


def included_files(text):
    files = set()
    for line in text.splitlines():
        match = LINE_MARKER.match(line)
        if match and not match.group(1).startswith("<"):
            files.add(match.group(1))
    return files


def definitions(text, roots):
    """Top-level function definitions: name -> [origin file, token digest].

    The origin is the file holding the name (from the line markers), so a
    header's inline function is told apart from the unit's own definitions
    and those of a textually included `.c` file (the event backend).
    """
    tokens = []
    origin = None
    for line in text.splitlines():
        match = LINE_MARKER.match(line)
        if match:
            origin = placeholder(match.group(1), roots)
            continue
        if line.startswith("#"):
            continue
        tokens.extend((token, origin) for token in TOKEN.findall(line))
    found = {}
    depth = 0
    start = 0
    name = None
    name_origin = None
    definition_start = 0
    for index, (token, _) in enumerate(tokens):
        if token == "{":
            if depth == 0:
                # A definition: `name ( ... ) {` at file scope.
                name = None
                close = index - 1
                while close >= 0 and tokens[close][0] != ")":
                    if tokens[close][0] in (";", "}", "="):
                        close = -1
                        break
                    close -= 1
                if close >= 0:
                    level = 0
                    open_index = close
                    while open_index >= 0:
                        if tokens[open_index][0] == ")":
                            level += 1
                        elif tokens[open_index][0] == "(":
                            level -= 1
                            if level == 0:
                                break
                        open_index -= 1
                    if open_index > 0:
                        candidate, candidate_origin = tokens[open_index - 1]
                        if re.match(r"[A-Za-z_]\w*$", candidate) and candidate != "__attribute__":
                            name, name_origin = candidate, candidate_origin
                definition_start = start
            depth += 1
        elif token == "}":
            depth -= 1
            if depth == 0:
                if name:
                    words_ = " ".join(token for token, _ in tokens[definition_start : index + 1])
                    digest = hashlib.sha256(words_.encode()).hexdigest()
                    found.setdefault(name, []).append([name_origin, digest])
                    name = None
                start = index + 1
        elif token == ";" and depth == 0:
            start = index + 1
    return found


def unit_level(entries):
    """Whether a definition is the unit's own (a Monkey `.c` file)."""
    return any(origin.startswith("<monkey-source>/") and origin.endswith(".c") for origin, _ in entries)


def digests(entries):
    return sorted(digest for _, digest in entries)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--preparation", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--output", required=True)
    arguments = parser.parse_args()
    preparation = Path(arguments.preparation)
    scratch = Path(arguments.scratch)
    scratch.mkdir(parents=True, exist_ok=True)
    fb_database = preparation / "fluent-bit-clang-production/compile_commands.json"
    mk_database = preparation / "monkey/monkey-build/compile_commands.json"
    fb_source = str(preparation / "fluent-bit-clang-source")
    fb_build = str(preparation / "fluent-bit-clang-production")
    mk_source = str(preparation / "monkey/monkey-source")
    mk_build = str(preparation / "monkey/monkey-build")
    roots = [
        ("fb-build/lib/monkey", fb_build + "/lib/monkey"),
        ("monkey-source", fb_source + "/lib/monkey"),
        ("fb-build", fb_build),
        ("fb-source", fb_source),
        ("monkey-build", mk_build),
        ("monkey-source", mk_source),
    ]
    databases = {}
    for name, path, root in (
        ("linked", fb_database, fb_source + "/lib/monkey"),
        ("standalone", mk_database, mk_source),
    ):
        raw = path.read_bytes()
        entries = json.loads(rewrite(raw.decode()))
        selected = {}
        for entry in entries:
            relative = monkey_relative(entry["file"], root)
            if relative is not None:
                if relative in selected:
                    raise SystemExit(f"{name}: {relative} has more than one configuration")
                selected[relative] = entry
        databases[name] = {
            "path": str(path),
            "sha256": hashlib.sha256(raw).hexdigest(),
            "units": selected,
        }
    units = []
    files = sorted(set(databases["linked"]["units"]) | set(databases["standalone"]["units"]))
    for relative in files:
        row = {"unit": relative, "in": []}
        readings = {}
        for name in ("linked", "standalone"):
            entry = databases[name]["units"].get(relative)
            if entry is None:
                continue
            row["in"].append(name)
            argv = words(entry)
            defines, includes, other = classify(argv)
            out = scratch / name / (relative.replace("/", "__") + ".i")
            out.parent.mkdir(parents=True, exist_ok=True)
            text = preprocess(argv, entry["directory"], out)
            readings[name] = {
                "defines": sorted(defines),
                "includes": [placeholder(include, roots) for include in includes],
                "other": [placeholder(word, roots) for word in other],
                "command_sha256": hashlib.sha256(
                    (entry.get("command") or " ".join(entry["arguments"])).encode()
                ).hexdigest(),
                "included": sorted(
                    placeholder(path, roots)
                    for path in included_files(text)
                    if "/lib/monkey" in path or "monkey-" in path
                ),
                "definitions": definitions(placeholder(text, roots), roots),
                "unit_tokens_sha256": hashlib.sha256(
                    " ".join(
                        TOKEN.findall(
                            "\n".join(
                                line
                                for line in placeholder(text, roots).splitlines()
                                if not line.startswith("#")
                            )
                        )
                    ).encode()
                ).hexdigest(),
            }
        if len(readings) == 2:
            linked, standalone = readings["linked"], readings["standalone"]
            row["defines_only_linked"] = sorted(set(linked["defines"]) - set(standalone["defines"]))
            row["defines_only_standalone"] = sorted(
                set(standalone["defines"]) - set(linked["defines"])
            )
            row["other_words"] = {"linked": linked["other"], "standalone": standalone["other"]}
            row["include_directories"] = {
                "linked": linked["includes"],
                "standalone": standalone["includes"],
            }
            row["included_only_linked"] = sorted(
                set(linked["included"]) - set(standalone["included"])
            )
            row["included_only_standalone"] = sorted(
                set(standalone["included"]) - set(linked["included"])
            )
            for scope, keep in (("functions", True), ("header_functions", False)):
                names = sorted(
                    function
                    for reading in (linked, standalone)
                    for function, entries in reading["definitions"].items()
                    if unit_level(entries) == keep
                )
                same, differ, only_linked, only_standalone = set(), set(), set(), set()
                for function in names:
                    a = linked["definitions"].get(function)
                    b = standalone["definitions"].get(function)
                    if a is None:
                        only_standalone.add(function)
                    elif b is None:
                        only_linked.add(function)
                    elif digests(a) == digests(b):
                        same.add(function)
                    else:
                        differ.add(function)
                row[scope] = {
                    "same": sorted(same),
                    "differ": sorted(differ),
                    "only_linked": sorted(only_linked),
                    "only_standalone": sorted(only_standalone),
                }
            functions = row["functions"]
            row["same_unit_tokens"] = linked["unit_tokens_sha256"] == standalone["unit_tokens_sha256"]
            row["same_text"] = not (
                functions["differ"] or functions["only_linked"] or functions["only_standalone"]
            )
        else:
            (name,) = readings
            definitions_ = readings[name]["definitions"]
            row["functions"] = {
                "only_" + name: sorted(f for f, e in definitions_.items() if unit_level(e))
            }
            row["defines"] = readings[name]["defines"]
            row["included"] = readings[name]["included"]
        row["command_sha256"] = {name: readings[name]["command_sha256"] for name in readings}
        units.append(row)
    first = next(iter(databases["linked"]["units"].values()))
    common_linked = set(classify(words(first))[0])
    first_standalone = next(iter(databases["standalone"]["units"].values()))
    common_standalone = set(classify(words(first_standalone))[0])
    report = {
        "schema": "current-monkey-linked-configuration/census/1",
        "preparation": str(preparation),
        "databases": {
            name: {"path": value["path"], "sha256": value["sha256"], "units": len(value["units"])}
            for name, value in databases.items()
        },
        "first_unit_defines": {
            "only_linked": sorted(common_linked - common_standalone),
            "only_standalone": sorted(common_standalone - common_linked),
        },
        "units": units,
    }
    Path(arguments.output).write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    both = [row for row in units if len(row["in"]) == 2]
    print(f"units: {len(units)}; both {len(both)}; same text {sum(row['same_text'] for row in both)}")
    for row in units:
        functions = row["functions"]
        header = row.get("header_functions", {})
        print(
            row["unit"],
            "+".join(row["in"]),
            "tokens-equal" if row.get("same_unit_tokens") else "tokens-differ",
            {key: len(value) for key, value in functions.items()},
            "differ:" + ",".join(functions.get("differ", [])) if functions.get("differ") else "",
            "header:" + str({key: value for key, value in header.items() if key != "same" and value}),
        )


if __name__ == "__main__":
    sys.exit(main())
