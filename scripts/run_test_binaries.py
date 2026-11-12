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

"""Run a cargo workspace's test binaries concurrently, fail-closed.

This is the parallel form of the standing gate's
``cargo test --workspace --no-fail-fast`` (card W44-TEST-SUITE-WALL-TIME).
``cargo test`` runs its test executables one after another; this runner builds
the same executables, then runs them several at a time, longest first, and
runs the known load-sensitive binaries alone afterwards.  It uses only the
Python standard library and the cargo on PATH; nothing is installed.

Phases:

1. **Build.** ``cargo test --workspace --no-run --message-format=json-render-diagnostics``
   builds exactly the targets the plain command builds (examples included) and
   names every test executable (``profile.test`` artifacts).
2. **Capture.** ``cargo test --workspace --no-fail-fast --tests`` is run with a
   ``target.<host>.runner`` that records, for every test executable cargo would
   launch, the argv, working directory and complete environment cargo gave it,
   and then exits without running it.  Replaying that record is how this runner
   preserves ``CARGO_MANIFEST_DIR``, ``CARGO_TARGET_TMPDIR``, the dynamic-library
   path, cwd = package root and everything else exactly: nothing is recomputed.
   The captured set must equal the built set, or the run fails.
3. **Parallel phase.** Every non-exclusive binary, plus one ``cargo test --doc``
   unit for the doc-tests, runs under a job limit, longest recorded duration
   first.
4. **Exclusive phase.** Binaries named in ``EXCLUSIVE`` (constraint 16's
   load-sensitive rows and their siblings) run one at a time with nothing else
   of this runner's running, exactly as ``cargo test`` would run them.

Fail-closed: a unit passes only if it exited 0, printed at least one libtest
summary, and every summary it printed is ``ok`` with zero failures.  A missing
executable, a crash, a signal, an unparsable summary, a build failure, a
captured/built mismatch or a stale ``EXCLUSIVE`` entry all make the run fail.
Every unit's stdout and stderr are written to the log directory together with
``summary.json`` (per-unit outcomes, durations and sorted test lines).

The equivalent reference command stays ``cargo test --workspace --no-fail-fast``.

``--packages a,b`` (card GATE-ELASTIC) restricts every phase to those workspace
members: the build and capture use ``-p a -p b`` instead of ``--workspace``, the
doc-test unit covers the selected members that have a library target, and the
stale-``EXCLUSIVE`` check covers the selected members only.  Every other rule is
unchanged: an ``EXCLUSIVE`` binary of a selected member still runs alone after
the parallel phase, and a stale entry for a selected member still fails the run.
A restricted run is the scoped card gate's suite (``scripts/gate_scope.py``),
never the standing gate's: ``summary.json`` and the totals line name the
selection.

``--targets PKG::test::NAME,...`` (card GATE-SCOPE-PRECISE-SELECTION) adds
integration-test targets of members ``--packages`` does not run whole: each
such member is built and captured with cargo's own ``-p PKG --test NAME``
filters, so exactly those executables run (``EXCLUSIVE`` ones alone, as
always), and no doc-test unit runs for it.  It refuses before building a
malformed id, a non-member, a name that is not an integration test of that
member, a member also named by ``--packages`` and ``--batch-size``; the run
fails closed if a named target is not built and launched, or if anything
outside the selection is.  A library's unit-test executable is named the same
way, ``PKG::lib::NAME`` (card GATE-SCOPE-LIB-UNIT-TEST-MODULES): it is built and
captured with cargo's ``--lib`` filter beside that member's ``--test`` filters,
for a change only that library's ``#[cfg(test)]`` code compiles.  Only the
library's own target is accepted (a library with ``test = false`` refuses).

Every cargo command it runs carries the test-build configuration of
``scripts/test_link_config.py`` (card TEST-STORAGE-BUDGET): no incremental
state, and on Linux dynamically linked test executables, binaries and
examples (``--link dynamic``, the default there).  ``--link static`` builds
as plain ``cargo test`` does; the test set and its results are the same
either way, only the bytes on disk differ.  ``summary.json`` records the mode
and the arguments.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import datetime
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import threading
import time
from typing import Callable, Iterable, Optional, Sequence

import test_link_config

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_WEIGHTS = Path(__file__).resolve().with_name("test_binary_weights.json")

# Test binaries that must not share the machine with any other binary of this
# runner.  Each is run alone, after the parallel phase, with its own internal
# test threads exactly as under `cargo test`.  Keys are unit ids
# (`<package>::<target kind>::<target name>`); an entry whose package belongs to
# the workspace being run but which matches no built binary fails the run, so
# a rename cannot silently drop a load-sensitive row into the parallel phase.
#
# The rule for membership is conservative: constraint 16's load-sensitive rows,
# plus every other binary that launches a process provider under a declared
# wall-clock timeout and CLI checkpoint progress guards, because these are
# wall-clock budgets.
EXCLUSIVE: dict[str, str] = {
    "qcore-audit::test::checkpoint_cli": (
        "the interrupt/resume control requires the first unit checkpoint "
        "within an unchanged 30 s wall-clock budget"
    ),
    "qcore-compiler::test::cycle20_verifier_plugins": (
        "constraint 16: vp04_vp07_vp08_process_lifecycle_and_payload_budgets "
        "asserts a wall-clock budget"
    ),
    "qcore-verifier-host::test::standalone_host": (
        "constraint 16: closed_input_does_not_mask_descendant_timeout_or_"
        "accept_a_partial_request asserts a one-second wall-clock bound"
    ),
    "qcore-verifier-host::test::enforced_process_limits": (
        "process providers under declared timeouts with wall-clock tolerance "
        "rows (constraint 16's Io(Broken pipe) and ETXTBSY shapes here were "
        "repaired by card EXECUTION-QUOTA-LOAD-INDEPENDENCE)"
    ),
    "qcore-verifier-host::test::confined_process_execution": (
        "process providers under a declared timeout (constraint 16's "
        "confined-fixture Io(Broken pipe) row was repaired by card "
        "EXECUTION-QUOTA-LOAD-INDEPENDENCE)"
    ),
    "qcore-verifier-host::test::execution_quota": (
        "process providers under a declared 30 s timeout, and a fork-storm "
        "control that compares task peaks of 3 s sleepers (no RLIMIT_NPROC "
        "row: card EXECUTION-QUOTA-LOAD-INDEPENDENCE)"
    ),
    "qcore-verifier-host::test::package_identity": (
        "process providers under a declared 5 s timeout"
    ),
    "qcore-verifier-host::test::request_consumption": (
        "process providers under a declared 20 s timeout (card "
        "HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING; its outcomes are "
        "measured stall- and order-independent, the membership rule still "
        "applies)"
    ),
    "qcore-audit::test::user_contract_plugins": (
        "checker-plugin process packages under declared 5 s and 300 ms timeouts "
        "(card W45-USER-CONTRACT-PLUGINS)"
    ),
    "qcore-audit::test::subcommand_plugins": (
        "external subcommand plugins under a declared 500 ms timeout, killed "
        "by process group (card W45-AUDIT-SUBCOMMAND-PLUGINS)"
    ),
    "qcore-compiler::test::w3_evidence_v2": (
        "an out-of-process provider under a declared timeout"
    ),
    "qcorec::test::plugin_packages": (
        "process providers launched through the CLI under declared timeouts"
    ),
}

SUMMARY_RE = re.compile(
    r"^test result: (?P<status>ok|FAILED)\. (?P<passed>\d+) passed; "
    r"(?P<failed>\d+) failed; (?P<ignored>\d+) ignored; (?P<measured>\d+) measured; "
    r"(?P<filtered>\d+) filtered out"
)
TEST_LINE_RE = re.compile(r"^test (?P<name>.+) \.\.\. (?P<outcome>ok|FAILED|ignored(?:, .*)?)$")
COUNT_KEYS = ("passed", "failed", "ignored", "measured", "filtered")


@dataclasses.dataclass
class Unit:
    """One process to run: a test executable or the doc-test command."""

    uid: str
    argv: list[str]
    cwd: str
    env: dict[str, str]
    exclusive: bool = False
    weight: float = 1.0
    reason: str = ""


@dataclasses.dataclass
class Result:
    uid: str
    exclusive: bool
    returncode: Optional[int]
    signal: Optional[str]
    error: Optional[str]
    started: float
    finished: float
    summaries: list[dict[str, int | str]]
    tests: list[str]
    stdout_log: str
    stderr_log: str
    executable: str = ""

    @property
    def duration(self) -> float:
        return self.finished - self.started

    @property
    def ok(self) -> bool:
        return unit_ok(self.returncode, self.error, self.summaries)


def unit_ok(returncode: Optional[int], error: Optional[str], summaries: Sequence[dict]) -> bool:
    """Fail-closed verdict: exit 0, at least one summary, every summary clean."""
    if error is not None or returncode != 0 or not summaries:
        return False
    return all(s["status"] == "ok" and s["failed"] == 0 for s in summaries)


def parse_output(text: str) -> tuple[list[dict[str, int | str]], list[str]]:
    """Return the libtest summaries and the sorted `test <name> ... <outcome>` lines."""
    summaries: list[dict[str, int | str]] = []
    tests: list[str] = []
    for line in text.splitlines():
        line = line.rstrip("\r")
        match = SUMMARY_RE.match(line)
        if match:
            summary: dict[str, int | str] = {"status": match["status"]}
            for key in COUNT_KEYS:
                summary[key] = int(match[key])
            summaries.append(summary)
            continue
        match = TEST_LINE_RE.match(line)
        if match:
            tests.append(line)
    return summaries, sorted(tests)


def totals(results: Iterable[Result]) -> dict[str, int]:
    total = {key: 0 for key in COUNT_KEYS}
    total["summaries"] = 0
    for result in results:
        for summary in result.summaries:
            total["summaries"] += 1
            for key in COUNT_KEYS:
                total[key] += int(summary[key])
    return total


def safe_name(uid: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", uid)


class Runner:
    """Runs units; the parallel phase under a job limit, then exclusives alone."""

    def __init__(self, log_dir: Path, jobs: int, echo: Callable[[str], None] = print):
        self.log_dir = log_dir
        self.jobs = jobs
        self.echo = echo
        self._lock = threading.Lock()
        self._live: set[subprocess.Popen] = set()
        self._stopping = False
        # (uid, exclusive, start, end) for every unit, for exclusivity evidence.
        self.intervals: list[tuple[str, bool, float, float]] = []

    def run_unit(self, unit: Unit) -> Result:
        name = safe_name(unit.uid)
        stdout_path = self.log_dir / f"{name}.stdout.log"
        stderr_path = self.log_dir / f"{name}.stderr.log"
        returncode: Optional[int] = None
        sig: Optional[str] = None
        error: Optional[str] = None
        started = time.monotonic()
        with open(stdout_path, "wb") as out, open(stderr_path, "wb") as err:
            try:
                with self._lock:
                    if self._stopping:
                        raise RuntimeError("runner interrupted before start")
                    process = subprocess.Popen(
                        unit.argv,
                        cwd=unit.cwd,
                        env=unit.env,
                        stdin=subprocess.DEVNULL,
                        stdout=out,
                        stderr=err,
                    )
                    self._live.add(process)
                try:
                    returncode = process.wait()
                finally:
                    with self._lock:
                        self._live.discard(process)
            except (OSError, RuntimeError, ValueError) as exc:
                error = f"{type(exc).__name__}: {exc}"
        finished = time.monotonic()
        if returncode is not None and returncode < 0:
            try:
                sig = signal.Signals(-returncode).name
            except ValueError:
                sig = f"signal {-returncode}"
        text = stdout_path.read_text(encoding="utf-8", errors="replace")
        summaries, tests = parse_output(text)
        if error is None and returncode == 0 and not summaries:
            error = "exited 0 without a libtest summary"
        result = Result(
            uid=unit.uid,
            exclusive=unit.exclusive,
            returncode=returncode,
            signal=sig,
            error=error,
            started=started,
            finished=finished,
            summaries=summaries,
            tests=tests,
            stdout_log=str(stdout_path),
            stderr_log=str(stderr_path),
            executable=unit.argv[0] if unit.argv else "",
        )
        with self._lock:
            self.intervals.append((unit.uid, unit.exclusive, started, finished))
        self.echo(describe(result))
        return result

    def terminate(self) -> None:
        with self._lock:
            self._stopping = True
            live = list(self._live)
        for process in live:
            try:
                process.terminate()
            except OSError:
                pass

    def run(self, units: Sequence[Unit]) -> list[Result]:
        parallel = sorted(
            (u for u in units if not u.exclusive), key=lambda u: (-u.weight, u.uid)
        )
        exclusive = sorted((u for u in units if u.exclusive), key=lambda u: u.uid)
        results: list[Result] = []
        pool = concurrent.futures.ThreadPoolExecutor(max_workers=self.jobs)
        try:
            futures = [pool.submit(self.run_unit, unit) for unit in parallel]
            for future in futures:
                results.append(future.result())
        except BaseException:
            self.terminate()
            pool.shutdown(wait=True, cancel_futures=True)
            raise
        pool.shutdown(wait=True)
        # Every parallel unit has finished (the pool is shut down) before any
        # exclusive unit starts, and exclusive units run strictly one at a time.
        try:
            for unit in exclusive:
                results.append(self.run_unit(unit))
        except BaseException:
            self.terminate()
            raise
        return results


def print_failure_excerpt(uid: str, result: Result, limit: int = 200) -> None:
    """Print a failed unit's captured `failures:` section (or its tail) and its
    stderr tail, at most `limit` lines each, so a CI log carries the panics."""
    def read(path: str) -> list[str]:
        try:
            return Path(path).read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError as error:
            return [f"(unreadable: {error})"]

    stdout = read(result.stdout_log)
    start = next((i for i, line in enumerate(stdout) if line.strip() == "failures:"), None)
    excerpt = stdout[start:] if start is not None else stdout[-limit:]
    stderr = read(result.stderr_log)[-limit:]
    print(f"::group::{uid} failure output", file=sys.stderr)
    for line in excerpt[:limit]:
        print(f"  | {line}", file=sys.stderr)
    if len(excerpt) > limit:
        print(f"  | ... {len(excerpt) - limit} more lines in {result.stdout_log}", file=sys.stderr)
    if any(line.strip() for line in stderr):
        print("  stderr tail:", file=sys.stderr)
        for line in stderr:
            print(f"  | {line}", file=sys.stderr)
    print("::endgroup::", file=sys.stderr)


def describe(result: Result) -> str:
    status = "ok" if result.ok else "FAILED"
    counts = totals([result])
    detail = ""
    if result.signal:
        detail = f" (killed by {result.signal})"
    elif result.error:
        detail = f" ({result.error})"
    elif result.returncode not in (0, None):
        detail = f" (exit {result.returncode})"
    mode = " [exclusive]" if result.exclusive else ""
    return (
        f"{status:6} {result.duration:8.2f}s {result.uid}{mode}: "
        f"{counts['passed']} passed; {counts['failed']} failed; "
        f"{counts['ignored']} ignored{detail}"
    )


# --- cargo integration -------------------------------------------------------


def host_triple(cargo_env: dict[str, str]) -> str:
    output = subprocess.run(
        ["rustc", "-vV"], capture_output=True, text=True, check=True, env=cargo_env
    ).stdout
    for line in output.splitlines():
        if line.startswith("host: "):
            return line.split(": ", 1)[1].strip()
    raise RuntimeError("rustc -vV reported no host triple")


def metadata(cargo: str, extra: Sequence[str]) -> dict:
    metadata_args = [a for a in extra if a in ("--locked", "--offline", "--frozen")]
    manifest = manifest_args(extra)
    output = subprocess.run(
        [cargo, "metadata", "--no-deps", "--format-version", "1", *manifest, *metadata_args],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return json.loads(output)


def package_names(cargo: str, extra: Sequence[str]) -> dict[str, str]:
    """Map package id -> package name for the workspace members."""
    return {package["id"]: package["name"] for package in metadata(cargo, extra)["packages"]}


LIBRARY_KINDS = frozenset({"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"})


def library_packages(document: dict) -> set[str]:
    """Workspace members with a library target (the only ones with doc-tests)."""
    return {
        package["name"]
        for package in document["packages"]
        if any(LIBRARY_KINDS & set(target["kind"]) for target in package["targets"])
    }


def parse_packages(values: Optional[Sequence[str]]) -> Optional[list[str]]:
    """`--packages a,b --packages c` -> sorted unique names; None = whole workspace."""
    if values is None:
        return None
    names = sorted({name.strip() for value in values for name in value.split(",") if name.strip()})
    if not names:
        raise ValueError("--packages names no package")
    return names


def parse_targets(values: Optional[Sequence[str]], document: dict,
                  packages: Optional[Sequence[str]]
                  ) -> Optional[dict[str, list[tuple[str, str]]]]:
    """`--targets a::test::x,b::lib::b` -> {package: [(kind, name)]}; None = none named.

    A unit id is ``PKG::test::NAME`` (an integration test) or the library's
    unit-test executable ``PKG::KIND::NAME`` exactly as the build names it
    (``KIND`` its target kinds joined by ``+``, usually ``lib``; card
    GATE-SCOPE-LIB-UNIT-TEST-MODULES).  Refuses (ValueError) a malformed id, a
    non-member, a name that is not such a target of that member (a library
    cargo does not test included), and a member --packages already runs
    whole: a misspelt target must not shrink the suite."""
    if values is None:
        return None
    uids = sorted({uid.strip() for value in values for uid in value.split(",") if uid.strip()})
    if not uids:
        raise ValueError("--targets names no target")
    tests: dict[str, set[str]] = {}
    libraries: dict[str, tuple[str, str]] = {}
    for package in document["packages"]:
        tests[package["name"]] = {target["name"] for target in package["targets"]
                                  if target["kind"] == ["test"]}
        for target in package["targets"]:
            if LIBRARY_KINDS & set(target["kind"]) and target.get("test", True):
                libraries[package["name"]] = ("+".join(target["kind"]), target["name"])
    whole = set(packages or ())
    selected: dict[str, list[tuple[str, str]]] = {}
    for uid in uids:
        parts = uid.split("::")
        if len(parts) != 3:
            raise ValueError(f"--targets takes PKG::test::NAME or PKG::lib::NAME unit ids, "
                             f"not {uid!r}")
        package, kind, name = parts
        if package not in tests:
            raise ValueError(f"--targets names a non-member of the workspace: {package}")
        if kind == "test":
            if name not in tests[package]:
                raise ValueError(f"--targets: {package} has no integration-test target {name!r}")
        elif libraries.get(package) != (kind, name):
            raise ValueError(f"--targets: {uid!r} is neither an integration test nor the "
                             f"unit-test executable of {package}'s library")
        if package in whole:
            raise ValueError(f"--targets: {package} already runs whole under --packages")
        selected.setdefault(package, []).append((kind, name))
    return selected


def target_filters(selection: Sequence[tuple[str, str]]) -> list[str]:
    """cargo's own filters for (kind, name) pairs: ``--lib`` or ``--test NAME``."""
    filters: list[str] = []
    for kind, name in sorted(selection):
        filters += ["--test", name] if kind == "test" else ["--lib"]
    return filters


