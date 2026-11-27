#!/usr/bin/env python3
"""W45-FB-ENGINE-PROGRESS: bind the native runs and copy their small excerpts.

Usage: bind_native.py ATTEMPT_DIR --out native/evidence.json --excerpts native/evidence

Read-only over the private attempt, except that it writes the JSON record and
copies each canonical run's `[wd]` excerpt (a few KB) into the report. Records
the revision, the prepare re-check that followed the native build (it refuses a
clone whose HEAD differs from the pin or that has tracked changes), the
compiler, CMake and configure command, the binary's SHA-256 and version, the
fixture, and for every run: the command, the watchdog's exit, the SHA-256 of the
gdb script (for canonical runs, equal to the published script), the full log's
SHA-256 and size, the `[wd] SUMMARY` line and counts of Fluent Bit's own log
lines that corroborate it.
"""

import argparse
import hashlib
import json
import pathlib
import re
import shutil

REVISION = "046accf6aded7906f66a1baf0202da349e521736"

# (run name, gdb script, config, experiment id, role, watchdog seconds)
CANONICAL = [
    ("r2-c1-valid-control", "c1-valid-control.gdb", "phase2-backlog.conf", "C1", "valid progressing control", 120),
    ("r3-e1-persistent-error", "e1-persistent-error.gdb", "phase2-backlog.conf", "E1", "zero-progress failure outcome, persistent CIO_ERROR premise", 150),
    ("r3-e2-oneshot-fstat", "e2-oneshot-fstat.gdb", "phase2-backlog.conf", "E2", "zero-progress failure outcome from one transient fstat failure", 150),
    ("r2-c2-oneshot-open", "c2-oneshot-open.gdb", "phase2-backlog.conf", "C2", "control: one transient open failure that leaves no descriptor", 120),
    ("e3-retry-setup-error", "e3-retry-setup-error.gdb", "retry.conf", "E3", "revisit after an error outcome: retry chunk cannot be brought up", 120),
    ("c3-retry-control", "c3-retry-control.gdb", "retry.conf", "C3", "retry control: attempts spent until retry_limit", 90),
]

# Earlier runs of the same experiments with earlier script revisions (the
# collector breakpoint by bare name, which also matched in_emitter's static
# cb_queue_chunks although no emitter was configured; caps of 400 or 2000
# observations; no flush counter). Kept for the record, not as the evidence.
SUPERSEDED = [
    ("c1-valid-control", "C1", "bare `break cb_queue_chunks` (2 locations)"),
    ("c1b-valid-control", "C1", "no flush counter"),
    ("e1-persistent-error", "E1", "cap 400, no flush counter"),
    ("e2-oneshot-fstat", "E2", "cap 400, no flush counter"),
    ("c2-oneshot-open", "C2", "no flush counter"),
    ("r2-e1-persistent-error", "E1", "cap 2000 (0.93 s of wall time: too short for the 1 s flush timer)"),
    ("r2-e2-oneshot-fstat", "E2", "cap 2000 (1.13 s of wall time)"),
]

FB_LINES = {
    "queueing": re.compile(r"\[input:storage_backlog:.*\] queueing "),
    "fd_exists": re.compile(r"file descriptor already exists"),
    "reschedule": re.compile(r"\[task\] re-schedule retry="),
    "cannot_be_retried": re.compile(r"cannot be retried"),
    "failed_to_flush": re.compile(r"failed to flush chunk"),
}


