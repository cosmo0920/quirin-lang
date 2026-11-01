#!/usr/bin/env python3
"""Build and verify deterministic first-party executable provider packages."""
import argparse
import gzip
import hashlib
import importlib.util
import io
import json
import os
import posixpath
import re
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import tomllib

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


# Card PACKAGE-BYTES-DEPEND-ON-CHECKOUT. The release build runs in a staged
# copy of the provider workspace, so a package's bytes do not depend on where
# the repository, CARGO_HOME, the target directory or the toolchain lie.
#
# Cargo's unit hash (`-C metadata`, which rustc turns into every crate's symbol
# disambiguator) hashes a path package by its path relative to the workspace
# root, or by its absolute path when it lies outside that root. The provider
# workspace's root is `providers/`, and its path dependencies `qcore-schemas`
# (and, for `conformance`, the analysis crates) lie in `../crates`, so their
# hashes, and the symbols and code layout of every executable linking them,
# took the checkout's absolute path. The stage puts the workspace root one level
# up: `<stage>/Cargo.toml` is `providers/Cargo.toml` with its members and path
# dependencies rebased under `providers/`, beside a copy of `providers/Cargo.lock`
# and of every path package's directory at its repository-relative path, so
# every path package lies inside the root. Then every remaining absolute path a
# build embeds is remapped to a constant: the stage (to `/qcore`, so source
# paths read `/qcore/providers/...` and `/qcore/crates/...` as before), the
# target directory, CARGO_HOME (registry sources) and the toolchain's `rust-src`
# (to rustc's own `/rustc/<commit-hash>`, the path a toolchain without
# `rust-src` embeds). `build` refuses an executable that still contains any of
# those paths, the repository's or the home directory, and `reproduce` builds a
# second time from another checkout path with another CARGO_HOME and compares
# the outputs byte for byte.
STAGE_DIRECTORY = "qcore-package-source"
STAGE_MEMBERS = "providers"
SOURCE_PREFIX = "/qcore"
TARGET_PREFIX = "/target"
CARGO_HOME_PREFIX = "/cargo"
WORKSPACE_KEYS = {"members", "resolver", "dependencies", "exclude"}
MANIFEST_KEYS = {"workspace", "profile"}
CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"


def cargo_home(env):
    """CARGO_HOME as cargo resolves it: the variable, else `~/.cargo`."""
    value = env.get("CARGO_HOME")
    return Path(os.path.abspath(value)) if value else Path.home() / ".cargo"


def rustc_identity(env):
    rustc = env.get("RUSTC", "rustc")
    version = subprocess.check_output([rustc, "-vV"], text=True, env=env)
    fields = dict(line.split(": ", 1) for line in version.splitlines() if ": " in line)
    sysroot = Path(subprocess.check_output([rustc, "--print", "sysroot"], text=True,
                                           env=env).strip())
    commit = fields.get("commit-hash", "")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError(f"rustc reports no commit hash to remap rust-src to: {commit!r}")
    return version, sysroot, commit


def relative_posix(path, root, what):
    try:
        relative = Path(path).relative_to(root)
    except ValueError:
        raise ValueError(f"{what} lies outside the repository: {path}") from None
    if not relative.parts:
        raise ValueError(f"{what} is the repository root itself: {path}")
    return relative.as_posix()


def workspace_path_packages(repo, env):
    """The provider workspace's members and other path packages, as
    repository-relative directories. A git or non-crates.io source is
    refused: the stage and `reproduce` only know path and registry packages."""
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline",
         "--manifest-path", str(repo / "providers" / "Cargo.toml")], env=env))
    members = set(metadata["workspace_members"])
    member_roots, other_roots = [], []
    for package in metadata["packages"]:
        source = package["source"]
        if source is not None:
            if source != CRATES_IO:
                raise ValueError(f"package {package['name']} has an unsupported source {source}")
            continue
        root = relative_posix(Path(package["manifest_path"]).parent, repo,
                              f"path package {package['name']}")
        (member_roots if package["id"] in members else other_roots).append(root)
    return sorted(member_roots), sorted(other_roots)


def rebase(path, what):
    rebased = posixpath.normpath(posixpath.join(STAGE_MEMBERS, path))
    if posixpath.isabs(path) or rebased == ".." or rebased.startswith("../"):
        raise ValueError(f"{what} path {path!r} leaves the repository")
    return rebased