def selection_args(packages: Optional[Sequence[str]]) -> list[str]:
    """cargo package selection: the whole workspace, or `-p` per selected member."""
    if packages is None:
        return ["--workspace"]
    args: list[str] = []
    for name in packages:
        args += ["-p", name]
    return args


def manifest_args(extra: Sequence[str]) -> list[str]:
    result = []
    items = list(extra)
    for index, arg in enumerate(items):
        if arg == "--manifest-path" and index + 1 < len(items):
            result += [arg, items[index + 1]]
        elif arg.startswith("--manifest-path="):
            result.append(arg)
    return result


def unit_id(package: str, target: dict) -> str:
    return f"{package}::{'+'.join(target['kind'])}::{target['name']}"


def build(cargo: str, extra: Sequence[str], log_dir: Path,
          packages: Optional[Sequence[str]] = None, filters: Sequence[str] = (),
          label: str = "") -> tuple[int, dict[str, str]]:
    """Build like `cargo test` does; return (exit, executable -> unit id).

    ``filters`` (cargo target filters such as ``--test NAME``) restrict the
    build to one batch of ``--batch-size``; none builds what ``cargo test``
    builds.
    """
    names = package_names(cargo, extra)
    argv = [cargo, "test", *selection_args(packages), "--no-run",
            "--message-format=json-render-diagnostics", *filters, *extra]
    print(f"$ {shlex.join(argv)}", flush=True)
    executables: dict[str, str] = {}
    with open(log_dir / f"build{label}.stdout.jsonl", "wb") as raw:
        process = subprocess.Popen(argv, stdout=subprocess.PIPE)
        assert process.stdout is not None
        for line in process.stdout:
            raw.write(line)
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if message.get("reason") != "compiler-artifact":
                continue
            if not message.get("profile", {}).get("test") or not message.get("executable"):
                continue
            package = names.get(message["package_id"])
            if package is None:
                raise RuntimeError(f"artifact of a non-member package: {message['package_id']}")
            executables[message["executable"]] = unit_id(package, message["target"])
        code = process.wait()
    return code, executables


