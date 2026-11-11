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

"""Record which repository paths each root-workspace test binary reads at run time.

Card GATE-SCOPE-PRECISE-SELECTION (requirement R1 of
docs/plans/gate-time-reduction/README.md).  ``scripts/gate_scope.py`` finds
run-time readers statically (string literals and its declared walker table);
this script observes them.  Usage::

    gate_read_census.py record --log-dir DIR [--jobs N] [--packages A,B]
    gate_read_census.py reduce --log-dir DIR --out CENSUS.json

``record`` builds and captures the test executables exactly as
``scripts/run_test_binaries.py`` does (same functions, same test-build
configuration), then runs every captured unit under
``strace -ff --seccomp-bpf -y`` tracing the path system calls (``open``,
``openat``, ``openat2``, the ``stat`` family, ``access``/``faccessat``,
``readlink``, ``execve``, ``chdir``).  The doc-test unit is not traced: a
doc-test reads what rustdoc compiles, which the scoper attributes by ownership.
``reduce`` turns the per-process traces into ``CENSUS.json``: for every unit,
the repository-relative files it opened (``read``: their content can change
its result) and the directories it opened (``dirs``: a listing), with the
number of other paths it only probed (stat family, ``access``, ``readlink``:
a tracked-file listing stats every path it names) and of paths it probed that
did not exist (``--full`` lists both), excluding ``target/``, ``.git/`` and
``private/``, plus the pathspecs of every ``git ls-files`` it ran (a
name-only walk opens no file).

The census is evidence of one tree at one commit, not a proof: a code path a
test does not take at that commit is not observed.  It is what the walker
table's checks in ``scripts/tests/test_gate_scope_census.py`` are compared
against.  Units run in parallel (``--jobs``, default 4) and the census is not
a gate: a unit failing under the tracer is recorded with its exit, and its
reads are kept (a failure can only shorten what it read).
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from typing import Iterable, Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))

import run_test_binaries as runner  # noqa: E402
import test_link_config  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "quirin-gate-read-census/1"
EXCLUDED_PREFIXES = ("target/", ".git/", "private/")
EXCLUDED_NAMES = ("target", ".git", "private")
TRACED = ("open,openat,openat2,stat,lstat,newfstatat,statx,access,faccessat,faccessat2,"
          "readlink,readlinkat,execve,chdir")

# One strace line: `syscall(args) = result`, with -y decoding fds as `N</path>`.
LINE_RE = re.compile(r"^(?P<call>\w+)\((?P<args>.*)\)\s+=\s+(?P<ret>.*)$")
STRING_RE = re.compile(r'"((?:[^"\\]|\\.)*)"')
FD_PATH_RE = re.compile(r"^(?:-?\d+)<(?P<path>[^>]*)>")
DIRFD_RE = re.compile(r"^\s*(?:AT_FDCWD|-?\d+)(?:<(?P<dir>[^>]*)>)?\s*,")
# A file's content is read only by an open (or by executing it); the stat
# family, access and readlink probe its existence and metadata.
OPENING_CALLS = ("open", "openat", "openat2", "execve")
DIRFD_CALLS = ("openat(", "openat2(", "newfstatat(", "statx(", "faccessat(", "faccessat2(",
               "readlinkat(")
ARRAY_RE =re.compile(r"\[(?P<items>(?:\s*\"(?:[^\"\\]|\\.)*\"\s*,?)*)\]")


def unescape(text: str) -> str:
    """strace's C escapes back to text (\\n, \\t, \\", \\\\, \\NNN octal, \\xNN)."""
    def replace(match: re.Match) -> str:
        token = match.group(1)
        simple = {"n": "\n", "t": "\t", "r": "\r", "\"": "\"", "\\": "\\", "v": "\v", "f": "\f"}
        if token in simple:
            return simple[token]
        if token.startswith("x"):
            return chr(int(token[1:], 16))
        return chr(int(token, 8))
    return re.sub(r"\\(x[0-9a-fA-F]{2}|[0-7]{1,3}|.)", replace, text)


