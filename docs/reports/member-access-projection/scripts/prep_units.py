#!/usr/bin/env python3
"""MEMBER-ACCESS-PROJECTION step 1: rebuild each unit's analysed text
(`clang -x c -std=DIALECT -E -P` with the unit's recorded preprocessor
options, as legacy_preparation.rs's preprocess_plain builds it) from the
user's run report, and accept it only when its sha256 equals the report's
`preprocessed_source_sha256`. Writes OUT/unit-<n>.c and OUT/units.jsonl in
the layout access_census.py reads (dialect argument, frontend flags).

usage: prep_units.py REPORT OUT_DIR
"""
import concurrent.futures
import hashlib
import json
import pathlib
import subprocess
import sys

CLANG = "/usr/lib/llvm-21/bin/clang"
FLAGS = {
    "AllWarnings": ["-Wall"], "ExtraWarnings": ["-Wextra"], "DebugInformation": ["-g"],
    "PositionIndependentCode": ["-fPIC"], "AssemblerNoExecStack": ["-Wa,--noexecstack"],
    "StrictAliasingWarnings": ["-Wstrict-aliasing"], "StrictPrototypes": ["-Wstrict-prototypes"],
    "UndefinedIdentifierWarnings": ["-Wundef"], "MissingPrototypes": ["-Wmissing-prototypes"],
    "InitSelf": ["-Winit-self"], "MissingFieldInitializers": ["-Wmissing-field-initializers"],
    "DeclarationAfterStatement": ["-Wdeclaration-after-statement"], "AddressComparisons": ["-Waddress"],
    "OverrideInit": ["-Woverride-init"], "WriteStrings": ["-Wwrite-strings"],
    "VariableLengthArrays": ["-Wvla"], "NoUnusedParameter": ["-Wno-unused-parameter"],
    "NoUnusedFunction": ["-Wno-unused-function"], "NoPragmas": ["-Wno-pragmas"],
    "NoVoidPointerToEnumCast": ["-Wno-void-pointer-to-enum-cast"],
    "NoStrictAliasing": ["-fno-strict-aliasing"], "FunctionSections": ["-ffunction-sections"],
    "DataSections": ["-fdata-sections"], "AddressUndefinedSanitizers": ["-fsanitize=address,undefined"],
}


def flag(value):
    if isinstance(value, str):
        if value not in FLAGS:
            raise SystemExit(f"unmapped frontend flag {value}")
        return FLAGS[value]
    raise SystemExit(f"unmapped frontend flag {value}")


def one(arguments):
    index, unit, out = arguments
    ident = unit["compile_configuration"]["compile_task_identity"]
    dialect = "-std=" + ident["dialect"]
    command = [CLANG, "-x", "c", dialect, "-E", "-P"]
    frontend = []
    for option in ident["preprocessor_options"]:
        (kind, value), = option.items()
        if kind == "Include":
            command += ["-I", value]
        elif kind == "Define":
            command += ["-D", value]
        elif kind == "FrontendFlag":
            command += flag(value)
            frontend += flag(value)
        else:
            raise SystemExit(f"unmapped option {option}")
    source = ident["source_input"]["DeclaredFile"]
    command.append(source["spelled"])
    done = subprocess.run(command, cwd=source["directory"], capture_output=True)
    if done.returncode != 0:
        return index, unit["file"], None, done.stderr.decode(errors="replace")[:500]
    text = done.stdout
    digest = hashlib.sha256(text).hexdigest()
    name = f"unit-{index}.c"
    (pathlib.Path(out) / name).write_bytes(text)
    record = {"unit_seq": index, "file": name, "document": unit["file"], "dialect": dialect,
              "frontend_flags": frontend, "sha256": digest,
              "matches": digest == ident["preprocessed_source_sha256"]}
    return index, unit["file"], record, None


def main():
    report = json.load(open(sys.argv[1]))
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=False)
    jobs = [(index, unit, str(out)) for index, unit in enumerate(report["units"])]
    records, errors = [], []
    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        for index, file, record, error in pool.map(one, jobs):
            if record is None:
                errors.append({"unit": index, "file": file, "error": error})
            else:
                records.append(record)
    records.sort(key=lambda r: r["unit_seq"])
    with open(out / "units.jsonl", "w") as handle:
        for record in records:
            handle.write(json.dumps(record) + "\n")
    mismatched = [r["document"] for r in records if not r["matches"]]
    print(f"units {len(records)}; errors {len(errors)}; sha mismatches {len(mismatched)}")
    for item in errors[:5]:
        print(item)
    print(mismatched[:10])
    return 0 if not errors and not mismatched else 1


if __name__ == "__main__":
    sys.exit(main())