# The `target.<host>.runner` cargo launches instead of each test executable. It
# records the launch -- argv, cwd and the complete environment cargo built --
# and exits 0 without running the test. Kept inline and import-light (`-I -S`)
# because cargo starts it once per test executable.
CAPTURE_SOURCE = """\
import json, os, sys, time
record = {"sequence": time.time_ns(), "argv": sys.argv[2:], "cwd": os.getcwd(),
          "env": dict(os.environ)}
path = os.path.join(sys.argv[1], "%d-%d.json" % (record["sequence"], os.getpid()))
with open(path, "w", encoding="utf-8") as handle:
    json.dump(record, handle)
"""


def capture_command(capture_dir: Path) -> list[str]:
    """The runner argv; cargo appends `<executable> <args...>` to it."""
    return [sys.executable, "-I", "-S", "-c", CAPTURE_SOURCE, str(capture_dir.resolve())]


def capture(cargo: str, extra: Sequence[str], log_dir: Path,
            packages: Optional[Sequence[str]] = None, filters: Sequence[str] = (),
            label: str = "") -> tuple[int, list[dict]]:
    """Record cargo's own launch of every test executable without running it."""
    capture_dir = log_dir / f"capture{label}"
    capture_dir.mkdir()
    runner = capture_command(capture_dir)
    triple = host_triple(dict(os.environ))
    config = f"target.{triple}.runner=[{', '.join(json.dumps(part) for part in runner)}]"
    selection = selection_args(packages)
    targets = list(filters) or ["--tests"]
    argv = [cargo, "test", *selection, "--no-fail-fast", *targets,
            "--config", config, *extra]
    print(f"$ cargo test {shlex.join(selection)} --no-fail-fast {shlex.join(targets)} "
          f"--config target.{triple}.runner=<capture> {shlex.join(extra)}", flush=True)
    with open(log_dir / f"capture{label}.stdout.log", "wb") as out, \
            open(log_dir / f"capture{label}.stderr.log", "wb") as err:
        code = subprocess.run(argv, stdout=out, stderr=err).returncode
    records = []
    for path in sorted(capture_dir.glob("*.json")):
        records.append(json.loads(path.read_text(encoding="utf-8")))
    records.sort(key=lambda record: record["sequence"])
    return code, records