def parse_trace_line(line: str) -> Optional[dict]:
    """One traced call: {call, paths: [(path, dirfd_dir)], ok, ret_path, argv}."""
    match = LINE_RE.match(line.strip())
    if not match:
        return None
    call, args, ret = match.group("call"), match.group("args"), match.group("ret")
    ok = not ret.startswith("-1")
    ret_path = None
    fd = FD_PATH_RE.match(ret)
    if fd:
        ret_path = fd.group("path")
    record = {"call": call, "ok": ok, "ret_path": ret_path, "paths": [], "argv": None}
    strings = [unescape(s) for s in STRING_RE.findall(args)]
    if call == "execve":
        array = ARRAY_RE.search(args)
        record["argv"] = [unescape(s) for s in STRING_RE.findall(array.group("items"))] \
            if array else []
        if strings:
            record["paths"].append((strings[0], None))
        return record
    if not strings:
        return record
    if call in ("openat", "openat2", "newfstatat", "statx", "faccessat", "faccessat2",
                "readlinkat"):
        dirfd = DIRFD_RE.match(args)
        directory = dirfd.group("dir") if dirfd and dirfd.group("dir") else None
        record["paths"].append((strings[0], directory))
    else:
        record["paths"].append((strings[0], None))
    return record


def relative_to_repo(path: str, root: Path) -> Optional[str]:
    """A repository-relative posix path, or None outside it or in an excluded tree."""
    normal = os.path.normpath(path)
    root_text = str(root)
    if normal == root_text:
        return "."
    if not normal.startswith(root_text + os.sep):
        return None
    relative = Path(normal).relative_to(root).as_posix()
    if relative in EXCLUDED_NAMES or relative.startswith(EXCLUDED_PREFIXES):
        return None
    return relative


def reduce_process(lines: Iterable[str], cwd: str, root: Path) -> dict:
    """Paths one traced process reached: {read, dirs, missing, ls_files, outside}."""
    read: set[str] = set()
    probed: set[str] = set()
    missing: set[str] = set()
    ls_files: list[list[str]] = []
    current = cwd
    root_text = str(root)
    for line in lines:
        # With -y every resolved path is absolute, so a line that never names
        # the repository is outside it (a relative path's base is decoded too),
        # unless the process is still somewhere under it by a relative chdir.
        if root_text not in line and not line.startswith(("chdir", "execve")) and (
                line.startswith(DIRFD_CALLS) or '("/' in line):
            continue
        record = parse_trace_line(line)
        if record is None:
            continue
        if record["call"] == "chdir" and record["ok"] and record["paths"]:
            current = os.path.normpath(os.path.join(current, record["paths"][0][0]))
            continue
        if record["call"] == "execve" and record["ok"] and record["argv"]:
            argv = record["argv"]
            if os.path.basename(argv[0]) == "git" and "ls-files" in argv:
                ls_files.append(argv[argv.index("ls-files") + 1:])
        for path, directory in record["paths"]:
            if record["ret_path"]:
                absolute = record["ret_path"]
            elif os.path.isabs(path):
                absolute = path
            else:
                absolute = os.path.join(directory or current, path)
            relative = relative_to_repo(absolute, root)
            if relative is None or relative == ".":
                continue
            if not record["ok"]:
                missing.add(relative)
            elif record["call"] in OPENING_CALLS:
                read.add(relative)
            else:
                probed.add(relative)
    # An opened directory is a listing (read_dir); a probed one is a stat.
    # Split once per path, not per call.
    dirs = {path for path in read if (root / path).is_dir()}
    read -= dirs
    probed -= dirs | read
    return {"read": read, "probed": probed, "dirs": dirs,
            "missing": missing - read - probed - dirs, "ls_files": ls_files}


def reduce_unit(trace_dir: Path, cwd: str, root: Path) -> dict:
    keys = ("read", "probed", "dirs", "missing")
    merged = {**{key: set() for key in keys}, "ls_files": []}
    for trace in sorted(trace_dir.glob("trace.*")):
        with open(trace, encoding="utf-8", errors="replace") as handle:
            part = reduce_process(handle, cwd, root)
        for key in keys:
            merged[key] |= part[key]
        merged["ls_files"] += part["ls_files"]
    merged["probed"] -= merged["read"]
    merged["missing"] -= merged["read"] | merged["probed"] | merged["dirs"]
    return {**{key: sorted(merged[key]) for key in keys},
            "ls_files": sorted({json.dumps(spec) for spec in merged["ls_files"]})}