def staged_manifest(manifest, excluded):
    """`providers/Cargo.toml` rebased for a workspace root one level up.

    Members and path dependencies get the `providers/` prefix; every path
    package that is not a member is excluded, so it stays a dependency rather
    than an automatic member. A key this function does not know is refused,
    never copied unrebased."""
    unknown = set(manifest) - MANIFEST_KEYS
    workspace = manifest.get("workspace")
    if unknown or not isinstance(workspace, dict) or set(workspace) - WORKSPACE_KEYS:
        raise ValueError(f"provider manifest has keys the stage cannot rebase: "
                         f"{sorted(unknown | set(workspace or {}) - WORKSPACE_KEYS)}")
    staged = {}
    for member in workspace["members"]:
        if any(char in member for char in "*?[") or member != posixpath.normpath(member):
            raise ValueError(f"provider workspace member {member!r} is not a plain path")
    staged["members"] = [rebase(member, "member") for member in workspace["members"]]
    staged["exclude"] = sorted(set(excluded) | {rebase(path, "exclude")
                                                 for path in workspace.get("exclude", [])})
    if "resolver" in workspace:
        staged["resolver"] = workspace["resolver"]
    dependencies = {}
    for name, spec in workspace.get("dependencies", {}).items():
        if isinstance(spec, dict) and "path" in spec:
            spec = dict(spec, path=rebase(spec["path"], f"dependency {name}"))
        dependencies[name] = spec
    if dependencies:
        staged["dependencies"] = dependencies
    result = {"workspace": staged}
    if "profile" in manifest:
        result["profile"] = manifest["profile"]
    return result


def toml_key(key):
    return key if re.fullmatch(r"[A-Za-z0-9_-]+", key) else json.dumps(key)


def toml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{toml_key(key)} = {toml_value(item)}"
                                for key, item in value.items()) + " }"
    raise ValueError(f"cannot write {type(value).__name__} value to the staged manifest")


def toml_document(data):
    """Write `data` as TOML; `stage` checks the result parses back to it."""
    lines = []

    def table(path, values):
        scalars = {key: item for key, item in values.items() if not isinstance(item, dict)}
        if scalars or not values:
            if path:
                lines.append(f"[{'.'.join(toml_key(key) for key in path)}]")
            lines.extend(f"{toml_key(key)} = {toml_value(item)}" for key, item in scalars.items())
            lines.append("")
        for key, item in values.items():
            if isinstance(item, dict):
                table(path + [key], item)

    table([], data)
    return "\n".join(lines)


def sync_tree(files, stage):
    """Make `stage` hold exactly `files` (relative path -> source path or bytes).

    A file whose bytes are unchanged keeps its modification time, so a reused
    target directory rebuilds only what changed."""
    for relative, source in files.items():
        destination = stage / relative
        data = source if isinstance(source, bytes) else source.read_bytes()
        if destination.is_symlink() or destination.is_dir():
            raise ValueError(f"stage entry is not a regular file: {destination}")
        if not destination.exists() or destination.read_bytes() != data:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
    for directory, names, entries in os.walk(stage, topdown=False):
        directory = Path(directory)
        for name in entries:
            path = directory / name
            if path.relative_to(stage).as_posix() not in files:
                path.unlink()
        if directory != stage and not any(directory.iterdir()):
            directory.rmdir()


def package_files(repo, roots, skip):
    """Every file under each package root, skipping `target` directories and
    the paths in `skip` (the target directory, which may lie inside)."""
    files = {}
    for root in roots:
        for directory, names, entries in os.walk(repo / root):
            directory = Path(directory)
            names[:] = sorted(name for name in names if name != "target"
                              and (directory / name).resolve() not in skip)
            for name in entries:
                path = directory / name
                files[path.relative_to(repo).as_posix()] = path
    return files


def stage(repo, target, env):
    """Write the staged workspace under `target` and return its root."""
    member_roots, other_roots = workspace_path_packages(repo, env)
    manifest = tomllib.loads((repo / "providers" / "Cargo.toml").read_text())
    staged = staged_manifest(manifest, other_roots)
    if sorted(staged["workspace"]["members"]) != member_roots:
        raise ValueError("staged members differ from the provider workspace's members")
    text = toml_document(staged)
    if tomllib.loads(text) != staged:
        raise ValueError("staged manifest does not parse back to its rebased content")
    root = target / STAGE_DIRECTORY
    root.mkdir(parents=True, exist_ok=True)
    files = package_files(repo, member_roots + other_roots, {target.resolve()})
    files["Cargo.toml"] = text.encode()
    files["Cargo.lock"] = repo / "providers" / "Cargo.lock"
    sync_tree(files, root)
    return root