def load_weights(path: Path) -> dict[str, float]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}
    return {str(key): float(value) for key, value in data.get("seconds", {}).items()}


def plan_units(
    executables: dict[str, str],
    records: Sequence[dict],
    workspace_packages: Iterable[str],
    weights: dict[str, float],
    doc_unit: Optional[Unit],
    check_exclusive: bool = True,
) -> tuple[list[Unit], list[str]]:
    """Turn captured launches into units; return (units, fail-closed problems).

    ``check_exclusive=False`` (one batch of ``--batch-size``) leaves the stale
    ``EXCLUSIVE`` check to ``stale_exclusive`` over the whole run.
    """
    problems: list[str] = []
    units: list[Unit] = []
    seen: set[str] = set()
    for record in records:
        argv = record["argv"]
        if not argv:
            problems.append("a captured launch had no executable")
            continue
        executable = argv[0]
        uid = executables.get(executable)
        if uid is None:
            problems.append(f"cargo launched an executable the build did not name: {executable}")
            continue
        if executable in seen:
            problems.append(f"cargo launched {executable} twice")
            continue
        seen.add(executable)
        units.append(Unit(
            uid=uid,
            argv=list(argv),
            cwd=record["cwd"],
            env=dict(record["env"]),
            exclusive=uid in EXCLUSIVE,
            weight=weights.get(uid, 1.0),
            reason=EXCLUSIVE.get(uid, ""),
        ))
    for executable, uid in sorted(executables.items(), key=lambda item: item[1]):
        if executable not in seen:
            problems.append(f"built test executable was never launched by cargo: {uid}")
    if check_exclusive:
        problems += stale_exclusive(workspace_packages, {unit.uid for unit in units})
    ids = [unit.uid for unit in units]
    duplicates = sorted({uid for uid in ids if ids.count(uid) > 1})
    for uid in duplicates:
        problems.append(f"two test executables share the unit id {uid}")
    if doc_unit is not None:
        doc_unit.weight = weights.get(doc_unit.uid, doc_unit.weight)
        units.append(doc_unit)
    return units, problems


