#!/usr/bin/env python3
"""Maintain the inventories for canonical provider-owned contracts, and the
published contract load path (card CONTRACT-INHERIT-ADOPTION).

Edit contracts in their provider crate, then run `sync`. `check` never writes.
Both also maintain `contract-load-path.json`: one record per
`quirin-provider-*` and `qcore-provider-*` contract crate from `cargo
metadata` and the crate's `NAMESPACE` and `PUBLISHED` constants, with every
published module's SHA-256 pinned; and a provider inventory's `includes`
pins, resolved on that load path.
Each load-path crate record also carries its `resource_view` (card
W44-PROVIDER-RESOURCE-VIEW): the owned modules the resource-contract loader
accepts, which `quirin-audit --provider NAME` selects. `sync` generates it by
running the loader (`qcore-audit`'s example `provider_resource_views`,
built by `package.py`'s `root_workspace_executable`); `check` recomputes it
and refuses a view that declares a module the loader refuses or omits one it
accepts.
Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: each record also carries its
`bounds_view`, the owned modules the bounds contract v1 loader accepts (a
separate property vocabulary, never in a resource view), computed by the same
probe; the audit routes load every bounds view when the bounds property is
selected (`QCORE_BOUNDS_PROPERTY`), and `quirin-audit pin --bounds` pins them.
Card OUTER-CONTRACT: each record also carries its `outer_view`, the owned
modules the outer contract v1 loader accepts (disjoint from both other views),
computed by the same probe; the runtime projection loads every outer view
(`QCORE_OUTER_PROJECTION`), and `quirin-audit pin --outer` pins them.
Card CONTRACT-LOCKS: both also maintain each load-path crate's
`contract-inventory.lock` (per-file SHA-256, declarations with their
elaborated clause counts, inheritance edges as written, the premise manifest
and the three views), computed with the loaders' own parser by `qcore-contract`'s
example `contract_inventory`; the corpus tests read these locks
instead of literal tables, so a contract change is a reviewed lock diff.
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
    "qcore-provider-chunkio": ("chunkio", "fluent_bit/busy_state.qv"),
    "qcore-provider-cmetrics": ("cmetrics",),
    "qcore-provider-ctraces": ("ctraces",),
    "qcore-provider-libevent": ("monkey",),
    "qcore-provider-librdkafka": ("librdkafka",),
}


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def owned(name, selectors):
    return any(name == selector or name.startswith(selector + "/")
               or (selector.endswith(".qcore") and name == selector[:-6] + ".qv")
               for selector in selectors)


def files(directory):
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"missing or symbolic-link contract directory: {directory}")
    result = {}
    stems = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"symbolic link in contract inventory: {path}")
        if path.is_file():
            name = path.relative_to(directory).as_posix()
            if path.suffix not in (".qcore", ".qv") or not re.fullmatch(r"[a-z0-9_/-]+\.(?:qcore|qv)", name):
                raise ValueError(f"unexpected contract asset: {path}")
            stem = name.removesuffix(path.suffix)
            if stem in stems:
                raise ValueError(f"ambiguous contract asset stem: {directory}/{stems[stem]} and {path}")
            stems[stem] = name
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
# contract crate, and (card W45-USER-CONTRACT-PLUGINS) the SDK for user
# contract crates and checker plugins (`providers/quirin-provider-plugin`)
# carries no production contract. Listed by name, and checked to own no
# contracts.
NON_CONTRACT_PACKAGES = ("qcore-provider-conformance", "quirin-provider-plugin")
NAMESPACE_CONST = re.compile(r'^pub const NAMESPACE: &str = "([a-z_][a-z0-9_]*)";$', re.M)
PUBLISHED_CONST = re.compile(r'^pub const PUBLISHED: &\[&str\] = &\[([^\]]*)\];$', re.M)
STRING_LITERAL = re.compile(r'\s*"([a-z0-9_/-]+\.(?:qcore|qv))"\s*')


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
    source_paths = {}
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
        source_paths.update({f"{name}/{module}": path for module, path in modules.items()})
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
    package_module().validate_source_modes(source_paths)
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


# Card W44-PROVIDER-RESOURCE-VIEW: each crate's resource view is what the
# resource-contract loader itself accepts, so it is computed by the loader and
# never listed by hand. The probe is `quirin-audit`'s own selection and
# loader (`qcore_audit::providers::probe_resource_views`), built from the
# repository workspace that contains this providers workspace.
RESOURCE_VIEW = "resource_view"
# Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds loader's view, from the
# same probe run.
BOUNDS_VIEW = "bounds_view"
# Card OUTER-CONTRACT: the outer contract loader's view, from the same probe.
OUTER_VIEW = "outer_view"


def loader_resource_views(root, load_path_file):
    """The loaders' views of every crate on `load_path_file`: `{crate:
    {"resource_view": [module, ...], "bounds_view": [module, ...],
    "outer_view": [module, ...]}}`, sorted owned modules (card
    CHUNKIO-META-LEN-BOUNDS-CONTRACTS added the bounds view, card OUTER-CONTRACT
    the outer view). Runs the probe example in the repository workspace; any failure of
    the probe is an error, never an empty view."""
    workspace = Path(root).resolve().parent / "Cargo.toml"
    command = [package_module().root_workspace_executable(
                   "example", "provider_resource_views", "resource view probe", workspace),
               str(Path(load_path_file).resolve())]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode != 0:
        raise ValueError(f"resource view probe failed (exit {completed.returncode}): "
                         f"{completed.stderr.strip()}")
    probe = json.loads(completed.stdout)
    return {name: {RESOURCE_VIEW: record[RESOURCE_VIEW], BOUNDS_VIEW: record[BOUNDS_VIEW],
                   OUTER_VIEW: record[OUTER_VIEW]}
            for name, record in probe["crates"].items()}


def split_views(value):
    """A probe value: `{"resource_view": [...], "bounds_view": [...],
    "outer_view": [...]}`, or a bare list, which is a resource view beside
    empty bounds and outer views (the form of a stand-in loader that knows no
    other vocabulary); a value without `outer_view` has an empty one."""
    if isinstance(value, list):
        return value, [], []
    return value[RESOURCE_VIEW], value[BOUNDS_VIEW], value.get(OUTER_VIEW, [])


# Card W45-CONTRACT-REVISION-MODE: every crate with a resource view keeps a
# generated `contracts.lock` beside its Cargo.toml. `quirin-audit pin
# --verify-contracts` (no source needed) checks that each lock lists exactly
# the crate's current declarations, revision modes and contract-text digests;
# the source identities themselves are compared at audit time.
LOCK_FILE = "contracts.lock"


