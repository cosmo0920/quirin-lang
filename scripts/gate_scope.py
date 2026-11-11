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

"""The scoped card gate: run the standing gate's checks on what a diff reaches.

Card GATE-ELASTIC (seam E of docs/plans/pluggable-cards/README.md; user
decision 4 of 2026-10-24: a scoped gate per card, the full standing gate once
per landing, run by the integrator).  Usage::

    gate_scope.py --base <rev> [--plan] [--full] [--no-slots] [--out DIR]

1. **Diff.**  ``git diff --name-only --no-renames <rev>...HEAD`` (both sides of
   a rename), plus every uncommitted or untracked-unignored path, because the
   commands below test the working tree.
2. **Attribution.**  Each path goes to the workspace crate whose directory holds
   it (``cargo metadata`` of the root and the ``providers/`` workspaces), and to
   every crate that reads it:

   * at compile time, by ``include_str!``/``include_bytes!``/``include!`` or a
     ``#[path]`` module (constraint ``CT-FIXTURE-EXTERNAL-READERS``) -- found by
     scanning every tracked ``.rs`` file a crate compiles, including files it
     reaches through ``#[path]``, never from a hand list;
   * at run time, by a string literal naming the path or one of its parent
     directories (``.join("providers")``, ``"contract/profiles"``,
     ``"../../compiler/tests/fixtures"``), resolved against the crate root, the
     source file and the repository root and kept only when it names a tracked
     path.  This over-approximates; it cannot see a path computed from data
     (a manifest entry) or a whole-repository walk filtered afterwards.

   The crates reached are closed over reverse dependencies (normal, dev and
   build edges; across the two workspaces).  A read compiled only into test
   executables claims those alone, with no reverse dependency: an integration
   test (``PKG::test::NAME``), or a library's unit-test executable
   (``PKG::lib::NAME``) for a module reached only through ``#[cfg(test)]``
   declarations and an include or literal inside a ``#[cfg(test)]`` item
   (GATE-SCOPE-LIB-UNIT-TEST-MODULES; ``cfg_test_spans``).
3. **Mode.**  The Rust suite runs whole (``full``) when the diff reaches
   ``qcore-c-gateway``, ``qcore-core``, ``qcore-contract`` or ``qcore-schemas``,
   a workspace manifest or lockfile, the test runner or this scoper, or any path
   that no crate owns or reads and that is not prose, Python, Lean or provider
   tooling, or a record the Python suite checks (``contract/evaluations/**``,
   the evaluation-profile records ``scripts/evaluation_profiles.py`` validates,
   and ``contract/contributions/**``, the per-rule contribution ledgers
   ``scripts/contribution_ledgers.py`` validates; a crate found reading one
   still claims it).  Otherwise it runs ``scripts/run_test_binaries.py --packages`` over
   the affected root-workspace members (its fail-closed ``EXCLUSIVE`` phase
   unchanged).
4. **Steps**, every exit, count and log recorded in ``gate.json``: submodule
   state; fmt; Clippy on the affected crates (whole workspace in full mode);
   the Rust suite; the two reevaluation examples when ``qcore-compiler`` is
   affected; Lean (``scripts/gate_lean.py`` for the changed ``proof/QCore``
   modules, the modules importing them and the audits importing those, or the
   whole ``scripts/check_lean_audits.py`` when a ``proof/`` root changes); the
   Python suite; the provider gate (``providers/README.md``'s commands) when
   ``providers/`` changed.

Heavy steps run under ``scripts/gate_slot.py`` (``rust-suite`` for both Rust
suites, ``lean`` for Lean) unless ``--no-slots`` says the caller already holds
a lock, e.g. the legacy ``flock``.  Exit 0 only if every step exited 0; 1 if a
step failed; 2 if the scope could not be computed (nothing ran).

The scoped gate is a card's gate.  The full standing gate in
``docs/agents/validation.md`` stays the landing gate and the arbiter.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime
import json
import os
from pathlib import Path
import posixpath
import re
import subprocess
import sys
import time
from typing import Iterable, Optional, Sequence

import test_link_config

REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = Path(__file__).resolve().parent

# A diff reaching one of these runs the whole Rust suite (seam E).
FULL_CRATES = ("qcore-c-gateway", "qcore-core", "qcore-contract", "qcore-schemas")
# Workspace-level files: every member builds against them.
WORKSPACE_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml")
WORKSPACE_PREFIXES = (".cargo/",)
# Tools whose change makes their own selection untrustworthy.
FULL_RUST_TOOLS = ("scripts/run_test_binaries.py", "scripts/gate_scope.py",
                   "scripts/test_link_config.py", "scripts/rustc_test_dylib.py")
FULL_LEAN_TOOLS = ("scripts/check_lean_audits.py", "scripts/gate_lean.py")
# Prose and CI configuration: no gate step reads them unless a crate is found
# reading them (and the Python suite, which always runs, may).
PROSE_PREFIXES = ("docs/", "editors/", ".github/")
# Records no crate reads and the Python suite checks (card RECORDS-TOOLING:
# scripts/tests/test_evaluation_profiles.py checks the committed evaluation
# profiles; a string-literal census found no Rust reader; card
# CONTRIBUTION-LEDGERS adds the ledgers scripts/tests/test_contribution_ledgers.py
# checks).  A crate the scanner finds reading one is still selected; only the
# fail-closed fallback is lifted.
PYTHON_RECORD_PREFIXES = ("contract/evaluations/", "contract/contributions/")
LEAN_ROOT_FILES = ("proof/QCore.lean", "proof/AuditAll.lean")
PROBES = ("reevaluation_probe", "reevaluation_followup")
GRANULARITIES = ("directory", "file")
PROBE_CRATE = "qcore-compiler"
# The card's base, given to the Python suite on a card gate (not --landing).
CARD_GATE_BASE_ENV = "QUIRIN_CARD_GATE_BASE"

# The provider gate: providers/README.md "Build, verify and reproduce", with the
# package output in this run's own directory (package.py refuses an existing
# one) and every built package verified, as CI does.  A test keeps this list
# equal to the README block.
PROVIDER_PACKAGE_DIR = "/tmp/qcore-packages"
PROVIDER_COMMANDS = (
    "python3 providers/package.py build --output /tmp/qcore-packages",
    "python3 providers/package.py verify /tmp/qcore-packages/qcore.libevent",
    "cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline",
    "cargo fmt --manifest-path providers/Cargo.toml --all -- --check",
    "cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets "
    "--locked --offline -- -D warnings",
)

INCLUDE_RE = re.compile(r'\b(include_str|include_bytes|include)!\s*\(\s*"((?:[^"\\\n]|\\.)*)"\s*,?\s*\)')
INCLUDE_CONCAT_RE = re.compile(
    r'\b(include_str|include_bytes|include)!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"'
    r'\s*\)\s*,\s*"((?:[^"\\\n]|\\.)*)"\s*,?\s*\)\s*,?\s*\)')
INCLUDE_ANY_RE = re.compile(r'\b(?:include_str|include_bytes|include)!\s*\(')
# Other outer attributes may sit between `#[path]` and the `mod` (`#[cfg_attr(..)]`).
PATH_ATTR_RE = re.compile(r'#\[path\s*=\s*"((?:[^"\\\n]|\\.)*)"\s*\]\s*(?:#\[[^\]"]*\]\s*)*'
                          r'(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)')
MOD_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;', re.M)
STRING_RE = re.compile(r'"((?:[^"\\\n]|\\.)*)"')
JOIN_BARE_RE = re.compile(r'\.join\(\s*"([^"/\\\n]+)"\s*\)')
PLACEHOLDER_RE = re.compile(r'\{[^{}]*\}')
SPLIT_RE = re.compile(r'[\s,;\'`()<>|*\[\]]+')
PARENT_ONLY_RE = re.compile(r'^(?:\.\.?/?)+$')


# --- #[cfg(test)] items (GATE-SCOPE-LIB-UNIT-TEST-MODULES) ---------------------
#
# A library's code under a test-only `#[cfg(...)]` is compiled into that
# library's unit-test executable alone: dependents link the library built
# without `cfg(test)`, and doc-tests too.  The scanner finds the spans of such
# items so a module edge, an include or a literal inside one is attributed to
# that executable.  Anything it cannot classify is treated as ordinary code
# (the crate as a whole, as before): an inner `#![cfg(test)]`, a predicate
# other than `test` / `all(.., test, ..)` / `any` of test-only predicates, an
# attribute on something that is not an item or a `let`, and an item whose end
# it cannot find.

# Crate-wide markers of module structure the scanner cannot see: a generated
# or included module tree (`include!` of Rust code, a `mod $name` in a macro,
# `cfg_attr(.., path = ..)`).  A crate whose library shows one keeps today's
# selection for every file.
OPAQUE_MODULES_RE = re.compile(r'\binclude!\s*\(|\bmod\s+\$|\bcfg_attr\s*\([^\]]*\bpath\s*=')
ITEM_KEYWORDS = frozenset({"fn", "mod", "impl", "trait", "struct", "enum", "union", "type",
                           "const", "static", "use", "extern", "unsafe", "async",
                           "macro_rules", "let"})
SEMICOLON_ITEMS = frozenset({"const", "static", "use", "type", "let"})
OPEN = {"(": ")", "[": "]", "{": "}"}
CLOSE = frozenset(OPEN.values())


def rust_tokens(text: str) -> list[tuple[str, int, str]]:
    """(kind, offset, text) tokens: ``ident``, ``punct`` (one character),
    ``literal`` (strings, characters, numbers) and ``lifetime``; comments and
    whitespace dropped.  Unterminated strings or comments end the token list
    (the rest of the file then holds no span)."""
    tokens: list[tuple[str, int, str]] = []
    index, size = 0, len(text)
    while index < size:
        char = text[index]
        if char.isspace():
            index += 1
            continue
        if text.startswith("//", index):
            end = text.find("\n", index)
            index = size if end < 0 else end + 1
            continue
        if text.startswith("/*", index):
            depth, index = 1, index + 2
            while index < size and depth:
                if text.startswith("/*", index):
                    depth, index = depth + 1, index + 2
                elif text.startswith("*/", index):
                    depth, index = depth - 1, index + 2
                else:
                    index += 1
            if depth:
                return tokens
            continue
        start = index
        raw = re.match(r'(?:b|c)?r(#*)"', text[index:index + 260])
        if raw:
            closing = '"' + raw.group(1)
            end = text.find(closing, index + raw.end())
            if end < 0:
                return tokens
            index = end + len(closing)
            tokens.append(("literal", start, text[start:index]))
            continue
        if char == '"' or (char in "bc" and text.startswith('"', index + 1)):
            index = index + 1 if char == '"' else index + 2
            while index < size and text[index] != '"':
                index += 2 if text[index] == "\\" else 1
            if index >= size:
                return tokens
            index += 1
            tokens.append(("literal", start, text[start:index]))
            continue
        if char == "'" or (char == "b" and text.startswith("'", index + 1)):
            quote = index if char == "'" else index + 1
            if text.startswith("\\", quote + 1):
                end = quote + 2
                while end < size and text[end] != "'" and text[end] != "\n":
                    end += 1
                if end >= size or text[end] != "'":
                    return tokens
                index = end + 1
                tokens.append(("literal", start, text[start:index]))
                continue
            if quote + 2 < size and text[quote + 2] == "'":
                index = quote + 3
                tokens.append(("literal", start, text[start:index]))
                continue
            if char == "'":
                match = re.match(r"'[A-Za-z_][A-Za-z0-9_]*", text[index:index + 260])
                index += match.end() if match else 1
                tokens.append(("lifetime", start, text[start:index]))
                continue
        if char == "r" and text.startswith("#", index + 1) and \
                re.match(r"[A-Za-z_]", text[index + 2:index + 3]):
            match = re.match(r"r#[A-Za-z_][A-Za-z0-9_]*", text[index:index + 260])
            index += match.end()
            tokens.append(("ident", start, text[start:index]))
            continue
        if char.isalnum() or char == "_" or ord(char) > 127:
            while index < size and (text[index].isalnum() or text[index] == "_"
                                    or ord(text[index]) > 127):
                index += 1
            kind = "literal" if char.isdigit() else "ident"
            tokens.append((kind, start, text[start:index]))
            continue
        tokens.append(("punct", start, char))
        index += 1
    return tokens


def _group_end(tokens, index: int) -> Optional[int]:
    """The index of the bracket closing the one at ``index``, or None."""
    stack = []
    for position in range(index, len(tokens)):
        kind, _, value = tokens[position]
        if kind != "punct":
            continue
        if value in OPEN:
            stack.append(OPEN[value])
        elif value in CLOSE:
            if not stack or stack.pop() != value:
                return None
            if not stack:
                return position
    return None


def _split_args(tokens) -> list[list]:
    """Top-level comma-separated arguments of a token list."""
    args, current, depth = [], [], 0
    for token in tokens:
        if token[0] == "punct" and token[2] in OPEN:
            depth += 1
        elif token[0] == "punct" and token[2] in CLOSE:
            depth -= 1
        if depth == 0 and token[0] == "punct" and token[2] == ",":
            args.append(current)
            current = []
        else:
            current.append(token)
    if current:
        args.append(current)
    return args


def cfg_test_only(predicate) -> bool:
    """Whether a `cfg` predicate (tokens) holds only when `test` does."""
    values = [token[2] for token in predicate]
    if values == ["test"]:
        return True
    if len(values) >= 3 and values[0] in ("all", "any") and values[1] == "(" \
            and values[-1] == ")" and _group_end(predicate, 1) == len(predicate) - 1:
        args = _split_args(predicate[2:-1])
        if values[0] == "all":
            return any(cfg_test_only(arg) for arg in args)
        return bool(args) and all(cfg_test_only(arg) for arg in args)
    return False


def _item_end(tokens, index: int) -> Optional[int]:
    """The index of the last token of the item starting at ``index``, or None."""
    position = index
    if position < len(tokens) and tokens[position][2] == "pub":
        position += 1
        if position < len(tokens) and tokens[position][2] == "(":
            group = _group_end(tokens, position)
            if group is None:
                return None
            position = group + 1
    if position >= len(tokens) or tokens[position][0] != "ident" \
            or tokens[position][2] not in ITEM_KEYWORDS:
        return None
    keyword = tokens[position][2]
    following = tokens[position + 1][2] if position + 1 < len(tokens) else ""
    semicolon_only = keyword in SEMICOLON_ITEMS and not (
        keyword == "const" and following in ("fn", "unsafe", "async", "extern"))
    depth = 0
    for current in range(position, len(tokens)):
        kind, _, value = tokens[current]
        if kind != "punct":
            continue
        if value in OPEN:
            depth += 1
        elif value in CLOSE:
            depth -= 1
            if depth < 0:
                return None  # the enclosing group closed first: not an item
            if depth == 0 and value == "}" and not semicolon_only:
                return current
        elif value == ";" and depth == 0:
            return current
    return None


def cfg_test_spans(text: str) -> list[tuple[int, int]]:
    """Character spans ``[start, end)`` of the items a test-only ``#[cfg]``
    gates, each from its first outer attribute to its last character."""
    tokens = rust_tokens(text)
    spans: list[tuple[int, int]] = []
    index = 0
    while index < len(tokens):
        if not (tokens[index][2] == "#" and index + 1 < len(tokens)
                and tokens[index + 1][2] == "["):
            index += 1
            continue
        run_start, test_only, position = index, False, index
        while position + 1 < len(tokens) and tokens[position][2] == "#" \
                and tokens[position + 1][2] == "[":
            close = _group_end(tokens, position + 1)
            if close is None:
                return spans
            body = tokens[position + 2:close]
            if len(body) >= 3 and body[0][2] == "cfg" and body[1][2] == "(" \
                    and _group_end(body, 1) == len(body) - 1 and cfg_test_only(body[2:-1]):
                test_only = True
            position = close + 1
        if test_only:
            end = _item_end(tokens, position)
            if end is not None:
                spans.append((tokens[run_start][1], tokens[end][1] + len(tokens[end][2])))
        index = position
    return spans


def runtime_scanned(crate_dir: str, path: str) -> bool:
    """Run-time reads matter only in code `cargo test` runs: an example or a
    bench is compiled (so its includes count) but not run, except the two
    reevaluation examples the gate runs."""
    for skipped in ("examples", "benches"):
        if path.startswith(f"{crate_dir}/{skipped}/"):
            return posixpath.splitext(posixpath.basename(path))[0] in PROBES
    return True


class ScopeError(Exception):
    """The scope cannot be computed; nothing runs."""


# --- inputs -------------------------------------------------------------------


def git(*args: str, root: Path = REPO_ROOT) -> str:
    completed = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    if completed.returncode != 0:
        raise ScopeError(f"git {' '.join(args)} failed: {completed.stderr.strip()}")
    return completed.stdout


def diff_paths(base: str, root: Path = REPO_ROOT) -> tuple[str, str, list[str], list[str]]:
    """(base sha, head sha, committed paths, uncommitted paths)."""
    base_sha = git("rev-parse", "--verify", "--end-of-options", base + "^{commit}", root=root).strip()
    head_sha = git("rev-parse", "--verify", "HEAD", root=root).strip()
    committed = [p for p in git("diff", "--name-only", "--no-renames", "-z",
                                f"{base_sha}...{head_sha}", root=root).split("\0") if p]
    uncommitted = set()
    entries = git("status", "--porcelain=v1", "-z", "--untracked-files=all", "--no-renames",
                  root=root).split("\0")
    for entry in entries:
        if len(entry) > 3:
            uncommitted.add(entry[3:])
    return base_sha, head_sha, sorted(set(committed)), sorted(uncommitted - set(committed))


def tracked_files(root: Path = REPO_ROOT) -> list[str]:
    return [p for p in git("ls-files", "-z", root=root).split("\0") if p]


@dataclasses.dataclass(frozen=True)
class Target:
    """One cargo target of a crate (``cargo metadata``'s ``targets[]``)."""

    name: str
    kinds: tuple[str, ...]
    src_path: str  # repository-relative, posix
    # cargo's `test` flag: whether `cargo test` builds and runs this target's
    # unit-test executable (a library's is `PKG::lib::NAME`).
    test: bool = True

    @property
    def is_test(self) -> bool:
        """A ``tests/``-style integration test: its own executable, depended on by nothing."""
        return self.kinds == ("test",)

    @property
    def is_library(self) -> bool:
        return bool(set(self.kinds) & {"lib", "rlib", "dylib", "cdylib", "staticlib",
                                       "proc-macro"})


def unit_uid(crate: str, target: Target) -> str:
    """The unit id ``scripts/run_test_binaries.py`` gives this target's executable."""
    return f"{crate}::{'+'.join(target.kinds)}::{target.name}"


@dataclasses.dataclass
class Crate:
    name: str
    workspace: str  # "root" or "providers"
    directory: str  # repository-relative, posix
    deps: set[str]  # names of path dependencies (any kind)
    has_lib: bool = True
    # None: inferred from cargo's auto-discovery layout (synthetic models in tests).
    targets: Optional[list[Target]] = None


def cargo_metadata(manifest: Optional[str], root: Path = REPO_ROOT) -> dict:
    argv = [os.environ.get("CARGO", "cargo"), "metadata", "--no-deps", "--format-version", "1"]
    if manifest:
        argv += ["--manifest-path", str(root / manifest)]
    completed = subprocess.run(argv, cwd=root, capture_output=True, text=True)
    if completed.returncode != 0:
        raise ScopeError(f"cargo metadata ({manifest or 'root'}) failed: {completed.stderr.strip()}")
    return json.loads(completed.stdout)


def crates_from_metadata(document: dict, workspace: str, root: Path = REPO_ROOT) -> list[Crate]:
    members = set(document["workspace_members"])
    crates = []
    for package in document["packages"]:
        if package["id"] not in members:
            continue
        directory = Path(package["manifest_path"]).resolve().parent
        try:
            relative = directory.relative_to(root.resolve()).as_posix()
        except ValueError:
            raise ScopeError(f"{package['name']} lies outside the repository: {directory}") from None
        deps = {dep["name"] for dep in package["dependencies"] if dep.get("path")}
        kinds = {kind for target in package["targets"] for kind in target["kind"]}
        targets = []
        for target in package["targets"]:
            source = Path(target["src_path"]).resolve()
            try:
                src_path = source.relative_to(root.resolve()).as_posix()
            except ValueError:
                raise ScopeError(f"{package['name']} target {target['name']} lies outside the "
                                 f"repository: {source}") from None
            targets.append(Target(target["name"], tuple(target["kind"]), src_path,
                                  bool(target.get("test", True))))
        crates.append(Crate(package["name"], workspace, relative, deps,
                            has_lib=bool(kinds & {"lib", "rlib", "proc-macro"}),
                            targets=targets))
    return crates


def infer_targets(crate: Crate, files: Iterable[str]) -> list[Target]:
    """Cargo's auto-discovered targets, for a crate given without metadata."""
    directory = crate.directory
    targets = []
    for path in sorted(files):
        if not path.startswith(directory + "/") or not path.endswith(".rs"):
            continue
        relative = path[len(directory) + 1:]
        parts = relative.split("/")
        stem = posixpath.splitext(parts[-1])[0]
        if relative == "src/lib.rs":
            targets.append(Target(crate.name.replace("-", "_"), ("lib",), path))
        elif relative == "src/main.rs":
            targets.append(Target(crate.name, ("bin",), path))
        elif relative == "build.rs":
            targets.append(Target("build-script-build", ("custom-build",), path))
        elif len(parts) == 2 and parts[0] in ("tests", "examples", "benches"):
            kind = {"tests": "test", "examples": "example", "benches": "bench"}[parts[0]]
            targets.append(Target(stem, (kind,), path))
        elif len(parts) == 3 and parts[0] in ("tests", "examples", "benches") \
                and parts[2] == "main.rs":
            kind = {"tests": "test", "examples": "example", "benches": "bench"}[parts[0]]
            targets.append(Target(parts[1], (kind,), path))
        elif len(parts) == 3 and parts[:2] == ["src", "bin"]:
            targets.append(Target(stem, ("bin",), path))
    return targets


def provider_path_closure(crates: Sequence[Crate]) -> set[str]:
    """Root-workspace crates the provider workspace builds by path, transitively
    (GATE-SCOPE-PROVIDER-LOCK): a change to their dependency set can leave
    ``providers/Cargo.lock`` stale."""
    by_name = {crate.name: crate for crate in crates}
    queue = sorted({dep for crate in crates if crate.workspace == "providers"
                    for dep in crate.deps if dep in by_name
                    and by_name[dep].workspace == "root"})
    closure = set(queue)
    while queue:
        current = queue.pop()
        for dep in sorted(by_name[current].deps):
            if dep in by_name and by_name[dep].workspace == "root" and dep not in closure:
                closure.add(dep)
                queue.append(dep)
    return closure


def load_crates(root: Path = REPO_ROOT) -> list[Crate]:
    crates = crates_from_metadata(cargo_metadata(None, root), "root", root)
    crates += crates_from_metadata(cargo_metadata("providers/Cargo.toml", root), "providers", root)
    names = [crate.name for crate in crates]
    duplicates = sorted({name for name in names if names.count(name) > 1})
    if duplicates:
        raise ScopeError(f"crate names shared by the two workspaces: {duplicates}")
    return crates


# --- attribution ----------------------------------------------------------------


def ancestors(path: str) -> list[str]:
    """`a/b/c` -> [`a/b/c`, `a/b`, `a`]."""
    parts = path.split("/")
    return ["/".join(parts[:i]) for i in range(len(parts), 0, -1)]


def known_paths(files: Iterable[str]) -> set[str]:
    known: set[str] = set()
    for path in files:
        known.update(ancestors(path))
    return known


def inside_repo(path: str) -> Optional[str]:
    normal = posixpath.normpath(path)
    if normal in (".", "") or normal == ".." or normal.startswith("../") or normal.startswith("/"):
        return None
    return normal


def line_of(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


@dataclasses.dataclass
class Reader:
    crate: str
    target: str  # a file (compile time) or a path prefix (run time)
    kind: str  # "include_str" | "include_bytes" | "include" | "path" | "runtime" | "walker"
    source: str  # file:line
    # The test targets (unit ids) whose executables contain the reading source,
    # when it is compiled into test targets only; None: the crate as a whole
    # (its library, a binary, a build script or an example reads it, or the
    # reading file is in no target the scanner could resolve).
    units: Optional[frozenset] = None


# --- the walker table (R1) -------------------------------------------------------

WALKERS_FILE = SCRIPTS / "gate_scope_walkers.json"
WALKERS_SCHEMA = "quirin-gate-walkers/1"
# A walker entry with this literal attaches its reads to the source itself.
OBSERVED_LITERAL = "*"


def glob_regex(pattern: str) -> "re.Pattern[str]":
    """A repository glob: `*` and `?` within one path segment, `**/` any depth."""
    out, index = "", 0
    while index < len(pattern):
        if pattern.startswith("**/", index):
            out, index = out + "(?:[^/]+/)*", index + 3
        elif pattern.startswith("**", index):
            out, index = out + ".*", index + 2
        elif pattern[index] == "*":
            out, index = out + "[^/]*", index + 1
        elif pattern[index] == "?":
            out, index = out + "[^/]", index + 1
        else:
            out, index = out + re.escape(pattern[index]), index + 1
    return re.compile(out + r"\Z")


def _check_entry(entry: dict, keys: Sequence[str], table: str) -> None:
    for key in keys:
        if not isinstance(entry.get(key), str) or not entry[key]:
            raise ScopeError(f"{table}: entry {entry} needs a non-empty {key!r}")
    if not isinstance(entry.get("reads"), list) or \
            not all(isinstance(p, str) and p for p in entry["reads"]):
        raise ScopeError(f"{table}: entry {entry} needs a list of globs in 'reads'")


def load_table(path: Path = WALKERS_FILE) -> dict:
    """The walker table: ``walkers`` (a literal at a source site, and the files
    that site reads through it) and ``expansions`` (a file whose readers also
    read the files it lists).  A malformed table refuses (nothing runs)."""
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ScopeError(f"walker table {path}: {error}") from None
    if document.get("schema") != WALKERS_SCHEMA:
        raise ScopeError(f"walker table {path}: schema must be {WALKERS_SCHEMA}")
    unknown = set(document) - {"schema", "note", "walkers", "expansions"}
    if unknown:
        raise ScopeError(f"walker table {path}: unknown keys {sorted(unknown)}")
    seen: set[tuple[str, str]] = set()
    for entry in document.get("walkers", []):
        _check_entry(entry, ("source", "literal", "why"), "walker table")
        key = (entry["source"], entry["literal"])
        if key in seen:
            raise ScopeError(f"walker table: {key} is declared twice")
        seen.add(key)
    files: set[str] = set()
    for entry in document.get("expansions", []):
        _check_entry(entry, ("file", "why"), "walker table expansions")
        if entry["file"] in files:
            raise ScopeError(f"walker table: expansion of {entry['file']} is declared twice")
        files.add(entry["file"])
    return {"walkers": list(document.get("walkers", [])),
            "expansions": list(document.get("expansions", []))}


def load_walkers(path: Path = WALKERS_FILE) -> list[dict]:
    return load_table(path)["walkers"]


class Model:
    """Owners and readers of repository paths, from cargo metadata and sources."""

    def __init__(self, crates: Sequence[Crate], files: Sequence[str],
                 read_text=lambda path: (REPO_ROOT / path).read_text(encoding="utf-8",
                                                                     errors="replace"),
                 granularity: str = "file", walkers: Sequence[dict] = (),
                 expansions: Sequence[dict] = ()):
        if granularity not in GRANULARITIES:
            raise ScopeError(f"reader granularity must be one of {GRANULARITIES}")
        self.granularity = granularity
        self.crates = {crate.name: crate for crate in crates}
        self.by_depth = sorted(crates, key=lambda c: -len(c.directory))
        self.files = list(files)
        self.file_set = set(files)
        self.read_text = read_text
        self.known = known_paths(files)
        self.compile_readers: dict[str, list[Reader]] = {}
        self.compile_dirs: dict[str, dict[str, list[Reader]]] = {}
        self.runtime_readers: dict[str, list[Reader]] = {}
        # A crate's run-time literals naming its own paths: they decide which of
        # its test targets a change to one of its own non-Rust files selects.
        self.own_readers: dict[str, list[Reader]] = {}
        self.walker_readers: list[tuple[Reader, "re.Pattern[str]"]] = []
        self.walkers = {(entry["source"], entry["literal"]): entry for entry in walkers}
        self.walker_hits: set[tuple[str, str]] = set()
        # (source, literal) -> {(crate, units)}: who runs each declared site,
        # also for a base path that attributes nothing (for table tooling).
        self.walker_sites: dict[tuple[str, str], set[tuple[str, Optional[frozenset]]]] = {}
        # A file whose every reader also reads the files it lists (a manifest).
        self.expansions = [(entry["file"], [glob_regex(p) for p in entry["reads"]])
                           for entry in expansions]
        # Directory literals no walker entry declares: still attributed by
        # directory (fail-closed), and listed so a test can require a declaration.
        self.undeclared: list[dict] = []
        self.unresolved: list[dict] = []
        self.reverse: dict[str, set[str]] = {name: set() for name in self.crates}
        for crate in crates:
            for dep in crate.deps:
                if dep in self.reverse:
                    self.reverse[dep].add(crate.name)
        self.provider_closure = provider_path_closure(crates)
        self.targets: dict[str, list[Target]] = {}
        # (crate, file) -> unit ids of the test targets compiling it; and the
        # (crate, file) pairs some non-test target compiles.
        self.file_units: dict[tuple[str, str], set[str]] = {}
        self.file_nontest: set[tuple[str, str]] = set()
        # (crate, file) -> [(target, reached by a path of ordinary edges)]
        # for every target whose module closure holds the file.
        self.file_contexts: dict[tuple[str, str], list[tuple[Target, bool]]] = {}
        # Crates whose library shows module structure the scanner cannot see
        # (OPAQUE_MODULES_RE): no cfg(test) narrowing for them.
        self.opaque: dict[str, str] = {}
        self._spans: dict[str, list[tuple[int, int]]] = {}
        # path -> the JSON files that name it (see _scan_json_manifests).
        self.json_lists: dict[str, set[str]] = {}
        self._resolve_targets()
        self._scan()
        self._scan_json_manifests()

    def owner(self, path: str) -> Optional[str]:
        for crate in self.by_depth:
            if path == crate.directory or path.startswith(crate.directory + "/"):
                return crate.name
        return None

    def test_spans(self, path: str) -> list[tuple[int, int]]:
        """The `#[cfg(test)]` item spans of a source (cached; none if unreadable)."""
        if path not in self._spans:
            text = self._text(path)
            self._spans[path] = cfg_test_spans(text) if text is not None else []
        return self._spans[path]

    def in_test_item(self, path: str, offset: int) -> bool:
        return any(start <= offset < end for start, end in self.test_spans(path))

    def _module_children(self, path: str, text: str, is_root: bool) -> list[tuple[str, bool]]:
        """Files a source declares as modules: `#[path]` (relative to its directory)
        and `mod x;` (beside a crate root or `mod.rs`, else under its stem), each
        with whether the declaration sits in a `#[cfg(test)]` item."""
        directory = posixpath.dirname(path)
        stem = posixpath.splitext(posixpath.basename(path))[0]
        base = directory if is_root or stem == "mod" else posixpath.join(directory, stem)
        children = []
        for match in PATH_ATTR_RE.finditer(text):
            target = inside_repo(posixpath.join(directory, match.group(1)))
            if target:
                children.append((target, self.in_test_item(path, match.start())))
        for match in MOD_RE.finditer(text):
            test_only = self.in_test_item(path, match.start(1))
            for candidate in (f"{base}/{match.group(1)}.rs", f"{base}/{match.group(1)}/mod.rs"):
                candidate = inside_repo(candidate)
                if candidate:
                    children.append((candidate, test_only))
        return children

    def _resolve_targets(self) -> None:
        """Each target's module closure, from its root file: every file it
        reaches, and the files it reaches through ordinary (non-test) edges."""
        rs_set = {path for path in self.files if path.endswith(".rs")}
        for name, crate in self.crates.items():
            targets = crate.targets if crate.targets is not None \
                else infer_targets(crate, self.files)
            self.targets[name] = list(targets)
            for target in targets:
                # (path, is_root, reached through ordinary edges only)
                ordinary = {target.src_path}
                closure = {target.src_path}
                queue = [(target.src_path, True, True)]
                while queue:
                    path, is_root, plain = queue.pop()
                    text = self._text(path)
                    if text is None:
                        continue
                    if target.is_library and name not in self.opaque:
                        found = OPAQUE_MODULES_RE.search(text)
                        if found:
                            self.opaque[name] = f"{path}:{line_of(text, found.start())}"
                    for child, test_only in self._module_children(path, text, is_root):
                        if child not in rs_set:
                            continue
                        child_plain = plain and not test_only
                        if child_plain and child not in ordinary:
                            ordinary.add(child)
                            closure.add(child)
                            queue.append((child, False, True))
                        elif child not in closure:
                            closure.add(child)
                            queue.append((child, False, False))
                for path in closure:
                    self.file_contexts.setdefault((name, path), []).append(
                        (target, path in ordinary))
        for (name, path), contexts in self.file_contexts.items():
            for target, plain in contexts:
                if target.is_test:
                    self.file_units.setdefault((name, path), set()).add(unit_uid(name, target))
                elif not plain and self.lib_unit(name, target):
                    # Compiled only into the library's unit-test executable.
                    self.file_units.setdefault((name, path), set()).add(unit_uid(name, target))
                else:
                    self.file_nontest.add((name, path))

    def lib_unit(self, crate: str, target: Target) -> bool:
        """Whether this target's `cfg(test)` code is narrowed to its unit-test
        executable: a library cargo tests, of a crate with no opaque modules."""
        return target.is_library and target.test and crate not in self.opaque

    def reader_units(self, crate: str, path: str,
                     offset: Optional[int] = None) -> Optional[frozenset]:
        """The test executables a reading source at ``offset`` is compiled into,
        or None for the crate as a whole."""
        if offset is not None and crate not in self.opaque and self.in_test_item(path, offset):
            contexts = self.file_contexts.get((crate, path), [])
            units = set()
            for target, _ in contexts:
                if target.is_test or self.lib_unit(crate, target):
                    units.add(unit_uid(crate, target))
                else:
                    return None  # a binary, example or build script compiles it
            return frozenset(units) if units else None
        if (crate, path) in self.file_nontest:
            return None
        units = self.file_units.get((crate, path))
        return frozenset(units) if units else None

    def _compiled_files(self) -> dict[str, set[str]]:
        """Every tracked .rs file each crate compiles: its own, plus #[path] and
        plain `mod` children reached from those outside its directory."""
        rs = [path for path in self.files if path.endswith(".rs")]
        rs_set = set(rs)
        compiled: dict[str, set[str]] = {name: set() for name in self.crates}
        for path in rs:
            owner = self.owner(path)
            if owner is not None:
                compiled[owner].add(path)
        for name, files in compiled.items():
            queue = sorted(files)
            while queue:
                path = queue.pop()
                text = self._text(path)
                if text is None:
                    continue
                directory = posixpath.dirname(path)
                for match in PATH_ATTR_RE.finditer(text):
                    target = inside_repo(posixpath.join(directory, match.group(1)))
                    if target and target in rs_set and target not in files:
                        files.add(target)
                        queue.append(target)
                if self.owner(path) == name:
                    continue  # plain `mod` children inside the crate are already its own
                stem = posixpath.splitext(posixpath.basename(path))[0]
                base = directory if stem == "mod" else posixpath.join(directory, stem)
                for match in MOD_RE.finditer(text):
                    for candidate in (f"{base}/{match.group(1)}.rs",
                                      f"{base}/{match.group(1)}/mod.rs"):
                        candidate = inside_repo(candidate)
                        if candidate and candidate in rs_set and candidate not in files:
                            files.add(candidate)
                            queue.append(candidate)
        return compiled

    def _text(self, path: str) -> Optional[str]:
        try:
            return self.read_text(path)
        except (OSError, KeyError):
            return None  # deleted in the working tree; its readers are scanned elsewhere

    def _add(self, table: dict[str, list[Reader]], reader: Reader) -> None:
        table.setdefault(reader.target, []).append(reader)
        if table is self.compile_readers:
            self.compile_dirs.setdefault(posixpath.dirname(reader.target), {}) \
                .setdefault(reader.target, []).append(reader)

    def _scan(self) -> None:
        for name, files in self._compiled_files().items():
            crate_dir = self.crates[name].directory
            for path in sorted(files):
                text = self._text(path)
                if text is None:
                    continue
                units = self.reader_units(name, path)

                def units_at(offset: int, name=name, path=path) -> Optional[frozenset]:
                    # A read inside a #[cfg(test)] item is compiled into the
                    # unit-test executables of the targets holding the file.
                    return self.reader_units(name, path, offset)

                directory = posixpath.dirname(path)
                resolved_spans = []
                for regex, base in ((INCLUDE_RE, directory), (INCLUDE_CONCAT_RE, crate_dir)):
                    for match in regex.finditer(text):
                        literal = match.group(2)
                        if regex is INCLUDE_CONCAT_RE:
                            literal = literal.lstrip("/")
                        target = inside_repo(posixpath.join(base, literal))
                        resolved_spans.append(match.start())
                        if target is None:
                            self.unresolved.append({"crate": name, "source": f"{path}:{line_of(text, match.start())}",
                                                    "literal": match.group(2), "why": "outside the repository"})
                            continue
                        self._add(self.compile_readers, Reader(
                            name, target, match.group(1), f"{path}:{line_of(text, match.start())}",
                            units_at(match.start())))
                for match in INCLUDE_ANY_RE.finditer(text):
                    if match.start() not in resolved_spans:
                        self.unresolved.append({
                            "crate": name, "source": f"{path}:{line_of(text, match.start())}",
                            "literal": text[match.start():match.start() + 80].split("\n")[0],
                            "why": "argument is not a string literal (or the macro is inside a string)"})
                for match in PATH_ATTR_RE.finditer(text):
                    target = inside_repo(posixpath.join(directory, match.group(1)))
                    if target is not None:
                        self._add(self.compile_readers, Reader(
                            name, target, "path", f"{path}:{line_of(text, match.start())}",
                            units_at(match.start())))
                if runtime_scanned(crate_dir, path):
                    self._scan_literals(name, crate_dir, path, directory, text, units_at)
                    observed = self.walkers.get((path, OBSERVED_LITERAL))
                    if observed is not None:
                        # A read no literal shows (a whole-tree listing filtered
                        # afterwards, a path computed from data): declared at
                        # its source and held to the read census.
                        self.walker_hits.add((path, OBSERVED_LITERAL))
                        for pattern in observed["reads"]:
                            self.walker_readers.append((Reader(
                                name, f"{pattern} (declared reads of {path})", "walker",
                                f"{path}:0", units), glob_regex(pattern)))

    def _scan_literals(self, name: str, crate_dir: str, path: str, directory: str,
                       text: str, units_at=lambda offset: None) -> None:
        # Keyed by the units too: a literal in a #[cfg(test)] item must not
        # hide the same literal in ordinary code (which reaches the crate).
        seen: set[tuple[str, Optional[frozenset]]] = set()

        def consider(piece: str, offset: int) -> None:
            piece = piece.rstrip("/")
            if not piece or PARENT_ONLY_RE.match(piece):
                return
            stripped = re.sub(r"^(?:\.\./|\./|/)+", "", piece)
            units = units_at(offset)
            for candidate in (posixpath.join(crate_dir, piece.lstrip("/")),
                              posixpath.join(directory, piece.lstrip("/")),
                              stripped):
                target = inside_repo(candidate)
                if target is None or target not in self.known or (target, units) in seen:
                    continue
                seen.add((target, units))
                source = f"{path}:{line_of(text, offset)}"
                if target == crate_dir or target.startswith(crate_dir + "/"):
                    # Its own files: the crate owns them; this only says which
                    # of its test targets read them.
                    self._add(self.own_readers, Reader(name, target, "runtime", source, units))
                    continue
                walker = self.walkers.get((path, target))
                if walker is not None:
                    self.walker_hits.add((path, target))
                    self.walker_sites.setdefault((path, target), set()).add((name, units))
                    for pattern in walker["reads"]:
                        self.walker_readers.append((Reader(
                            name, f"{pattern} (walker of {target})", "walker", source, units),
                            glob_regex(pattern)))
                if target in self.file_set:
                    # A literal naming a file attributes exactly that file.
                    self._add(self.runtime_readers, Reader(name, target, "runtime", source, units))
                elif walker is None:
                    # A directory no walker declares: the whole directory, fail-closed.
                    self.undeclared.append({"crate": name, "source": source, "literal": target})
                    self._add(self.runtime_readers, Reader(name, target, "runtime", source, units))

        for match in STRING_RE.finditer(text):
            for chunk in PLACEHOLDER_RE.split(match.group(1)):
                for piece in SPLIT_RE.split(chunk):
                    # `path.rs::test_name` (a test id) names path.rs.
                    piece = piece.split("::", 1)[0]
                    if "/" in piece or (piece in self.file_set and "." in piece):
                        # A bare name counts only as a tracked top-level file
                        # (`cycle19-resource-contracts.json`), never a directory.
                        consider(piece, match.start())
        for match in JOIN_BARE_RE.finditer(text):
            consider(match.group(1), match.start())

    def stale_walkers(self) -> list[dict]:
        """Walker entries whose site the scanner no longer finds."""
        return [entry for key, entry in sorted(self.walkers.items())
                if key not in self.walker_hits]

    def _scan_json_manifests(self) -> None:
        """Data-derived reads: a tracked JSON file that names a tracked path (by
        a string value, relative to its own directory or to the repository
        root) lists it, and every reader of the JSON file reads what it lists.
        Every tracked JSON file is scanned; one nothing reads lists for nobody."""
        for path in self.files:
            if not path.endswith(".json"):
                continue
            text = self._text(path)
            if text is None:
                continue
            try:
                document = json.loads(text)
            except ValueError:
                continue
            base = posixpath.dirname(path)
            stack = [document]
            while stack:
                value = stack.pop()
                if isinstance(value, dict):
                    stack.extend(value.values())
                elif isinstance(value, list):
                    stack.extend(value)
                elif isinstance(value, str) and "/" in value and len(value) < 512:
                    for candidate in (posixpath.join(base, value), value):
                        target = inside_repo(candidate)
                        if target and target in self.known and target != path:
                            self.json_lists.setdefault(target, set()).add(path)

    def readers_of(self, path: str) -> list[Reader]:
        """Every reader of a path, including the readers of a manifest that lists it."""
        found = self._direct_readers(path)
        seen = {path}
        queue = [path]
        while queue:
            current = queue.pop()
            manifests = [manifest for manifest, regexes in self.expansions
                         if any(regex.match(current) for regex in regexes)]
            for prefix in ancestors(current):
                manifests += sorted(self.json_lists.get(prefix, ()))
            for manifest in manifests:
                if manifest in seen:
                    continue
                seen.add(manifest)
                queue.append(manifest)
                for reader in self._direct_readers(manifest):
                    found.append(Reader(reader.crate, f"{reader.target}, which lists {path}",
                                        "expansion", reader.source, reader.units))
        return found

    def _direct_readers(self, path: str) -> list[Reader]:
        if self.granularity == "file":
            found = list(self.compile_readers.get(path, []))
        else:
            # The brief's rule: a directory read by include_str!/#[path] belongs
            # to the reading crate, so a sibling of a read file counts too.
            directory = posixpath.dirname(path)
            found = [reader for readers in self.compile_dirs.get(directory, {}).values()
                     for reader in readers]
        for prefix in ancestors(path):
            found += self.runtime_readers.get(prefix, [])
            found += self.own_readers.get(prefix, [])
        found += [reader for reader, regex in self.walker_readers if regex.match(path)]
        return found

    def owned_units(self, path: str, owner: str, readers: Sequence[Reader]) -> Optional[frozenset]:
        """The test targets of its owner a path selects, or None for the whole crate
        (GATE-SCOPE-FULL-CRATES-TEST-ONLY).

        Narrowed only for a path in no library, binary, build-script or example
        outside `#[cfg(test)]`: a Rust file only test executables compile (an
        integration test's module, or a module the library declares only under
        `#[cfg(test)]`, whose unit is ``PKG::lib::NAME``), or a non-Rust file whose
        readers in its crate are all in test executables.  The crate's manifest,
        build script and the rest of ``src/`` stay crate-wide, and so does a file
        nothing in the crate is found reading (fail-closed)."""
        directory = self.crates[owner].directory
        relative = path[len(directory) + 1:] if path.startswith(directory + "/") else ""
        if relative in ("", "Cargo.toml", "Cargo.lock", "build.rs"):
            return None
        if (owner, path) in self.file_nontest:
            return None
        selected = set(self.file_units.get((owner, path), set()))
        # Under src/, only a Rust module the library reaches through
        # #[cfg(test)] edges alone (GATE-SCOPE-LIB-UNIT-TEST-MODULES) narrows.
        if relative.startswith("src/") and not (path.endswith(".rs") and selected):
            return None
        own = [reader for reader in readers if reader.crate == owner]
        # A literal naming one of its directories shows a test reads somewhere
        # under it, not that this file has no other reader: narrowing needs a
        # reader that names the file itself (or a target that compiles it).
        exact = [reader for reader in own if not (reader.kind == "runtime"
                                                  and reader.target != path)]
        if not selected and not exact:
            return None
        for reader in own:
            if reader.units is None:
                return None
            selected |= reader.units
        return frozenset(selected)

    def reverse_closure(self, names: Iterable[str]) -> dict[str, str]:
        """{crate: the crate it depends on that put it in} for every reverse dependency."""
        reasons: dict[str, str] = {}
        queue = sorted(set(names))
        seen = set(queue)
        while queue:
            current = queue.pop(0)
            for dependent in sorted(self.reverse.get(current, ())):
                if dependent not in seen:
                    seen.add(dependent)
                    reasons[dependent] = current
                    queue.append(dependent)
        return reasons


def real_model(granularity: str = "file", root: Path = REPO_ROOT) -> Model:
    """This repository's model: both workspaces, tracked files and the walker table."""
    table = load_table()
    return Model(load_crates(root), tracked_files(root), granularity=granularity,
                 walkers=table["walkers"], expansions=table["expansions"])


# --- Lean ----------------------------------------------------------------------


def lean_module(path: str) -> Optional[str]:
    if path.startswith("proof/QCore/") and path.endswith(".lean"):
        return path[len("proof/"):-len(".lean")].replace("/", ".")
    return None


def lean_imports(text: str) -> list[str]:
    imports = []
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] == "import":
            imports.append(parts[1])
    return imports


def plan_lean(changed: Sequence[str], files: Sequence[str], exists, read_text) -> dict:
    """The Lean step for these changed paths: none, modules + audits, or all."""
    reasons: list[str] = []
    all_reasons: list[str] = []
    modules: set[str] = set()
    audits: set[str] = set()
    for path in changed:
        if path in FULL_LEAN_TOOLS:
            all_reasons.append(f"{path}: the Lean runner changed")
        if not path.startswith("proof/"):
            continue
        if path in LEAN_ROOT_FILES:
            all_reasons.append(f"{path}: a proof root changed")
        elif not path.endswith(".lean"):
            all_reasons.append(f"{path}: proof configuration changed")
        elif not exists(path):
            all_reasons.append(f"{path}: a Lean file was deleted")
        elif lean_module(path):
            modules.add(lean_module(path))
            reasons.append(f"{path}: module {lean_module(path)} changed")
        elif posixpath.dirname(path) == "proof" and posixpath.basename(path).startswith("Audit"):
            audits.add(posixpath.basename(path))
            reasons.append(f"{path}: audit changed")
        else:
            all_reasons.append(f"{path}: a Lean file outside proof/QCore/ and the audits")
    if all_reasons:
        return {"mode": "all", "reasons": all_reasons, "modules": [], "audits": []}
    if not modules and not audits:
        return {"mode": "none", "reasons": [], "modules": [], "audits": []}
    importers: dict[str, set[str]] = {}
    audit_imports: dict[str, list[str]] = {}
    for path in files:
        if not path.startswith("proof/") or not path.endswith(".lean") or not exists(path):
            continue
        imports = lean_imports(read_text(path))
        module = lean_module(path)
        if module:
            for imported in imports:
                importers.setdefault(imported, set()).add(module)
        elif posixpath.dirname(path) == "proof" and posixpath.basename(path).startswith("Audit") \
                and path != "proof/AuditAll.lean":
            audit_imports[posixpath.basename(path)] = imports
    closure = set(modules)
    queue = sorted(modules)
    while queue:
        current = queue.pop()
        for dependent in sorted(importers.get(current, ())):
            if dependent not in closure:
                closure.add(dependent)
                queue.append(dependent)
                reasons.append(f"module {dependent} imports {current}")
    for audit, imports in sorted(audit_imports.items()):
        hit = sorted(set(imports) & closure)
        if hit and audit not in audits:
            audits.add(audit)
            reasons.append(f"{audit} imports {', '.join(hit)}")
    return {"mode": "modules", "reasons": reasons, "modules": sorted(closure),
            "audits": sorted(audits)}


# --- the plan ------------------------------------------------------------------


def classify(path: str) -> Optional[str]:
    """The tooling class of a path no crate owns, or None."""
    if path.startswith(PROSE_PREFIXES) or ("/" not in path and path.endswith(".md")):
        return "prose"
    if path.startswith("scripts/") or path.endswith(".py"):
        return "python"
    if path.startswith(PYTHON_RECORD_PREFIXES):
        return "records"
    if path.startswith("proof/"):
        return "lean"
    if path.startswith("providers/"):
        return "providers"
    return None


def describe_reader(reader: Reader, path: str) -> str:
    if reader.kind == "runtime":
        return f"names {reader.target} in a string literal"
    if reader.kind == "walker":
        return f"walks {reader.target}"
    if reader.kind == "expansion":
        return f"reads {reader.target}"
    verb = "compiles" if reader.kind == "path" else "reads"
    what = "it" if reader.target == path else f"{reader.target} in its directory"
    return (f"{verb} {what} via #[path]" if reader.kind == "path"
            else f"{verb} {what} with {reader.kind}!")


def plan(paths: Sequence[str], model: Model, *, force_full: bool = False,
         exists=None, read_text=None, lean: bool = True) -> dict:
    exists = exists or (lambda p: (REPO_ROOT / p).exists())
    read_text = read_text or model.read_text
    attribution: dict[str, list[str]] = {}
    # Crate-wide reasons (the crate and its reverse dependencies), and reasons
    # that reach only some of a crate's test targets (no reverse dependencies:
    # nothing depends on a test target).
    direct: dict[str, list[str]] = {}
    unit_reasons: dict[str, dict[str, list[str]]] = {}
    full_reasons: list[str] = ["requested with --full"] if force_full else []
    provider_reasons: list[str] = []
    provider_lock_reasons: list[str] = []
    python_reasons: list[str] = []

    def reach(crate: str, units: Optional[frozenset], reason: str) -> None:
        if units is None:
            direct.setdefault(crate, []).append(reason)
        else:
            for uid in sorted(units):
                unit_reasons.setdefault(crate, {}).setdefault(uid, []).append(reason)

    for path in paths:
        notes: list[str] = []
        owner = model.owner(path)
        readers = model.readers_of(path)
        if owner:
            units = model.owned_units(path, owner, readers)
            reach(owner, units, f"owns {path}")
            notes.append(f"owned by {owner}" + ("" if units is None else
                                                 f" (test targets {', '.join(sorted(units))})"))
        for reader in readers:
            if reader.crate == owner:
                continue  # decided by owned_units above
            reach(reader.crate, reader.units, f"{describe_reader(reader, path)}: {path} "
                                              f"({reader.source})")
            notes.append(f"read by {reader.crate} ({reader.kind}, {reader.source})"
                         + ("" if reader.units is None else
                            f" in {', '.join(sorted(reader.units))}"))
        cls = classify(path) if owner is None else None
        if path in WORKSPACE_FILES or path.startswith(WORKSPACE_PREFIXES):
            full_reasons.append(f"{path}: workspace manifest or toolchain")
        if path in FULL_RUST_TOOLS:
            full_reasons.append(f"{path}: the test runner or this scoper changed")
        if path.startswith("providers/"):
            provider_reasons.append(f"{path}: providers/ changed")
        if path in ("Cargo.toml", "Cargo.lock") or (
                owner in model.provider_closure
                and path == f"{model.crates[owner].directory}/Cargo.toml"):
            provider_lock_reasons.append(
                f"{path}: a manifest or lock the provider workspace builds by path "
                "(GATE-SCOPE-PROVIDER-LOCK)")
        if cls == "python" or path.endswith(".py"):
            python_reasons.append(f"{path}: Python")
        if path.startswith(PYTHON_RECORD_PREFIXES):
            python_reasons.append(f"{path}: a record the Python suite checks")
        if owner is None and cls is None and not readers \
                and path not in WORKSPACE_FILES and not path.startswith(WORKSPACE_PREFIXES):
            full_reasons.append(f"{path}: no crate owns or reads it and it is not prose, "
                                "Python, Lean or provider tooling, or a Python-checked "
                                "record (fail-closed)")
            notes.append("unattributed")
        if cls:
            notes.append(f"class {cls}")
        attribution[path] = notes or ["no crate owns or reads it"]
    for name in FULL_CRATES:
        if name in direct:
            full_reasons.append(f"the diff reaches {name}")
    rdeps = model.reverse_closure(direct)
    affected: dict[str, list[str]] = {name: list(reasons) for name, reasons in direct.items()}
    for name, via in rdeps.items():
        if name not in affected:
            affected[name] = [f"depends on {via}"]
    # A crate reached as a whole runs every target; otherwise only its targets.
    targets: dict[str, dict[str, list[str]]] = {
        name: {uid: reasons for uid, reasons in sorted(units.items())}
        for name, units in sorted(unit_reasons.items()) if name not in affected}
    root_members = sorted(name for name, crate in model.crates.items() if crate.workspace == "root")
    full = bool(full_reasons)
    rust_packages = root_members if full else sorted(
        name for name in affected if model.crates[name].workspace == "root")
    rust_targets = [] if full else sorted(
        uid for name, units in targets.items() if model.crates[name].workspace == "root"
        for uid in units)
    provider_via_core = sorted(
        {name for name in affected if model.crates[name].workspace == "providers"
         and not any(r.startswith("owns ") for r in affected[name])}
        | {name for name in targets if model.crates[name].workspace == "providers"})
    lean = plan_lean(paths, model.files, exists, read_text) if lean else \
        {"mode": "not planned", "reasons": [], "modules": [], "audits": []}
    provider_gate = bool(provider_reasons)
    return {
        "paths": list(paths),
        "attribution": attribution,
        "affected": {name: affected[name] for name in sorted(affected)},
        "targets": targets,
        "mode": "full" if full else "scoped",
        "full_reasons": full_reasons,
        "rust_packages": rust_packages,
        "rust_targets": rust_targets,
        "probes": full or PROBE_CRATE in affected,
        "lean": lean,
        "python_reasons": python_reasons,
        "provider_gate": provider_gate,
        "provider_reasons": provider_reasons,
        # The provider gate's --locked steps already resolve the lock.
        "provider_lock_check": bool(provider_lock_reasons) and not provider_gate,
        "provider_lock_reasons": provider_lock_reasons,
        "provider_members_reached_without_providers_change": [] if provider_reasons
        else provider_via_core,
        "unresolved_includes": model.unresolved,
        "undeclared_directory_literals": model.undeclared,
        "stale_walkers": model.stale_walkers(),
    }


def selects(scope: dict, uid: str) -> bool:
    """Whether a plan's attribution (not its full-mode escalation) runs this unit."""
    crate = uid.split("::", 1)[0]
    return crate in scope["affected"] or uid in scope["targets"].get(crate, {})


# --- running -------------------------------------------------------------------


@dataclasses.dataclass
class Step:
    name: str
    argv: list[str]
    slot: Optional[str] = None  # gate_slot class
    counts: str = ""  # how to read counts: rust-summary | libtest | unittest | lean
    cwd: Optional[str] = None
    note: str = ""
    env: Optional[dict] = None  # added to the inherited environment


def steps_for(scope: dict, out: Path, python: str = sys.executable,
              card_base: Optional[str] = None) -> list[Step]:
    steps = [Step("submodules", ["git", "-C", str(REPO_ROOT), "submodule", "status", "--recursive"],
                  counts="submodules")]
    steps.append(Step("fmt", ["cargo", "fmt", "--all", "--", "--check"]))
    packages = scope["rust_packages"]
    if scope["mode"] == "full":
        steps.append(Step("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]))
        steps.append(Step("rust-suite", [python, str(SCRIPTS / "run_test_binaries.py"),
                                         "--log-dir", str(out / "rust-suite")],
                          slot="rust-suite", counts="rust-summary"))
    elif packages or scope.get("rust_targets"):
        targets = scope.get("rust_targets", [])
        if packages:
            selection = [arg for name in packages for arg in ("-p", name)]
            steps.append(Step("clippy", ["cargo", "clippy", *selection, "--all-targets", "--",
                                         "-D", "warnings"]))
        by_crate: dict[str, list[str]] = {}
        lib_crates: set[str] = set()
        for uid in targets:
            crate, kind, name = uid.split("::", 2)
            if kind == "test":
                by_crate.setdefault(crate, []).append(name)
            else:
                lib_crates.add(crate)  # PKG::lib::NAME, the library's unit tests
        for crate, names in sorted(by_crate.items()):
            filters = [arg for name in names for arg in ("--test", name)]
            steps.append(Step(f"clippy-{crate}", ["cargo", "clippy", "-p", crate, *filters, "--",
                                                  "-D", "warnings"]))
        for crate in sorted(lib_crates):
            # `--profile test` checks the library as its unit-test executable
            # compiles it (cfg(test) on); plain `--lib` would skip that code.
            steps.append(Step(f"clippy-{crate}-lib", ["cargo", "clippy", "-p", crate, "--lib",
                                                      "--profile", "test", "--", "-D",
                                                      "warnings"]))
        argv = [python, str(SCRIPTS / "run_test_binaries.py")]
        if packages:
            argv += ["--packages", ",".join(packages)]
        if targets:
            argv += ["--targets", ",".join(targets)]
        steps.append(Step("rust-suite", [*argv, "--log-dir", str(out / "rust-suite")],
                          slot="rust-suite", counts="rust-summary"))
    if scope.get("provider_lock_check"):
        # GATE-SCOPE-PROVIDER-LOCK: resolving the provider workspace under
        # --locked fails when its lock no longer matches a root manifest.
        steps.append(Step("provider-lock", ["cargo", "metadata", "--locked", "--offline",
                                            "--format-version", "1", "--manifest-path",
                                            str(REPO_ROOT / "providers" / "Cargo.toml")]))
    if scope["probes"]:
        # The test-build configuration (card TEST-STORAGE-BUDGET), so the
        # examples reuse the suite's build instead of rebuilding every crate.
        config = test_link_config.cargo_config_args()
        for probe in PROBES:
            steps.append(Step(f"probe-{probe}", ["cargo", "run", *config, "-p", PROBE_CRATE,
                                                 "--example", probe]))
    lean = scope["lean"]
    if lean["mode"] == "all":
        steps.append(Step("lean", [python, str(SCRIPTS / "check_lean_audits.py")], slot="lean",
                          counts="lean"))
    elif lean["mode"] == "modules":
        argv = [python, str(SCRIPTS / "gate_lean.py")]
        for module in lean["modules"]:
            argv += ["--module", module]
        for audit in lean["audits"]:
            argv += ["--audit", audit]
        steps.append(Step("lean", argv, slot="lean", counts="lean"))
    # CARD-GATE-NEW-AUDIT-MATRIX-COUNT: on a card's gate the Python suite is
    # told the card's base, so the committed matrix's derived Lean-audit list
    # may lag the tree by exactly the audits the card adds or removes; the
    # landing gate (the standing gate, or --landing here) keeps the exact check.
    steps.append(Step("python", [python, "-m", "unittest", "discover", "-s", "scripts/tests"],
                      counts="unittest",
                      env={CARD_GATE_BASE_ENV: card_base} if card_base else None))
    if scope["provider_gate"]:
        packages_dir = str(out / "provider-packages")
        for index, command in enumerate(PROVIDER_COMMANDS):
            argv = command.replace(PROVIDER_PACKAGE_DIR, packages_dir).split()
            if argv[0] == "python3":
                argv[0] = python
            name = ["provider-package-build", "provider-package-verify", "provider-test",
                    "provider-fmt", "provider-clippy"][index]
            slot = "rust-suite" if name == "provider-test" else None
            steps.append(Step(name, argv, slot=slot,
                              counts="libtest" if name == "provider-test" else ""))
            if name == "provider-package-build":
                # Verify every package the build produced, not only libevent.
                steps.append(Step("provider-package-verify-all",
                                  [python, str(SCRIPTS / "gate_scope.py"), "--verify-packages",
                                   packages_dir]))
    return steps


def read_counts(step: Step, log: Path, out: Path) -> dict:
    text = log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    if step.counts == "rust-summary":
        summary = out / "rust-suite" / "summary.json"
        if summary.exists():
            document = json.loads(summary.read_text(encoding="utf-8"))
            totals = dict(document["totals"])
            return {"units": document["units_run"], **totals, "packages": document.get("packages"),
                    "targets": document.get("targets"),
                    "summary": str(summary), "ok": document["ok"]}
        return {"summary": None}
    if step.counts == "libtest":
        sys.path.insert(0, str(SCRIPTS))
        import run_test_binaries as runner  # noqa: E402
        summaries, _ = runner.parse_output(text)
        totals = {key: sum(int(s[key]) for s in summaries) for key in runner.COUNT_KEYS}
        return {"summaries": len(summaries), **totals}
    if step.counts == "unittest":
        match = re.search(r"^Ran (\d+) tests? in", text, re.M)
        skipped = re.search(r"skipped=(\d+)", text)
        return {"tests": int(match.group(1)) if match else None,
                "skipped": int(skipped.group(1)) if skipped else 0,
                "ok": bool(re.search(r"^OK\b", text, re.M))}
    if step.counts == "lean":
        match = re.search(r"^Passed (\d+) Lean audits", text, re.M)
        return {"audits": int(match.group(1)) if match else None}
    if step.counts == "submodules":
        lines = [line for line in text.splitlines() if line.strip()]
        return {"submodules": len(lines),
                "not_checked_out_or_moved": [line for line in lines if line[:1] in "-+U"]}
    return {}


def run_steps(steps: Sequence[Step], out: Path, use_slots: bool, echo=print) -> list[dict]:
    records = []
    for step in steps:
        log = out / f"{step.name}.log"
        argv = list(step.argv)
        receipt = None
        if step.slot and use_slots:
            receipt = out / "slots" / f"{step.name}.json"
            argv = [sys.executable, str(SCRIPTS / "gate_slot.py"), step.slot,
                    "--receipt", str(receipt), "--", *argv]
        echo(f"[{step.name}] $ {' '.join(argv)}")
        started = time.monotonic()
        with open(log, "wb") as handle:
            try:
                env = {**os.environ, **step.env} if step.env else None
                code = subprocess.run(argv, cwd=step.cwd or REPO_ROOT, stdin=subprocess.DEVNULL,
                                      stdout=handle, stderr=subprocess.STDOUT,
                                      env=env).returncode
            except OSError as error:
                handle.write(f"cannot run: {error}\n".encode())
                code = 127
        seconds = round(time.monotonic() - started, 1)
        counts = read_counts(step, log, out)
        if step.name == "submodules" and counts.get("not_checked_out_or_moved"):
            code = code or 1  # an uninitialised or moved submodule fails the gate
        record = {"name": step.name, "argv": argv, "exit": code, "seconds": seconds,
                  "log": str(log), "counts": counts}
        if step.env:
            record["env"] = dict(step.env)
        if receipt is not None:
            record["slot_receipt"] = str(receipt)
        records.append(record)
        echo(f"[{step.name}] exit {code} in {seconds}s {json.dumps(counts) if counts else ''}")
    return records


def verify_packages(directory: Path) -> int:
    """Run `providers/package.py verify` on every package the build produced."""
    packages = sorted(p for p in directory.iterdir() if p.is_dir())
    if not packages:
        print(f"no package directories under {directory}", file=sys.stderr)
        return 1
    worst = 0
    for package in packages:
        code = subprocess.run([sys.executable, str(REPO_ROOT / "providers" / "package.py"),
                               "verify", str(package)], stdout=subprocess.DEVNULL).returncode
        print(f"verify {package.name}: exit {code}")
        worst = worst or code
    print(f"verified {len(packages)} packages")
    return worst


def cross_crate_readers(model: Model) -> dict:
    """Every compile-time read of a path its reader does not own, by directory."""
    rows: dict[tuple, set] = {}
    for target, readers in model.compile_readers.items():
        owner = model.owner(target)
        for reader in readers:
            if reader.crate != owner:
                key = (posixpath.dirname(target), owner or "", reader.crate, reader.kind)
                rows.setdefault(key, set()).add(reader.source)
    return {
        "schema": "quirin-gate-readers/1",
        "reads": [{"directory": d, "owner": o or None, "reader": r, "kind": k,
                   "sites": len(sites)} for (d, o, r, k), sites in sorted(rows.items())],
        "unresolved_includes": model.unresolved,
    }


def describe(scope: dict, echo=print) -> None:
    echo(f"mode: {scope['mode']}")
    for reason in scope["full_reasons"]:
        echo(f"  full because {reason}")
    echo(f"affected crates ({len(scope['affected'])}):")
    for name, reasons in scope["affected"].items():
        shown = reasons[:3] + ([f"... {len(reasons) - 3} more"] if len(reasons) > 3 else [])
        echo(f"  {name}: {'; '.join(shown)}")
    if scope["targets"]:
        echo(f"crates reached only in test targets ({len(scope['targets'])}):")
        for name, units in scope["targets"].items():
            for uid, reasons in units.items():
                shown = reasons[:2] + ([f"... {len(reasons) - 2} more"] if len(reasons) > 2
                                       else [])
                echo(f"  {uid}: {'; '.join(shown)}")
    echo(f"rust suite packages: {', '.join(scope['rust_packages']) or '(none)'}")
    if scope["rust_targets"]:
        echo(f"rust suite test targets ({len(scope['rust_targets'])}): "
             f"{', '.join(scope['rust_targets'])}")
    if scope["provider_lock_check"]:
        echo("provider lock check: yes (cargo metadata --locked on providers/Cargo.toml)")
        for reason in scope["provider_lock_reasons"]:
            echo(f"  {reason}")
    undeclared = scope["undeclared_directory_literals"]
    if undeclared:
        echo(f"undeclared directory literals: {len(undeclared)} (each attributes its whole "
             "directory, fail-closed; declare them in scripts/gate_scope_walkers.json)")
        for site in undeclared[:3]:
            echo(f"  {site['literal']} ({site['source']})")
    for entry in scope["stale_walkers"][:10]:
        echo(f"  stale walker entry: {entry['source']} {entry['literal']}")
    lean = scope["lean"]
    echo(f"lean: {lean['mode']}" + (f" ({len(lean['modules'])} modules, "
                                   f"{len(lean['audits'])} audits)" if lean["mode"] == "modules" else ""))
    for reason in lean["reasons"][:10]:
        echo(f"  {reason}")
    echo(f"provider gate: {'yes' if scope['provider_gate'] else 'no'}")
    if scope["provider_members_reached_without_providers_change"]:
        echo("  (provider members reached through core crates, not gated here: "
             + ", ".join(scope["provider_members_reached_without_providers_change"]) + ")")
    echo("python suite: always")


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="The scoped card gate (card GATE-ELASTIC).")
    parser.add_argument("--base", help="the card's base revision")
    parser.add_argument("--paths", nargs="+", help="scope these paths instead of a diff "
                        "(recorded as explicit; for previews and tests)")
    parser.add_argument("--plan", action="store_true", help="compute and print the scope only")
    parser.add_argument("--full", action="store_true", help="run the whole Rust suite regardless")
    parser.add_argument("--no-slots", action="store_true",
                        help="do not take gate_slot.py slots (the caller holds a lock)")
    parser.add_argument("--out", type=Path, help="log directory "
                        "(default private/gate-scope/<stamp>-<pid>)")
    parser.add_argument("--reader-granularity", choices=GRANULARITIES, default="file",
                        help="a compile-time read claims only the file the macro compiles in "
                             "(default, user decision 2026-10-25) or the read file's whole "
                             "directory (CT-FIXTURE-EXTERNAL-READERS, more conservative)")
    parser.add_argument("--readers", action="store_true",
                        help="print the cross-crate compile-time readers found and exit")
    parser.add_argument("--landing", action="store_true",
                        help="a landing run: the Python suite keeps the exact derived-matrix "
                             "checks (no card base is passed; CARD-GATE-NEW-AUDIT-MATRIX-COUNT)")
    parser.add_argument("--verify-packages", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    if args.verify_packages:
        return verify_packages(args.verify_packages)
    if args.readers:
        try:
            model = real_model()
        except ScopeError as error:
            print(f"gate_scope: {error}", file=sys.stderr)
            return 2
        print(json.dumps(cross_crate_readers(model), indent=1))
        return 0
    if not args.base and not args.paths:
        parser.error("--base or --paths is required")
    card_base = None
    try:
        if args.paths:
            source = {"source": "explicit --paths"}
            paths = sorted(set(args.paths))
        else:
            base_sha, head_sha, committed, uncommitted = diff_paths(args.base)
            source = {"source": "git diff", "base": base_sha, "head": head_sha,
                      "committed": committed, "uncommitted": uncommitted}
            paths = sorted(set(committed) | set(uncommitted))
            card_base = None if args.landing else base_sha
        model = real_model(args.reader_granularity)
        scope = plan(paths, model, force_full=args.full)
    except ScopeError as error:
        print(f"gate_scope: {error}", file=sys.stderr)
        return 2
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = (args.out or REPO_ROOT / "private" / "gate-scope" / f"{stamp}-{os.getpid()}").resolve()
    out.mkdir(parents=True, exist_ok=False)
    describe(scope)
    steps = steps_for(scope, out, card_base=card_base)
    document = {"schema": "quirin-gate-scope/1", **source, "scope": scope,
                "planned_steps": [{"name": s.name, "argv": s.argv, "slot": s.slot} for s in steps],
                "slots": not args.no_slots, "landing": args.landing,
                "card_base": card_base, "started": stamp}
    gate_json = out / "gate.json"
    if args.plan:
        document["ran"] = False
        gate_json.write_text(json.dumps(document, indent=1) + "\n", encoding="utf-8")
        print(f"plan only; gate.json: {gate_json}")
        return 0
    records = run_steps(steps, out, use_slots=not args.no_slots)
    document["ran"] = True
    document["steps"] = records
    document["ok"] = all(record["exit"] == 0 for record in records)
    document["finished"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_json.write_text(json.dumps(document, indent=1) + "\n", encoding="utf-8")
    failed = [record["name"] for record in records if record["exit"] != 0]
    print(f"gate.json: {gate_json}")
    if failed:
        print(f"gate_scope: failed steps: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"gate_scope: {len(records)} steps passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
