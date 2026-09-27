#!/usr/bin/env python3
"""Bind corpus unit indices to parity source hashes by exact Clang replay.

Usage: replay_corpus_sources.py AFTER_DIAGNOSTICS PARITY_JSONL FRONTEND_CONTEXTS_JSONL
       COMPILE_DATABASES PINNED_SOURCE_ROOTS CHANGED_FUNCTIONS_GZ OUTPUT_JSON

COMPILE_DATABASES and PINNED_SOURCE_ROOTS are comma-separated absolute paths;
the corpus includes both Fluent Bit libraries and Monkey units.

The replay uses the archived Rust request's include paths, definitions,
HistoricalStdin input and parity dialect, then requires the SHA-256 of Clang's
authoritative -E -P bytes to equal a parity source digest. This is evidence
processing, not a gateway audit. Failed/unmatched attempts remain recorded.
"""

import ast
import collections
import gzip
import hashlib
import json
import pathlib
import re
import subprocess
import sys


def read(path):
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as stream:
        return json.load(stream)


def request(debug):
    include = ast.literal_eval(re.search(r"include_paths: (\[.*?\]), definitions:", debug).group(1))
    definitions = ast.literal_eval(re.search(r"definitions: (\[.*?\]), preprocessor_options:", debug).group(1))
    if "preprocessor_options: [], input: HistoricalStdin }" not in debug:
        raise ValueError("request is not empty-options HistoricalStdin")
    return include, definitions


def main():
    if len(sys.argv) != 8:
        raise SystemExit(__doc__)
    diagnostics, parity, contexts, compile_dbs, source_roots, changes, output = map(pathlib.Path, sys.argv[1:8])
    source_roots = [pathlib.Path(value).resolve() for value in str(source_roots).split(",")]
    units = read(diagnostics)["units"]
    wanted = {json.loads(line)["unit_index"] for line in gzip.open(changes, "rt")}
    with contexts.open() as stream:
        frontends = {row["frontend_sha256"]: row["frontend_debug"]
                     for row in (json.loads(line) for line in stream)}
    parity_by_function = collections.defaultdict(set)
    parity_pairs = set()
    with parity.open() as stream:
        for line in stream:
            row = json.loads(line)
            parity_by_function[row["function"]].add((row["source_sha256"], row["frontend_sha256"], row["dialect"]))
            parity_pairs.add((row["source_sha256"], row["frontend_sha256"], row["dialect"]))
    entries = collections.defaultdict(set)
    for database in str(compile_dbs).split(","):
        for entry in read(pathlib.Path(database)):
            path = pathlib.Path(entry["file"])
            if not path.is_absolute():
                path = pathlib.Path(entry["directory"]) / path
            path = path.resolve()
            if any(path.is_relative_to(root) for root in source_roots):
                entries[path.name].add(path)
    dialects = {"C11": "-std=c11", "Gnu99": "-std=gnu99",
                "Gnu11": "-std=gnu11", "Gnu17": "-std=gnu17"}
    cache = {}
    records = []
    for index in sorted(wanted):
        unit = units[index]
        names = {row["function"] for row in unit["functions"]}
        candidates = set().union(*(parity_by_function[name] for name in names))
        source_candidates = [path for path in sorted(entries[pathlib.Path(unit["file"]).name])
                             if str(path).endswith(unit["file"])]
        matches, attempts = [], []
        for path in source_candidates:
            raw = path.read_bytes()
            for source_sha, frontend_sha, dialect in sorted(candidates):
                key = (str(path), frontend_sha, dialect)
                if key not in cache:
                    if (source_sha, frontend_sha, dialect) not in parity_pairs:
                        raise SystemExit("candidate parity tuple vanished")
                    include, definitions = request(frontends[frontend_sha])
                    command = ["/usr/bin/clang", "-x", "c", dialects[dialect], "-E", "-P"]
                    command += ["-I" + value for value in include]
                    command += ["-D" + value for value in definitions]
                    command.append("-")
                    result = subprocess.run(command, input=raw, capture_output=True)
                    cache[key] = {
                        "command": command,
                        "exit": result.returncode,
                        "raw_sha256": hashlib.sha256(raw).hexdigest(),
                        "preprocessed_sha256": hashlib.sha256(result.stdout).hexdigest() if result.returncode == 0 else None,
                        "stderr_sha256": hashlib.sha256(result.stderr).hexdigest(),
                    }
                evidence = cache[key]
                if evidence["exit"] == 0 and evidence["preprocessed_sha256"] == source_sha:
                    matches.append({"source_sha256": source_sha, "frontend_sha256": frontend_sha,
                                    "dialect": dialect, "source_file": str(path), **evidence})
                else:
                    attempts.append({"source_file": str(path), "frontend_sha256": frontend_sha,
                                     "dialect": dialect, "expected_source_sha256": source_sha,
                                     "exit": evidence["exit"],
                                     "observed_preprocessed_sha256": evidence["preprocessed_sha256"]})
        records.append({"unit_index": index, "library": unit["library"], "file": unit["file"],
                        "matches": matches, "attempts": attempts})
        print(f"unit {index}: {len(matches)} exact source match(es), {len(attempts)} nonmatches", flush=True)
    result = {"schema": "qcore.w46-access-path-corpus-replay.v1",
              "source_roots": [str(root) for root in source_roots], "clang": "/usr/bin/clang",
              "units": records}
    output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"matched {sum(bool(row['matches']) for row in records)}/{len(records)} changed units")


if __name__ == "__main__":
    main()
