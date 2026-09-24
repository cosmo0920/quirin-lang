#!/usr/bin/env python3
"""Constraint-14 census for card CONTRACT-INHERIT.

Measures, over every tracked Quirin source (`*.qcore`, `*.qc`, archived probes
included) and every tracked Rust/Python/Markdown/JSON file that could embed a
Quirin program in a string, the spellings this card claims:

* the word `inherit` anywhere (as a whole word);
* `contract` followed by anything other than `{` (the position `inherit` takes);
* a `#` in code position of a Quirin source (after comment stripping);
* the word `include` directly after a `#`.

Read-only. Writes one JSON document to the path given as the only argument.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
WORD = re.compile(r"\binherit\b")
HASH_INCLUDE = re.compile(r"#\s*include\b")
CONTRACT_TAIL = re.compile(r"\bcontract\s+(?!\{)([A-Za-z_]\w*)")


def strip_comments(text):
    """Remove `//` and `/* */` comments, keeping string literals intact."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        if text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
            continue
        if text[i] == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i:j + 1])
            i = j + 1
            continue
        out.append(text[i])
        i += 1
    return "".join(out)


def tracked():
    listing = subprocess.run(["git", "ls-files"], cwd=ROOT, check=True,
                             capture_output=True, text=True).stdout.split("\n")
    return [name for name in listing if name]


def main():
    output = Path(sys.argv[1])
    files = tracked()
    sources = [name for name in files if name.endswith((".qcore", ".qc"))]
    embedders = [name for name in files
                 if name.endswith((".rs", ".py", ".json", ".el", ".lean"))
                 and not name.startswith("third_party/")]
    result = {
        "quirin_sources": len(sources),
        "archived_probe_sources": sum(1 for name in sources if name.startswith("docs/reports/")),
        "source_inherit_words": [],
        "source_code_hashes": [],
        "source_hash_include": [],
        "source_contract_non_block_tails": [],
        "embedder_files_scanned": len(embedders),
        "embedder_inherit_words": [],
        "embedder_hash_include_in_quirin_context": [],
    }
    for name in sources:
        text = (ROOT / name).read_text(encoding="utf-8", errors="replace")
        code = strip_comments(text)
        for match in WORD.finditer(text):
            result["source_inherit_words"].append(
                {"file": name, "offset": match.start(), "in_code": bool(WORD.search(code))})
        for index, char in enumerate(code):
            if char == "#":
                result["source_code_hashes"].append(
                    {"file": name, "context": code[max(0, index - 20):index + 20]})
        for match in HASH_INCLUDE.finditer(code):
            result["source_hash_include"].append({"file": name, "offset": match.start()})
        for match in CONTRACT_TAIL.finditer(code):
            result["source_contract_non_block_tails"].append(
                {"file": name, "word": match.group(1)})
    for name in embedders:
        path = ROOT / name
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, FileNotFoundError):
            continue
        for match in WORD.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            result["embedder_inherit_words"].append(
                {"file": name, "line": line, "text": text.split("\n")[line - 1].strip()[:160]})
        # A `#include` inside an embedded Quirin program would sit next to
        # `module contract;`; C fixtures carry `#include` legitimately.
        if "module contract;" in text:
            for match in HASH_INCLUDE.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                result["embedder_hash_include_in_quirin_context"].append(
                    {"file": name, "line": line,
                     "text": text.split("\n")[line - 1].strip()[:160]})
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({key: (len(value) if isinstance(value, list) else value)
                      for key, value in result.items()}, sort_keys=True))


if __name__ == "__main__":
    main()
