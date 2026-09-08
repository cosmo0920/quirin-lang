import json, subprocess, sys
from pathlib import Path

CLANG = "/usr/bin/clang"
FLAGS = ["-Wall", "-Wextra","-Wno-unused-parameter","-Wstrict-aliasing","-Wstrict-prototypes","-Wundef",
         "-fno-strict-aliasing","-Wmissing-prototypes","-Winit-self","-Wmissing-field-initializers",
         "-Wdeclaration-after-statement","-Waddress","-Woverride-init","-Wwrite-strings",
         "-Wno-unused-function","-Wno-pragmas","-Wvla","-Wno-void-pointer-to-enum-cast",
         "-O0","-O1","-O2","-O3","-Os","-Oz","-Og","-g","-fPIC"]
src = Path(sys.argv[1])
extra_includes = sys.argv[2:]
base = [CLANG, "-std=gnu17", "-x", "c"] + extra_includes


def run(extra, mode):
    args = base + extra + mode + [str(src)]
    p = subprocess.run(args, capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


rows = {}
_, base_pp, _ = run([], ["-E", "-P"])
_, base_macros, _ = run([], ["-dM", "-E"])
for f in FLAGS:
    rc, pp, err = run([f], ["-E", "-P"])
    _, macros, _ = run([f], ["-dM", "-E"])
    bm = set(base_macros.splitlines())
    nm = set(macros.splitlines())
    rows[f] = {
        "preprocess_exit": rc,
        "preprocessed_identical": pp == base_pp,
        "macros_added": sorted(nm - bm),
        "macros_removed": sorted(bm - nm),
        "stderr_lines": len(err.strip().splitlines()) if err.strip() else 0,
    }
print(json.dumps(rows, indent=2))