def stale_exclusive(workspace_packages: Iterable[str], present: set[str]) -> list[str]:
    packages = set(workspace_packages)
    return [f"EXCLUSIVE entry matches no built binary (stale?): {uid}"
            for uid in sorted(EXCLUSIVE)
            if uid.split("::", 1)[0] in packages and uid not in present]


# --- batches (--batch-size) ----------------------------------------------------


def test_targets(document: dict, selected: Iterable[str]) -> tuple[list[list[str]], set[str],
                                                                  list[str]]:
    """What `cargo test` builds for the selected members, as batchable filters.

    Returns: one cargo target filter per test-run target name, in a stable
    order; the unit id of every test executable those filters build; and the
    names of the examples `cargo test` only compiles.  Under ``--workspace``
    or ``-p`` a filter selects that target in every selected member that has
    it, so a name two members share is one filter, in one batch.
    """
    chosen = set(selected)
    lib = False
    filters: dict[tuple[str, str], list[str]] = {}
    expected: set[str] = set()
    examples: set[str] = set()
    for package in document["packages"]:
        if package["name"] not in chosen:
            continue
        for target in package["targets"]:
            kinds = set(target["kind"])
            name = target["name"]
            runs = bool(target.get("test"))
            if kinds & LIBRARY_KINDS:
                if runs:
                    lib = True
                    expected.add(unit_id(package["name"], target))
                continue
            kind = next((k for k in ("bin", "test", "bench", "example") if k in kinds), None)
            if kind is None:
                continue  # a build script
            if runs:
                filters[(kind, name)] = [f"--{kind}", name]
                expected.add(unit_id(package["name"], target))
            elif kind == "example":
                examples.add(name)
    ordered = ([["--lib"]] if lib else []) + [filters[key] for key in sorted(filters)]
    return ordered, expected, sorted(examples)


def chunks(items: Sequence, size: int) -> list[list]:
    return [list(items[start:start + size]) for start in range(0, len(items), size)]


def remove_outputs(paths: Iterable[str]) -> int:
    """Delete built executables; return the bytes freed.

    Also deletes a Windows ``.pdb`` beside one, and every other name of the
    same file in its directory: cargo reports an uplifted example as
    ``examples/<name>``, a hard link of ``examples/<name>-<hash>``, and
    deleting one name frees nothing.
    """
    freed = 0
    for path in paths:
        for candidate in (Path(path), Path(path).with_suffix(".pdb")):
            try:
                stat = candidate.stat()
            except FileNotFoundError:
                continue
            names = [candidate]
            if stat.st_nlink > 1:
                for sibling in candidate.parent.iterdir():
                    try:
                        other = sibling.stat()
                    except FileNotFoundError:
                        continue
                    if (sibling != candidate and other.st_ino == stat.st_ino
                            and other.st_dev == stat.st_dev):
                        names.append(sibling)
            removed = 0
            for name in names:
                try:
                    name.unlink(missing_ok=True)
                    removed += 1
                except OSError as error:
                    # A storage measure, not a verdict: a file still in use
                    # (Windows) stays, and the next batch is built beside it.
                    print(f"warning: could not delete {name}: {error}", file=sys.stderr)
            if removed >= stat.st_nlink:
                freed += stat.st_size
    return freed


