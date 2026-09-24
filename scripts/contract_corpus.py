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


def sources(repository):
    repository = Path(repository).resolve()
    index, manifest = index_manifest(repository)
    if not isinstance(manifest["roots"], list) or not manifest["roots"]:
        raise ValueError("empty or invalid corpus roots")
    result = {}
    for name in manifest["roots"]:
        directory = index.parent / name
        if not directory.resolve().is_relative_to(repository) or directory.is_symlink() or not directory.is_dir():
            raise ValueError(f"invalid corpus root: {directory}")
        for path in sorted(directory.rglob("*")):
            if path.is_symlink():
                raise ValueError(f"symbolic link in corpus: {path}")
            if path.is_file() and path.suffix == ".qcore":
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
    """`extern fn` names a `.qcore` module declares, `//` comments removed."""
    text = "\n".join(line.split("//", 1)[0] for line in Path(path).read_text().split("\n"))
    return _EXTERN_FN.findall(text)


def libc_names(repository):
    """The libc/POSIX name list owned by the libc crate; fail closed if absent or empty."""
    path = Path(repository).resolve() / LIBC_NAMES
    names = {line.strip() for line in path.read_text().split("\n") if line.strip() and not line.startswith("#")}
    if not names:
        raise ValueError(f"empty libc name list: {path}")
    return names


def libc_placement_violations(repository):
    """Every (module, name) declaring a libc/POSIX function outside the libc crate.

    Scanned: every canonical corpus module and every `.qcore` under `providers/`.
    A name is libc/POSIX when it is in the libc crate's generated name list or is
    declared by a module of the libc crate itself. Card W43-LIBC-CONTRACTS.
    """
    repository = Path(repository).resolve()
    crate = (repository / LIBC_CRATE).resolve()
    names = libc_names(repository)
    modules = {path.resolve() for path in sources(repository).values()}
    providers = repository / "providers"
    if providers.is_dir():
        modules |= {path.resolve() for path in providers.rglob("*.qcore") if path.is_file()}
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


_INCLUDE = re.compile(r'^[ \t]*#[ \t]*include[ \t]+"([a-z0-9][a-z0-9_-]*):([a-z0-9_/-]+\.qcore)"[ \t]*;', re.M)
_INHERIT = re.compile(r"\bcontract\s+inherit\s+(?:([A-Za-z_]\w*)::)?([A-Za-z_]\w*)\s*;")


def _uncommented(text):
    return "\n".join(line.split("//", 1)[0] for line in text.split("\n"))


def libc_inheritance(repository):
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
    modules = {path.resolve() for path in sources(repository).values()}
    providers = repository / "providers"
    if providers.is_dir():
        modules |= {path.resolve() for path in providers.rglob("*.qcore") if path.is_file()}
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
