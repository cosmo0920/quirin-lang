#!/usr/bin/env python3
"""Compare two quirin-audit binaries on identical, pinned small input fixtures.

Clang wrappers measure subprocess actions, including metadata probes separately.
GNU time reports maximum RSS (not aggregate concurrent memory). Wrapper overhead
and host load make these observations unsuitable as end-to-end speedup claims.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


WRAPPER = '''#!/usr/bin/env python3
import json, os, resource, subprocess, sys, time
args = sys.argv[1:]
stage = ('plain' if '-P' in args else 'marked') if '-E' in args else ('syntax' if '-fsyntax-only' in args else 'metadata')
start = time.monotonic()
code = subprocess.call([os.environ['REVIEW_REAL_CLANG'], *args])
with open(os.environ['REVIEW_CLANG_LOG'], 'a') as log:
    log.write(json.dumps({'stage':stage, 'args':args, 'exit':code, 'seconds':time.monotonic()-start, 'child_max_rss_kib':resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss})+'\\n')
raise SystemExit(code)
'''


def fixture(root, source):
    checkout = root / "checkout"
    (checkout / "src").mkdir(parents=True)
    (checkout / "src/unit.c").write_text(source)
    for args in (["init", "--quiet"], ["add", "src/unit.c"],
                 ["-c", "user.name=Audit Fixture", "-c",
                  "user.email=audit@example.invalid", "commit", "--quiet", "-m", "fixture"]):
        subprocess.run(["git", *args], cwd=checkout, check=True)
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=checkout, text=True).strip()
    profile = root / "profiles/sample"
    profile.mkdir(parents=True)
    (root / "alloc.qcore").write_text(
        "module contract;\nextern fn alloc() -> ptr<mut, u8>\n"
        "contract { requires resource_contract_v1; ensures normal resource_return_owned(nonnull); };\n")
    write_json(root / "contracts.json", {"schema_version": 1, "modules": ["alloc.qcore"]})
    library = {
        "pin": "sample", "source_roots": [{"root": "src", "selection": "top_level"}],
        "audited_files": [{"path": "src/unit.c", "sha256": digest(checkout / "src/unit.c")}],
        "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
        "api_dispositions": {}, "accepted_builds": [], "required_evidence": [],
        "typed_import": None, "callback_summaries": {}, "boolean_field_models": [], "provider": None,
    }
    write_json(profile / "profile.json", {
        "schema_version": 1, "source_set": "sample",
        "pins": {"sample": {"git_commit": commit, "clean_paths": [], "description": "measurement fixture"}},
        "libraries": {"sample": library}, "resource_contract_manifest": "../../contracts.json",
        "provider_manifests": [],
    })
    write_json(root / "compile_commands.json", [{
        "directory": str(checkout), "file": "src/unit.c",
        "arguments": ["cc", "-std=c11", "-c", "src/unit.c", "-o", "unit.o"],
    }])
    write_json(root / "audit.json", {
        "schema_version": 1, "profile_directory": "profiles/sample", "checkout": "checkout",
        "compile_database": "compile_commands.json", "libraries": ["sample"], "report": "output/report.json",
    })
    (root / "clang-wrapper").write_text(WRAPPER)
    (root / "clang-wrapper").chmod(0o755)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("before", "after", "before-revision", "after-revision", "work-dir"):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    root = Path(args.work_dir).resolve()
    root.mkdir(parents=True, exist_ok=False)
    binaries = {}
    for label in ("before", "after"):
        original = Path(getattr(args, label)).resolve()
        binary = root / (label + "-quirin-audit")
        shutil.copyfile(original, binary)
        binary.chmod(0o755)
        binaries[label] = {"path": str(binary), "sha256": digest(binary),
                           "source_revision": getattr(args, label + "_revision")}
    results = {"binaries": binaries, "cases": {}}
    sources = {
        "Safe": "void subject(void) { return; }\n",
        "Unsafe": "void *alloc(void); void subject(void) { void *p = alloc(); }\n",
        "Unknown": "int opaque(void); void subject(void) { if (opaque()) return; }\n",
    }
    for name, source in sources.items():
        case = root / name
        fixture(case, source)
        rows = {}
        for label, binary in binaries.items():
            env = dict(os.environ, QCORE_CLANG=str(case / "clang-wrapper"),
                       REVIEW_REAL_CLANG=shutil.which("clang"),
                       REVIEW_CLANG_LOG=str(case / (label + "-clang.jsonl")))
            command = ["/usr/bin/time", "-v", "-o", str(case / (label + "-rss.txt")),
                       binary["path"], str(case / "audit.json")]
            with (case / (label + ".stdout")).open("w") as out, (case / (label + ".stderr")).open("w") as err:
                completed = subprocess.run(command, cwd=case, env=env, stdout=out, stderr=err)
            calls = [json.loads(line) for line in (case / (label + "-clang.jsonl")).read_text().splitlines()]
            stages = {stage: {"invocations": sum(c["stage"] == stage for c in calls),
                              "seconds": sum(c["seconds"] for c in calls if c["stage"] == stage)}
                      for stage in ("plain", "syntax", "marked", "metadata")}
            rows[label] = {"command": command, "exit": completed.returncode, "stages": stages,
                           "stdout": (case / (label + ".stdout")).read_text(),
                           "rss_observation": (case / (label + "-rss.txt")).read_text(),
                           "artifacts": {str(p.relative_to(case / "output")): digest(p)
                                         for p in (case / "output").rglob("*") if p.is_file()}}
            shutil.move(case / "output", case / (label + "-output"))
            if completed.returncode:
                raise RuntimeError(f"{name}/{label} failed; inspect {case}")
        rows["artifacts_byte_identical"] = rows["before"]["artifacts"] == rows["after"]["artifacts"]
        results["cases"][name] = rows
    write_json(root / "measurement.json", results)
    print(json.dumps(results, indent=2))
    if not all(row["artifacts_byte_identical"] for row in results["cases"].values()):
        raise SystemExit("artifact parity failed")


if __name__ == "__main__":
    main()
