#!/usr/bin/env python3
"""Build and verify deterministic first-party executable provider packages."""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def templates():
    return sorted(ROOT.glob("qcore-provider-*/provider-template.json"))


TEMPLATE_BEGIN = b"<<<qcore.provider-template.v1:begin>>>"
TEMPLATE_END = b"<<<qcore.provider-template.v1:end>>>"
CONTRACTS_BEGIN = b"<<<qcore.provider-contracts.v1:begin>>>"
CONTRACTS_END = b"<<<qcore.provider-contracts.v1:end>>>"


def embedded_template(executable_bytes):
    """Read the manifest template out of the executable without running it.

    Mirrors `qcore_verifier_host::embedded_template`, which is the checked
    implementation; this supplementary copy keeps `package.py verify` able to
    report the same refusal before it spawns anything.
    """
    if executable_bytes.count(TEMPLATE_BEGIN) != 1 or executable_bytes.count(TEMPLATE_END) != 1:
        raise ValueError("executable does not carry exactly one manifest template envelope")
    start = executable_bytes.index(TEMPLATE_BEGIN) + len(TEMPLATE_BEGIN)
    end = executable_bytes.index(TEMPLATE_END)
    if end < start:
        raise ValueError("executable's manifest template envelope is unterminated")
    return json.loads(executable_bytes[start:end].decode())


def embedded_contracts(executable_bytes):
    """Read the optional crate-owned contract inventory from an executable."""
    begins = executable_bytes.count(CONTRACTS_BEGIN)
    ends = executable_bytes.count(CONTRACTS_END)
    if begins == 0 and ends == 0:
        return None
    if begins != 1 or ends != 1:
        raise ValueError("executable does not carry exactly one provider-contracts envelope")
    start = executable_bytes.index(CONTRACTS_BEGIN) + len(CONTRACTS_BEGIN)
    end = executable_bytes.index(CONTRACTS_END)
    if end < start:
        raise ValueError("executable's provider-contracts envelope is unterminated")
    payload = executable_bytes[start:end]
    return payload, json.loads(payload.decode())


def regular_bytes(path, description):
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"missing or symbolic-link {description}: {path}")
    return path.read_bytes()


def contract_path(name):
    if not isinstance(name, str):
        raise ValueError("contract inventory paths must be strings")
    path = PurePosixPath(name)
    if (path.is_absolute() or not path.parts or path.parts[0] != "contracts"
            or any(part in ("", ".", "..") for part in path.parts)
            or path.suffix != ".qcore" or path.as_posix() != name or "\\" in name):
        raise ValueError(f"invalid contract inventory path: {name!r}")
    return path


def installed_contract_files(package):
    root = package / "contracts"
    if root.is_symlink() or not root.is_dir():
        raise ValueError(f"missing or symbolic-link contract directory: {root}")
    found = {}
    def unreadable(error):
        raise error

    for directory, names, files in os.walk(root, followlinks=False, onerror=unreadable):
        directory = Path(directory)
        for name in names:
            child = directory / name
            if child.is_symlink():
                raise ValueError(f"symbolic link in contract inventory: {child}")
        for name in files:
            child = directory / name
            relative = child.relative_to(package).as_posix()
            found[relative] = regular_bytes(child, "contract asset")
    return found