def verify_locks(root, load_path_file):
    """Run `quirin-audit pin --verify-contracts` over the load path and
    return its report lines. Any failure of the probe, or a stale or missing
    lock, is an error naming the command that regenerates the locks."""
    workspace = Path(root).resolve().parent / "Cargo.toml"
    command = [package_module().root_workspace_executable(
                   "bin", "quirin-audit", "contract lock probe", workspace),
               "pin", "--verify-contracts", "--load-path", str(Path(load_path_file).resolve())]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode != 0:
        raise ValueError(f"contract locks are stale or unreadable (exit {completed.returncode}): "
                         f"{completed.stdout.strip()} {completed.stderr.strip()}; review the "
                         "contracts, then run `quirin-audit pin --source-root <checkout>`")
    return completed.stdout.strip().splitlines()


# Card CONTRACT-LOCKS (user decision 1 of 2026-10-24): every load-path crate
# keeps a generated `contract-inventory.lock` beside its Cargo.toml. It is the
# contract pin the tests read instead of literal tables: per module its
# SHA-256, its `extern fn` declarations in source order with their elaborated
# clause counts (and the per-file totals), its `contract inherit` edges as
# written, its premise manifest (the premises each function reads, and the
# `qcore-premise-binding-manifest-v1` pins) and the crate's three loader views
# as the load path records them. `sync` writes it; `check` refuses a lock that
# differs by one byte. It is not the `contracts.lock` above, which `quirin-audit
# pin` writes and whose format this card does not touch.
INVENTORY_LOCK = "contract-inventory.lock"
INVENTORY_SCHEMA = 1


