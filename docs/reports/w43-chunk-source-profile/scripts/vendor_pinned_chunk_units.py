#!/usr/bin/env python3
"""Copy the pinned ChunkIO units the lifecycle route imports, byte for byte.

Card W43-CHUNK-SOURCE-PROFILE. The lifecycle route derives a selected
function's composition from that function's *own pinned* translation unit. So
that the standing gate can re-derive it without the out-of-tree pinned checkout,
this script copies exactly the files Clang reads for the selected units — the
unit, its in-tree include closure and the two headers the pinned build
generates — into ``compiler/tests/fixtures/pinned/fluent-bit/``, and writes a
manifest with every file's SHA-256, its origin and each unit's recorded macro
definitions and include-root order.

Nothing is edited. ``--check`` recomputes the closure from the pinned build and
compares every vendored byte with the pinned checkout instead of writing.

The include closure is measured with ``clang -M`` under the unit's own compile
database entry, so the copied set is what the pinned build actually resolves.
Only the directories the closure uses are kept, in the entry's own order; the
first matching directory for every include is therefore the same one.
"""
import argparse
import hashlib
import json
import pathlib
import shlex
import shutil
import subprocess
import sys

PINNED_REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
UNITS = [
    "lib/chunkio/src/cio_chunk.c",
    "lib/chunkio/src/cio_memfs.c",
    "lib/chunkio/src/cio_error.c",
]
LICENSES = ["LICENSE", "lib/chunkio/LICENSE", "lib/monkey/LICENSE", "lib/monkey/NOTICE"]
# Independently pinned by contract/profiles/fluent-bit/profile.json.
PROFILE_DIGESTS = {
    "lib/chunkio/src/cio_chunk.c": "8f6064c742b3fcc53d75b66dbd07d5c7f03d2f245ec7716f8605a2c3a6efd6c8",
}
ROOT = pathlib.Path(__file__).resolve().parents[4]
DEST = ROOT / "compiler/tests/fixtures/pinned/fluent-bit"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def words(entry):
    return entry.get("arguments") or shlex.split(entry["command"])


def closure(entry):
    argv = [w for w in words(entry)[1:]]
    cleaned, skip = [], False
    for word in argv:
        if skip:
            skip = False
            continue
        if word == "-o":
            skip = True
            continue
        if word == "-c" or word == entry["file"]:
            continue
        cleaned.append(word)
    result = subprocess.run(["clang", *cleaned, "-M", entry["file"]], cwd=entry["directory"],
                            capture_output=True, text=True)
    if result.returncode:
        raise SystemExit(result.stderr)
    deps = result.stdout.replace("\\\n", " ").split()[1:]
    return [pathlib.Path(d).resolve() for d in deps]


def locate(path, source, build):
    for origin, root in (("pinned-source", source), ("pinned-build-generated", build)):
        try:
            return origin, path.relative_to(root)
        except ValueError:
            continue
    return None, None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=pathlib.Path, required=True)
    parser.add_argument("--build-root", type=pathlib.Path, required=True)
    parser.add_argument("--compile-database", type=pathlib.Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    source = args.source_root.resolve(strict=True)
    build = args.build_root.resolve(strict=True)
    database = json.loads(args.compile_database.read_text())

    files = {}
    units = {}
    for unit in UNITS:
        path = (source / unit).resolve(strict=True)
        matches = [e for e in database if pathlib.Path(e["file"]).resolve() == path]
        if len(matches) != 1:
            raise SystemExit(f"{unit}: expected one compile entry, found {len(matches)}")
        entry = matches[0]
        used_dirs = set()
        for dep in closure(entry):
            if str(dep).startswith("/usr/"):
                continue
            origin, relative = locate(dep, source, build)
            if origin is None:
                raise SystemExit(f"{unit}: dependency outside the pinned trees: {dep}")
            key = ("source/" if origin == "pinned-source" else "build/") + relative.as_posix()
            files[key] = {"origin": origin, "path": relative.as_posix(), "absolute": dep}
            used_dirs.add(dep.parent)
        definitions, include_roots = [], []
        # The compatibility adapter puts the unit's own directory first.
        candidates = [path.parent]
        for word in words(entry)[1:]:
            if word.startswith("-D"):
                definitions.append(word[2:])
            elif word.startswith("-I"):
                candidates.append(pathlib.Path(word[2:]).resolve())
        for candidate in candidates:
            origin, relative = locate(candidate, source, build)
            if origin is None:
                continue
            # Keep a root only if some resolved header lives at or below it.
            if any(d == candidate or candidate in d.parents for d in used_dirs):
                label = ("source/" if origin == "pinned-source" else "build/") + relative.as_posix()
                if label not in include_roots:
                    include_roots.append(label)
        units[unit] = {"definitions": definitions, "include_roots": include_roots,
                       "dialect": "gnu17"}
    for license_path in LICENSES:
        absolute = (source / license_path).resolve(strict=True)
        files["source/" + license_path] = {"origin": "pinned-source", "path": license_path,
                                           "absolute": absolute}

    manifest = {
        "schema_version": 1,
        "card": "W43-CHUNK-SOURCE-PROFILE",
        "source_revision": PINNED_REVISION,
        "scope": "verbatim copies of the pinned ChunkIO units the lifecycle route imports, their "
                 "in-tree include closure under the pinned compile database, and the two headers "
                 "the pinned build generates; no byte is edited",
        "files": {},
        "units": units,
    }
    failures = []
    for key in sorted(files):
        record = files[key]
        data = record["absolute"].read_bytes()
        digest = sha256(data)
        expected = PROFILE_DIGESTS.get(record["path"])
        if expected and expected != digest:
            raise SystemExit(f"{record['path']} is not the profile-pinned digest")
        manifest["files"][key] = {"origin": record["origin"], "sha256": digest}
        target = DEST / key
        if args.check:
            if not target.exists() or target.read_bytes() != data:
                failures.append(key)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    rendered = json.dumps(manifest, indent=2) + "\n"
    manifest_path = DEST / "manifest.json"
    if args.check:
        if not manifest_path.exists() or manifest_path.read_text() != rendered:
            failures.append("manifest.json")
        vendored = sorted(p.relative_to(DEST).as_posix() for p in DEST.rglob("*") if p.is_file())
        extra = sorted(set(vendored) - set(manifest["files"]) - {"manifest.json"})
        failures.extend(f"unexpected file {name}" for name in extra)
        if failures:
            print("vendored pinned units differ from the pinned checkout:", *failures, sep="\n  ")
            return 1
        print(f"{len(manifest['files'])} vendored files and the manifest match the pinned checkout")
        return 0
    manifest_path.write_text(rendered)
    print(f"wrote {len(manifest['files'])} files and {manifest_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
