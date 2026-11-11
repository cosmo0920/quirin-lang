#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Reconcile the Cycle 19 source-definition and projection denominators.

The input anchor ledger names exact raw-source definition lines.  This program
does not infer an exclusion from a missing projected row or from a function
name.  For each anchored source file it reconstructs
``compiler_options_compatibility`` and the pinned ``HistoricalStdin`` transport
for every matching compile-database entry, then replays it twice:

* an instrumented preprocessing emits a marker for the branch Clang actually
  selected, hashes the active preprocessed translation unit, and must retain
  the uninstrumented token stream exactly;
* a typed Clang AST dump establishes whether a definition survived that build.

Pinned source files and compile databases are read-only.  Temporary
preprocessed text and AST dumps live below a temporary directory and are
deleted after their hashes and observations have been recorded.
"""

from __future__ import annotations

import argparse
import collections
import gzip
import hashlib
import json
import pathlib
import re
import shlex
import subprocess
import tempfile
from dataclasses import dataclass, field
from typing import Iterable


MARKER_PREFIX = "W42_BRANCH_"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: pathlib.Path):
    raw = path.read_bytes()
    if path.suffix == ".gz":
        raw = gzip.decompress(raw)
    return json.loads(raw)


@dataclass
class Branch:
    directive_line: int
    kind: str
    expression: str
    start_line: int
    end_line: int | None = None


@dataclass
class Group:
    opening_line: int
    branches: list[Branch] = field(default_factory=list)
    endif_line: int | None = None


DIRECTIVE = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)$")


def conditional_groups(lines: list[str]) -> list[Group]:
    groups: list[Group] = []
    stack: list[Group] = []
    for line_number, line in enumerate(lines, 1):
        match = DIRECTIVE.match(line)
        if not match:
            continue
        kind, expression = match.group(1), match.group(2).strip()
        if kind in {"if", "ifdef", "ifndef"}:
            group = Group(line_number)
            group.branches.append(Branch(line_number, kind, expression, line_number + 1))
            groups.append(group)
            stack.append(group)
        elif kind in {"elif", "else"}:
            if not stack:
                raise ValueError(f"unmatched #{kind} at line {line_number}")
            stack[-1].branches[-1].end_line = line_number - 1
            stack[-1].branches.append(
                Branch(line_number, kind, expression, line_number + 1)
            )
        else:
            if not stack:
                raise ValueError(f"unmatched #endif at line {line_number}")
            group = stack.pop()
            group.branches[-1].end_line = line_number - 1
            group.endif_line = line_number
    if stack:
        raise ValueError(f"unterminated conditional at line {stack[-1].opening_line}")
    return groups


def branch_for_line(group: Group, line_number: int) -> int | None:
    for index, branch in enumerate(group.branches):
        assert branch.end_line is not None
        if branch.start_line <= line_number <= branch.end_line:
            return index
    return None


def marker(group_line: int, branch_index: int) -> str:
    return f"{MARKER_PREFIX}{group_line}_{branch_index}"


def instrument_source(
    lines: list[str], groups: Iterable[Group], logical_source: pathlib.Path
) -> str:
    after: dict[int, list[str]] = collections.defaultdict(list)
    before: dict[int, list[str]] = collections.defaultdict(list)
    for group in groups:
        assert group.endif_line is not None
        # Always-active restoration is required because marker/#line lines in
        # an inactive sibling are skipped and still shift later physical lines.
        after[group.endif_line].append(
            f'#line {group.endif_line + 1} "{logical_source}"\n'
        )
        for index, branch in enumerate(group.branches):
            if lines[branch.directive_line - 1].rstrip().endswith("\\"):
                raise ValueError(
                    "continued conditional directives are not instrumented: "
                    f"line {branch.directive_line}"
                )
            after[branch.directive_line].append(
                f"#warning {marker(group.opening_line, index)}\n"
            )
            after[branch.directive_line].append(
                f'#line {branch.directive_line + 1} "{logical_source}"\n'
            )
        if group.branches[-1].kind != "else":
            synthetic = len(group.branches)
            before[group.endif_line].extend(
                [
                    "#else\n",
                    f"#warning {marker(group.opening_line, synthetic)}\n",
                    f'#line {group.endif_line} "{logical_source}"\n',
                ]
            )
    output: list[str] = []
    for line_number, line in enumerate(lines, 1):
        output.extend(before.get(line_number, []))
        output.append(line)
        output.extend(after.get(line_number, []))
    return "".join(output)


def parse_database_argument(value: str) -> tuple[str, pathlib.Path]:
    if "=" not in value:
        raise argparse.ArgumentTypeError("compile databases use LABEL=PATH")
    label, raw_path = value.split("=", 1)
    if not label:
        raise argparse.ArgumentTypeError("compile database label is empty")
    return label, pathlib.Path(raw_path)


def resolve_path(directory: str, value: str) -> pathlib.Path:
    path = pathlib.Path(value)
    if not path.is_absolute():
        path = pathlib.Path(directory) / path
    return path.resolve()


def entry_arguments(entry: dict) -> list[str]:
    if "arguments" in entry:
        return list(entry["arguments"])
    return shlex.split(entry["command"])


def matching_entries(database: list[dict], source_suffix: str) -> list[dict]:
    matches = []
    wanted = "/" + source_suffix.lstrip("/")
    for index, entry in enumerate(database):
        source = resolve_path(entry["directory"], entry["file"])
        if str(source).endswith(wanted):
            identified = dict(entry)
            identified["_database_index"] = index
            matches.append(identified)
    return matches


PAIRED_OPTIONS = {"-o", "-MF", "-MT", "-MQ", "--serialize-diagnostics"}
DROP_OPTIONS = {"-c", "-MD", "-MMD", "-MP", "-MG"}


def analysis_arguments(entry: dict, source: pathlib.Path) -> list[str]:
    original = entry_arguments(entry)
    result: list[str] = []
    skip = False
    for argument in original:
        if skip:
            skip = False
            continue
        if argument in PAIRED_OPTIONS:
            skip = True
            continue
        if argument in DROP_OPTIONS:
            continue
        if argument.startswith("-o") and argument != "-ObjC":
            continue
        candidate = resolve_path(entry["directory"], argument)
        if candidate == source:
            continue
        result.append(argument)
    return result


def historical_arguments(entry: dict, source: pathlib.Path) -> list[str]:
    """Reproduce compiler_options_compatibility's reduced Cycle 19 flags."""
    words = entry_arguments(entry)
    includes = [str(source.parent)]
    definitions = []
    standard = "gnu17"
    cursor = 1
    while cursor < len(words):
        word = words[cursor]
        if word in {"-I", "-isystem"}:
            cursor += 1
            if cursor >= len(words):
                raise ValueError(f"missing path after {word}")
            includes.append(str(resolve_path(entry["directory"], words[cursor])))
        elif word.startswith("-I") and word != "-I":
            includes.append(str(resolve_path(entry["directory"], word[2:])))
        elif word == "-D":
            cursor += 1
            if cursor >= len(words):
                raise ValueError("missing value after -D")
            if "$" not in words[cursor]:
                definitions.append(words[cursor])
        elif word.startswith("-D") and word != "-D":
            if "$" not in word[2:]:
                definitions.append(word[2:])
        elif word.startswith("-std="):
            standard = word[5:]
            if standard not in {"c11", "gnu99", "gnu11", "gnu17"}:
                raise ValueError(f"unsupported historical dialect {standard}")
        cursor += 1
    result = [words[0], "-x", "c", f"-std={standard}"]
    result.extend(f"-I{path}" for path in includes)
    result.extend(f"-D{definition}" for definition in definitions)
    return result


