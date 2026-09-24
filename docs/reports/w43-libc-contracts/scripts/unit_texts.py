#!/usr/bin/env python3
"""Pair each corpus unit with its analysed `clang -E -P` text.

Usage: unit_texts.py DIAGNOSTICS.json PREPROCESSED_P_DIR OUT_DIR

PREPROCESSED_P_DIR holds the `-E -P` outputs a transparent Clang wrapper saved
during a private run of the pinned audit (`*.i`; the text the analyzer's
`line` coordinate indexes). Identical texts are de-duplicated; each unit is
paired with the text whose file-scope function DEFINITIONS (an identifier at
brace depth 0 followed by a parenthesised list and then `{`) include every one
of its projected functions and whose line count covers the unit's leaf
coordinates; among several such texts the one with the fewest definitions is
taken only if it is unique. A unit with zero or several equally small
candidates is reported and left unpaired, never guessed. Writes `OUT_DIR/<library>__<file>.i`
(slashes as `__`) and `OUT_DIR/pairing.json`.
"""

import hashlib
import json
import pathlib
import re
import sys

IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def definitions(text):
    """File-scope `NAME ( ... ) {` definitions in one preprocessed text."""
    found = set()
    depth = 0
    position = 0
    length = len(text)
    while position < length:
        char = text[position]
        if char in "\"'":
            quote = char
            position += 1
            while position < length and text[position] != quote:
                position += 2 if text[position] == "\\" else 1
            position += 1
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth = max(0, depth - 1)
        elif depth == 0 and (char.isalpha() or char == "_"):
            match = IDENT.match(text, position)
            name, position = match.group(0), match.end()
            rest = position
            while rest < length and text[rest] in " \t\n":
                rest += 1
            if rest < length and text[rest] == "(":
                level, cursor = 0, rest
                while cursor < length:
                    if text[cursor] == "(":
                        level += 1
                    elif text[cursor] == ")":
                        level -= 1
                        if level == 0:
                            break
                    cursor += 1
                cursor += 1
                while cursor < length and text[cursor] in " \t\n":
                    cursor += 1
                if cursor < length and text[cursor] == "{":
                    found.add(name)
            continue
        position += 1
    return found


def main() -> int:
    diagnostics = json.loads(pathlib.Path(sys.argv[1]).read_text())
    source_dir, out = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    texts = {}
    for path in sorted(source_dir.glob("*.i")):
        text = path.read_text(errors="replace")
        texts.setdefault(hashlib.sha256(text.encode()).hexdigest(), text)
    defined = {digest: definitions(text) for digest, text in texts.items()}
    pairing = {}
    for unit in diagnostics["units"]:
        names = [f["function"] for f in unit["functions"]]
        lines = [d["line"] for f in unit["functions"] for d in f.get("diagnostics") or []]
        candidates = []
        for digest, text in texts.items():
            if set(names) <= defined[digest]:
                if not lines or max(lines) <= text.count("\n") + 1:
                    candidates.append(digest)
        key = f"{unit['library']}/{unit['file']}"
        if names and len(candidates) > 1:
            smallest = min(len(defined[d]) for d in candidates)
            candidates = [d for d in candidates if len(defined[d]) == smallest]
        if not names:
            candidates = []
        if len(candidates) == 1:
            name = key.replace("/", "__") + ".i"
            (out / name).write_text(texts[candidates[0]])
            pairing[key] = {"text": name, "sha256": candidates[0]}
        else:
            pairing[key] = {"text": None, "candidates": len(candidates)}
    (out / "pairing.json").write_text(json.dumps(pairing, indent=1, sort_keys=True) + "\n")
    unpaired = [k for k, v in pairing.items() if v["text"] is None]
    print(json.dumps({"units": len(pairing), "texts": len(texts), "unpaired": unpaired}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
