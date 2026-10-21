#!/usr/bin/env python3
"""Freeze the gated binary, compile exact-artifact helpers, run three TIP routes."""
import argparse
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
CODE = ["Cargo.toml", "Cargo.lock", "compiler", "crates", "proof", "contract", "providers", "scripts", "cycle19-resource-contracts.json", "docs/reports/premise-guarded-decomposition/applications"]
EXTERNS = ["qcore_audit", "qcore_c_gateway", "qcore_contract", "qcore_schemas", "serde_json"]

def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run")
    args = parser.parse_args()
    out = ROOT / "private/symbolic-decomposition-cycle3" / args.run
    out.mkdir(parents=True, exist_ok=False)
    gate = ROOT / "private/symbolic-decomposition-cycle3/joint-gate-1"
    completion = json.loads((gate / "completion.json").read_bytes())
    commands = json.loads((gate / "commands.json").read_bytes())
    assert completion["wrapper_exit"] == 0 and completion["source_stable"]
    assert len(commands) == 11 and all(c["exit"] == 0 for c in commands)
    assert completion["commands_sha256"] == sha(gate / "commands.json")
    source = completion["source"]
    records = []
    env = dict(os.environ, QCORE_AUDIT_BUILD_COMMIT=source,
               CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_INCREMENTAL="0")
    with open("/tmp/qcore-c-card-validation-20261022.lock", "a") as lock, open("/tmp/qcore-c-cycle2-small-compile.lock", "a") as small:
        print("WAITING FOR NATIVE LOCK", flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        fcntl.flock(small, fcntl.LOCK_EX)
        head = git("rev-parse", "HEAD")
        def guard():
            assert git("rev-parse", "HEAD") == head and not git("status", "--porcelain")
            for path in CODE:
                assert git("rev-parse", source + ":" + path) == git("rev-parse", "HEAD:" + path)
            return {"reporting_head": head, "analyzer_source": source, "clean": True}
        guard()
        (out / "acquired.json").write_text(json.dumps(dict(utc=now(), **guard()), indent=2) + "\n")
        def run(name, command):
            before = guard()
            print("START", name, now(), flush=True)
            with (out / (name + ".stdout.log")).open("wb") as stdout, (out / (name + ".stderr.log")).open("wb") as stderr:
                result = subprocess.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
            record = dict(name=name, command=command, cwd=str(ROOT), exit=result.returncode,
                          source_before=before, source_after=guard(), ended=now())
            records.append(record)
            (out / "commands.json").write_text(json.dumps(records, indent=2) + "\n")
            print("END", name, result.returncode, flush=True)
            assert result.returncode == 0, name
        binary = ROOT / "target/release/quirin-audit"
        binary_sha = sha(binary)
        release = next(c for c in commands if c["name"] == "release")
        receipt = dict(source=source, sha256=binary_sha, binary=str(binary), build=release,
                       gate_commands_sha256=completion["commands_sha256"], reporting_source=head)
        (out / "binary-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        freeze = out / "tip-freeze"
        run("freeze", [sys.executable, "docs/reports/premise-symbolic-integration/scripts/freeze_inputs.py", str(freeze), "--source", source, "--binary", str(binary), "--binary-sha256", binary_sha, "--binary-receipt", str(out / "binary-receipt.json")])
        run("cargo-artifacts", ["cargo", "build", "--release", "-p", "qcore-audit", "--bin", "quirin-audit", "--message-format=json", "-j1"])
        assert sha(binary) == binary_sha, "Artifact discovery changed the gated binary"
        artifacts = {}
        for line in (out / "cargo-artifacts.stdout.log").read_bytes().splitlines():
            value = json.loads(line)
            if value.get("reason") != "compiler-artifact":
                continue
            name = value["target"]["name"]
            if name not in EXTERNS:
                continue
            candidates = [str(Path(p).resolve()) for p in value["filenames"] if p.endswith(".rlib")]
            assert len(candidates) == 1
            assert name not in artifacts or artifacts[name] == candidates[0]
            artifacts[name] = candidates[0]
        assert set(artifacts) == set(EXTERNS)
        helpers = {}
        for name, src in [("reading-proof", "docs/reports/premise-resolution-cycle2/scripts/attribute_reading_environments.rs"), ("metadata-replay", "docs/reports/w48-bottom-up-batch4/scripts/metadata_replay.rs")]:
            executable = out / name
            command = ["rustc", "--edition=2024", src, "-C", "debuginfo=0", "-L", "dependency=" + str(ROOT / "target/release/deps"), "-o", str(executable)]
            for external in EXTERNS:
                command += ["--extern", external + "=" + artifacts[external]]
            run(name + "-build", command)
            helpers[name] = dict(path=str(executable), sha256=sha(executable), source=src, source_sha256=sha(ROOT / src))
        exporter = ROOT / "target/debug/examples/premise_environment"
        assert exporter.is_file()
        (out / "helpers.json").write_text(json.dumps(dict(analyzer_source=source, reporting_source=head,
            helpers=helpers, environment_exporter=dict(path=str(exporter), sha256=sha(exporter), gate_stage="audit-examples", source=source),
            exact_rlibs={name:dict(path=path, sha256=sha(path)) for name,path in artifacts.items()}), indent=2) + "\n")
        link = ROOT / "docs/reports/w48-bottom-up-batch4/link-map.corpus.json"
        assert sha(link) == "98a4db761ace9fd4846b000c4c98defdc2e39ce6f95684c67c30f9b8b6f38ad3"
        for route in ["system", "slice", "corpus"]:
            run(route, [sys.executable, "docs/reports/w48-bottom-up-batch5/scripts/measure_strong.py", route, str(out / route / "tip"), "--link-map", str(link), "--inputs-snapshot", str(freeze / "inputs"), "--binary-source", source, "--jobs", "16"])
        guard()
        (out / "completion.json").write_text(json.dumps(dict(exit=0, reporting_source=head, analyzer_source=source,
            binary_sha256=binary_sha, command_count=len(records), commands_sha256=sha(out / "commands.json")), indent=2) + "\n")
    (out / "released.json").write_text(json.dumps(dict(utc=now()), indent=2) + "\n")
    print("NATIVE TIP ROUTES COMPLETE", flush=True)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
