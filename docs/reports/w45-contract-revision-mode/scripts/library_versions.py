#!/usr/bin/env python3
"""The versions of the libraries vendored in a Fluent Bit source tree that the
provider locks were taken against, read from their build files (read-only).

usage: library_versions.py FLUENT_BIT_SOURCE_ROOT
"""
import json
import pathlib
import re
import sys

DEFAULT_ROOT = ("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/"
                "fluent-bit-clang-source")

LIBRARIES = {
    "chunkio": ("lib/chunkio/CMakeLists.txt", "CIO_VERSION"),
    "cmetrics": ("lib/cmetrics/CMakeLists.txt", "CMT_VERSION"),
    "ctraces": ("lib/ctraces/CMakeLists.txt", "CTR_VERSION"),
    "cfl": ("lib/cfl/CMakeLists.txt", "CFL_VERSION"),
    "monkey": ("lib/monkey/CMakeLists.txt", "MK_VERSION"),
    "msgpack-c": ("lib/msgpack-c/CMakeLists.txt", "VERSION"),
}


def version(root, path, prefix):
    text = (root / path).read_text(errors="replace")
    parts = []
    for part in ("MAJOR", "MINOR", "PATCH"):
        match = re.search(rf"set\(\s*{prefix}_{part}\s+(\d+)", text)
        if match:
            parts.append(match.group(1))
    if parts:
        return ".".join(parts)
    match = re.search(r"project\([^)]*VERSION\s+([0-9.]+)", text)
    return match.group(1) if match else None


def main() -> int:
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ROOT)
    result = {name: {"path": path.rsplit("/", 1)[0], "version": version(root, path, prefix)}
              for name, (path, prefix) in LIBRARIES.items()}
    lib = root / "lib"
    result["librdkafka"] = {"path": next(str(p.relative_to(root)) for p in lib.glob("librdkafka-*")),
                            "version": next(p.name.split("-", 1)[1] for p in lib.glob("librdkafka-*"))}
    result["mpack"] = {"path": next(str(p.relative_to(root)) for p in lib.glob("mpack-amalgamation-*")),
                       "version": next(p.name.rsplit("-", 1)[1] for p in lib.glob("mpack-amalgamation-*"))}
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
