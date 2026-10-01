#!/usr/bin/env python3
"""CG30-DIALECTS-FLAGS-TYPES census: dialect, target and extension-type features
in the corpus, system and tail-pause compile databases.

usage: census.py OUT_DIR

Read-only over every input: each compile command is re-run as `clang -E -P`
to standard output from its own directory, with the dependency-output words
(-MD, -MMD, -MF X, -MT X, -MQ X) and the object output (-c, -o X) removed, so
nothing is written beside the build. Writes `census.json` (counts) and
`units.jsonl` (one row per unit) under OUT_DIR, which must not exist.
"""

import collections
import concurrent.futures
import json
import pathlib
import re
import shlex
import subprocess
import sys

DATABASES = {
    "corpus-fluent-bit": "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json",
    "corpus-monkey": "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json",
    "system": "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json",
    "tail-pause-slice": "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-production/compile_commands.json",
}

# Compile-command flags of the card's three axes.
FLAG_PATTERNS = {
    "std": re.compile(r"^-std=(.*)$"),
    "target": re.compile(r"^(?:--target=|-target$|--target$)(.*)$"),
    "mabi": re.compile(r"^-mabi=(.*)$"),
    "m32_m64": re.compile(r"^-m(32|64|x32)$"),
    "arch": re.compile(r"^-arch$"),
    "march": re.compile(r"^-march=(.*)$"),
}

# Extension-type and dialect spellings counted in the preprocessed text.
SPELLINGS = {
    "complex": re.compile(r"\b(?:_Complex|__complex__|_Imaginary|__real__|__imag__|__builtin_complex)\b"),
    "imaginary_literal": re.compile(r"\b(?:\d+\.?\d*(?:[eE][+-]?\d+)?[fFlL]?[ij][fFlL]?)\b"),
    "vector": re.compile(r"\b(?:vector_size|__vector_size__|ext_vector_type|neon_vector_type|neon_polyvector_type|matrix_type|__SV[A-Za-z0-9_]*_t|__rvv_[A-Za-z0-9_]*)\b"),
    "bit_int": re.compile(r"\b(?:_BitInt|_ExtInt)\b"),
    "fixed_point": re.compile(r"\b(?:_Fract|_Accum|_Sat)\b"),
    "c23_keywords": re.compile(r"\b(?:nullptr|constexpr|typeof_unqual|static_assert|alignas|alignof|thread_local)\b"),
    "c23_bool_literals": re.compile(r"\b(?:true|false)\b"),
    "c23_attributes": re.compile(r"\[\["),
    "c2y_countof": re.compile(r"\b(?:_Countof|countof)\b"),
}

DROP_WITH_VALUE = {"-MF", "-MT", "-MQ", "-o"}
DROP = {"-MD", "-MMD", "-c"}


def words_of(entry):
    if "arguments" in entry:
        return list(entry["arguments"])
    return shlex.split(entry["command"])


def flags(words):
    found = collections.defaultdict(list)
    for index, word in enumerate(words):
        for name, pattern in FLAG_PATTERNS.items():
            match = pattern.match(word)
            if match:
                value = match.group(1) if match.groups() else ""
                if word in ("-target", "--target", "-arch") and index + 1 < len(words):
                    value = words[index + 1]
                found[name].append(value)
    return found


def preprocess(entry):
    words = words_of(entry)
    command = [words[0]]
    skip = False
    for word in words[1:]:
        if skip:
            skip = False
            continue
        if word in DROP_WITH_VALUE:
            skip = True
            continue
        if word in DROP:
            continue
        command.append(word)
    command += ["-E", "-P", "-o", "-"]
    result = subprocess.run(
        command, cwd=entry["directory"], capture_output=True, check=False, timeout=600
    )
    if result.returncode != 0:
        return None, result.stderr.decode(errors="replace")[:400]
    return result.stdout.decode(errors="replace"), None


def unit_row(database, entry):
    words = words_of(entry)
    row = {
        "database": database,
        "file": entry["file"],
        "flags": flags(words),
    }
    text, error = preprocess(entry)
    if text is None:
        row["preprocessing_error"] = error
        return row
    raw = pathlib.Path(entry["file"])
    if not raw.is_absolute():
        raw = pathlib.Path(entry["directory"]) / raw
    raw_text = raw.read_text(errors="replace") if raw.is_file() else ""
    row["preprocessed"] = {name: len(p.findall(text)) for name, p in SPELLINGS.items()}
    row["raw_source"] = {name: len(p.findall(raw_text)) for name, p in SPELLINGS.items()}
    return row


def main():
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=False)
    rows = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        futures = []
        for database, path in DATABASES.items():
            for entry in json.loads(pathlib.Path(path).read_text()):
                if not entry["file"].endswith(".c"):
                    continue
                futures.append(pool.submit(unit_row, database, entry))
        for future in futures:
            rows.append(future.result())
    with (out / "units.jsonl").open("w") as handle:
        for row in rows:
            handle.write(json.dumps(row, sort_keys=True) + "\n")
    summary = {}
    for database in DATABASES:
        selected = [row for row in rows if row["database"] == database]
        flag_counts = collections.defaultdict(collections.Counter)
        for row in selected:
            for name, values in row["flags"].items():
                for value in values:
                    flag_counts[name][value] += 1
        spelled = {"preprocessed": collections.Counter(), "raw_source": collections.Counter()}
        units = {"preprocessed": collections.Counter(), "raw_source": collections.Counter()}
        for row in selected:
            for side in ("preprocessed", "raw_source"):
                for name, count in row.get(side, {}).items():
                    spelled[side][name] += count
                    units[side][name] += 1 if count else 0
        summary[database] = {
            "units": len(selected),
            "preprocessing_errors": sum(1 for row in selected if "preprocessing_error" in row),
            "flags": {name: dict(counter) for name, counter in flag_counts.items()},
            "spellings": {side: dict(counter) for side, counter in spelled.items()},
            "units_with_spelling": {side: dict(counter) for side, counter in units.items()},
        }
    (out / "census.json").write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
    print(json.dumps(summary, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
