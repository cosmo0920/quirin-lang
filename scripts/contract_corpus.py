"""Discover canonical contract assets for repository validation (not runtime loading)."""
import json
import re
from pathlib import Path


def index_manifest(repository):
    """The corpus index. Schema 1 is `{schema_version, roots}`; schema 2 (card
    CONTRACT-INHERIT-ADOPTION) adds `load_path`, the published contract load
    path a module using `#include` is elaborated against. Roots resolve
    identically under both."""
    repository = Path(repository).resolve()
    index = repository / "contract/corpus.json"
    manifest = json.loads(index.read_text())
    version = manifest.get("schema_version")
    expected = {1: {"schema_version", "roots"}, 2: {"schema_version", "roots", "load_path"}}.get(version)
    if expected is None or set(manifest) != expected:
        raise ValueError("unsupported corpus index")
    if version == 2:
        load_path = index.parent / manifest["load_path"]
        if (not isinstance(manifest["load_path"], str) or load_path.is_symlink()
                or not load_path.resolve().is_relative_to(repository) or not load_path.is_file()):
            raise ValueError(f"invalid corpus load path: {load_path}")
    return index, manifest


def load_path(repository):
    """The load-path manifest the corpus index names, or None under schema 1."""
    index, manifest = index_manifest(repository)
    if "load_path" not in manifest:
        return None
    return json.loads((index.parent / manifest["load_path"]).read_text())


# Card CONTRACT-COUNT-PINS-FROM-LOCKS: the aggregate contract counts, derived
# from the providers' inventory locks. `tests/support/contract_locks.rs` is the
# same reader for the Rust tests: the same fields, the same refusals.
LOCK_SCHEMA = 2
RESOURCE_TABLES = ("functions", "pending_review", "relational", "retaining")
LOCK_TOTAL_FIELDS = ("crates", "corpus_modules", "qcore_provider_modules",
                     "quirin_provider_modules", "resource_view_modules", "v1_declarations",
                     "rejected_declarations", *RESOURCE_TABLES)


def lock_totals(repository):
    """The aggregate counts of `repository`'s `contract-inventory.lock` files,
    one per crate of `providers/contract-load-path.json`: locked modules (all,
    and per `qcore-provider-*`/`quirin-provider-*` prefix), Σ resource views,
    `extern fn` declarations with a resource table (v1) and per table, and the
    rejected rest (declarations without a table plus every non-`extern fn`
    declaration). Raises ValueError on a lock the load path does not state:
    missing, another schema or crate, a view differing from the load path's or
    naming no locked file, or a module whose view membership disagrees with
    its declarations' tables."""
    load_path_file = Path(repository) / "providers/contract-load-path.json"
    totals = dict.fromkeys(LOCK_TOTAL_FIELDS, 0)
    for record in json.loads(load_path_file.read_text())["crates"]:
        name = record["name"]
        lock_file = (load_path_file.parent / record["contracts"]).parent / "contract-inventory.lock"
        lock = json.loads(lock_file.read_text())
        if lock.get("schema_version") != LOCK_SCHEMA or lock.get("crate") != name:
            raise ValueError(f"{lock_file}: not a schema {LOCK_SCHEMA} lock of {name}")
        if lock["resource_view"] != record["resource_view"]:
            raise ValueError(f"{name}: the lock's resource view is not the load path's")
        files, view = lock["files"], set(lock["resource_view"])
        if not view <= set(files):
            raise ValueError(f"{name}: view modules {sorted(view - set(files))} are not locked")
        totals["crates"] += 1
        totals["corpus_modules"] += len(files)
        for prefix in ("qcore-provider-", "quirin-provider-"):
            if name.startswith(prefix):
                totals[prefix.replace("-", "_").rstrip("_") + "_modules"] += len(files)
        totals["resource_view_modules"] += len(view)
        for logical, entry in files.items():
            tables = [d["resource_table"] for d in entry["declarations"]]
            if not set(tables) <= {None, *RESOURCE_TABLES}:
                raise ValueError(f"{name}:{logical}: resource tables {tables}")
            # A function-map declaration carries its loaded mode; no other does.
            if any((d.get("resource_mode") is None) == (d["resource_table"] == "functions")
                   for d in entry["declarations"]):
                raise ValueError(f"{name}:{logical}: a resource mode outside the function "
                                 "map, or none inside it")
            others = len(entry["other_declarations"])
            accepted = bool(tables) and None not in tables and others == 0
            if (logical in view) != accepted:
                raise ValueError(f"{name}:{logical}: resource-view membership and resource "
                                 "tables disagree")
            totals["rejected_declarations"] += others + tables.count(None)
            for table in RESOURCE_TABLES:
                totals[table] += tables.count(table)
                totals["v1_declarations"] += tables.count(table)
    return totals


def _walk(directory, tracked):
    """Every path under `directory`: the filesystem walk by default, or, when
    `tracked` is given (absolute paths of the repository's tracked files),
    only those tracked paths under it, spelled as `directory / relative`.
    Card W45-TEST-HYGIENE: a test passes `tracked` so that untracked or
    ignored files never change its result."""
    if tracked is None:
        return sorted(directory.rglob("*"))
    resolved = directory.resolve()
    return sorted(directory / path.relative_to(resolved)
                  for path in tracked if path.is_relative_to(resolved))


def _tracked_paths(repository, tracked):
    """`tracked` (repository-relative paths) as absolute paths, or None."""
    if tracked is None:
        return None
    repository = Path(repository).resolve()
    return {repository / path for path in tracked}


