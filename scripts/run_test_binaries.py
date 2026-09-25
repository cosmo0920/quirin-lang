#!/usr/bin/env python3
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

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_WEIGHTS = Path(__file__).resolve().with_name("test_binary_weights.json")

# Test binaries that must not share the machine with any other binary of this
# runner.  Each is run alone, after the parallel phase, with its own internal
# test threads exactly as under `cargo test`.  Keys are unit ids
# (`<package>::<target kind>::<target name>`); an entry whose package belongs to
# the workspace being run but which matches no built binary fails the run, so
# a rename cannot silently drop a load-sensitive row into the parallel phase.
EXCLUSIVE: dict[str, str] = {
    "qcore-compiler::test::cycle20_verifier_plugins": (
        "constraint 16: vp04_vp07_vp08_process_lifecycle_and_payload_budgets "
        "asserts a wall-clock budget"
    ),
    "qcore-verifier-host::test::standalone_host": (
        "constraint 16: closed_input_does_not_mask_descendant_timeout_or_"
        "accept_a_partial_request asserts a one-second wall-clock bound"
    ),
    "qcore-verifier-host::test::enforced_process_limits": (
        "constraint 16: memory_growth_beyond_the_declared_limit_is_bounded "
        "(Io(Broken pipe) under load) and the RLIMIT_NPROC row"
    ),
    "qcore-verifier-host::test::confined_process_execution": (
        "constraint 16: the confined-fixture write test (Io(Broken pipe) under load)"
    ),
    "qcore-verifier-host::test::execution_quota": (
        "constraint 16's RLIMIT_NPROC rows count every process of the real UID; "
        "process quotas and timeouts"
    ),
    "qcore-provider-conformance::test::packages": (
        "constraint 16: real_packages_preserve_unknown_and_reject_mutations; "
        "process providers under declared timeouts"
    ),
    "qcore-provider-conformance::test::configured_audit": (
        "pinned_second_codebase_runs_configured_process_checks_without_a_rust_"
        "import_adapter has a 5 s plugin process quota and failed closed under "
        "concurrent load"
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


def package_names(cargo: str, extra: Sequence[str]) -> dict[str, str]:
    """Map package id -> package name for the workspace members."""
    metadata_args = [a for a in extra if a in ("--locked", "--offline", "--frozen")]
    manifest = manifest_args(extra)
    output = subprocess.run(
        [cargo, "metadata", "--no-deps", "--format-version", "1", *manifest, *metadata_args],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    data = json.loads(output)
    return {package["id"]: package["name"] for package in data["packages"]}


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


def build(cargo: str, extra: Sequence[str], log_dir: Path) -> tuple[int, dict[str, str]]:
    """Build like `cargo test` does; return (exit, executable -> unit id)."""
    names = package_names(cargo, extra)
    argv = [cargo, "test", "--workspace", "--no-run",
            "--message-format=json-render-diagnostics", *extra]
    print(f"$ {shlex.join(argv)}", flush=True)
    executables: dict[str, str] = {}
    with open(log_dir / "build.stdout.jsonl", "wb") as raw:
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


def capture(cargo: str, extra: Sequence[str], log_dir: Path) -> tuple[int, list[dict]]:
    """Record cargo's own launch of every test executable without running it."""
    capture_dir = log_dir / "capture"
    capture_dir.mkdir()
    runner = [sys.executable, "-I", str(Path(__file__).resolve()),
              "--capture-into", str(capture_dir), "--"]
    triple = host_triple(dict(os.environ))
    config = f"target.{triple}.runner=[{', '.join(json.dumps(part) for part in runner)}]"
    argv = [cargo, "test", "--workspace", "--no-fail-fast", "--tests",
            "--config", config, *extra]
    print(f"$ {shlex.join(argv)}", flush=True)
    with open(log_dir / "capture.stdout.log", "wb") as out, \
            open(log_dir / "capture.stderr.log", "wb") as err:
        code = subprocess.run(argv, stdout=out, stderr=err).returncode
    records = []
    for path in sorted(capture_dir.glob("*.json")):
        records.append(json.loads(path.read_text(encoding="utf-8")))
    records.sort(key=lambda record: record["sequence"])
    return code, records


def capture_into(directory: str, argv: list[str]) -> int:
    """`target.<host>.runner` mode: record the launch and do not run the test."""
    record = {
        "sequence": time.time_ns(),
        "argv": argv,
        "cwd": os.getcwd(),
        "env": dict(os.environ),
    }
    path = Path(directory) / f"{time.time_ns()}-{os.getpid()}.json"
    path.write_text(json.dumps(record, indent=1, sort_keys=True), encoding="utf-8")
    return 0


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
) -> tuple[list[Unit], list[str]]:
    """Turn captured launches into units; return (units, fail-closed problems)."""
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
    packages = set(workspace_packages)
    present = {unit.uid for unit in units}
    for uid in sorted(EXCLUSIVE):
        if uid.split("::", 1)[0] in packages and uid not in present:
            problems.append(f"EXCLUSIVE entry matches no built binary (stale?): {uid}")
    ids = [unit.uid for unit in units]
    duplicates = sorted({uid for uid in ids if ids.count(uid) > 1})
    for uid in duplicates:
        problems.append(f"two test executables share the unit id {uid}")
    if doc_unit is not None:
        doc_unit.weight = weights.get(doc_unit.uid, doc_unit.weight)
        units.append(doc_unit)
    return units, problems


def write_summary(
    log_dir: Path,
    results: Sequence[Result],
    problems: Sequence[str],
    phases: dict[str, float],
    jobs: int,
    runner: Optional[Runner],
) -> dict:
    total = totals(results)
    failed_units = sorted(r.uid for r in results if not r.ok)
    document = {
        "schema": "qcore-test-binaries/1",
        "jobs": jobs,
        "phases_seconds": phases,
        "problems": list(problems),
        "totals": total,
        "units_run": len(results),
        "units_failed": failed_units,
        "ok": not problems and not failed_units,
        "units": [
            {
                "uid": r.uid,
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
    parser.add_argument("--capture-into", help=argparse.SUPPRESS)
    parser.add_argument("cargo_args", nargs="*",
                        help="extra cargo arguments after `--`, e.g. "
                             "--manifest-path providers/Cargo.toml --locked --offline")
    return parser.parse_args(argv)


def main(argv: Optional[Sequence[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv[:1] == ["--capture-into"]:
        # Runner mode: `--capture-into DIR -- <executable> <args...>`.
        if len(argv) < 3 or argv[2] != "--":
            print("malformed --capture-into invocation", file=sys.stderr)
            return 2
        return capture_into(argv[1], argv[3:])
    args = parse_args(argv)
    extra = list(args.cargo_args)
    cargo = os.environ.get("CARGO", "cargo")
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    log_dir = args.log_dir or REPO_ROOT / "private" / "test-binaries" / f"{stamp}-{os.getpid()}"
    log_dir.mkdir(parents=True, exist_ok=False)
    print(f"logs: {log_dir}", flush=True)
    phases: dict[str, float] = {}
    problems: list[str] = []

    t0 = time.monotonic()
    code, executables = build(cargo, extra, log_dir)
    phases["build"] = round(time.monotonic() - t0, 3)
    if code != 0:
        problems.append(f"build failed with exit {code}")
        write_summary(log_dir, [], problems, phases, args.jobs, None)
        print(f"error: build failed (exit {code})", file=sys.stderr)
        return 101

    t0 = time.monotonic()
    code, records = capture(cargo, extra, log_dir)
    phases["capture"] = round(time.monotonic() - t0, 3)
    if code != 0:
        problems.append(f"capture pass failed with exit {code}")

    packages = set(package_names(cargo, extra).values())
    doc_unit = None
    if not args.no_doc:
        doc_unit = Unit(
            uid="doc-tests",
            argv=[cargo, "test", "--workspace", "--doc", "--no-fail-fast", *extra],
            cwd=os.getcwd(),
            env=dict(os.environ),
            weight=1.0,
        )
    units, plan_problems = plan_units(
        executables, records, packages, load_weights(args.weights), doc_unit
    )
    problems += plan_problems
    for problem in problems:
        print(f"error: {problem}", file=sys.stderr)
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
    document = write_summary(log_dir, results, problems, phases, args.jobs, runner)

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