def build_examples(cargo: str, extra: Sequence[str], log_dir: Path,
                   packages: Optional[Sequence[str]], names: Sequence[str],
                   label: str) -> tuple[int, list[str]]:
    """Compile examples as `cargo test` does (to check that they build); return their files."""
    argv = [cargo, "build", *selection_args(packages), "--message-format=json-render-diagnostics"]
    for name in names:
        argv += ["--example", name]
    argv += list(extra)
    print(f"$ {shlex.join(argv)}", flush=True)
    outputs: list[str] = []
    with open(log_dir / f"build{label}.stdout.jsonl", "wb") as raw:
        process = subprocess.Popen(argv, stdout=subprocess.PIPE)
        assert process.stdout is not None
        for line in process.stdout:
            raw.write(line)
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if (message.get("reason") == "compiler-artifact"
                    and "example" in message.get("target", {}).get("kind", [])):
                outputs += message.get("filenames", [])
        code = process.wait()
    return code, outputs


def write_summary(
    log_dir: Path,
    results: Sequence[Result],
    problems: Sequence[str],
    phases: dict[str, float],
    jobs: int,
    runner: Optional[Runner],
    packages: Optional[Sequence[str]] = None,
    link: Optional[dict] = None,
    batches: Optional[list[dict]] = None,
    targets: Optional[Sequence[str]] = None,
) -> dict:
    total = totals(results)
    failed_units = sorted(r.uid for r in results if not r.ok)
    document = {
        "schema": "qcore-test-binaries/1",
        "jobs": jobs,
        # The test-build configuration (scripts/test_link_config.py).
        "link": link,
        # None: one build of everything; a list: the --batch-size batches, each
        # built, run and deleted before the next is built.
        "batches": batches,
        # None: the whole workspace (the standing gate); a list: a scoped run.
        "packages": list(packages) if packages is not None else None,
        # None: no --targets; a list: integration-test targets run beside the
        # whole members of "packages" (the scoped gate's test-only selection).
        "targets": list(targets) if targets is not None else None,
        "phases_seconds": phases,
        "problems": list(problems),
        "totals": total,
        "units_run": len(results),
        "units_failed": failed_units,
        "ok": not problems and not failed_units,
        "units": [
            {
                "uid": r.uid,
                "executable": r.executable,
                "exclusive": r.exclusive,
                "ok": r.ok,
                "returncode": r.returncode,
                "signal": r.signal,
                "error": r.error,
                "seconds": round(r.duration, 3),
                "summaries": r.summaries,
                "tests": r.tests,
                "stdout_log": r.stdout_log,
                "stderr_log": r.stderr_log,
            }
            for r in sorted(results, key=lambda r: r.uid)
        ],
        "intervals": [
            {"uid": uid, "exclusive": exclusive, "start": round(start, 3), "end": round(end, 3)}
            for uid, exclusive, start, end in (runner.intervals if runner else [])
        ],
    }
    (log_dir / "summary.json").write_text(json.dumps(document, indent=1) + "\n", encoding="utf-8")
    return document