def record(args: argparse.Namespace) -> int:
    strace = shutil.which("strace")
    if not strace:
        print("error: strace is not on PATH", file=sys.stderr)
        return 2
    config = test_link_config.cargo_config_args()
    cargo = os.environ.get("CARGO", "cargo")
    log_dir = args.log_dir.resolve()
    log_dir.mkdir(parents=True, exist_ok=False)
    packages = runner.parse_packages(args.packages)
    document = runner.metadata(cargo, config)
    members = {package["name"] for package in document["packages"]}
    selected = set(packages) if packages is not None else members
    code, executables = runner.build(cargo, config, log_dir, packages)
    if code != 0:
        print(f"error: build failed (exit {code})", file=sys.stderr)
        return 101
    code, records = runner.capture(cargo, config, log_dir, packages)
    if code != 0:
        print(f"error: capture failed (exit {code})", file=sys.stderr)
        return 101
    units, problems = runner.plan_units(executables, records, selected, {}, None)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 101
    traces = log_dir / "traces"
    traced = []
    plan = {}
    for unit in units:
        trace_dir = traces / runner.safe_name(unit.uid)
        trace_dir.mkdir(parents=True)
        argv = [strace, "-ff", "-qq", "-y", "--seccomp-bpf", "-e", f"trace={TRACED}",
                "-e", "signal=none", "-s", "4096", "-o", str(trace_dir / "trace"), "--",
                *unit.argv]
        plan[unit.uid] = {"cwd": unit.cwd, "trace_dir": str(trace_dir)}
        # Not exclusive: the census is not a gate, nothing here asserts a budget.
        traced.append(runner.Unit(uid=unit.uid, argv=argv, cwd=unit.cwd, env=unit.env,
                                  exclusive=False, weight=1.0))
    (log_dir / "plan.json").write_text(json.dumps(plan, indent=1) + "\n", encoding="utf-8")
    results = runner.Runner(log_dir, args.jobs).run(traced)
    outcome = {result.uid: {"exit": result.returncode, "ok": result.ok,
                            "seconds": round(result.duration, 1)} for result in results}
    (log_dir / "outcomes.json").write_text(json.dumps(outcome, indent=1) + "\n",
                                           encoding="utf-8")
    print(f"traced {len(results)} units; {sum(1 for r in results if not r.ok)} not ok; "
          f"logs {log_dir}")
    return 0


def reduce(args: argparse.Namespace) -> int:
    log_dir = args.log_dir.resolve()
    plan = json.loads((log_dir / "plan.json").read_text(encoding="utf-8"))
    outcomes = json.loads((log_dir / "outcomes.json").read_text(encoding="utf-8"))
    head = subprocess.run(["git", "-C", str(REPO_ROOT), "rev-parse", "HEAD"], capture_output=True,
                          text=True, check=True).stdout.strip()
    units = {}
    uids = sorted(plan)
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        reduced_all = pool.map(reduce_unit, [Path(plan[uid]["trace_dir"]) for uid in uids],
                               [plan[uid]["cwd"] for uid in uids], [REPO_ROOT] * len(uids))
        for uid, reduced in zip(uids, reduced_all):
            if not args.full:
                # Existence probes (a tracked-file listing stats every path it
                # names) are counted, not listed; --full keeps them.
                for key in ("probed", "missing"):
                    reduced[f"{key}_count"] = len(reduced.pop(key))
            units[uid] = {**outcomes.get(uid, {}), **reduced}
    document = {"schema": SCHEMA, "commit": head, "root": "the recording checkout",
                "excluded": list(EXCLUDED_PREFIXES), "units": units}
    args.out.write_text(json.dumps(document, indent=0, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {args.out}: {len(units)} units")
    return 0


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    rec = sub.add_parser("record")
    rec.add_argument("--log-dir", type=Path, required=True)
    rec.add_argument("--jobs", type=runner.positive_int, default=4)
    rec.add_argument("--packages", action="append", metavar="A,B")
    red = sub.add_parser("reduce")
    red.add_argument("--log-dir", type=Path, required=True)
    red.add_argument("--out", type=Path, required=True)
    red.add_argument("--jobs", type=runner.positive_int, default=4)
    red.add_argument("--full", action="store_true",
                     help="list probed and missing paths too, not only their counts")
    args = parser.parse_args(argv)
    return record(args) if args.command == "record" else reduce(args)


if __name__ == "__main__":
    sys.exit(main())
