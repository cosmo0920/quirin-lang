#!/usr/bin/env python3
"""scope_check.py PREPASS.json OUT.json -- OP-APP-RK-CALLBACKS-VIA-POLL-CB:
evidence for `contract/profiles/fluent-bit/member-facts-scope.json`.

* For each generated parser unit the scope names (absent from the pinned
  inputs), the grammar it is generated from is read and every identifier of
  the pre-pass's final token set is searched for in it, and in every local
  header it includes (`#include "…"`, resolved beside the grammar and in the
  source tree's `include/`).
* For librdkafka's C++ wrapper, the fluent-bit link lines are read: they name
  `librdkafka.a` and no `rdkafka++`.

A reading aid: the grammar's generated C keeps the grammar's C text and adds
flex/bison skeleton code, which names no Kafka record or function. Writes
OUT.json and prints it."""
import json
import pathlib
import re
import shlex
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
SOURCE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
BUILD = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production")
GRAMMARS = {
    "processor-sql-parser_lex.c": "plugins/processor_sql/parser/sql-parser.l",
    "processor-sql_parser.c": "plugins/processor_sql/parser/sql-parser.y",
    "ra_lex.c": "src/record_accessor/ra.l",
    "ra_parser.c": "src/record_accessor/ra.y",
    "sql_lex.c": "src/stream_processor/parser/sql.l",
    "sql_parser.c": "src/stream_processor/parser/sql.y",
}
LINKS = ["src/CMakeFiles/fluent-bit-bin.dir/link.txt", "src/CMakeFiles/fluent-bit-shared.dir/link.txt"]


def headers(path, seen):
    text = path.read_text(errors="replace")
    out = [(path, text)]
    for name in re.findall(r'#\s*include\s+"([^"]+)"', text):
        for base in [path.parent, SOURCE / "include", SOURCE / "include/fluent-bit"]:
            candidate = (base / name).resolve()
            if candidate.is_file() and candidate not in seen:
                seen.add(candidate)
                out += headers(candidate, seen)
                break
    return out


def main():
    tokens = json.loads(pathlib.Path(sys.argv[1]).read_text())["tokens"]
    words = re.compile(r"\b(%s)\b" % "|".join(map(re.escape, tokens)))
    grammars = {}
    for unit, grammar in GRAMMARS.items():
        found = {}
        for path, text in headers(SOURCE / grammar, set()):
            hits = sorted(set(words.findall(text)))
            if hits:
                found[str(path.relative_to(SOURCE)) if path.is_relative_to(SOURCE) else str(path)] = hits
        grammars[unit] = {"grammar": grammar, "token_hits": found}
    # Units that exist but do not preprocess (a generated header is absent):
    # the raw text of the unit and of every header its own -I directories
    # resolve, transitively; the absent headers are listed.
    scope = json.loads((REPO / "contract/profiles/fluent-bit/member-facts-scope.json").read_text())
    entries = json.loads((SOURCE / "build/compile_commands.json").read_text())
    unpreprocessed = {}
    for outside in scope["outside_program"]:
        fragment = outside["fragment"]
        if "src-cpp" in fragment or "fluent-bit-clang-production" in fragment:
            continue
        for entry in entries:
            path = pathlib.Path(entry["file"])
            if fragment not in str(path) or not path.is_file():
                continue
            words_ = shlex.split(entry["command"])
            dirs = [pathlib.Path(entry["directory"], word[2:]) for word in words_
                    if word.startswith("-I") and len(word) > 2]
            dirs += [pathlib.Path(entry["directory"], words_[i + 1]) for i, word in enumerate(words_)
                     if word in ("-I", "-isystem") and i + 1 < len(words_)]
            seen, missing, hits, pending = set(), set(), {}, [path]
            while pending:
                current = pending.pop()
                if current in seen:
                    continue
                seen.add(current)
                text = current.read_text(errors="replace")
                found = sorted(set(words.findall(text)))
                if found:
                    hits[str(current)] = found
                for name in re.findall(r'#\s*include\s+[<"]([^>"]+)[>"]', text):
                    for base in [current.parent] + dirs:
                        candidate = base / name
                        if candidate.is_file():
                            pending.append(candidate.resolve())
                            break
                    else:
                        missing.add(name)
            unpreprocessed[str(path.relative_to(SOURCE))] = {
                "files_read": len(seen), "token_hits": hits,
                "unresolved_includes": sorted(missing)}
    links = {}
    for link in LINKS:
        text = (BUILD / link).read_text()
        links[link] = {"librdkafka.a": "librdkafka.a" in text, "rdkafka++": "rdkafka++" in text}
    out = {"tokens": len(tokens), "grammars": grammars, "unpreprocessed": unpreprocessed, "links": links}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