def default_jobs() -> int:
    return max(1, (os.cpu_count() or 2) // 2)


def positive_int(text: str) -> int:
    value = int(text)
    if value < 1:
        raise argparse.ArgumentTypeError("must be at least 1")
    return value


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Parallel, fail-closed equivalent of `cargo test --workspace --no-fail-fast`.",
    )
    parser.add_argument("--jobs", "-j", type=positive_int, default=default_jobs(),
                        help="test binaries run at once in the parallel phase "
                             "(default: half the CPUs)")
    parser.add_argument("--log-dir", type=Path,
                        help="where logs and summary.json go "
                             "(default: private/test-binaries/<timestamp>)")
    parser.add_argument("--weights", type=Path, default=DEFAULT_WEIGHTS,
                        help="recorded per-binary seconds used to start the longest first")
    parser.add_argument("--write-weights", type=Path,
                        help="after a successful run, write measured seconds here")
    parser.add_argument("--no-doc", action="store_true",
                        help="skip the doc-test unit (NOT gate-equivalent)")
    parser.add_argument("--packages", action="append", metavar="A,B",
                        help="restrict the run to these workspace members (repeatable, "
                             "comma-separated); the scoped card gate, NOT the standing gate")
    parser.add_argument("--targets", action="append", metavar="PKG::test::NAME,...",
                        help="also run these integration-test targets (PKG::test::NAME) or "
                             "library unit-test executables (PKG::lib::NAME) of members not "
                             "named by --packages (repeatable, comma-separated unit ids); the "
                             "scoped card gate's test-only selection, NOT the standing gate")
    parser.add_argument("--link", choices=test_link_config.LINK_MODES,
                        default=test_link_config.default_link(),
                        help="how test executables link the workspace libraries "
                             "(default on this platform: %(default)s); same tests, same results")
    parser.add_argument("--batch-size", type=positive_int, metavar="N",
                        help="build, run and then delete the test executables N target names "
                             "at a time, so the disk holds one batch, not the suite "
                             "(card TEST-STORAGE-BUDGET; for platforms without --link dynamic)")
    parser.add_argument("cargo_args", nargs="*",
                        help="extra cargo arguments after `--`, e.g. --locked --offline")
    return parser.parse_args(argv)


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(list(sys.argv[1:] if argv is None else argv))
    config = test_link_config.cargo_config_args(args.link)
    link = {"mode": args.link, "cargo_config": config}
    # The configuration goes first so that every cargo command below -- the
    # build, the capture pass and the doc-test unit -- builds the same units.
    extra = [*config, *args.cargo_args]
    cargo = os.environ.get("CARGO", "cargo")
    document_meta = metadata(cargo, extra)
    root = Path(document_meta["workspace_root"]).resolve()
    if root != REPO_ROOT:
        # EXCLUSIVE is audited for the root workspace only. The provider
        # workspace's conformance binaries launch process providers under
        # declared timeouts throughout, so it keeps plain `cargo test`.
        print(f"error: only the root workspace ({REPO_ROOT}) is supported, not {root}",
              file=sys.stderr)
        return 2
    try:
        packages = parse_packages(args.packages)
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    members = {package["name"] for package in document_meta["packages"]}
    if packages is not None:
        unknown = sorted(set(packages) - members)
        if unknown:
            # Refused before building: a misspelt member must not shrink the suite.
            print(f"error: --packages names non-members of the workspace: {', '.join(unknown)}",
                  file=sys.stderr)
            return 2
    try:
        targets = parse_targets(args.targets, document_meta, packages)
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    if targets is not None:
        if args.batch_size:
            print("error: --targets is not supported with --batch-size", file=sys.stderr)
            return 2
        # Only the named targets of these members, and every member of
        # --packages whole (none when --packages is not given).
        packages = packages or []
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    log_dir = (args.log_dir or REPO_ROOT / "private" / "test-binaries" / f"{stamp}-{os.getpid()}")
    # Absolute: the capture runner is launched by cargo from each package root.
    log_dir = log_dir.resolve()
    log_dir.mkdir(parents=True, exist_ok=False)
    print(f"logs: {log_dir}", flush=True)
    if packages is not None:
        print(f"packages: {', '.join(packages) or '(none whole)'} (scoped run, not the standing "
              "gate)", flush=True)
    target_uids = sorted(f"{package}::{kind}::{name}" for package, names in (targets or {}).items()
                         for kind, name in names)
    if target_uids:
        print(f"test targets: {', '.join(target_uids)} (scoped run, not the standing gate)",
              flush=True)
    print(f"link: {args.link} ({shlex.join(config)})", flush=True)
    phases: dict[str, float] = {}
    problems: list[str] = []
    selected = set(packages) if packages is not None else members
    doc_unit = None if args.no_doc else make_doc_unit(cargo, extra, packages, selected,
                                                       document_meta)
    if args.batch_size:
        return run_batched(args, cargo, extra, log_dir, packages, selected, document_meta,
                           doc_unit, link)

    # One build and capture for the members run whole (none when only
    # --targets names anything), then one per member of --targets, restricted
    # to its named integration tests with cargo's own `--test NAME` filters.
    groups: list[tuple[str, Optional[Sequence[str]], list[str]]] = []
    if packages is None or packages:
        groups.append(("", packages, []))
    for package, names in sorted((targets or {}).items()):
        groups.append((f"-targets-{safe_name(package)}", [package], target_filters(names)))
    executables: dict[str, str] = {}
    records: list[dict] = []
    phases["build"] = phases["capture"] = 0.0
    for label, selection, filters in groups:
        t0 = time.monotonic()
        code, built = build(cargo, extra, log_dir, selection, filters, label)
        phases["build"] = round(phases["build"] + time.monotonic() - t0, 3)
        if code != 0:
            problems.append(f"build{label} failed with exit {code}")
            write_summary(log_dir, [], problems, phases, args.jobs, None, packages, link,
                          targets=target_uids)
            print(f"error: build{label} failed (exit {code})", file=sys.stderr)
            return 101
        twice = sorted(set(executables.values()) & set(built.values()))
        problems += [f"built by two selections: {uid}" for uid in twice]
        executables.update(built)
        t0 = time.monotonic()
        code, captured = capture(cargo, extra, log_dir, selection, filters, label)
        phases["capture"] = round(phases["capture"] + time.monotonic() - t0, 3)
        if code != 0:
            problems.append(f"capture pass{label} failed with exit {code}")
        records += captured
    records.sort(key=lambda record: record["sequence"])

    units, plan_problems = plan_units(
        executables, records, selected, load_weights(args.weights), doc_unit,
        check_exclusive=targets is None,
    )
    problems += plan_problems
    if targets is not None:
        present = {unit.uid for unit in units}
        problems += stale_exclusive(selected, present)
        problems += [f"named test target was never built and launched: {uid}"
                     for uid in target_uids if uid not in present]
        named = set(target_uids)
        problems += [f"built a unit outside the selection: {uid}" for uid in sorted(present)
                     if uid != "doc-tests" and uid not in named
                     and uid.split("::", 1)[0] not in selected]
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    if problems:
        # Fail closed before running anything: the plan is not the suite.
        write_summary(log_dir, [], problems, phases, args.jobs, None, packages, link,
                      targets=target_uids)
        print("error: the test plan is not trustworthy; nothing was run", file=sys.stderr)
        return 101
    exclusive_count = sum(1 for unit in units if unit.exclusive)
    print(
        f"running {len(units)} units ({len(units) - exclusive_count} parallel with "
        f"--jobs {args.jobs}, then {exclusive_count} exclusive)",
        flush=True,
    )

    runner = Runner(log_dir, args.jobs)
    t0 = time.monotonic()
    try:
        results = runner.run(units)
    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        return 130
    phases["run"] = round(time.monotonic() - t0, 3)
    document = write_summary(log_dir, results, problems, phases, args.jobs, runner, packages, link,
                             targets=target_uids if targets is not None else None)
    return report(args, document, results, phases, log_dir)


def make_doc_unit(cargo: str, extra: Sequence[str], packages: Optional[Sequence[str]],
                  selected: set[str], document_meta: dict) -> Optional[Unit]:
    doc_selection: Optional[list[str]] = None
    if packages is not None:
        # `cargo test -p X --doc` refuses a member without a library target;
        # `--workspace --doc` skips it. Select exactly the members that can
        # have doc-tests, and run no doc unit when none of them can.
        doc_selection = sorted(selected & library_packages(document_meta))
    if doc_selection is not None and not doc_selection:
        print("doc-tests: no selected member has a library target; no doc unit", flush=True)
        return None
    return Unit(
        uid="doc-tests",
        argv=[cargo, "test", *selection_args(doc_selection), "--doc", "--no-fail-fast", *extra],
        cwd=os.getcwd(),
        env=dict(os.environ),
        weight=1.0,
    )


