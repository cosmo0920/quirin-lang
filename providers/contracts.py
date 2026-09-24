#!/usr/bin/env python3
"""Maintain the inventories for canonical provider-owned contracts, and the
published contract load path (card CONTRACT-INHERIT-ADOPTION).

Edit contracts in their provider crate, then run `sync`. `check` never writes.
Both also maintain `contract-load-path.json`: one record per
`quirin-provider-*` and `qcore-provider-*` contract crate from `cargo
metadata` and the crate's `NAMESPACE` and `PUBLISHED` constants, with every
published module's SHA-256 pinned; and a provider inventory's `includes`
pins, resolved on that load path.
The retired ``contract/qcore`` compatibility tree is refused so it cannot
silently become a second source of provider contracts again.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
# Ownership is intentionally narrow: shared language/support policies stay in
# the providers/quirin-provider-*/contracts data crates. The ChunkIO provider also handles
# Fluent Bit's busy-state API.
OWNERS = {
    "qcore-provider-chunkio": ("chunkio", "fluent_bit/busy_state.qcore"),
    "qcore-provider-cmetrics": ("cmetrics",),
    "qcore-provider-ctraces": ("ctraces",),
    "qcore-provider-libevent": ("monkey",),
    "qcore-provider-librdkafka": ("librdkafka",),
}


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def owned(name, selectors):
    return any(name == selector or name.startswith(selector + "/") for selector in selectors)


def files(directory):
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"missing or symbolic-link contract directory: {directory}")
    result = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"symbolic link in contract inventory: {path}")
        if path.is_file():
            name = path.relative_to(directory).as_posix()
            if path.suffix != ".qcore" or not re.fullmatch(r"[a-z0-9_/-]+\.qcore", name):
                raise ValueError(f"unexpected contract asset: {path}")
            result[name] = path
    if not result:
        raise ValueError(f"empty provider contract inventory: {directory}")
    return result


# Card CONTRACT-INHERIT-ADOPTION: the published contract load path, generated
# from Cargo metadata and each contract crate's recorded constants, and read by
# core consumers as data (`qcore_contract::load_path_manifest`).
LOAD_PATH = "contract-load-path.json"
# Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING (the user's option B): exactly
# two package-name prefixes, `qcore_contract::contract_inclusion::
# CONTRACT_CRATE_PREFIXES`. The namespace is always derived from the package
# name; there is no freely recorded namespace.
CONTRACT_CRATE_PREFIXES = ("quirin-provider-", "qcore-provider-")
# Workspace packages that carry an admitted prefix but own no contracts: the
# conformance harness (`providers/conformance`) is a test crate, not a
# contract crate. Listed by name, and checked to own no contracts.
NON_CONTRACT_PACKAGES = ("qcore-provider-conformance",)
NAMESPACE_CONST = re.compile(r'^pub const NAMESPACE: &str = "([a-z_][a-z0-9_]*)";$', re.M)
PUBLISHED_CONST = re.compile(r'^pub const PUBLISHED: &\[&str\] = &\[([^\]]*)\];$', re.M)
STRING_LITERAL = re.compile(r'\s*"([a-z0-9_/-]+\.qcore)"\s*')


def cargo_metadata(root):
    """`cargo metadata --no-deps` for the providers workspace: every workspace
    package with its version and its declared dependencies."""
    command = ["cargo", "metadata", "--manifest-path", str(Path(root) / "Cargo.toml"),
               "--format-version", "1", "--no-deps", "--offline", "--locked"]
    completed = subprocess.run(command, check=True, capture_output=True, text=True)
    return json.loads(completed.stdout)


def derive_namespace(name):
    """The namespace a contract crate's package name derives: the name without
    its admitted prefix, `-` read as `_`, identifier-shaped. None otherwise
    (the Rust `derive_contract_namespace`)."""
    for prefix in CONTRACT_CRATE_PREFIXES:
        if name.startswith(prefix):
            namespace = name[len(prefix):].replace("-", "_")
            return namespace if re.fullmatch(r"[a-z_][a-z0-9_]*", namespace) else None
    return None


def constants_source(package):
    """The Rust source that records a crate's constants: its library root, or
    for an executable crate (no library target) its one binary root."""
    targets = package.get("targets", [])
    libs = [t for t in targets if "lib" in t.get("kind", [])]
    bins = [t for t in targets if "bin" in t.get("kind", [])]
    chosen = libs if libs else bins
    if len(chosen) != 1:
        raise ValueError(f"{package['name']}: expected one library or one binary target")
    return Path(chosen[0]["src_path"])


def recorded_constants(lib):
    """The `NAMESPACE` and `PUBLISHED` constants a contract crate records beside
    `CONTRACTS`. Exactly one of each, in the one admitted spelling; anything
    else is refused, never guessed."""
    text = lib.read_text()
    namespaces = NAMESPACE_CONST.findall(text)
    published = PUBLISHED_CONST.findall(text)
    if len(namespaces) != 1 or len(published) != 1:
        raise ValueError(f"{lib}: expected one NAMESPACE and one PUBLISHED constant")
    items = [item for item in published[0].split(",") if item.strip()]
    modules = []
    for item in items:
        match = STRING_LITERAL.fullmatch(item)
        if match is None:
            raise ValueError(f"{lib}: malformed PUBLISHED entry {item!r}")
        modules.append(match.group(1))
    if modules != sorted(set(modules)):
        raise ValueError(f"{lib}: PUBLISHED must be sorted and unique")
    return namespaces[0], modules


def load_path_manifest(root=ROOT, metadata=None):
    """The load-path manifest: one record per `quirin-provider-*` and
    `qcore-provider-*` workspace contract crate (card
    CONTRACT-INHERIT-QCORE-PROVIDER-NAMING). Name, version and dependencies
    come from Cargo metadata; a dependency counts only when it is a normal (not
    dev or build) dependency on another load-path crate. Namespace and
    published list come from the crate's constants (its `src/lib.rs`, or an
    executable crate's `src/main.rs`); the namespace must be the derived one,
    and no two crates may derive one namespace. Every published module's
    SHA-256 is pinned."""
    root = Path(root)
    metadata = cargo_metadata(root) if metadata is None else metadata
    packages = {package["name"]: package for package in metadata["packages"]}
    for name in NON_CONTRACT_PACKAGES:
        package = packages.get(name)
        if package is not None and (Path(package["manifest_path"]).resolve().parent
                                    / "contracts").exists():
            raise ValueError(f"{name} is listed as owning no contracts but has a contracts/ tree")
    names = sorted(name for name in packages
                   if name.startswith(CONTRACT_CRATE_PREFIXES) and name not in NON_CONTRACT_PACKAGES)
    if not names:
        raise ValueError("no contract crate on the load path")
    crates = []
    owners = {}
    for name in names:
        package = packages[name]
        directory = Path(package["manifest_path"]).resolve().parent
        if directory != (root / name).resolve():
            raise ValueError(f"contract crate {name} is not at providers/{name}")
        source = constants_source(package).resolve()
        if source.parent != directory / "src":
            raise ValueError(f"{name}: constants source {source} is not under src/")
        namespace, published = recorded_constants(source)
        derived = derive_namespace(name)
        if derived is None:
            raise ValueError(f"{name}: the package name derives no identifier-shaped namespace")
        if derived in owners:
            raise ValueError(f"{name}: namespace {derived!r} is also derived by {owners[derived]}")
        owners[derived] = name
        if namespace != derived:
            raise ValueError(f"{name}: recorded namespace {namespace!r} is not the derived {derived!r}")
        modules = files(directory / "contracts")
        missing = [module for module in published if module not in modules]
        if missing:
            raise ValueError(f"{name}: publishes modules it does not own: {missing}")
        dependencies = sorted({dependency["name"] for dependency in package["dependencies"]
                               if dependency["name"] in names and dependency.get("kind") is None})
        crates.append({
            "name": name,
            "namespace": namespace,
            "version": package["version"],
            "contracts": f"{name}/contracts",
            "dependencies": dependencies,
            "modules": sorted(modules),
            "published": {module: sha256(modules[module]) for module in published},
        })
    return {"schema_version": 1, "crates": crates}


def package_module():
    """`package.py`, loaded by path, so pins are computed with the verifier's
    own directive scanner."""
    spec = importlib.util.spec_from_file_location("qcore_provider_package", ROOT / "package.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def include_pins(contracts, load_path, root=ROOT):
    """The `includes` pins of a provider inventory: every module its contracts
    reach through `#include "crate:module";`, transitively, resolved on the
    load path (crate, crate version, module, SHA-256). Empty when nothing is
    included. A target that is not a published module of a load-path crate is
    refused here, before any package is built."""
    contract_include_targets = package_module().contract_include_targets
    crates = {record["name"]: record for record in load_path["crates"]}
    pending = []
    for name, path in contracts.items():
        pending.extend(contract_include_targets(path.read_text(), name))
    pins = {}
    while pending:
        key = pending.pop()
        if key in pins:
            continue
        crate, module = key.split(":", 1)
        record = crates.get(crate)
        if record is None or module not in record["published"]:
            raise ValueError(f"#include \"{key}\" is not a published load-path module")
        path = Path(root) / record["contracts"] / module
        pins[key] = {"crate": crate, "version": record["version"], "module": module,
                     "sha256": sha256(path)}
        pending.extend(contract_include_targets(path.read_text(), key))
    return dict(sorted(pins.items()))


def maintain_load_path(root=ROOT, sync=False, metadata=None):
    """Write (`sync`) or compare (`check`) the load-path manifest. Returns it."""
    root = Path(root)
    load_path = load_path_manifest(root, metadata)
    load_path_file = root / LOAD_PATH
    if load_path_file.is_symlink():
        raise ValueError(f"symbolic-link load-path manifest: {load_path_file}")
    if sync:
        load_path_file.write_text(json.dumps(load_path, indent=2) + "\n")
    elif not load_path_file.is_file() or json.loads(load_path_file.read_text()) != load_path:
        raise ValueError(f"stale contract load-path manifest: {load_path_file}")
    return load_path


def maintain(root=ROOT, sync=False, load_path=None):
    """Provider inventories. `load_path` resolves include pins; without one no
    module is published, so any `#include` in a provider contract is refused."""
    root = Path(root)
    legacy = root.parent / "contract/qcore"
    if legacy.exists() or legacy.is_symlink():
        raise ValueError(f"legacy contract corpus must be absent: {legacy}")
    load_path = {"schema_version": 1, "crates": []} if load_path is None else load_path
    rows = []
    claimed = set()
    for crate, selectors in OWNERS.items():
        provider = root / crate
        if provider.is_symlink():
            raise ValueError(f"symbolic-link provider directory: {provider}")
        canonical = files(provider / "contracts")
        template_path = provider / "provider-template.json"
        if template_path.is_symlink() or not template_path.is_file():
            raise ValueError(f"missing or symbolic-link provider template: {template_path}")
        template = json.loads(template_path.read_text())
        for name in canonical:
            if not owned(name, selectors) or name in claimed:
                raise ValueError(f"unowned or multiply owned contract: {crate}/{name}")
            claimed.add(name)
        expected = {
            "schema_version": 1,
            "plugin_id": template["plugin_id"],
            "contracts": {"contracts/" + name: sha256(path) for name, path in canonical.items()},
        }
        pins = include_pins(canonical, load_path, root)
        if pins:
            expected["includes"] = pins
        inventory = provider / "contracts.json"
        if inventory.is_symlink():
            raise ValueError(f"symbolic-link contract inventory: {inventory}")
        if sync:
            inventory.write_text(json.dumps(expected, indent=2) + "\n")
        else:
            if inventory.is_symlink() or not inventory.is_file():
                raise ValueError(f"missing or symbolic-link contract inventory: {inventory}")
            if json.loads(inventory.read_text()) != expected:
                raise ValueError(f"stale provider contract inventory: {inventory}")
        rows.append({"provider": template["plugin_id"], "crate": crate,
                     "contracts": len(canonical), "includes": len(pins),
                     "inventory_sha256": sha256(inventory)})
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["check", "sync"])
    args = parser.parse_args()
    try:
        load_path = maintain_load_path(sync=args.action == "sync")
        rows = maintain(sync=args.action == "sync", load_path=load_path)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"provider contracts: {error}\n")
    print(json.dumps({
        "providers": rows,
        "total_contracts": sum(r["contracts"] for r in rows),
        "load_path": [{"crate": c["name"], "namespace": c["namespace"], "version": c["version"],
                       "dependencies": c["dependencies"], "modules": len(c["modules"]),
                       "published": sorted(c["published"])} for c in load_path["crates"]],
    }, indent=2))


if __name__ == "__main__":
    main()
