#!/usr/bin/env python3
"""Build and verify deterministic first-party executable provider packages."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def templates():
    return sorted(ROOT.glob("qcore-provider-*/provider-template.json"))


def verify(package):
    manifest = json.loads((package / "verifier-plugin.json").read_text())
    if manifest.get("entrypoint") != {"kind": "executable", "path": "provider"}:
        raise ValueError("first-party package entrypoint must be provider")
    executable = package / "provider"
    if digest(executable) != manifest["package_digest"]:
        raise ValueError(f"executable digest mismatch: {package}")
    checked = subprocess.run([str(executable.resolve()), "--verify-package"],
                             cwd=package, check=True, capture_output=True, text=True, timeout=10, env={})
    if json.loads(checked.stdout) != manifest:
        raise ValueError(f"executable manifest mismatch: {package}")
    return manifest


def archive(package, output):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        for path in sorted(package.iterdir()):
            payload = path.read_bytes()
            info = tarfile.TarInfo(f"{package.name}/{path.name}")
            info.size = len(payload)
            info.mode = 0o755 if path.name == "provider" else 0o644
            info.mtime = 0
            tar.addfile(info, io.BytesIO(payload))
    with output.open("wb") as handle:
        with gzip.GzipFile(filename="", mode="wb", fileobj=handle, mtime=0) as zipped:
            zipped.write(buffer.getvalue())


def build(output, target):
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    env = os.environ.copy()
    env.update(CARGO_INCREMENTAL="0", SOURCE_DATE_EPOCH="0")
    # Prevent ambient build flags from changing a supposedly identical package.
    env.pop("RUSTFLAGS", None)
    env["CARGO_ENCODED_RUSTFLAGS"] = f"--remap-path-prefix={REPO}=/qcore"
    command = ["cargo", "build", "--manifest-path", str(ROOT / "Cargo.toml"),
               "--locked", "--offline", "--release", "--target-dir", str(target), "-j", "2"]
    for template in templates():
        command += ["-p", template.parent.name]
    subprocess.run(command, check=True, env=env)
    output.mkdir(parents=True)
    artifacts = []
    for template in templates():
        manifest = json.loads(template.read_text())
        package = output / manifest["plugin_id"]
        package.mkdir()
        executable = target / "release" / template.parent.name
        shutil.copyfile(executable, package / "provider")
        (package / "provider").chmod(0o755)
        manifest["package_digest"] = digest(package / "provider")
        (package / "verifier-plugin.json").write_text(json.dumps(manifest, indent=2) + "\n")
        verify(package)
        archive_path = output / (package.name + ".tar.gz")
        archive(package, archive_path)
        artifacts.append({"plugin_id": manifest["plugin_id"],
                          "executable_sha256": manifest["package_digest"],
                          "manifest_sha256": digest(package / "verifier-plugin.json"),
                          "archive_sha256": digest(archive_path)})
    record = {"schema_version": 1, "protocol_version": 1, "evidence_versions": [1, 2],
              "semantic_profile": "unknown_only_transport", "artifacts": artifacts,
              "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
              "lock_sha256": digest(ROOT / "Cargo.lock")}
    (output / "packages.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build_parser = sub.add_parser("build")
    build_parser.add_argument("--output", required=True, type=Path)
    build_parser.add_argument("--target-dir", type=Path, default=ROOT / "target" / "packaging")
    verify_parser = sub.add_parser("verify")
    verify_parser.add_argument("package", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        build(args.output.resolve(), args.target_dir.resolve())
    else:
        print(json.dumps(verify(args.package.resolve()), indent=2))


if __name__ == "__main__":
    main()
