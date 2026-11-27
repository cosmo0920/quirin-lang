#!/usr/bin/env python3
"""Card BUILTIN-EXPECT-NULL-TEST census: every `__builtin_expect` site in the
analysed function bodies of a run, read from each unit's own preprocessed text.

For each unit of a resource-diagnostics report the unit is preprocessed twice
with the gateway's own preprocessor arguments (`-x c <dialect> -E -P` plus the
recorded options): once with `-P` to check the bytes against the report's
`preprocessed_source_sha256` where the report records one, once without `-P`
so every token keeps its original file and line. Every `__builtin_expect` /
`__builtin_expect_with_probability` token inside a function definition of the
main file is one site; the macros that spelled it (`likely`, `unlikely`,
`mk_bug`, ...) are whatever the unit's own preprocessed text expanded.

Per site the census records the position (the controlling expression of an
`if`/`while`/`for`/`do`/`switch`, a `return`, or another expression), whether
the call sits in a Boolean leaf position of that expression (only grouping
parentheses, `!`, `&&`, `||`, `?:` between the slot and the call), the
operand's shape, the hint's shape, and what the run's own diagnostics say at
the site's original line in its function ("today").

Read-only over its inputs; writes one JSON file. Usage:

    python3 -I census.py --out census.json \
        --route current=<report> --route system=<report> ... \
        [--corpus <report> --corpus-db <compile_commands.json> ...]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from collections import Counter

CLANG = "/usr/lib/llvm-21/bin/clang"
SPELLED_PREFIX = "/home/cosmo/Documents/GitHub/qcore-c"
CANONICAL_PREFIX = "/media/Data/GitHub/quirin-lang"
FLUENT_BIT_PIN_SOURCE = (
    CANONICAL_PREFIX + "/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
)
MONKEY_PIN_SOURCE = CANONICAL_PREFIX + "/build/cycle19/c-gateway-wave-pin/monkey/monkey-source"
CORPUS_ROOTS = {
    "cmetrics": FLUENT_BIT_PIN_SOURCE + "/lib/cmetrics/src",
    "ctraces": FLUENT_BIT_PIN_SOURCE + "/lib/ctraces/src",
    "librdkafka": FLUENT_BIT_PIN_SOURCE + "/lib/librdkafka-2.15.0/src",
    "chunkio": FLUENT_BIT_PIN_SOURCE + "/lib/chunkio/src",
    "fluent-bit-chunk": FLUENT_BIT_PIN_SOURCE + "/src",
    "monkey-libevent": MONKEY_PIN_SOURCE,
}

FRONTEND = {
    "AllWarnings": ["-Wall"],
    "ExtraWarnings": ["-Wextra"],
    "DebugInformation": ["-g"],
    "PositionIndependentCode": ["-fPIC"],
    "AssemblerNoExecStack": ["-Wa,--noexecstack"],
    "StrictAliasingWarnings": ["-Wstrict-aliasing"],
    "StrictPrototypes": ["-Wstrict-prototypes"],
    "UndefinedIdentifierWarnings": ["-Wundef"],
    "MissingPrototypes": ["-Wmissing-prototypes"],
    "InitSelf": ["-Winit-self"],
    "MissingFieldInitializers": ["-Wmissing-field-initializers"],
    "DeclarationAfterStatement": ["-Wdeclaration-after-statement"],
    "AddressComparisons": ["-Waddress"],
    "OverrideInit": ["-Woverride-init"],
    "WriteStrings": ["-Wwrite-strings"],
    "VariableLengthArrays": ["-Wvla"],
    "NoUnusedParameter": ["-Wno-unused-parameter"],
    "NoUnusedFunction": ["-Wno-unused-function"],
    "NoPragmas": ["-Wno-pragmas"],
    "NoVoidPointerToEnumCast": ["-Wno-void-pointer-to-enum-cast"],
    "NoStrictAliasing": ["-fno-strict-aliasing"],
    "FunctionSections": ["-ffunction-sections"],
    "DataSections": ["-fdata-sections"],
}
DIALECTS = {
    "c11": "-std=c11",
    "gnu99": "-std=gnu99",
    "gnu11": "-std=gnu11",
    "gnu17": "-std=gnu17",
    "c17": "-std=c17",
}

TOKEN = re.compile(
    r"""
    (?P<ws>[ \t\r\f\v]+)
  | (?P<nl>\n)
  | (?P<str>L?"(?:\\.|[^"\\\n])*")
  | (?P<chr>L?'(?:\\.|[^'\\\n])*')
  | (?P<num>\.?[0-9](?:[eEpP][+-]|[0-9A-Za-z_.])*)
  | (?P<id>[A-Za-z_][A-Za-z_0-9]*)
  | (?P<punct>\.\.\.|<<=|>>=|->|\+\+|--|<<|>>|<=|>=|==|!=|&&|\|\||[-+*/%&|^]=|\#\#|[][(){}.,;:?~!<>=+\-*/%&|^\#])
  | (?P<other>.)
    """,
    re.VERBOSE,
)
MARKER = re.compile(r'^#\s*(\d+)\s+"((?:\\.|[^"\\])*)"')
KEYWORDS = {
    "if", "while", "for", "switch", "return", "sizeof", "_Alignof", "__alignof__",
    "do", "else", "case", "default", "goto", "break", "continue", "typeof",
    "__typeof__", "_Generic", "__builtin_offsetof", "__extension__",
}
WRAPPERS = {"__builtin_expect", "__builtin_expect_with_probability"}
NULL_SPELLINGS = (["0"], ["NULL"], ["(", "(", "void", "*", ")", "0", ")"], ["(", "void", "*", ")", "0"])


def tokenize(text: str):
    """(text, file, line) for every token of `clang -E` output with markers."""
    tokens = []
    file = "<unknown>"
    line = 1
    for raw in text.split("\n"):
        marker = MARKER.match(raw)
        if marker:
            line = int(marker.group(1))
            file = marker.group(2)
            continue
        for match in TOKEN.finditer(raw):
            kind = match.lastgroup
            if kind in ("ws", "nl"):
                continue
            tokens.append((match.group(), file, line))
        line += 1
    return tokens


def matching(texts, open_index):
    depth = 0
    for index in range(open_index, len(texts)):
        if texts[index] in ("(", "[", "{"):
            depth += 1
        elif texts[index] in (")", "]", "}"):
            depth -= 1
            if depth == 0:
                return index
    return None


def functions(tokens, main_file):
    """(name, body_start, body_end) of every function definition in the main file."""
    texts = [t[0] for t in tokens]
    found = []
    index = 0
    depth = 0
    while index < len(texts):
        text = texts[index]
        if text == "{" and depth == 0:
            close = matching(texts, index)
            if close is None:
                break
            # A definition: `NAME ( ... ) {` (possibly with attributes between
            # the `)` and the `{`); not an initializer (`= {`) or a record.
            previous = index - 1
            while previous >= 0 and texts[previous] != ")" and texts[previous] not in (";", "}", "=", "{"):
                previous -= 1
            name = None
            if previous >= 0 and texts[previous] == ")":
                depth_back = 0
                cursor = previous
                while cursor >= 0:
                    if texts[cursor] == ")":
                        depth_back += 1
                    elif texts[cursor] == "(":
                        depth_back -= 1
                        if depth_back == 0:
                            break
                    cursor -= 1
                if cursor > 0 and re.fullmatch(r"[A-Za-z_]\w*", texts[cursor - 1]):
                    name = texts[cursor - 1]
                    # `__attribute__ ((...))` before the body: walk back once more.
                    if name in ("__attribute__", "__asm__", "asm"):
                        name = None
            if name and tokens[index][1] == main_file:
                found.append((name, index, close))
            index = close + 1
            continue
        index += 1
    return found


def strip(texts):
    while len(texts) >= 2 and texts[0] == "(" and matching(texts, 0) == len(texts) - 1:
        texts = texts[1:-1]
    return texts


def split_top(texts, separator):
    parts, current, depth = [], [], 0
    for text in texts:
        if text in ("(", "[", "{"):
            depth += 1
        elif text in (")", "]", "}"):
            depth -= 1
        if text == separator and depth == 0:
            parts.append(current)
            current = []
            continue
        current.append(text)
    parts.append(current)
    return parts


def has_call(texts):
    for index in range(len(texts) - 1):
        if texts[index + 1] == "(" and re.fullmatch(r"[A-Za-z_]\w*", texts[index]) and texts[index] not in KEYWORDS:
            return True
    return False


def is_place(texts):
    """A canonical place: NAME, NAME->f, NAME.f, chains of those."""
    texts = strip(texts)
    if not texts or not re.fullmatch(r"[A-Za-z_]\w*", texts[0]) or texts[0] in KEYWORDS:
        return False
    index = 1
    while index < len(texts):
        if texts[index] in ("->", ".") and index + 1 < len(texts) and re.fullmatch(r"[A-Za-z_]\w*", texts[index + 1]):
            index += 2
            continue
        return False
    return True


def is_null(texts):
    texts = strip(texts)
    return any(texts == list(spelling) or strip(list(spelling)) == texts for spelling in NULL_SPELLINGS)


def top_level(texts, operators):
    depth = 0
    for index, text in enumerate(texts):
        if text in ("(", "[", "{"):
            depth += 1
        elif text in (")", "]", "}"):
            depth -= 1
        elif depth == 0 and text in operators:
            return index
    return None


def operand_shape(texts):
    texts = strip(texts)
    negations = 0
    while texts and texts[0] == "!":
        negations += 1
        texts = strip(texts[1:])
    if not texts:
        return "empty"
    if "=" in texts and top_level(texts, {"="}) is not None:
        return "assignment"
    if top_level(texts, {"&&", "||", "?"}) is not None:
        return "connective-with-call" if has_call(texts) else "connective"
    if is_place(texts):
        return "place-test"
    comparison = top_level(texts, {"==", "!="})
    if comparison is not None and (is_null(texts[:comparison]) and is_place(texts[comparison + 1:]) or is_null(texts[comparison + 1:]) and is_place(texts[:comparison])):
        return "place-test"
    if has_call(texts):
        return "call-bearing"
    if top_level(texts, {"==", "!=", "<", ">", "<=", ">="}) is not None:
        return "comparison"
    return "other"


def hint_shape(texts):
    texts = strip(texts)
    if len(texts) == 1 and re.fullmatch(r"[0-9]+", texts[0]):
        return "decimal-literal"
    body = texts[1:] if texts and texts[0] in ("-", "+") else texts
    body = strip(body)
    if len(body) == 1 and re.fullmatch(r"(0[xX][0-9A-Fa-f]+|[0-9]+)[uUlL]*", body[0]):
        return "integer-literal"
    if has_call(texts) or any(t in ("++", "--", "=") for t in texts):
        return "side-effect-possible"
    if all(re.fullmatch(r"[0-9][0-9A-Za-z_.]*|[-+~!()]", t) for t in texts):
        return "constant-expression"
    return "non-literal"


def position_of(texts, start, call_index):
    """The position of the call at `call_index` inside one function body."""
    stack = []
    for index in range(start, call_index):
        text = texts[index]
        if text == "(":
            stack.append(index)
        elif text == ")":
            if stack:
                stack.pop()
    # The enclosing statement-level parenthesis: the outermost open one whose
    # predecessor is a keyword slot.
    for depth, open_index in enumerate(stack):
        keyword = texts[open_index - 1]
        if keyword in ("if", "while", "switch", "for"):
            inner = stack[depth + 1:]
            leaf = all(
                not (re.fullmatch(r"[A-Za-z_]\w*", texts[i - 1]) and texts[i - 1] not in KEYWORDS)
                and texts[i - 1] not in (")", "]")
                for i in inner
            )
            if keyword == "for":
                close = matching(texts, open_index)
                clauses = 0
                level = 0
                for i in range(open_index + 1, call_index):
                    if texts[i] in ("(", "["):
                        level += 1
                    elif texts[i] in (")", "]"):
                        level -= 1
                    elif texts[i] == ";" and level == 0:
                        clauses += 1
                keyword = ("for-init", "for-condition", "for-step")[min(clauses, 2)]
            elif keyword == "while":
                # `do { … } while (…)`: the `while` follows a closing brace
                # that closes a `do` body.
                if texts[open_index - 2] == "}":
                    brace = open_index - 2
                    level = 0
                    cursor = brace
                    while cursor > start:
                        if texts[cursor] == "}":
                            level += 1
                        elif texts[cursor] == "{":
                            level -= 1
                            if level == 0:
                                break
                        cursor -= 1
                    if texts[cursor - 1] == "do":
                        keyword = "do-while"
            return keyword, leaf
    # No controlling slot: a return value, or another expression.
    cursor = call_index - 1
    while cursor > start and texts[cursor] not in (";", "{", "}"):
        cursor -= 1
    statement = texts[cursor + 1]
    if statement == "return":
        return "return", not stack
    return "expression", False


def diagnostics_index(unit):
    by_function = {}
    for function in unit.get("functions", []):
        rows = []
        for diagnostic in function.get("diagnostics", []):
            origin = diagnostic.get("origin") or ""
            line = origin.rsplit(":", 1)[-1] if ":" in origin else None
            rows.append((line, diagnostic.get("category"), diagnostic.get("description")))
        by_function[function["function"]] = (function.get("status"), rows)
    return by_function


CONDITION_REFUSALS = (
    "unmodeled call, aggregate or indirect access in",
    "call to uncontracted function `__builtin_expect",
)


def today(by_function, name, line):
    if name not in by_function:
        return "function-not-projected", []
    status, rows = by_function[name]
    here = [(category, description) for (l, category, description) in rows if l == str(line)]
    refusals = [d for (c, d) in here if c == "unsupported" and d and d.startswith(CONDITION_REFUSALS)]
    if refusals:
        return "refused-at-position", here
    return "read", here


def preprocess(arguments, directory, spelled, plain):
    command = [CLANG, "-x", "c", *arguments[:1], "-E"]
    if plain:
        command.append("-P")
    command.extend(arguments[1:])
    command.append(spelled)
    result = subprocess.run(command, cwd=directory, capture_output=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(result.stderr.decode(errors="replace")[-2000:])
    return result.stdout


def configured_arguments(configuration):
    arguments = [DIALECTS[configuration["dialect"]]]
    for option in configuration.get("preprocessor_options", []):
        (kind, value), = option.items()
        if kind == "Define":
            arguments += ["-D", value]
        elif kind == "Undefine":
            arguments += ["-U", value]
        elif kind == "Include":
            arguments += ["-I", value]
        elif kind == "SystemInclude":
            arguments += ["-isystem", value]
        elif kind == "QuoteInclude":
            arguments += ["-iquote", value]
        elif kind == "FrontendFlag":
            if isinstance(value, str):
                arguments += FRONTEND[value]
            else:
                raise RuntimeError(f"unrendered frontend flag {value!r}")
        else:
            raise RuntimeError(f"unknown preprocessor option {kind}")
    for path in configuration.get("include_paths", []):
        arguments.append(f"-I{path}")
    for definition in configuration.get("definitions", []):
        arguments.append(f"-D{definition}")
    source = configuration["source_input"]["DeclaredFile"]
    return arguments, source["directory"], source["spelled"]


def database_arguments(entry):
    """Preprocessor arguments of one compile-database entry (corpus route)."""
    import shlex

    words = shlex.split(entry["command"]) if "command" in entry else list(entry["arguments"])
    words = [w.replace(SPELLED_PREFIX, CANONICAL_PREFIX) for w in words]
    dialect = None
    arguments = []
    index = 1
    while index < len(words):
        word = words[index]
        if word in ("-o",):
            index += 2
            continue
        if word == "-c":
            index += 1
            continue
        if word.startswith("-std="):
            dialect = word
        elif word.startswith(("-D", "-U", "-I")) or word in ("-isystem", "-iquote"):
            if word in ("-D", "-U", "-I", "-isystem", "-iquote"):
                arguments += [word, words[index + 1]]
                index += 2
                continue
            arguments.append(word)
        elif word.startswith(("-W", "-f", "-g", "-m")) and not word.startswith("-Wl,"):
            arguments.append(word)
        index += 1
    directory = entry["directory"].replace(SPELLED_PREFIX, CANONICAL_PREFIX)
    file = entry["file"].replace(SPELLED_PREFIX, CANONICAL_PREFIX)
    return [dialect or "-std=gnu99", *arguments], directory, file


HARNESS = []


def census_unit(route, unit, arguments, directory, spelled, expected_sha):
    plain = preprocess(arguments, directory, spelled, plain=True)
    identity = hashlib.sha256(plain).hexdigest()
    identity_status = (
        "unrecorded" if expected_sha is None else ("equal" if identity == expected_sha else "DIFFERENT")
    )
    marked = preprocess(arguments, directory, spelled, plain=False).decode(errors="replace")
    tokens = tokenize(marked)
    main_file = os.path.normpath(os.path.join(directory, spelled))
    files = {t[1] for t in tokens}
    main = next((f for f in files if os.path.normpath(os.path.join(directory, f)) == main_file), spelled)
    texts = [t[0] for t in tokens]
    by_function = diagnostics_index(unit)
    rows = []
    for name, start, end in functions(tokens, main):
        for index in range(start, end):
            if texts[index] not in WRAPPERS or texts[index + 1] != "(":
                continue
            close = matching(texts, index + 1)
            arguments_texts = split_top(texts[index + 2:close], ",")
            line = tokens[index][2]
            position, leaf = position_of(texts, start, index)
            state, here = today(by_function, name, line)
            rows.append(
                {
                    "route": route,
                    "unit": unit["file"],
                    "function": name,
                    "origin": f"{unit['file']}:{line}",
                    "wrapper": texts[index],
                    "position": position,
                    "boolean_leaf": leaf,
                    "operand": " ".join(arguments_texts[0]),
                    "operand_shape": operand_shape(arguments_texts[0]),
                    "hint": " ".join(arguments_texts[1]) if len(arguments_texts) > 1 else "",
                    "hint_shape": hint_shape(arguments_texts[1]) if len(arguments_texts) > 1 else "missing",
                    "extra_arguments": [" ".join(a) for a in arguments_texts[2:]],
                    "today": state,
                    "diagnostics_at_line": [f"{c}: {d}" for (c, d) in here],
                }
            )
    return identity_status, rows


def corpus_entry(unit, databases):
    root = CORPUS_ROOTS[unit["library"]]
    wanted = os.path.normpath(os.path.join(root, unit["file"]))
    matches = [
        entry
        for entry in databases
        if os.path.normpath(
            os.path.join(
                entry["directory"].replace(SPELLED_PREFIX, CANONICAL_PREFIX),
                entry["file"].replace(SPELLED_PREFIX, CANONICAL_PREFIX),
            )
        )
        == wanted
    ]
    return matches


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--route", action="append", default=[], help="name=report (units carry compile_configuration)")
    parser.add_argument("--corpus", help="pinned corpus report (no compile_configuration)")
    parser.add_argument("--corpus-db", action="append", default=[])
    parser.add_argument("--harness-list", help="write the census harness's unit list (configured routes) here")
    options = parser.parse_args()
    rows = []
    units = []
    for spec in options.route:
        name, path = spec.split("=", 1)
        report = json.load(open(path))
        for unit in report["units"]:
            configuration = unit.get("compile_configuration")
            if configuration:
                configuration = configuration.get("compile_task_identity", configuration)
            if not configuration:
                units.append({"route": name, "unit": unit["file"], "identity": "no-configuration"})
                continue
            arguments, directory, spelled = configured_arguments(configuration)
            try:
                identity, found = census_unit(
                    name, unit, arguments, directory, spelled, configuration.get("preprocessed_source_sha256")
                )
            except RuntimeError as error:
                units.append({"route": name, "unit": unit["file"], "identity": f"preprocess-failed: {error}"})
                continue
            units.append({"route": name, "unit": unit["file"], "identity": identity, "sites": len(found)})
            rows.extend(found)
            if found:
                source = configuration["source_input"]["DeclaredFile"]
                HARNESS.append(
                    {
                        "unit": f"{name}:{unit['file']}",
                        "entry": found[0]["function"],
                        "dialect": configuration["dialect"],
                        "directory": source["directory"],
                        "spelled": source["spelled"],
                        "options": [
                            list(option.items())[0] for option in configuration.get("preprocessor_options", [])
                        ],
                    }
                )
    if options.corpus:
        report = json.load(open(options.corpus))
        databases = []
        for path in options.corpus_db:
            databases.extend(json.load(open(path)))
        for unit in report["units"]:
            matches = corpus_entry(unit, databases)
            distinct = {json.dumps(m, sort_keys=True) for m in matches}
            if len(distinct) > 1:
                texts = {
                    hashlib.sha256(preprocess(*database_arguments(m), plain=True)).hexdigest()
                    for m in matches
                }
                if len(texts) == 1:
                    distinct = {next(iter(distinct))}
            if len(distinct) != 1:
                units.append({"route": "corpus", "unit": unit["file"], "identity": f"database-matches: {len(distinct)}"})
                continue
            arguments, directory, spelled = database_arguments(matches[0])
            try:
                identity, found = census_unit("corpus", unit, arguments, directory, spelled, None)
            except RuntimeError as error:
                units.append({"route": "corpus", "unit": unit["file"], "identity": f"preprocess-failed: {error}"})
                continue
            units.append({"route": "corpus", "unit": unit["file"], "identity": identity, "sites": len(found)})
            rows.extend(found)
    if options.harness_list:
        with open(options.harness_list, "w") as handle:
            json.dump(HARNESS, handle, indent=1)
    summary = {
        "sites_by_route": Counter(r["route"] for r in rows),
        "by_route_position_today": Counter(
            f"{r['route']} | {r['position']} | leaf={r['boolean_leaf']} | {r['today']}" for r in rows
        ),
        "by_operand_hint_today": Counter(
            f"{r['operand_shape']} | {r['hint_shape']} | {r['wrapper']} | {r['today']}" for r in rows
        ),
        "unit_identity": Counter(u["identity"].split(":")[0] for u in units),
    }
    json.dump(
        {
            "schema": "builtin-expect-census/1",
            "about": __doc__.split("\n\n")[0],
            "summary": {k: dict(sorted(v.items())) for k, v in summary.items()},
            "units": units,
            "sites": rows,
        },
        open(options.out, "w"),
        indent=1,
        sort_keys=False,
    )
    print(json.dumps(summary, indent=1, default=dict))


if __name__ == "__main__":
    sys.exit(main())