def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("attempt", type=pathlib.Path)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--excerpts", type=pathlib.Path, required=True)
    args = parser.parse_args()
    attempt = args.attempt.resolve()
    report = pathlib.Path(__file__).resolve().parent.parent
    logs = attempt / "native-logs"
    root = attempt.parent
    binary = attempt / "native-debug" / "bin" / "fluent-bit"
    prepare = json.loads((attempt / "prepare.json").read_text())
    recheck_exit = (root / f"prepare-{attempt.name}-recheck-after-native-build.exit").read_text().strip()
    record = {
        "card": "W45-FB-ENGINE-PROGRESS",
        "kind": "native observation under injected premises; no Quirin verdict moves",
        "source": {
            "revision": REVISION,
            "user_checkout": "/home/cosmo/Documents/GitHub/fluent-bit (read only)",
            "clone": str(attempt / "fluent-bit-clang-source"),
            "clone_method": "quirin-audit prepare: git clone --no-hardlinks --no-checkout, then checkout --detach at the revision (crates/qcore-audit/src/prepare.rs:327-364)",
            "status_check": "prepare re-run over the same --output after the native build: it refuses a clone whose HEAD differs from the pin, that has tracked changes (git status --porcelain --untracked-files=no) or that borrows objects; exit " + recheck_exit,
            "status_check_log": f"prepare-{attempt.name}-recheck-after-native-build.log",
            "status_check_scope": "tracked files; untracked files are not checked (the native build is out of tree)",
            "no_source_change": True,
        },
        "toolchain": {
            "clang": prepare["compilers"]["c"]["version"].splitlines()[0],
            "cmake": prepare["cmake"]["version"].splitlines()[0],
            "ninja": "1.13.2",
            "gdb": "GNU gdb (Ubuntu 17.1-2ubuntu1) 17.1",
        },
        "build": {
            "directory": str(attempt / "native-debug"),
            "configure_command": (logs / "configure.cmd").read_text().strip(),
            "configure_exit": (logs / "configure.exit").read_text().strip(),
            "build_command": f"ninja -C {attempt / 'native-debug'} -j 8 fluent-bit",
            "build_exit": (logs / "build.exit").read_text().strip(),
            "binary_sha256": sha(binary),
            "binary_version": " ".join((logs / "binary-version.txt").read_text().split()),
            "script": "scripts/native-build.sh",
        },
        "fixture": {
            "generator": "scripts/make-fixture.sh (native/conf/phase1-generate.conf)",
            "generator_exit": (logs / "phase1.exit").read_text().strip(),
            "generator_watchdog": "timeout --signal=TERM -k 5 12 (exit 124 expected)",
            "listing": (attempt / "run" / "storage-pristine.listing").read_text().splitlines(),
            "pristine_sha256_listing_sha256": sha(attempt / "run" / "storage-pristine.sha256"),
            "restore": "scripts/run-native.sh copies storage-pristine to storage before every backlog run and refuses the run unless the SHA-256 listing of the restored chunks equals the pristine listing",
        },
        "runs": [],
        "superseded_runs": [],
    }
    args.excerpts.mkdir(parents=True, exist_ok=True)
    for name, script, conf, exp, role, seconds in CANONICAL:
        log = logs / f"{name}.log"
        text = log.read_text(errors="replace")
        wd = (logs / f"{name}.wd.txt").read_text()
        summary = [l for l in wd.splitlines() if l.startswith("[wd] SUMMARY") or l.startswith("[wd] ENDED")]
        times = [l for l in wd.splitlines() if l.startswith("[wd] TIME")]
        runner = "scripts/run-native.sh" if conf == "phase2-backlog.conf" else "scripts/run-native-retry.sh"
        shutil.copyfile(logs / f"{name}.wd.txt", args.excerpts / f"{exp.lower()}-{name}.wd.txt")
        record["runs"].append({
            "experiment": exp,
            "role": role,
            "run": name,
            "runner": f"{runner} attempt-1 {name} {script} {seconds}",
            "command": f"timeout -k 5 {seconds} gdb -batch -x native/gdb/{script} --args {binary} -c native/conf/{conf}",
            "watchdog_pid": (logs / f"{name}.pid").read_text().strip(),
            "exit": (logs / f"{name}.exit").read_text().strip(),
            "gdb_script": f"native/gdb/{script}",
            "gdb_script_sha256": sha(report / "native" / "gdb" / script),
            "config": f"native/conf/{conf}",
            "config_sha256": sha(report / "native" / "conf" / conf),
            "fixture_restore_sha256": sha(logs / f"{name}.fixture.sha256") if (logs / f"{name}.fixture.sha256").exists() else None,
            "log": str(log),
            "log_sha256": sha(log),
            "log_bytes": log.stat().st_size,
            "excerpt": f"native/evidence/{exp.lower()}-{name}.wd.txt",
            "summary": summary,
            "wall_clock": times,
            "fluent_bit_log_counts": {k: len(p.findall(text)) for k, p in FB_LINES.items()},
        })
    for name, exp, why in SUPERSEDED:
        log = logs / f"{name}.log"
        wd = (logs / f"{name}.wd.txt").read_text()
        record["superseded_runs"].append({
            "run": name, "experiment": exp, "difference": why,
            "exit": (logs / f"{name}.exit").read_text().strip(),
            "log_sha256": sha(log),
            "summary": [l for l in wd.splitlines() if "SUMMARY" in l or "ENDED" in l],
        })
    args.out.write_text(json.dumps(record, indent=1) + "\n")
    for r in record["runs"]:
        print(r["experiment"], r["exit"], r["summary"])


if __name__ == "__main__":
    main()
