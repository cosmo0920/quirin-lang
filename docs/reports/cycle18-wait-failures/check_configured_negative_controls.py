#!/usr/bin/env python3
"""Drive configured wait-failures refusal controls from an accepted config."""
import argparse, copy, hashlib, json, pathlib, subprocess

def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path, value): path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=pathlib.Path, required=True)
    parser.add_argument("--config", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    args.binary = args.binary.resolve()
    args.config = args.config.resolve()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    baseline = json.loads(args.config.read_text())
    controls = []

    def run(name, mutate):
        directory = args.output / name
        directory.mkdir()
        config = copy.deepcopy(baseline)
        mutate(config, directory)
        report = directory / "report.json"
        if name != "report-alias": config["report"] = str(report)
        config_path = directory / "config.json"
        write(config_path, config)
        result = subprocess.run([str(args.binary), "wait-failures", str(config_path)],
                                capture_output=True, text=True)
        (directory / "stdout.log").write_text(result.stdout)
        (directory / "stderr.log").write_text(result.stderr)
        if result.returncode == 0 or report.exists():
            raise SystemExit(f"negative control unexpectedly published: {name}")
        controls.append({"name": name, "exit": result.returncode,
                         "stderr": result.stderr.strip()})

    def substitute(field, filename):
        def mutation(config, directory):
            source = pathlib.Path(config[field]); target = directory / filename
            target.write_bytes(source.read_bytes() + b"\nchanged\n"); config[field] = str(target)
        return mutation

    run("harness-substitution", substitute("observer_harness", "harness.c"))
    run("header-substitution", substitute("observer_header", "harness.h"))
    run("fixture-substitution", substitute("wrapper_fixture", "wrappers.c"))
    run("policy-substitution", substitute("failure_policy", "policy.qcore"))

    def semantic_mutation(config, directory):
        evidence = json.loads(pathlib.Path(config["observations"]).read_text())
        evidence[4].pop("timeout_activated")
        evidence_path = directory / "observations.json"; write(evidence_path, evidence)
        manifest = json.loads(pathlib.Path(config["run_manifest"]).read_text())
        manifest["observations_sha256"] = digest(evidence_path)
        manifest_path = directory / "manifest.json"; write(manifest_path, manifest)
        config["observations"] = str(evidence_path); config["run_manifest"] = str(manifest_path)
    run("missing-required-evidence", semantic_mutation)

    def trust_mutation(config, directory):
        manifest = json.loads(pathlib.Path(config["run_manifest"]).read_text())
        manifest["scope"]["dependency_behavior"] = "real Libevent execution"
        path = directory / "manifest.json"; write(path, manifest); config["run_manifest"] = str(path)
    run("invented-real-libevent-class", trust_mutation)

    def revision_mutation(config, directory):
        manifest = json.loads(pathlib.Path(config["run_manifest"]).read_text())
        manifest["source_revision"] = "0" * 40
        path = directory / "manifest.json"; write(path, manifest); config["run_manifest"] = str(path)
    run("source-revision-substitution", revision_mutation)

    def unknown_config(config, _): config["accept_any_failure"] = True
    run("unknown-config-field", unknown_config)

    protected = pathlib.Path(baseline["observations"]); protected_before = digest(protected)
    def alias(config, _): config["report"] = str(protected)
    run("report-alias", alias)
    if digest(protected) != protected_before: raise SystemExit("report alias changed protected evidence")

    write(args.output / "results.json", {"schema_version": 1, "controls": controls})

if __name__ == "__main__": main()