def remap_flags(stage_root, target, home, sysroot, commit):
    """`--remap-path-prefix` flags; rustc applies the last one that matches,
    so the more specific prefixes come later."""
    pairs = [(sysroot / "lib" / "rustlib" / "src" / "rust", f"/rustc/{commit}"),
             (home, CARGO_HOME_PREFIX), (target, TARGET_PREFIX), (stage_root, SOURCE_PREFIX)]
    return [f"--remap-path-prefix={source}={replacement}" for source, replacement in pairs]


def embedded_paths(executable_bytes, paths):
    """The given absolute paths that occur in the executable's bytes."""
    found = []
    for path in paths:
        text = str(path)
        if len(Path(text).parts) > 1 and text.encode() in executable_bytes:
            found.append(text)
    return sorted(set(found))


def build(output, target, checks=True):
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    env = os.environ.copy()
    env.update(CARGO_INCREMENTAL="0", SOURCE_DATE_EPOCH="0")
    # Prevent ambient build flags from changing a supposedly identical package.
    env.pop("RUSTFLAGS", None)
    if checks:
        subprocess.run([sys.executable, str(ROOT / "contracts.py"), "check"], check=True)
    version, sysroot, commit = rustc_identity(env)
    home = cargo_home(env)
    stage_root = stage(REPO, target, env)
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(
        remap_flags(stage_root, target, home, sysroot, commit))
    command = ["cargo", "build", "--manifest-path", str(stage_root / "Cargo.toml"),
               "--locked", "--offline", "--release", "--target-dir", str(target), "-j", "2"]
    for template in templates():
        command += ["-p", template.parent.name]
    subprocess.run(command, check=True, env=env)
    if (stage_root / "Cargo.lock").read_bytes() != (ROOT / "Cargo.lock").read_bytes():
        raise ValueError("the staged build changed its copy of providers/Cargo.lock")
    forbidden = [REPO, stage_root, target, home, sysroot, Path.home()]
    forbidden += [path.resolve() for path in forbidden]
    for template in templates():
        executable = target / "release" / template.parent.name
        leaked = embedded_paths(executable.read_bytes(), forbidden)
        if leaked:
            raise ValueError(f"{executable} embeds build-host paths {leaked}: its bytes "
                             "would depend on the checkout")
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
        if checks:
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
              "rustc": version,
              "lock_sha256": digest(ROOT / "Cargo.lock")}
    (output / "packages.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


# The second build of `reproduce` copies the checkout's files except these
# top-level directories: `docs/` (1.4 GB of reports; no build reads it) and the
# submodule, which the provider build does not read either. Whatever else the
# second build would need and lacks fails it, never passes it.
REPRODUCE_SKIPPED = ("docs/", "third_party/")


def checkout_files(repo):
    """The checkout's tracked and untracked, not ignored, regular files."""
    listed = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        check=True, capture_output=True).stdout.decode().split("\0")
    files = {}
    for name in sorted(set(filter(None, listed))):
        path = repo / name
        if name.startswith(REPRODUCE_SKIPPED) or path.is_symlink() or not path.is_file():
            continue
        files[name] = path
    return files


def link_or_copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        os.link(source, destination)
    except OSError:
        shutil.copyfile(source, destination)


def index_entry(name):
    """A crate's path in a registry index (cargo's layout)."""
    name = name.lower()
    if len(name) <= 2:
        return f"{len(name)}/{name}"
    if len(name) == 3:
        return f"3/{name[0]}/{name}"
    return f"{name[:2]}/{name[2:4]}/{name}"