def run_batched(args: argparse.Namespace, cargo: str, extra: Sequence[str], log_dir: Path,
                packages: Optional[Sequence[str]], selected: set[str], document_meta: dict,
                doc_unit: Optional[Unit], link: dict) -> int:
    """``--batch-size N``: the suite in batches, each built, run and deleted.

    Every rule of the single build holds per batch: cargo's own launches are
    captured and must equal the batch's build; the batch's parallel units run,
    then its ``EXCLUSIVE`` units one at a time with nothing else running; then
    its executables are deleted before the next batch is built.  Over the
    whole run: the executables built must be exactly the test targets of the
    selected members (``test_targets``), no unit may be built twice, the
    stale-``EXCLUSIVE`` check covers the selection, the examples ``cargo
    test`` compiles are compiled (and deleted) in batches too, and the doc-test
    unit runs last.  The first failing build, capture or plan stops the run
    (fail-closed, exit 101); a failing test does not.
    """
    weights = load_weights(args.weights)
    filters, expected, examples = test_targets(document_meta, selected)
    groups = chunks(filters, args.batch_size)
    print(f"batches: {len(groups)} of up to {args.batch_size} target names "
          f"({len(expected)} test executables), then {len(examples)} examples to compile",
          flush=True)
    runner = Runner(log_dir, args.jobs)
    phases: dict[str, float] = {"build": 0.0, "capture": 0.0, "run": 0.0}
    problems: list[str] = []
    results: list[Result] = []
    batches: list[dict] = []
    built: dict[str, str] = {}
    try:
        for index, group in enumerate(groups, 1):
            label = f"-{index:03d}"
            flat = [arg for target in group for arg in target]
            record: dict = {"index": index, "filters": flat}
            batches.append(record)
            t0 = time.monotonic()
            code, executables = build(cargo, extra, log_dir, packages, flat, label)
            phases["build"] += time.monotonic() - t0
            record["build_exit"] = code
            if code != 0:
                problems.append(f"batch {index}: build failed with exit {code}")
                break
            t0 = time.monotonic()
            code, records = capture(cargo, extra, log_dir, packages, flat, label)
            phases["capture"] += time.monotonic() - t0
            record["capture_exit"] = code
            if code != 0:
                problems.append(f"batch {index}: capture pass failed with exit {code}")
                break
            units, plan_problems = plan_units(executables, records, selected, weights, None,
                                              check_exclusive=False)
            twice = sorted(set(built.values()) & set(executables.values()))
            plan_problems += [f"built in two batches: {uid}" for uid in twice]
            if plan_problems:
                problems += [f"batch {index}: {problem}" for problem in plan_problems]
                break
            built.update(executables)
            record["units"] = sorted(unit.uid for unit in units)
            t0 = time.monotonic()
            results += runner.run(units)
            phases["run"] += time.monotonic() - t0
            record["deleted_bytes"] = remove_outputs(executables)
        if not problems:
            for index, group in enumerate(chunks(examples, args.batch_size), 1):
                label = f"-examples-{index:03d}"
                t0 = time.monotonic()
                code, outputs = build_examples(cargo, extra, log_dir, packages, group, label)
                phases["build"] += time.monotonic() - t0
                batches.append({"examples": group, "build_exit": code,
                                "deleted_bytes": remove_outputs(outputs)})
                if code != 0:
                    problems.append(f"examples batch {index}: build failed with exit {code}")
                    break
        if not problems:
            present = set(built.values())
            problems += [f"test target never built: {uid}" for uid in sorted(expected - present)]
            problems += [f"built a unit no test target names: {uid}"
                         for uid in sorted(present - expected)]
            problems += stale_exclusive(selected, present)
        if not problems and doc_unit is not None:
            doc_unit.weight = weights.get(doc_unit.uid, doc_unit.weight)
            t0 = time.monotonic()
            results += runner.run([doc_unit])
            phases["run"] += time.monotonic() - t0
    except KeyboardInterrupt:
        print("interrupted", file=sys.stderr)
        return 130
    phases = {key: round(value, 3) for key, value in phases.items()}
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
    document = write_summary(log_dir, results, problems, phases, args.jobs, runner, packages, link,
                             batches)
    return report(args, document, results, phases, log_dir)


def report(args: argparse.Namespace, document: dict, results: Sequence[Result],
           phases: dict[str, float], log_dir: Path) -> int:
    total = document["totals"]
    print()
    print(
        f"test binaries: {len(results)} units, {total['summaries']} summaries; "
        f"{total['passed']} passed; {total['failed']} failed; {total['ignored']} ignored; "
        f"{total['measured']} measured; {total['filtered']} filtered out"
    )
    print(f"phases: {json.dumps(phases)}")
    for uid in document["units_failed"]:
        result = next(r for r in results if r.uid == uid)
        print(f"FAILED unit {uid}: {describe(result)}\n  stdout: {result.stdout_log}"
              f"\n  stderr: {result.stderr_log}", file=sys.stderr)
        if os.environ.get("GITHUB_ACTIONS") == "true":
            # The captured logs stay on the runner; put the failure detail in
            # the job log, where it is the only record a CI reader gets.
            print_failure_excerpt(uid, result)
    if args.write_weights and document["ok"]:
        seconds = {r.uid: round(r.duration, 1) for r in sorted(results, key=lambda r: r.uid)}
        args.write_weights.write_text(
            json.dumps({"note": "measured seconds per unit; scheduling hint only",
                        "seconds": seconds}, indent=1) + "\n",
            encoding="utf-8",
        )
    print(f"summary: {log_dir / 'summary.json'}")
    if not document["ok"]:
        print("error: test binaries failed (fail-closed)", file=sys.stderr)
        return 101
    return 0


if __name__ == "__main__":
    sys.exit(main())