def canonical_token_stream(compiler: str, path: pathlib.Path, cwd: pathlib.Path) -> bytes:
    result = run_checked(
        [compiler, "-x", "c", "-fsyntax-only", "-Xclang", "-dump-tokens", str(path)],
        cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    lines = []
    for line in result.stderr.splitlines():
        lines.append(line.split(b"Loc=<", 1)[0].rstrip())
    return b"\n".join(lines) + b"\n"


def run_checked(arguments: list[str], cwd: pathlib.Path, **kwargs) -> subprocess.CompletedProcess:
    result = subprocess.run(arguments, cwd=cwd, check=False, **kwargs)
    if result.returncode != 0:
        stderr = getattr(result, "stderr", b"")
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")
        raise RuntimeError(
            f"command exited {result.returncode}: {shlex.join(arguments)}\n{stderr[-4000:]}"
        )
    return result


def tree_depth(prefix: str) -> int:
    return len(prefix) // 2


AST_NODE = re.compile(r"^((?:\| |  )*(?:\|-|`-))([A-Za-z][A-Za-z0-9_]*)\b(.*)$")


def ast_definitions(path: pathlib.Path, wanted: set[str]) -> dict[str, list[str]]:
    found: dict[str, list[str]] = {name: [] for name in wanted}
    active: tuple[str, int, str] | None = None
    with path.open(errors="replace") as stream:
        for line in stream:
            match = AST_NODE.match(line)
            if not match:
                continue
            depth = tree_depth(match.group(1))
            kind = match.group(2)
            detail = re.sub(r"0x[0-9a-fA-F]+", "0xADDR", match.group(3).rstrip())
            if active is not None and depth <= active[1]:
                active = None
            if kind == "FunctionDecl":
                for name in wanted:
                    if re.search(rf"\b{re.escape(name)}\s+'", detail):
                        active = (name, depth, detail)
                        break
            elif kind == "CompoundStmt" and active is not None and depth > active[1]:
                found[active[0]].append(active[2])
                active = None
    return found


def normalized_ast_sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for line in stream:
            digest.update(re.sub(rb"0x[0-9a-fA-F]+", b"0xADDR", line))
    return digest.hexdigest()


def source_without_comments_and_literals(source: str) -> str:
    """Blank comments and literals while retaining positions and newlines."""
    output = list(source)
    index = 0
    state = "code"
    while index < len(source):
        if state == "code":
            if source.startswith("//", index):
                output[index] = output[index + 1] = " "
                index += 2
                state = "line-comment"
                continue
            if source.startswith("/*", index):
                output[index] = output[index + 1] = " "
                index += 2
                state = "block-comment"
                continue
            if source[index] in {'"', "'"}:
                output[index] = " "
                state = "string" if source[index] == '"' else "character"
                index += 1
                continue
        elif state == "line-comment":
            if source[index] == "\n":
                state = "code"
            else:
                output[index] = " "
            index += 1
            continue
        elif state == "block-comment":
            if source.startswith("*/", index):
                output[index] = output[index + 1] = " "
                index += 2
                state = "code"
                continue
            if source[index] != "\n":
                output[index] = " "
            index += 1
            continue
        else:
            quote = '"' if state == "string" else "'"
            if source[index] == "\\" and index + 1 < len(source):
                output[index] = output[index + 1] = " "
                index += 2
                continue
            if source[index] == quote:
                output[index] = " "
                state = "code"
            elif source[index] != "\n":
                output[index] = " "
            index += 1
            continue
        index += 1
    return "".join(output)


def verify_definition_site(source: str, name: str, line_number: int) -> int:
    """Return the body-opener line for a definition anchored on a source line."""
    cleaned = source_without_comments_and_literals(source)
    starts = [0]
    starts.extend(match.end() for match in re.finditer("\n", cleaned))
    if line_number < 1 or line_number > len(starts):
        raise ValueError(f"line {line_number} is outside the source")
    line_start = starts[line_number - 1]
    line_end = cleaned.find("\n", line_start)
    if line_end < 0:
        line_end = len(cleaned)
    matches = list(re.finditer(rf"\b{re.escape(name)}\s*\(", cleaned[line_start:line_end]))
    if len(matches) != 1:
        raise ValueError(
            f"expected one {name} signature start on line {line_number}, found {len(matches)}"
        )
    opening = (
        line_start
        + matches[0].start()
        + matches[0].group(0).rfind("(")
        + 1
    )
    depth = 1
    index = opening
    while index < len(cleaned) and depth:
        if cleaned[index] == "(":
            depth += 1
        elif cleaned[index] == ")":
            depth -= 1
        index += 1
    if depth:
        raise ValueError(f"unterminated signature for {name}:{line_number}")
    while index < len(cleaned) and cleaned[index] not in "{;=":
        index += 1
    if index >= len(cleaned) or cleaned[index] != "{":
        raise ValueError(f"anchor is not a function definition: {name}:{line_number}")
    return cleaned.count("\n", 0, index) + 1


def compile_evidence(
    entry: dict,
    source: pathlib.Path,
    instrumented: str,
    groups: list[Group],
    names: set[str],
) -> dict:
    base = historical_arguments(entry, source)
    cwd = pathlib.Path(entry["directory"])
    version = run_checked(
        [base[0], "--version"], cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    ).stdout
    with tempfile.TemporaryDirectory(prefix="qcore-w42-") as raw_temp:
        temp = pathlib.Path(raw_temp)
        uninstrumented = temp / "active-uninstrumented.i"
        uninstrumented_args = base + [
            "-E",
            "-P",
            "-fno-color-diagnostics",
            "-o",
            str(uninstrumented),
            "-",
        ]
        run_checked(
            uninstrumented_args,
            cwd,
            input=source.read_bytes(),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        preprocessed = temp / "active.i"
        preprocess_args = base + [
            "-E",
            "-P",
            "-fno-color-diagnostics",
            "-o",
            str(preprocessed),
            "-",
        ]
        pre = run_checked(
            preprocess_args,
            cwd,
            input=instrumented.encode(),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        stderr = pre.stderr.decode(errors="replace")
        selected = sorted(set(re.findall(rf"{MARKER_PREFIX}[0-9]+_[0-9]+", stderr)))
        original_tokens = canonical_token_stream(base[0], uninstrumented, cwd)
        instrumented_tokens = canonical_token_stream(base[0], preprocessed, cwd)
        if original_tokens != instrumented_tokens:
            original_lines = original_tokens.splitlines()
            instrumented_lines = instrumented_tokens.splitlines()
            mismatch = next(
                (
                    index
                    for index, pair in enumerate(zip(original_lines, instrumented_lines))
                    if pair[0] != pair[1]
                ),
                min(len(original_lines), len(instrumented_lines)),
            )
            raise RuntimeError(
                f"instrumented preprocessing changed the token stream for {source} at "
                f"token {mismatch}: {original_lines[mismatch:mismatch + 3]!r} != "
                f"{instrumented_lines[mismatch:mismatch + 3]!r}"
            )

        ast = temp / "ast.txt"
        ast_args = base + [
            "-fsyntax-only",
            "-fno-color-diagnostics",
            "-Xclang",
            "-ast-dump",
            "-o",
            "-",
            "-",
        ]
        with ast.open("wb") as output:
            run_checked(
                ast_args,
                cwd,
                input=source.read_bytes(),
                stdout=output,
                stderr=subprocess.PIPE,
            )
        definitions = ast_definitions(ast, names)

        command_identity = [
            argument
            for argument in entry_arguments(entry)
            if argument not in {entry.get("output", "")}
        ]
        return {
            "compile_database_index": entry["_database_index"],
            "compiler": {
                "executable": base[0],
                "version_first_line": version.decode(errors="replace").splitlines()[0],
                "version_sha256": sha256_bytes(version),
            },
            "command_sha256": sha256_bytes("\0".join(command_identity).encode()),
            "directory": entry["directory"],
            "transport": "compiler_options_compatibility/HistoricalStdin",
            "preprocess": {
                "exit": 0,
                "active_sha256": sha256_file(preprocessed),
                "active_bytes": preprocessed.stat().st_size,
                "instrumented_token_stream_equals_uninstrumented": True,
                "token_stream_sha256": sha256_bytes(original_tokens),
                "stderr_sha256": sha256_bytes(pre.stderr),
                "selected_branch_markers": selected,
            },
            "ast": {
                "exit": 0,
                "normalized_dump_sha256": normalized_ast_sha256(ast),
                "dump_bytes": ast.stat().st_size,
                "definitions": definitions,
            },
        }


def unit_key(unit: dict) -> tuple[str, str]:
    return unit.get("library", "?"), unit["file"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--diagnostics", type=pathlib.Path, required=True)
    parser.add_argument("--run-identity", type=pathlib.Path, required=True)
    parser.add_argument("--anchors", type=pathlib.Path, required=True)
    parser.add_argument(
        "--compile-db", action="append", type=parse_database_argument, required=True
    )
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    diagnostics = load_json(args.diagnostics)
    run_identity = load_json(args.run_identity)
    anchors = load_json(args.anchors)
    databases = {label: path for label, path in args.compile_db}
    if len(databases) != len(args.compile_db):
        raise SystemExit("duplicate compile database label")
    database_rows = {label: load_json(path) for label, path in databases.items()}

    source_total = sum(len(unit.get("source_functions", [])) for unit in diagnostics["units"])
    projected_total = sum(len(unit.get("functions", [])) for unit in diagnostics["units"])
    gaps: dict[tuple[str, str], list[str]] = {}
    for unit in diagnostics["units"]:
        projected = {row["function"] for row in unit.get("functions", [])}
        missing = sorted(set(unit.get("source_functions", [])) - projected)
        if missing != sorted(unit.get("not_projected", [])):
            raise SystemExit(f"not_projected mismatch in {unit_key(unit)}")
        if missing:
            gaps[unit_key(unit)] = missing

    expected = anchors["expected"]
    observed = {
        "units": len(diagnostics["units"]),
        "source_definitions": source_total,
        "projected_definitions": projected_total,
        "unprojected_definitions": source_total - projected_total,
        "unprojected_units": len(gaps),
    }
    if observed != expected:
        raise SystemExit(f"denominator identity mismatch: {observed!r} != {expected!r}")

    anchored_units = {(row["library"], row["file"]): row for row in anchors["units"]}
    if set(anchored_units) != set(gaps):
        raise SystemExit("anchor unit set does not equal baseline gap unit set")

    rows = []
    compile_evidence_rows = []
    for key in sorted(gaps):
        anchor = anchored_units[key]
        definitions = anchor["definitions"]
        if sorted(definitions) != gaps[key]:
            raise SystemExit(f"anchor definition set mismatch in {key}")
        label = anchor["compile_database"]
        if label not in database_rows:
            raise SystemExit(f"missing compile database {label!r}")
        entries = matching_entries(database_rows[label], anchor["source_suffix"])
        if not entries:
            raise SystemExit(f"no compile entry for {anchor['source_suffix']}")
        source_paths = {
            resolve_path(entry["directory"], entry["file"]) for entry in entries
        }
        if len(source_paths) != 1:
            raise SystemExit(f"ambiguous source paths for {key}: {sorted(map(str, source_paths))}")
        source = source_paths.pop()
        lines = source.read_text().splitlines(keepends=True)
        source_text = "".join(lines)
        groups = conditional_groups(lines)
        relevant_groups: dict[int, Group] = {}
        site_contexts: dict[str, list[dict]] = {}
        for name, sites in definitions.items():
            site_contexts[name] = []
            for line_number in sites:
                if line_number < 1 or line_number > len(lines):
                    raise SystemExit(f"out-of-range anchor for {key}/{name}: {line_number}")
                try:
                    body_open_line = verify_definition_site(source_text, name, line_number)
                except ValueError as error:
                    raise SystemExit(f"invalid anchor {key}/{error}") from error
                enclosing = []
                for group in groups:
                    branch_index = branch_for_line(group, line_number)
                    if branch_index is not None:
                        relevant_groups[group.opening_line] = group
                        enclosing.append(
                            {"group_line": group.opening_line, "source_branch": branch_index}
                        )
                site_contexts[name].append(
                    {
                        "line": line_number,
                        "body_open_line": body_open_line,
                        "enclosing_branches": enclosing,
                    }
                )
        instrumented = instrument_source(
            lines, relevant_groups.values(), pathlib.Path("<stdin>")
        )
        per_entry = [
            compile_evidence(entry, source, instrumented, groups, set(definitions))
            for entry in entries
        ]
        evidence_index = len(compile_evidence_rows)
        compile_evidence_rows.append(
            {
                "library": key[0],
                "file": key[1],
                "compile_database": label,
                "source_suffix": anchor["source_suffix"],
                "source_sha256": sha256_file(source),
                "entries": per_entry,
            }
        )
        for name in gaps[key]:
            sites_out = []
            all_sites_excluded = True
            ast_definitions_seen = []
            for entry_number, evidence in enumerate(per_entry):
                ast_defs = evidence["ast"]["definitions"][name]
                if ast_defs:
                    ast_definitions_seen.append(
                        {"entry": entry_number, "declarations": ast_defs}
                    )
            for site in site_contexts[name]:
                exclusions = []
                for entry_number, evidence in enumerate(per_entry):
                    selected = set(evidence["preprocess"]["selected_branch_markers"])
                    entry_exclusions = []
                    for context in site["enclosing_branches"]:
                        group = relevant_groups[context["group_line"]]
                        selected_indices = [
                            index
                            for index in range(len(group.branches) + 1)
                            if marker(group.opening_line, index) in selected
                        ]
                        # A nested conditional has no marker when an enclosing
                        # branch was excluded.  That is expected; the emitted
                        # outer sibling marker is already positive evidence.
                        if not selected_indices:
                            continue
                        if len(selected_indices) != 1:
                            raise SystemExit(
                                f"branch probe is not total for {key}/{name} "
                                f"group {group.opening_line}: {selected_indices}"
                            )
                        selected_index = selected_indices[0]
                        if selected_index != context["source_branch"]:
                            branch = group.branches[context["source_branch"]]
                            entry_exclusions.append(
                                {
                                    "group_line": group.opening_line,
                                    "source_branch": context["source_branch"],
                                    "source_directive": branch.kind,
                                    "source_expression": branch.expression,
                                    "selected_branch": selected_index,
                                }
                            )
                    if not entry_exclusions:
                        all_sites_excluded = False
                    exclusions.append(
                        {"entry": entry_number, "excluding_branches": entry_exclusions}
                    )
                sites_out.append(
                    {
                        "source_line": site["line"],
                        "body_open_line": site["body_open_line"],
                        "raw_definition_syntax_verified": True,
                        "compile_entries": exclusions,
                    }
                )
            if all_sites_excluded and not ast_definitions_seen:
                classification = "pinned-build-exclusion"
            elif ast_definitions_seen:
                # This full compile-database replay is not the historical
                # reduced-option/stdin importer transport. AST survival is a
                # candidate, not enough evidence to attribute a projection
                # defect to that importer.
                classification = "unresolved"
            else:
                classification = "unresolved"
            unit = next(unit for unit in diagnostics["units"] if unit_key(unit) == key)
            rows.append(
                {
                    "library": key[0],
                    "file": key[1],
                    "function": name,
                    "classification": classification,
                    "source_sites": sites_out,
                    "typed_ast_definitions": ast_definitions_seen,
                    "compile_evidence_index": evidence_index,
                    "unit_status": unit.get("status"),
                    "unit_reason_code": unit.get("reason_code"),
                }
            )

    classifications = collections.Counter(row["classification"] for row in rows)
    for category in (
        "pinned-build-exclusion",
        "enumerator-false-positive",
        "projection-defect",
        "unresolved",
    ):
        classifications[category] += 0
    reason_codes = collections.Counter(unit.get("reason_code") for unit in diagnostics["units"])
    document = {
        "schema_version": 1,
        "card": "W42-UNPROJECTED-DEFINITIONS",
        "measurement_only": True,
        "meaning": {
            "pinned-build-exclusion": (
                "every anchored raw definition site is in a branch for which the exact "
                "pinned compile entry emitted a different branch marker, and no typed AST "
                "definition survived"
            ),
            "projection-defect": (
                "reserved for a definition proved active under the historical importer "
                "transport and absent from its projection; a full compile-database AST "
                "alone does not establish this category"
            ),
            "enumerator-false-positive": (
                "the enumerator row has no anchored raw function-definition token form"
            ),
            "unresolved": "neither positive classification was established",
            "unit_reason_codes": (
                "whole-unit Unknown reasons; they are not assigned to individual missing "
                "definitions"
            ),
        },
        "identity": {
            "diagnostics_path": str(args.diagnostics),
            "diagnostics_sha256": sha256_file(args.diagnostics),
            "diagnostics_json_sha256": sha256_bytes(
                gzip.decompress(args.diagnostics.read_bytes())
                if args.diagnostics.suffix == ".gz"
                else args.diagnostics.read_bytes()
            ),
            "run_identity": {
                "path": str(args.run_identity),
                "sha256": sha256_file(args.run_identity),
                "source_commit": run_identity.get("source_commit"),
                "binary_sha256": run_identity.get("binary_sha256"),
                "exit": run_identity.get("exit"),
            },
            "anchors_sha256": sha256_file(args.anchors),
            "compile_databases": {
                label: {"path": str(path), "sha256": sha256_file(path)}
                for label, path in sorted(databases.items())
            },
            "fluent_bit_revision": diagnostics.get("fluent_bit_revision"),
        },
        "denominators": observed,
        "unit_identity_reconciliation": {
            "library_and_relative_file_units": len(gaps),
            "distinct_basenames": len({pathlib.PurePosixPath(key[1]).name for key in gaps}),
            "basename_collision": {
                "basename": "mk_utils.c",
                "units": [
                    "monkey-libevent/mk_core/mk_utils.c",
                    "monkey-libevent/mk_server/mk_utils.c",
                ],
            },
        },
        "raw_definition_site_census": {
            "verified_definition_names": len(rows),
            "enumerator_false_positives": classifications[
                "enumerator-false-positive"
            ],
        },
        "classification_counts": dict(sorted(classifications.items())),
        "unit_reason_census": {
            "unsupported-syntax": reason_codes["unsupported-syntax"],
            "incomplete-obligation-coverage": reason_codes[
                "incomplete-obligation-coverage"
            ],
            "no_reason_code": reason_codes[None],
        },
        "definitions": rows,
        "compile_evidence": compile_evidence_rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
