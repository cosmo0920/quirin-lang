#!/usr/bin/env python3
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
   build edges; across the two workspaces).
3. **Mode.**  The Rust suite runs whole (``full``) when the diff reaches
   ``qcore-c-gateway``, ``qcore-core``, ``qcore-contract`` or ``qcore-schemas``,
   a workspace manifest or lockfile, the test runner or this scoper, or any path
   that no crate owns or reads and that is not prose, Python, Lean or provider
   tooling, or a record the Python suite checks (``contract/evaluations/**``,
   the evaluation-profile records ``scripts/evaluation_profiles.py`` validates;
   a crate found reading one still claims it).  Otherwise it runs ``scripts/run_test_binaries.py --packages`` over
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

REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = Path(__file__).resolve().parent

# A diff reaching one of these runs the whole Rust suite (seam E).
FULL_CRATES = ("qcore-c-gateway", "qcore-core", "qcore-contract", "qcore-schemas")
# Workspace-level files: every member builds against them.
WORKSPACE_FILES = ("Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml")
WORKSPACE_PREFIXES = (".cargo/",)
# Tools whose change makes their own selection untrustworthy.
FULL_RUST_TOOLS = ("scripts/run_test_binaries.py", "scripts/gate_scope.py")
FULL_LEAN_TOOLS = ("scripts/check_lean_audits.py", "scripts/gate_lean.py")
# Prose and CI configuration: no gate step reads them unless a crate is found
# reading them (and the Python suite, which always runs, may).
PROSE_PREFIXES = ("docs/", "editors/", ".github/")
# Records no crate reads and the Python suite checks (card RECORDS-TOOLING:
# scripts/tests/test_evaluation_profiles.py checks the committed evaluation
# profiles; a string-literal census found no Rust reader).  A crate the scanner
# finds reading one is still selected; only the fail-closed fallback is lifted.
PYTHON_RECORD_PREFIXES = ("contract/evaluations/",)
LEAN_ROOT_FILES = ("proof/QCore.lean", "proof/AuditAll.lean")
PROBES = ("reevaluation_probe", "reevaluation_followup")
GRANULARITIES = ("directory", "file")
PROBE_CRATE = "qcore-compiler"

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
PATH_ATTR_RE = re.compile(r'#\[path\s*=\s*"((?:[^"\\\n]|\\.)*)"\s*\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)')
MOD_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;', re.M)
STRING_RE = re.compile(r'"((?:[^"\\\n]|\\.)*)"')
JOIN_BARE_RE = re.compile(r'\.join\(\s*"([^"/\\\n]+)"\s*\)')
PLACEHOLDER_RE = re.compile(r'\{[^{}]*\}')
SPLIT_RE = re.compile(r'[\s,;\'`()<>|*\[\]]+')
PARENT_ONLY_RE = re.compile(r'^(?:\.\.?/?)+$')


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


@dataclasses.dataclass
class Crate:
    name: str
    workspace: str  # "root" or "providers"
    directory: str  # repository-relative, posix
    deps: set[str]  # names of path dependencies (any kind)
    has_lib: bool = True


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
        crates.append(Crate(package["name"], workspace, relative, deps,
                            has_lib=bool(kinds & {"lib", "rlib", "proc-macro"})))
    return crates


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
    kind: str  # "include_str" | "include_bytes" | "include" | "path" | "runtime"
    source: str  # file:line


class Model:
    """Owners and readers of repository paths, from cargo metadata and sources."""

    def __init__(self, crates: Sequence[Crate], files: Sequence[str],
                 read_text=lambda path: (REPO_ROOT / path).read_text(encoding="utf-8",
                                                                     errors="replace"),
                 granularity: str = "file"):
        if granularity not in GRANULARITIES:
            raise ScopeError(f"reader granularity must be one of {GRANULARITIES}")
        self.granularity = granularity
        self.crates = {crate.name: crate for crate in crates}
        self.by_depth = sorted(crates, key=lambda c: -len(c.directory))
        self.files = list(files)
        self.read_text = read_text
        self.known = known_paths(files)
        self.compile_readers: dict[str, list[Reader]] = {}
        self.compile_dirs: dict[str, dict[str, list[Reader]]] = {}
        self.runtime_readers: dict[str, list[Reader]] = {}
        self.unresolved: list[dict] = []
        self.reverse: dict[str, set[str]] = {name: set() for name in self.crates}
        for crate in crates:
            for dep in crate.deps:
                if dep in self.reverse:
                    self.reverse[dep].add(crate.name)
        self._scan()

    def owner(self, path: str) -> Optional[str]:
        for crate in self.by_depth:
            if path == crate.directory or path.startswith(crate.directory + "/"):
                return crate.name
        return None

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
        except OSError:
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
                            name, target, match.group(1), f"{path}:{line_of(text, match.start())}"))
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
                            name, target, "path", f"{path}:{line_of(text, match.start())}"))
                if runtime_scanned(crate_dir, path):
                    self._scan_literals(name, crate_dir, path, directory, text)

    def _scan_literals(self, name: str, crate_dir: str, path: str, directory: str,
                       text: str) -> None:
        seen: set[str] = set()

        def consider(piece: str, offset: int) -> None:
            piece = piece.rstrip("/")
            if not piece or PARENT_ONLY_RE.match(piece):
                return
            stripped = re.sub(r"^(?:\.\./|\./|/)+", "", piece)
            for candidate in (posixpath.join(crate_dir, piece.lstrip("/")),
                              posixpath.join(directory, piece.lstrip("/")),
                              stripped):
                target = inside_repo(candidate)
                if target is None or target not in self.known or target in seen:
                    continue
                if target == crate_dir or target.startswith(crate_dir + "/"):
                    continue  # its own files: already attributed by ownership
                seen.add(target)
                self._add(self.runtime_readers, Reader(
                    name, target, "runtime", f"{path}:{line_of(text, offset)}"))

        for match in STRING_RE.finditer(text):
            for chunk in PLACEHOLDER_RE.split(match.group(1)):
                for piece in SPLIT_RE.split(chunk):
                    if "/" in piece:
                        consider(piece, match.start())
        for match in JOIN_BARE_RE.finditer(text):
            consider(match.group(1), match.start())

    def readers_of(self, path: str) -> list[Reader]:
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
        return found

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


def plan(paths: Sequence[str], model: Model, *, force_full: bool = False,
         exists=None, read_text=None) -> dict:
    exists = exists or (lambda p: (REPO_ROOT / p).exists())
    read_text = read_text or model.read_text
    attribution: dict[str, list[str]] = {}
    direct: dict[str, list[str]] = {}
    full_reasons: list[str] = ["requested with --full"] if force_full else []
    provider_reasons: list[str] = []
    python_reasons: list[str] = []
    for path in paths:
        notes: list[str] = []
        owner = model.owner(path)
        if owner:
            direct.setdefault(owner, []).append(f"owns {path}")
            notes.append(f"owned by {owner}")
        for reader in model.readers_of(path):
            if reader.kind == "runtime":
                how = f"names {reader.target} in a string literal"
            else:
                verb = "compiles" if reader.kind == "path" else "reads"
                what = "it" if reader.target == path else f"{reader.target} in its directory"
                how = (f"{verb} {what} via #[path]" if reader.kind == "path"
                       else f"{verb} {what} with {reader.kind}!")
            direct.setdefault(reader.crate, []).append(f"{how}: {path} ({reader.source})")
            notes.append(f"read by {reader.crate} ({reader.kind}, {reader.source})")
        cls = classify(path) if owner is None else None
        if path in WORKSPACE_FILES or path.startswith(WORKSPACE_PREFIXES):
            full_reasons.append(f"{path}: workspace manifest or toolchain")
        if path in FULL_RUST_TOOLS:
            full_reasons.append(f"{path}: the test runner or this scoper changed")
        if path.startswith("providers/"):
            provider_reasons.append(f"{path}: providers/ changed")
        if cls == "python" or path.endswith(".py"):
            python_reasons.append(f"{path}: Python")
        if path.startswith(PYTHON_RECORD_PREFIXES):
            python_reasons.append(f"{path}: a record the Python suite checks")
        if owner is None and cls is None and not model.readers_of(path) \
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
    root_members = sorted(name for name, crate in model.crates.items() if crate.workspace == "root")
    full = bool(full_reasons)
    rust_packages = root_members if full else sorted(
        name for name in affected if model.crates[name].workspace == "root")
    provider_via_core = sorted(name for name in affected
                               if model.crates[name].workspace == "providers"
                               and not any(r.startswith("owns ") for r in affected[name]))
    lean = plan_lean(paths, model.files, exists, read_text)
    return {
        "paths": list(paths),
        "attribution": attribution,
        "affected": {name: affected[name] for name in sorted(affected)},
        "mode": "full" if full else "scoped",
        "full_reasons": full_reasons,
        "rust_packages": rust_packages,
        "probes": full or PROBE_CRATE in affected,
        "lean": lean,
        "python_reasons": python_reasons,
        "provider_gate": bool(provider_reasons),
        "provider_reasons": provider_reasons,
        "provider_members_reached_without_providers_change": [] if provider_reasons
        else provider_via_core,
        "unresolved_includes": model.unresolved,
    }


# --- running -------------------------------------------------------------------


@dataclasses.dataclass
class Step:
    name: str
    argv: list[str]
    slot: Optional[str] = None  # gate_slot class
    counts: str = ""  # how to read counts: rust-summary | libtest | unittest | lean
    cwd: Optional[str] = None
    note: str = ""


def steps_for(scope: dict, out: Path, python: str = sys.executable) -> list[Step]:
    steps = [Step("submodules", ["git", "-C", str(REPO_ROOT), "submodule", "status", "--recursive"],
                  counts="submodules")]
    steps.append(Step("fmt", ["cargo", "fmt", "--all", "--", "--check"]))
    packages = scope["rust_packages"]
    if scope["mode"] == "full":
        steps.append(Step("clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]))
        steps.append(Step("rust-suite", [python, str(SCRIPTS / "run_test_binaries.py"),
                                         "--log-dir", str(out / "rust-suite")],
                          slot="rust-suite", counts="rust-summary"))
    elif packages:
        selection = [arg for name in packages for arg in ("-p", name)]
        steps.append(Step("clippy", ["cargo", "clippy", *selection, "--all-targets", "--", "-D",
                                     "warnings"]))
        steps.append(Step("rust-suite", [python, str(SCRIPTS / "run_test_binaries.py"),
                                         "--packages", ",".join(packages),
                                         "--log-dir", str(out / "rust-suite")],
                          slot="rust-suite", counts="rust-summary"))
    if scope["probes"]:
        for probe in PROBES:
            steps.append(Step(f"probe-{probe}", ["cargo", "run", "-p", PROBE_CRATE, "--example",
                                                 probe]))
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
    steps.append(Step("python", [python, "-m", "unittest", "discover", "-s", "scripts/tests"],
                      counts="unittest"))
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
                code = subprocess.run(argv, cwd=step.cwd or REPO_ROOT, stdin=subprocess.DEVNULL,
                                      stdout=handle, stderr=subprocess.STDOUT).returncode
            except OSError as error:
                handle.write(f"cannot run: {error}\n".encode())
                code = 127
        seconds = round(time.monotonic() - started, 1)
        counts = read_counts(step, log, out)
        if step.name == "submodules" and counts.get("not_checked_out_or_moved"):
            code = code or 1  # an uninitialised or moved submodule fails the gate
        record = {"name": step.name, "argv": argv, "exit": code, "seconds": seconds,
                  "log": str(log), "counts": counts}
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
    echo(f"rust suite packages: {', '.join(scope['rust_packages']) or '(none)'}")
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
    parser.add_argument("--verify-packages", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    if args.verify_packages:
        return verify_packages(args.verify_packages)
    if args.readers:
        try:
            model = Model(load_crates(), tracked_files())
        except ScopeError as error:
            print(f"gate_scope: {error}", file=sys.stderr)
            return 2
        print(json.dumps(cross_crate_readers(model), indent=1))
        return 0
    if not args.base and not args.paths:
        parser.error("--base or --paths is required")
    try:
        if args.paths:
            source = {"source": "explicit --paths"}
            paths = sorted(set(args.paths))
        else:
            base_sha, head_sha, committed, uncommitted = diff_paths(args.base)
            source = {"source": "git diff", "base": base_sha, "head": head_sha,
                      "committed": committed, "uncommitted": uncommitted}
            paths = sorted(set(committed) | set(uncommitted))
        model = Model(load_crates(), tracked_files(), granularity=args.reader_granularity)
        scope = plan(paths, model, force_full=args.full)
    except ScopeError as error:
        print(f"gate_scope: {error}", file=sys.stderr)
        return 2
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = (args.out or REPO_ROOT / "private" / "gate-scope" / f"{stamp}-{os.getpid()}").resolve()
    out.mkdir(parents=True, exist_ok=False)
    describe(scope)
    steps = steps_for(scope, out)
    document = {"schema": "quirin-gate-scope/1", **source, "scope": scope,
                "planned_steps": [{"name": s.name, "argv": s.argv, "slot": s.slot} for s in steps],
                "slots": not args.no_slots, "started": stamp}
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