def verify_contracts(package, executable_bytes, plugin_id):
    embedded = embedded_contracts(executable_bytes)
    inventory_path = package / "contracts.json"
    installed = inventory_path.exists() or inventory_path.is_symlink()
    contract_root = package / "contracts"
    assets = contract_root.exists() or contract_root.is_symlink()
    if embedded is None and not installed and not assets:
        return None  # Compatibility for old and test-only provider packages.
    if embedded is None:
        raise ValueError("installed contract bundle has no embedded inventory")
    if not installed:
        raise ValueError("embedded contract inventory has no installed contracts.json")
    payload, inventory = embedded
    installed_bytes = regular_bytes(inventory_path, "contract inventory")
    if json.loads(installed_bytes.decode()) != inventory:
        raise ValueError("installed contracts.json diverges from the embedded inventory")
    if set(inventory) != {"schema_version", "plugin_id", "contracts"}:
        raise ValueError("contract inventory has unknown or missing fields")
    if inventory["schema_version"] != 1 or inventory["plugin_id"] != plugin_id:
        raise ValueError("contract inventory schema or plugin identity mismatch")
    declared = inventory["contracts"]
    if not isinstance(declared, dict) or not declared:
        raise ValueError("contract inventory must declare a nonempty contracts object")
    normalized = {}
    for name, expected in declared.items():
        path = contract_path(name)
        if not isinstance(expected, str) or len(expected) != 64 or any(
                char not in "0123456789abcdef" for char in expected):
            raise ValueError(f"invalid SHA-256 for contract asset: {name!r}")
        normalized[path.as_posix()] = expected
    if len(normalized) != len(declared):
        raise ValueError("contract inventory contains colliding paths")
    found = installed_contract_files(package)
    if set(found) != set(normalized):
        missing = sorted(set(normalized) - set(found))
        extra = sorted(set(found) - set(normalized))
        raise ValueError(f"contract asset set mismatch: missing={missing}, extra={extra}")
    for name, expected in normalized.items():
        actual = hashlib.sha256(found[name]).hexdigest()
        if actual != expected:
            raise ValueError(f"contract asset digest mismatch: {name}")
    return {"payload_sha256": hashlib.sha256(payload).hexdigest(), "contracts": len(normalized)}


def verify(package):
    manifest = json.loads(regular_bytes(package / "verifier-plugin.json", "provider manifest"))
    if manifest.get("entrypoint") != {"kind": "executable", "path": "provider"}:
        raise ValueError("first-party package entrypoint must be provider")
    executable = package / "provider"
    executable_bytes = regular_bytes(executable, "provider executable")
    if digest(executable) != manifest["package_digest"]:
        raise ValueError(f"executable digest mismatch: {package}")
    # Bind every other declaration to the embedded template before executing.
    template = embedded_template(executable_bytes)
    declared = dict(manifest)
    declared.pop("package_digest")
    embedded = dict(template)
    embedded.pop("package_digest")
    for field in sorted(set(declared) | set(embedded)):
        if declared.get(field) != embedded.get(field):
            raise ValueError(
                f"installed manifest field `{field}` diverges from the executable's "
                f"embedded template: {package}"
            )
    contracts = verify_contracts(package, executable_bytes, manifest["plugin_id"])
    checked = subprocess.run([str(executable.resolve()), "--verify-package"],
                             cwd=package, check=True, capture_output=True, text=True, timeout=10, env={})
    if json.loads(checked.stdout) != manifest:
        raise ValueError(f"executable manifest mismatch: {package}")
    if contracts is not None:
        checked_contracts = subprocess.run(
            [str(executable.resolve()), "--contract-manifest"], cwd=package, check=True,
            capture_output=True, text=True, timeout=10, env={})
        if json.loads(checked_contracts.stdout) != json.loads(
                (package / "contracts.json").read_text()):
            raise ValueError(f"executable contract inventory mismatch: {package}")
    return manifest


def archive(package, output):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        contents = sorted(package.rglob("*"))
        for path in contents:
            if path.is_symlink():
                raise ValueError(f"symbolic link in package: {path}")
        files = sorted(
            (item for item in contents if item.is_file()),
            key=lambda item: item.relative_to(package).as_posix(),
        )
        for path in files:
            payload = path.read_bytes()
            relative = path.relative_to(package).as_posix()
            info = tarfile.TarInfo(f"{package.name}/{relative}")
            info.size = len(payload)
            info.mode = 0o755 if relative == "provider" else 0o644
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
    subprocess.run([sys.executable, str(ROOT / "contracts.py"), "check"], check=True)
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
        shutil.copyfile(template.parent / "contracts.json", package / "contracts.json")
        shutil.copytree(template.parent / "contracts", package / "contracts")
        manifest["package_digest"] = digest(package / "provider")
        (package / "verifier-plugin.json").write_text(json.dumps(manifest, indent=2) + "\n")
        verify(package)
        archive_path = output / (package.name + ".tar.gz")
        archive(package, archive_path)
        artifacts.append({"plugin_id": manifest["plugin_id"],
                          "executable_sha256": manifest["package_digest"],
                          "manifest_sha256": digest(package / "verifier-plugin.json"),
                          "contracts_manifest_sha256": digest(package / "contracts.json"),
                          "archive_sha256": digest(archive_path)})
    record = {"schema_version": 1, "protocol_version": 1, "evidence_versions": [1, 2],
              "semantic_profile": "per_package_declared_profiles", "artifacts": artifacts,
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