def probe_inventories(root, load_path_file):
    """The parse-derived facts of every module on `load_path_file`: `{crate:
    {module: {"declarations", "inherits", "premises", "premise_bindings"}}}`,
    from the probe example in the repository workspace, which parses,
    elaborates and reads premises with the loaders' own code. Any failure of
    the probe is an error, never an empty inventory."""
    workspace = Path(root).resolve().parent / "Cargo.toml"
    command = [package_module().root_workspace_executable(
                   "example", "contract_inventory", "contract inventory probe", workspace),
               str(Path(load_path_file).resolve())]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode != 0:
        raise ValueError(f"contract inventory probe failed (exit {completed.returncode}): "
                         f"{completed.stderr.strip()}")
    probe = json.loads(completed.stdout)
    if probe.get("schema_version") != 1:
        raise ValueError("contract inventory probe: unsupported schema")
    return {name: record["modules"] for name, record in probe["crates"].items()}


def inventory_lock(root, record, facts):
    """One crate's lock document from its load-path `record` (views included)
    and the probe's `facts` for it. The probe must describe exactly the
    crate's contract files; the digests are computed here, independently."""
    modules = files(Path(root) / record["contracts"])
    if set(facts) != set(modules):
        raise ValueError(f"{record['name']}: inventory probe describes {sorted(facts)}, "
                         f"not the contract files {sorted(modules)}")
    entries = {}
    for name, path in modules.items():
        fact = facts[name]
        declarations = [{"name": d["name"], "clauses": d["clauses"]} for d in fact["declarations"]]
        entries[name] = {
            "sha256": sha256(path),
            "declaration_count": len(declarations),
            "clause_count": sum(d["clauses"] for d in declarations),
            "declarations": declarations,
            "inherits": [{"child": i["child"], "parent": i["parent"]} for i in fact["inherits"]],
            "premises": {function: [{"name": p["name"], "origin": p["origin"]} for p in premises]
                         for function, premises in fact["premises"].items()},
            "premise_bindings": [{"declaration": b["declaration"], "premise": b["premise"],
                                  "sha256": b["sha256"]} for b in fact["premise_bindings"]],
        }
    return {
        "schema_version": INVENTORY_SCHEMA,
        "crate": record["name"],
        "contracts": record["contracts"],
        "regenerate": "python3 providers/contracts.py sync",
        "files": entries,
        RESOURCE_VIEW: record[RESOURCE_VIEW],
        BOUNDS_VIEW: record[BOUNDS_VIEW],
        OUTER_VIEW: record[OUTER_VIEW],
    }