def sources(repository, tracked=None):
    """The canonical corpus modules by logical path. `tracked`, when given,
    is the repository's tracked-file list (repository-relative paths) and
    replaces the filesystem walk."""
    repository = Path(repository).resolve()
    tracked = _tracked_paths(repository, tracked)
    index, manifest = index_manifest(repository)
    if not isinstance(manifest["roots"], list) or not manifest["roots"]:
        raise ValueError("empty or invalid corpus roots")
    result = {}
    for name in manifest["roots"]:
        directory = index.parent / name
        if not directory.resolve().is_relative_to(repository) or directory.is_symlink() or not directory.is_dir():
            raise ValueError(f"invalid corpus root: {directory}")
        for path in _walk(directory, tracked):
            if path.is_symlink():
                raise ValueError(f"symbolic link in corpus: {path}")
            if path.is_file() and path.suffix in {".qcore", ".qv"}:
                logical = path.relative_to(directory).as_posix()
                if logical in result:
                    raise ValueError(f"duplicate logical contract: {logical}")
                result[logical] = path
    if not result:
        raise ValueError("empty corpus")
    return dict(sorted(result.items()))


LIBC_CRATE = "providers/quirin-provider-libc"
LIBC_NAMES = LIBC_CRATE + "/libc-names.txt"
_EXTERN_FN = re.compile(r"^\s*(?:unsafe\s+)?extern\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)


def declared_functions(path):
    """`extern fn` names a contract module declares, `//` comments removed."""
    text = "\n".join(line.split("//", 1)[0] for line in Path(path).read_text().split("\n"))
    return _EXTERN_FN.findall(text)


def libc_names(repository):
    """The libc/POSIX name list owned by the libc crate; fail closed if absent or empty."""
    path = Path(repository).resolve() / LIBC_NAMES
    names = {line.strip() for line in path.read_text().split("\n") if line.strip() and not line.startswith("#")}
    if not names:
        raise ValueError(f"empty libc name list: {path}")
    return names


def libc_placement_violations(repository, tracked=None):
    """Every (module, name) declaring a libc/POSIX function outside the libc crate.

    Scanned: every canonical corpus module and every contract source under `providers/`.
    A name is libc/POSIX when it is in the libc crate's generated name list or is
    declared by a module of the libc crate itself. Card W43-LIBC-CONTRACTS.
    """
    repository = Path(repository).resolve()
    crate = (repository / LIBC_CRATE).resolve()
    names = libc_names(repository)
    modules = {path.resolve() for path in sources(repository, tracked).values()}
    providers = repository / "providers"
    if providers.is_dir():
        modules |= {path.resolve() for path in _walk(providers, _tracked_paths(repository, tracked))
                    if path.suffix in {".qcore", ".qv"} and path.is_file()}
    owned = [path for path in modules if path.is_relative_to(crate)]
    if not owned:
        raise ValueError(f"no contract module under {crate}")
    for path in owned:
        names |= set(declared_functions(path))
    violations = []
    for path in sorted(modules):
        if path.is_relative_to(crate):
            continue
        for name in declared_functions(path):
            if name in names:
                violations.append((path.relative_to(repository).as_posix(), name))
    return violations


_INCLUDE = re.compile(r'^[ \t]*#[ \t]*include[ \t]+"([a-z0-9][a-z0-9_-]*):([a-z0-9_/-]+\.(?:qcore|qv))"[ \t]*;', re.M)
_INHERIT = re.compile(r"\bcontract\s+inherit\s+(?:([A-Za-z_]\w*)::)?([A-Za-z_]\w*)\s*;")


def _uncommented(text):
    return "\n".join(line.split("//", 1)[0] for line in text.split("\n"))


def libc_inheritance(repository, tracked=None):
    """Card CONTRACT-INHERIT-ADOPTION: how modules outside the libc crate take
    libc contracts by reference instead of restating them.

    Returns `(references, violations)`. A reference is `(module, child,
    "libc::NAME")` for every `contract inherit libc::NAME;`. It is admitted
    only when the module `#include`s a libc module the libc crate publishes
    (per the load-path manifest the corpus index names) and that module
    declares NAME; anything else is a violation row `(module, problem)`, as is
    an include of an unpublished libc module. The placement rule is thereby
    kept: the libc contract text stays in the libc crate, and the child names a
    Fluent Bit (or other) function, never a libc one (`libc_placement_violations`).
    """
    repository = Path(repository).resolve()
    crate = (repository / LIBC_CRATE).resolve()
    manifest = load_path(repository)
    published = set()
    if manifest is not None:
        for record in manifest["crates"]:
            if record["name"] == "quirin-provider-libc":
                published = set(record["published"])
    modules = {path.resolve() for path in sources(repository, tracked).values()}
    providers = repository / "providers"
    if providers.is_dir():
        modules |= {path.resolve() for path in _walk(providers, _tracked_paths(repository, tracked))
                    if path.suffix in {".qcore", ".qv"} and path.is_file()}
    references, violations = [], []
    for path in sorted(modules):
        if path.is_relative_to(crate):
            continue
        text = _uncommented(path.read_text())
        name = path.relative_to(repository).as_posix()
        included = set()
        for crate_name, module in _INCLUDE.findall(text):
            if crate_name != "quirin-provider-libc":
                continue
            if module not in published:
                violations.append((name, f"includes unpublished libc module {module}"))
                continue
            included |= set(declared_functions(crate / "contracts" / module))
        for match in _INHERIT.finditer(text):
            namespace, parent = match.groups()
            if namespace != "libc":
                continue
            children = re.findall(r"extern\s+fn\s+([A-Za-z_]\w*)", text[:match.start()])
            child = children[-1] if children else "?"
            references.append((name, child, f"libc::{parent}"))
            if parent not in included:
                violations.append((name, f"{child} inherits libc::{parent}, which no included libc module declares"))
    return references, violations
