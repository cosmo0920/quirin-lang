#!/usr/bin/env python3
"""Build and verify deterministic first-party executable provider packages."""
import argparse
import gzip
import hashlib
import importlib.util
import io
import json
import os
import re
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
            or path.suffix not in (".qcore", ".qv") or path.as_posix() != name or "\\" in name):
        raise ValueError(f"invalid contract inventory path: {name!r}")
    return path


def installed_contract_files(package, tree="contracts"):
    root = package / tree
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


def reject_dual_stems(paths):
    """Refuse two extension spellings for one exact logical module path."""
    stems = {}
    for name in sorted(paths):
        path = PurePosixPath(name)
        stem = path.with_suffix("").as_posix()
        if stem in stems:
            raise ValueError(f"ambiguous contract asset stem: {stems[stem]} and {name}")
        stems[stem] = name


def root_test_config_args():
    """The root workspace's test-build `--config` arguments.

    `scripts/test_link_config.py` is their one authority (docs/agents/validation.md
    "Test build storage"); it is loaded by path, as this file is by its callers.
    """
    spec = importlib.util.spec_from_file_location(
        "qcore_test_link_config", REPO / "scripts" / "test_link_config.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.cargo_config_args()


def root_workspace_executable(kind, name, probe, manifest=REPO / "Cargo.toml"):
    """Build one root-workspace `example` or `bin` and return its executable.

    Card CI-RUST-JOB-PEAK. The probes this tooling runs (the source-mode probe
    here; the view, inventory and lock probes of `contracts.py`) are built in
    the root workspace's own `target/` with the test-build configuration and
    the whole workspace's feature selection (`--workspace`), so each is the
    unit a configured `cargo test --workspace` already built and nothing is
    rebuilt beside it. A plain `cargo run -p <crate>` built the crate and its
    dependencies a second time, statically and with that crate's features
    alone. The probes only read and classify sources, so neither the
    configuration nor the features reach a package's bytes, whose release
    build `build()` runs in its own target directory as before.
    """
    if kind not in ("example", "bin"):
        raise ValueError(f"unknown probe target kind {kind!r}")
    command = ["cargo", "build", "--quiet", "--offline", "--locked",
               *root_test_config_args(),
               "--manifest-path", str(manifest), "--workspace",
               f"--{kind}", name, "--message-format=json-render-diagnostics"]
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode:
        raise ValueError(f"{probe} failed (exit {result.returncode}): building {kind} {name}: "
                         f"{result.stderr.strip()}")
    executables = []
    for line in result.stdout.splitlines():
        message = json.loads(line) if line.startswith("{") else {}
        target = message.get("target") or {}
        if (message.get("reason") == "compiler-artifact" and target.get("name") == name
                and target.get("kind") == [kind] and message.get("executable")):
            executables.append(message["executable"])
    if len(executables) != 1:
        raise ValueError(f"{probe} failed: building {kind} {name} named "
                         f"{len(executables)} executables, expected 1")
    return executables[0]


def validate_source_modes(sources, warn_legacy=False):
    """Use the frontend's header parser for new contracts and legacy warnings.

    `sources` maps exact recorded names to their on-disk files. The probe
    classifies header intent only; package and loader verification still own
    all declaration checks. Never infer mode from suffix or source regexes.
    """
    selected = {name: Path(path).resolve() for name, path in sources.items()
                if name.endswith(".qv") or (warn_legacy and name.endswith(".qcore"))}
    if not selected:
        return
    command = [root_workspace_executable("example", "source_extension_modes",
                                        "source mode probe"),
               *(str(path) for path in selected.values())]
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode:
        raise ValueError(f"source mode probe failed (exit {result.returncode}): "
                         f"{result.stderr.strip()}")
    rows = json.loads(result.stdout)
    if (not isinstance(rows, list) or len(rows) != len(selected)
            or [row.get("path") for row in rows] != [str(path) for path in selected.values()]):
        raise ValueError("source mode probe returned missing, extra or reordered paths")
    for (name, path), row in zip(selected.items(), rows):
        mode = row.get("mode")
        if name.endswith(".qv") and mode != "contract":
            detail = row.get("error") or f"{mode} source mode"
            raise ValueError(f"{path}: .qv requires contract mode: {detail}")
        if name.endswith(".qcore"):
            recommended = ".qv" if mode in ("contract", None) else ".qr"
            print(f"warning[legacy-source-extension]: {path}: outdated extension "
                  f"({mode or 'unknown'} mode); use {recommended}", file=sys.stderr)


# Card CONTRACT-INHERIT. A contract module may name another crate's published
# module with `#include "crate:module";`. The grammar is the Rust scanner's
# (`qcore_frontend::contract_inclusion`): a lowercase crate name, `:`, and a
# relative `[a-z0-9_/-]+.(qcore|qv)` module path with nonempty segments.
INCLUDE_LINE = re.compile(r"^[ \t]*#[ \t]*include\b")
INCLUDE_DIRECTIVE = re.compile(
    r'^[ \t]*#include[ \t]+"([a-z0-9][a-z0-9_-]*):([a-z0-9_-]+(?:/[a-z0-9_-]+)*\.(?:qcore|qv))"[ \t]*;[ \t]*$')
INCLUDE_PIN_FIELDS = {"crate", "version", "module", "sha256"}


def contract_include_targets(text, name):
    """The `crate:module` targets a contract text includes, in order.

    Comments are removed first, so prose about a directive is not one. A line
    that starts a directive but is not exactly the admitted form is refused,
    never skipped."""
    without_blocks = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    targets = []
    for line in without_blocks.split("\n"):
        line = line.split("//", 1)[0]
        if not INCLUDE_LINE.match(line):
            continue
        match = INCLUDE_DIRECTIVE.match(line)
        if match is None:
            raise ValueError(f"malformed #include directive in {name}: {line.strip()!r}")
        targets.append(f"{match.group(1)}:{match.group(2)}")
    return targets


def is_sha256_hex(value):
    return (isinstance(value, str) and len(value) == 64
            and all(char in "0123456789abcdef" for char in value))


def verify_contract_includes(package, inventory, found):
    """Check the identities of every included parent a package's contracts
    reach (card CONTRACT-INHERIT).

    A package whose contracts include another module must pin each reached
    module (its crate, crate version, module path and SHA-256) in the
    inventory's `includes` object and install its exact text under
    `includes/<crate>/<module>`. Verification fails if a directive is
    unpinned, a pin is unused, a pinned text is missing or changed, or an
    `includes/` tree exists without pins. Returns the number of pins, or None
    for a package that includes nothing, which is every package today."""
    pins = inventory.get("includes")
    tree = package / "includes"
    tree_present = tree.exists() or tree.is_symlink()
    pending = []
    for name, data in sorted(found.items()):
        pending.extend(contract_include_targets(data.decode("utf-8"), name))
    if pins is None:
        if pending:
            raise ValueError(f"contract includes are not pinned: {sorted(set(pending))}")
        if tree_present:
            raise ValueError("installed includes/ tree has no pinned identities")
        return None
    if not isinstance(pins, dict) or not pins:
        raise ValueError("contract include pins must be a nonempty object")
    installed = installed_contract_files(package, "includes")
    expected_files = {}
    for key, pin in pins.items():
        if not isinstance(pin, dict) or set(pin) != INCLUDE_PIN_FIELDS:
            raise ValueError(f"include pin {key!r} has unknown or missing fields")
        crate, module = pin["crate"], pin["module"]
        if (not isinstance(crate, str) or not isinstance(module, str)
                or f"{crate}:{module}" != key
                or not INCLUDE_DIRECTIVE.match(f'#include "{key}";')):
            raise ValueError(f"include pin {key!r} does not name its crate and module")
        if not isinstance(pin["version"], str) or not pin["version"]:
            raise ValueError(f"include pin {key!r} has no crate version")
        if not is_sha256_hex(pin["sha256"]):
            raise ValueError(f"invalid SHA-256 for include pin {key!r}")
        expected_files[f"includes/{crate}/{module}"] = key
    reject_dual_stems(expected_files)
    reject_dual_stems(installed)
    if set(installed) != set(expected_files):
        missing = sorted(set(expected_files) - set(installed))
        extra = sorted(set(installed) - set(expected_files))
        raise ValueError(f"included parent set mismatch: missing={missing}, extra={extra}")
    reached = set()
    while pending:
        key = pending.pop()
        if key in reached:
            continue
        if key not in pins:
            raise ValueError(f"included parent {key} is not pinned")
        reached.add(key)
        path = f"includes/{pins[key]['crate']}/{pins[key]['module']}"
        data = installed[path]
        if hashlib.sha256(data).hexdigest() != pins[key]["sha256"]:
            raise ValueError(f"included parent changed: {key}")
        pending.extend(contract_include_targets(data.decode("utf-8"), path))
    unused = sorted(set(pins) - reached)
    if unused:
        raise ValueError(f"pinned included parents no contract reaches: {unused}")
    return len(pins)


def install_includes(package, inventory, root=ROOT):
    """Install every pinned included parent (card CONTRACT-INHERIT-ADOPTION).

    `providers/contracts.py sync` writes the pins into the provider's
    `contracts.json` from the load-path manifest; the build copies each pinned
    module's text from its crate's contract directory, as that manifest names
    it, to `includes/<crate>/<module>`, and refuses a text whose digest is not
    its pin. `verify` then checks the installed tree. A package without pins
    installs nothing, which is every package today."""
    pins = inventory.get("includes")
    if not pins:
        return 0
    load_path = json.loads((root / "contract-load-path.json").read_text())
    crates = {record["name"]: record for record in load_path["crates"]}
    for key, pin in sorted(pins.items()):
        record = crates.get(pin["crate"])
        if record is None or record["version"] != pin["version"] or f"{pin['crate']}:{pin['module']}" != key:
            raise ValueError(f"include pin {key} does not match the load path")
        source = root / record["contracts"] / pin["module"]
        data = regular_bytes(source, "included parent")
        if hashlib.sha256(data).hexdigest() != pin["sha256"]:
            raise ValueError(f"included parent changed since it was pinned: {key}")
        target = package / "includes" / pin["crate"] / pin["module"]
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    return len(pins)


def verify_contracts(package, executable_bytes, plugin_id, warn_legacy=False):
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
    if set(inventory) - {"includes"} != {"schema_version", "plugin_id", "contracts"}:
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
    reject_dual_stems(normalized)
    found = installed_contract_files(package)
    reject_dual_stems(found)
    if set(found) != set(normalized):
        missing = sorted(set(normalized) - set(found))
        extra = sorted(set(found) - set(normalized))
        raise ValueError(f"contract asset set mismatch: missing={missing}, extra={extra}")
    for name, expected in normalized.items():
        actual = hashlib.sha256(found[name]).hexdigest()
        if actual != expected:
            raise ValueError(f"contract asset digest mismatch: {name}")
    includes = verify_contract_includes(package, inventory, found)
    source_paths = {name: package / name for name in found}
    if includes is not None:
        source_paths.update({name: package / name
                             for name in installed_contract_files(package, "includes")})
    validate_source_modes(source_paths, warn_legacy=warn_legacy)
    result = {"payload_sha256": hashlib.sha256(payload).hexdigest(), "contracts": len(normalized)}
    if includes is not None:
        result["includes"] = includes
    return result


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
    contracts = verify_contracts(package, executable_bytes, manifest["plugin_id"],
                                 warn_legacy=True)
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
        install_includes(package, json.loads((template.parent / "contracts.json").read_text()))
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