def canonical(document):
    """The one byte form of a lock: sorted keys, two-space indent, ASCII,
    final newline. Lists keep their order, which is sorted or source order."""
    return json.dumps(document, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def maintain_inventory_locks(root=ROOT, load_path=None, sync=False, probe=None):
    """Write (`sync`) or compare (`check`) every load-path crate's
    `contract-inventory.lock`. `load_path` is the current manifest with its
    views (`maintain_load_path`'s result); `probe(root, load_path_file)`
    computes the parse-derived facts (default `probe_inventories`). A stale,
    missing or symbolic-link lock, or a lock in a directory that is no
    load-path crate, is an error. Returns one summary row per crate."""
    root = Path(root)
    probe = probe_inventories if probe is None else probe
    load_path = maintain_load_path(root) if load_path is None else load_path
    facts = probe(root, root / LOAD_PATH)
    crates = {record["name"] for record in load_path["crates"]}
    if set(facts) != crates:
        raise ValueError(f"inventory probe names crates {sorted(facts)}, "
                         f"not the load path's {sorted(crates)}")
    for stray in sorted(root.glob(f"*/{INVENTORY_LOCK}")):
        if stray.parent.name not in crates:
            raise ValueError(f"contract inventory lock outside a load-path crate: {stray}")
    rows = []
    for record in load_path["crates"]:
        document = inventory_lock(root, record, facts[record["name"]])
        text = canonical(document)
        lock = root / record["name"] / INVENTORY_LOCK
        if lock.is_symlink():
            raise ValueError(f"symbolic-link contract inventory lock: {lock}")
        if sync:
            lock.write_bytes(text.encode())
        elif not lock.is_file():
            raise ValueError(f"missing contract inventory lock: {lock}; review the contracts, "
                             "then run `python3 providers/contracts.py sync`")
        elif lock.read_bytes() != text.encode():
            raise ValueError(f"stale contract inventory lock: {lock}; "
                             f"{'; '.join(lock_difference(lock, document)) or 'byte form differs'}; "
                             "review the contracts, then run `python3 providers/contracts.py sync`")
        entries = document["files"].values()
        rows.append({"crate": record["name"], "files": len(document["files"]),
                     "declarations": sum(f["declaration_count"] for f in entries),
                     "clauses": sum(f["clause_count"] for f in entries),
                     "inventory_lock_sha256": hashlib.sha256(text.encode()).hexdigest()})
    return rows


def lock_difference(lock, document):
    """Which modules and top-level fields of a committed lock differ from the
    tree's document, for the refusal message."""
    try:
        declared = json.loads(lock.read_text())
    except ValueError:
        return ["unparsable lock"]
    reasons = []
    old, new = declared.get("files", {}), document["files"]
    for name in sorted(set(old) | set(new)):
        if name not in new:
            reasons.append(f"{name} is locked but not in the tree")
        elif name not in old:
            reasons.append(f"{name} is in the tree but not locked")
        elif old[name] != new[name]:
            fields = sorted(k for k in set(old[name]) | set(new[name])
                            if old[name].get(k) != new[name].get(k))
            reasons.append(f"{name} differs in {fields}")
    reasons.extend(f"{key} differs" for key in sorted(set(declared) | set(document))
                   if key != "files" and declared.get(key) != document.get(key))
    return reasons


def with_resource_views(load_path, views):
    """`load_path` with each crate's `resource_view`, `bounds_view` and
    `outer_view` from `views`, validated: one of each per crate, sorted,
    unique, owned and disjoint (a module is in at most one vocabulary's
    view)."""
    crates = {record["name"]: record for record in load_path["crates"]}
    if set(views) != set(crates):
        raise ValueError(f"resource view probe names crates {sorted(views)}, "
                         f"not the load path's {sorted(crates)}")
    result = {"schema_version": load_path["schema_version"], "crates": []}
    for record in load_path["crates"]:
        view, bounds, outer = (list(v) for v in split_views(views[record["name"]]))
        if view != sorted(set(view)) or not set(view) <= set(record["modules"]):
            raise ValueError(f"{record['name']}: resource view {view} is not sorted, unique and owned")
        if bounds != sorted(set(bounds)) or not set(bounds) <= set(record["modules"]):
            raise ValueError(f"{record['name']}: bounds view {bounds} is not sorted, unique and owned")
        if set(view) & set(bounds):
            raise ValueError(f"{record['name']}: modules in both the resource and the bounds view: "
                             f"{sorted(set(view) & set(bounds))}")
        if outer != sorted(set(outer)) or not set(outer) <= set(record["modules"]):
            raise ValueError(f"{record['name']}: outer view {outer} is not sorted, unique and owned")
        if set(outer) & (set(view) | set(bounds)):
            raise ValueError(f"{record['name']}: modules in the outer view and another view: "
                             f"{sorted(set(outer) & (set(view) | set(bounds)))}")
        result["crates"].append({**record, RESOURCE_VIEW: view, BOUNDS_VIEW: bounds,
                                 OUTER_VIEW: outer})
    return result


def view_difference(declared, computed):
    """Why a declared view differs from the loader's: per crate, the declared
    modules the loader refuses and the accepted modules the view omits."""
    reasons = []
    for key, kind in ((RESOURCE_VIEW, "resource"), (BOUNDS_VIEW, "bounds"),
                      (OUTER_VIEW, "outer")):
        computed_views = {record["name"]: record.get(key) for record in computed["crates"]}
        for record in declared.get("crates", []):
            expected = computed_views.get(record["name"])
            if expected is None:
                continue
            view = record.get(key)
            if view is None:
                reasons.append(f"{record['name']} declares no {kind} view")
                continue
            refused = sorted(set(view) - set(expected))
            omitted = sorted(set(expected) - set(view))
            if refused:
                reasons.append(f"{record['name']} declares modules the {kind} loader refuses: {refused}")
            if omitted:
                reasons.append(f"{record['name']} omits modules the {kind} loader accepts: {omitted}")
            if not refused and not omitted and view != expected:
                reasons.append(f"{record['name']}'s {kind} view is not sorted and unique")
    return reasons


def maintain_load_path(root=ROOT, sync=False, metadata=None, views=None):
    """Write (`sync`) or compare (`check`) the load-path manifest, resource
    views included. Returns it. `views(root, load_path_file)` computes the
    views; the default runs the resource loader (`loader_resource_views`)."""
    root = Path(root)
    views = loader_resource_views if views is None else views
    load_path = load_path_manifest(root, metadata)
    load_path_file = root / LOAD_PATH
    if load_path_file.is_symlink():
        raise ValueError(f"symbolic-link load-path manifest: {load_path_file}")
    if sync:
        # The probe reads the regenerated load path; the views it computes do
        # not depend on views already declared.
        load_path_file.write_text(json.dumps(load_path, indent=2) + "\n")
        load_path = with_resource_views(load_path, views(root, load_path_file))
        load_path_file.write_text(json.dumps(load_path, indent=2) + "\n")
        return load_path
    if not load_path_file.is_file():
        raise ValueError(f"stale contract load-path manifest: {load_path_file}")
    declared = json.loads(load_path_file.read_text())
    stripped = {**declared, "crates": [
        {key: value for key, value in record.items()
         if key not in (RESOURCE_VIEW, BOUNDS_VIEW, OUTER_VIEW)}
        for record in declared.get("crates", [])]}
    if stripped != load_path:
        raise ValueError(f"stale contract load-path manifest: {load_path_file}")
    load_path = with_resource_views(load_path, views(root, load_path_file))
    if declared != load_path:
        reasons = "; ".join(view_difference(declared, load_path)) or "resource views differ"
        raise ValueError(f"stale resource view in {load_path_file}: {reasons}; "
                         "run `python3 providers/contracts.py sync`")
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
    source_paths = {}
    canonical_by_crate = {}
    for crate in OWNERS:
        provider = root / crate
        if provider.is_symlink():
            raise ValueError(f"symbolic-link provider directory: {provider}")
        canonical_by_crate[crate] = files(provider / "contracts")
        source_paths.update({f"{crate}/{name}": path
                             for name, path in canonical_by_crate[crate].items()})
    package_module().validate_source_modes(source_paths)
    for crate, selectors in OWNERS.items():
        provider = root / crate
        canonical = canonical_by_crate[crate]
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
        inventory = maintain_inventory_locks(load_path=load_path, sync=args.action == "sync")
        rows = maintain(sync=args.action == "sync", load_path=load_path)
        locks = verify_locks(ROOT, ROOT / LOAD_PATH)
        legacy_sources = {}
        for record in load_path["crates"]:
            directory = ROOT / record["contracts"]
            legacy_sources.update({f"{record['name']}/{name}": path
                                   for name, path in files(directory).items()})
        for crate in OWNERS:
            legacy_sources.update({f"{crate}/{name}": path
                                   for name, path in files(ROOT / crate / "contracts").items()})
        package_module().validate_source_modes(legacy_sources, warn_legacy=True)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"provider contracts: {error}\n")
    print(json.dumps({
        "locks": locks,
        "inventory_locks": inventory,
        "providers": rows,
        "total_contracts": sum(r["contracts"] for r in rows),
        "load_path": [{"crate": c["name"], "namespace": c["namespace"], "version": c["version"],
                       "dependencies": c["dependencies"], "modules": len(c["modules"]),
                       "published": sorted(c["published"]),
                       "resource_view": len(c[RESOURCE_VIEW]),
                       "bounds_view": len(c[BOUNDS_VIEW]),
                       "outer_view": len(c[OUTER_VIEW])} for c in load_path["crates"]],
    }, indent=2))


if __name__ == "__main__":
    main()
