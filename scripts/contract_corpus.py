"""Discover canonical contract assets for repository validation (not runtime loading)."""
import json
from pathlib import Path


def sources(repository):
    repository = Path(repository).resolve()
    index = repository / "contract/corpus.json"
    manifest = json.loads(index.read_text())
    if set(manifest) != {"schema_version", "roots"} or manifest["schema_version"] != 1:
        raise ValueError("unsupported corpus index")
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