def populate_cargo_home(source, destination, lock):
    """A second CARGO_HOME holding exactly what an offline build of `lock`
    reads: each registry package's `.crate` file and index entry, and the
    cargo configuration files. Registry sources are unpacked there anew."""
    packages = [package for package in tomllib.loads(lock.read_text())["package"]
                if "source" in package]
    for package in packages:
        if package["source"] != CRATES_IO:
            raise ValueError(f"package {package['name']} has an unsupported source "
                             f"{package['source']}")
    copied = 0
    for name in ("config.toml", "config"):
        if (source / name).is_file():
            link_or_copy(source / name, destination / name)
    for kind in ("cache", "index"):
        for registry in sorted((source / "registry" / kind).glob("*")):
            target = destination / "registry" / kind / registry.name
            if kind == "index" and (registry / "config.json").is_file():
                link_or_copy(registry / "config.json", target / "config.json")
            for package in packages:
                relative = (f"{package['name']}-{package['version']}.crate" if kind == "cache"
                            else f".cache/{index_entry(package['name'])}")
                if (registry / relative).is_file():
                    link_or_copy(registry / relative, target / relative)
                    copied += 1
    return copied


def tree_bytes(root):
    return {path.relative_to(root).as_posix(): path.read_bytes()
            for path in sorted(root.rglob("*")) if path.is_file()}


def reproduce(output, target, work):
    """Build the packages twice and require identical bytes.

    The first build is `build` itself, into `output`. The second runs the
    `package.py` of a copy of this checkout at another path (under `work`),
    with another CARGO_HOME (also under `work`) and its own fresh target
    directory. It skips only the checks, which write nothing: `contracts.py
    check`, which the first build ran over the same sources, and each
    package's `verify`, which the first build ran on bytes the second must
    equal. (Both build root-workspace probes, which a second checkout would
    rebuild from nothing.) Every file of the two outputs, `packages.json` and
    the archives included, must be equal. After a match `work` and the first
    build's stage are removed; after a difference both are kept."""
    if work.exists():
        raise ValueError(f"work directory already exists: {work}")
    build(output, target)
    checkout = work / "second-checkout"
    for name, path in checkout_files(REPO).items():
        destination = checkout / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(path.read_bytes())
    env = os.environ.copy()
    home = work / "second-cargo-home"
    populate_cargo_home(cargo_home(env), home, ROOT / "Cargo.lock")
    env["CARGO_HOME"] = str(home)
    second = work / "second-packages"
    subprocess.run([sys.executable, str(checkout / "providers" / "package.py"), "build",
                    "--output", str(second), "--target-dir", str(work / "second-target"),
                    "--skip-checks"], check=True, env=env, stdout=subprocess.DEVNULL)
    first, again = tree_bytes(output), tree_bytes(second)
    differing = sorted(name for name in set(first) | set(again)
                       if first.get(name) != again.get(name))
    if differing:
        raise ValueError(f"package bytes depend on the checkout: {differing} differ between "
                         f"{output} and {second} (kept for inspection)")
    shutil.rmtree(work)
    # The first build's stage too (38 MB), so the target directory a check run
    # leaves is the release build alone, as before the stage existed (CI's
    # provider suite runs beside it); the next build writes the stage anew.
    shutil.rmtree(target / STAGE_DIRECTORY)
    print(f"reproduced {len(first)} files byte for byte from a second checkout path and "
          f"CARGO_HOME", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build_parser = sub.add_parser("build")
    build_parser.add_argument("--output", required=True, type=Path)
    build_parser.add_argument("--target-dir", type=Path, default=ROOT / "target" / "packaging")
    # `reproduce`'s second build only: no contracts.py check, no verify.
    build_parser.add_argument("--skip-checks", action="store_true",
                              help=argparse.SUPPRESS)
    reproduce_parser = sub.add_parser(
        "reproduce", help="build, then build again from a second checkout path and "
        "CARGO_HOME, and require byte-identical outputs")
    reproduce_parser.add_argument("--output", required=True, type=Path)
    reproduce_parser.add_argument("--target-dir", type=Path,
                                  default=ROOT / "target" / "packaging")
    reproduce_parser.add_argument("--work-dir", type=Path,
                                  help="the second checkout, CARGO_HOME, target and output "
                                  "(default: <output>.reproduce)")
    verify_parser = sub.add_parser("verify")
    verify_parser.add_argument("package", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        build(args.output.resolve(), args.target_dir.resolve(),
              checks=not args.skip_checks)
    elif args.command == "reproduce":
        output = args.output.resolve()
        work = (args.work_dir or output.with_name(output.name + ".reproduce")).resolve()
        reproduce(output, args.target_dir.resolve(), work)
    else:
        print(json.dumps(verify(args.package.resolve()), indent=2))


if __name__ == "__main__":
    main()
