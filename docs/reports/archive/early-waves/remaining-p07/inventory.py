#!/usr/bin/env python3
"""P07 librdkafka imported-operation inventory and admission census.

This script answers one question with measured evidence, before any witness
form is chosen: *which librdkafka operations does the QCore adapter actually
import today, and what do those operations look like in the audited build
configuration?*

It reads only committed inputs:

  * ``contract/profiles/fluent-bit/profile.json`` -- the W2 source profile,
    which is the sole declaration of what the librdkafka adapter imports.
  * ``contract/cycle19-resource-contracts.json`` -- the W1 manifest, which
    selects the ``.qcore`` modules that actually reach the resource analyzer.
  * ``contract/qcore/librdkafka/*.qcore`` -- the declared operations.
  * the audited librdkafka source files named by the profile, in a Fluent Bit
    checkout, together with that checkout's generated ``config.h``.

Two measurements need a C frontend and are skipped (recorded as ``null``) when
Clang or the checkout is unavailable, so the script always terminates with a
report:

  * ``definition_form`` -- whether a declared operation is an externally linked
    function, a ``static inline`` function, a function-like macro, or absent in
    the audited configuration. Macro-ness is decided from ``clang -E -dM``;
    function-ness from a Clang JSON AST of ``rdkafka_queue.c``, which
    transitively includes both other audited files.
  * ``boolean_field_candidates`` -- how many functions defined in the audited
    files satisfy the *structural* admission shape of the existing
    ``qcore.c_boolean_field_contract.v2``/``v3`` capability. This is a necessary
    -condition census, not an admission: it does not run the gateway importer.

Usage::

    python3 -B docs/reports/remaining-p07/inventory.py \
        --fluent-bit ../fluent-bit \
        --output docs/reports/remaining-p07/inventory.json

The output is data for the P07 report. It certifies nothing and promotes
nothing: every verdict vocabulary word in this project stays with the checker
that earns it.
"""

from __future__ import annotations

import argparse
import json
import re
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
LIBRARY = "librdkafka"

# The declared operations, per `.qcore` module, with the arity and result the
# module declares. Kept literal so a contract edit shows up as a mismatch here
# rather than being silently absorbed.
DECLARED = {
    "memory.qcore": [
        ("rd_malloc", 1, "ptr<mut, u8>"),
        ("rd_calloc", 2, "ptr<mut, u8>"),
        ("rd_strdup", 1, "ptr<mut, u8>"),
        ("rd_strndup", 2, "ptr<mut, u8>"),
        ("rd_free", 1, "unit"),
        ("rd_realloc", 2, "ptr<mut, u8>"),
    ],
    "refcount.qcore": [
        ("rd_refcnt_add0", 1, "i32"),
        ("rd_refcnt_sub0", 1, "i32"),
        ("rd_refcnt_destroy_final", 1, "unit"),
    ],
    "typed_queue.qcore": [
        ("rd_kafka_q_enq0", 2, "unit"),
        ("rd_kafka_q_deq0", 1, "result<own<KafkaQueueEntry>, QueueEmpty>"),
    ],
    "queue_destructor.qcore": [
        ("rd_kafka_q_destroy_final", 1, "unit"),
    ],
}

# Declared resource-type destructors, which must also name a real operation.
DECLARED_DESTRUCTORS = {
    "KafkaReference": "rd_refcnt_destroy_final",
    "KafkaQueue": "rd_kafka_q_destroy_final",
    "KafkaQueueOwner": "rd_kafka_q_destroy_final",
    "KafkaQueueEntry": "rd_kafka_op_destroy",
}


def profile_import_surface(profile: dict) -> dict:
    """What the W2 profile declares the adapter imports for librdkafka."""
    library = profile["libraries"][LIBRARY]
    return {
        "source_roots": [root["root"] for root in library["source_roots"]],
        "audited_files": [entry["path"] for entry in library["audited_files"]],
        "source_witnesses": [w["name"] for w in library["source_witnesses"]],
        "permitted_mechanisms": library["permitted_mechanisms"],
        # These are the fields that carry importable operations. Every one of
        # them being empty is the central finding of this inventory.
        "operation_summaries": len(library["operation_summaries"]),
        "api_dispositions": len(library["api_dispositions"]),
        "accepted_builds": len(library["accepted_builds"]),
        "required_evidence": len(library["required_evidence"]),
        "callback_summaries": len(library["callback_summaries"]),
        "boolean_field_models": len(library["boolean_field_models"]),
        "typed_import": library["typed_import"],
        "provider": library["provider"],
    }


def loaded_modules(manifest: dict) -> list[str]:
    prefix = f"qcore/{LIBRARY}/"
    return [m[len(prefix):] for m in manifest["modules"] if m.startswith(prefix)]


def clang_macros(clang: str, command: list[str], directory: Path) -> dict[str, str] | None:
    """Predefined + source-visible macros for the audited configuration."""
    args = [clang]
    args += [a for a in command if a.startswith(("-D", "-I", "-isystem", "-iquote"))]
    args += ["-std=gnu11", "-E", "-dM", command[-1]]
    try:
        out = subprocess.run(
            args, cwd=directory, capture_output=True, text=True, timeout=300
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    macros = {}
    for line in out.stdout.splitlines():
        match = re.match(r"#define\s+(\w+)(\([^)]*\))?\s*(.*)$", line)
        if match:
            macros[match.group(1)] = (match.group(2) or "") + " " + match.group(3)
    return macros


def clang_ast(clang: str, command: list[str], directory: Path) -> dict | None:
    args = [clang]
    args += [a for a in command if a.startswith(("-D", "-I", "-isystem", "-iquote"))]
    args += [
        "-std=gnu11",
        "-fsyntax-only",
        "-Xclang",
        "-ast-dump=json",
        command[-1],
    ]
    try:
        out = subprocess.run(
            args, cwd=directory, capture_output=True, text=True, timeout=900
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0 or not out.stdout.strip():
        return None
    try:
        return json.loads(out.stdout)
    except json.JSONDecodeError:
        return None


def walk_functions(ast: dict) -> dict[str, dict]:
    """Top-level FunctionDecls, keyed by name.

    Clang's JSON omits ``loc.file`` for most nodes in this translation unit and
    its ``includedFrom`` names the *includer*, not the defining file, so a
    carried-forward file is unreliable. The line number is always present, and
    attribution is resolved separately by `attribute_file`, which confirms the
    candidate file actually spells this function at that line.
    """
    functions: dict[str, dict] = {}
    for node in ast.get("inner", []):
        if node.get("kind") != "FunctionDecl":
            continue
        name = node.get("name")
        if not name:
            continue
        has_body = any(
            child.get("kind") == "CompoundStmt" for child in node.get("inner", [])
        )
        record = {
            "line": node.get("loc", {}).get("line"),
            "type": node.get("type", {}).get("qualType"),
            "storage": node.get("storageClass"),
            "inline": bool(node.get("inline")),
            "has_body": has_body,
            "parameters": [
                child.get("type", {}).get("qualType")
                for child in node.get("inner", [])
                if child.get("kind") == "ParmVarDecl"
            ],
            "node": node,
        }
        # A definition supersedes a prior prototype.
        if name not in functions or has_body:
            functions[name] = record
    return functions


def split_signature(qual_type: str | None) -> tuple[str, str] | None:
    if not qual_type or "(" not in qual_type:
        return None
    result, _, rest = qual_type.partition("(")
    return result.strip(), "(" + rest


def attribute_file(
    name: str, line: int | None, audited: dict[str, list[str]]
) -> str | None:
    """Resolve a function to one audited file, by confirming the spelling.

    A candidate file is accepted only when its own text at the reported line
    (allowing one line of slack for a declarator split across lines) actually
    contains the function's name. Functions defined outside the audited set
    resolve to ``None`` and are excluded from the census rather than guessed at.
    """
    if line is None:
        return None
    for path, lines in audited.items():
        for offset in (0, -1, 1):
            index = line - 1 + offset
            if 0 <= index < len(lines) and name in lines[index]:
                return path
    return None


def checkout_commit(checkout: Path) -> str | None:
    """The analysed checkout's HEAD, so the artifact names a revision."""
    try:
        out = subprocess.run(
            ["git", "-C", str(checkout), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return out.stdout.strip() if out.returncode == 0 else None


def relative_file(path: str | None, checkout: Path) -> str | None:
    """Report checkout-relative paths so the artifact is host-independent."""
    if not path:
        return None
    try:
        return str(Path(path).resolve().relative_to(checkout.resolve()))
    except ValueError:
        return path


def resolve(
    name: str,
    functions: dict,
    macros: dict | None,
    audited: dict[str, list[str]] | None = None,
) -> dict:
    """Classify a declared operation against the audited configuration."""
    entry: dict = {"name": name}
    if functions is None or macros is None:
        entry["definition_form"] = None
        return entry
    if name in functions:
        record = functions[name]
        signature = split_signature(record["type"])
        if not record["has_body"]:
            form = "declared_prototype_only"
        elif record["storage"] == "static" or record["inline"]:
            form = "static_inline_function"
        else:
            form = "external_function"
        entry.update(
            {
                "definition_form": form,
                "actual_arity": len(record["parameters"]),
                "actual_parameters": record["parameters"],
                "actual_result": signature[0] if signature else None,
                "declared_at_line": record["line"],
                "audited_file": attribute_file(name, record["line"], audited or {}),
            }
        )
    elif name in macros:
        entry.update(
            {
                "definition_form": "function_like_macro",
                "macro_expansion": macros[name].strip(),
                "actual_arity": None,
                "actual_result": None,
            }
        )
    else:
        entry["definition_form"] = "absent"
    return entry


# --- Boolean-field structural census -------------------------------------

# The existing capability's importer (crates/qcore-c-gateway/src/boolean_source.rs)
# admits, at minimum: exactly one parameter, that parameter a pointer to the
# modelled record, a result of `int` or `void`, and a body whose only member
# accesses are `param->field` where the field's declared type is exactly `int`.
# This census applies those necessary conditions only.
ALLOWED_EXPR = {
    "IntegerLiteral",
    "ImplicitCastExpr",
    "ParenExpr",
    "MemberExpr",
    "UnaryOperator",
    "BinaryOperator",
    "DeclRefExpr",
    "CompoundStmt",
    "IfStmt",
    "ReturnStmt",
    "CallExpr",
}
ALLOWED_UNARY = {"!", "-"}
ALLOWED_BINARY = {"==", "!=", "="}


def boolean_shape_reasons(name: str, record: dict) -> list[str]:
    """Necessary-condition failures for the Boolean-field admission shape."""
    reasons = []
    signature = split_signature(record["type"])
    result = signature[0] if signature else None
    if result not in ("int", "void"):
        reasons.append(f"result type {result!r} is neither int nor void")
    if len(record["parameters"]) != 1:
        reasons.append(f"arity {len(record['parameters'])} is not 1")
    elif not record["parameters"][0].endswith("*"):
        reasons.append("single parameter is not a pointer to a record")
    if not record["has_body"]:
        reasons.append("no body in this translation unit")
        return reasons

    bad_kinds: set[str] = set()
    bad_ops: set[str] = set()
    non_int_fields: set[str] = set()
    foreign_assignments: set[str] = set()
    foreign_callees: set[str] = set()
    field_reads = 0

    def is_record_field(node) -> bool:
        """`param->field`, arrow, with the field declared exactly `int`."""
        return (
            node.get("kind") == "MemberExpr"
            and node.get("isArrow") is True
            and node.get("type", {}).get("qualType") == "int"
        )

    def visit(node):
        nonlocal field_reads
        kind = node.get("kind")
        inner = node.get("inner", []) or []
        if kind and kind.endswith(("Stmt", "Expr", "Operator", "Literal")):
            if kind not in ALLOWED_EXPR:
                bad_kinds.add(kind)
            if kind == "UnaryOperator" and node.get("opcode") not in ALLOWED_UNARY:
                bad_ops.add(node.get("opcode", "?"))
            if kind == "BinaryOperator" and node.get("opcode") not in ALLOWED_BINARY:
                bad_ops.add(node.get("opcode", "?"))
            if kind == "CompoundAssignOperator":
                bad_ops.add(node.get("opcode", "?"))
            if kind == "MemberExpr":
                field_type = node.get("type", {}).get("qualType")
                if field_type != "int":
                    non_int_fields.add(f"{node.get('name')}:{field_type}")
                else:
                    field_reads += 1
            # The importer only ever writes the modelled field; every other
            # assignment target leaves the capability's state model.
            if kind == "BinaryOperator" and node.get("opcode") == "=":
                place = inner[0] if inner else {}
                if not is_record_field(place):
                    foreign_assignments.add(
                        place.get("kind", "?")
                        + ":"
                        + str(place.get("name") or place.get("type", {}).get("qualType"))
                    )
            # The only admitted call is a source-validated frame helper with
            # the exact `void (struct R *)` prototype.
            if kind == "CallExpr":
                callee = inner[0] if inner else {}
                callee_type = callee.get("type", {}).get("qualType", "")
                if not re.fullmatch(r"void \(struct \w+ \*\)", callee_type or ""):
                    foreign_callees.add(callee_type or "?")
        for child in inner:
            visit(child)

    for child in record["node"].get("inner", []):
        if child.get("kind") == "CompoundStmt":
            visit(child)

    if bad_kinds:
        reasons.append("unmodelled node kinds: " + ", ".join(sorted(bad_kinds)))
    if bad_ops:
        reasons.append("unmodelled operators: " + ", ".join(sorted(bad_ops)))
    if non_int_fields:
        reasons.append(
            "member accesses outside the int-field model: "
            + ", ".join(sorted(non_int_fields))
        )
    if foreign_assignments:
        reasons.append(
            "assignments to places other than the modelled record field: "
            + ", ".join(sorted(foreign_assignments))
        )
    if foreign_callees:
        reasons.append(
            "calls that are not a `void (struct R *)` frame helper: "
            + ", ".join(sorted(foreign_callees))
        )
    if field_reads == 0:
        reasons.append("body never accesses an int record field, so no field is modelled")
    return reasons


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, default=ROOT.parent / "fluent-bit")
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()

    profile = json.loads(
        (ROOT / "contract/profiles/fluent-bit/profile.json").read_text()
    )
    manifest = json.loads((ROOT / "contract/cycle19-resource-contracts.json").read_text())

    surface = profile_import_surface(profile)
    selected = loaded_modules(manifest)

    report: dict = {
        "schema_version": 1,
        "scope": (
            "librdkafka adapter imported-operation inventory and a "
            "necessary-condition admission census. Measurement only: no "
            "verdict, no certificate, no promotion."
        ),
        "library": LIBRARY,
        "profile_import_surface": surface,
        "resource_contract_manifest": {
            "selected_librdkafka_modules": selected,
            "declared_librdkafka_modules": sorted(DECLARED),
            "declared_but_unselected": sorted(set(DECLARED) - set(selected)),
        },
        "clang": {"available": False, "checkout_commit": None},
    }

    # --- resolve declared operations against the audited configuration ---
    database = arguments.fluent_bit / "build/compile_commands.json"
    functions = macros = None
    if shutil.which(arguments.clang) and database.is_file():
        entries = json.loads(database.read_text())
        unit = next(
            (
                entry
                for entry in entries
                if entry["file"].endswith(f"{LIBRARY}-2.15.0/src/rdkafka_queue.c")
            ),
            None,
        )
        if unit is not None:
            command = shlex.split(unit["command"])
            directory = Path(unit["directory"])
            macros = clang_macros(arguments.clang, command, directory)
            ast = clang_ast(arguments.clang, command, directory)
            functions = walk_functions(ast) if ast else None
            config = arguments.fluent_bit / (
                f"build/lib/{LIBRARY}-2.15.0/generated/config.h"
            )
            report["clang"] = {
                "available": functions is not None and macros is not None,
                "checkout_commit": checkout_commit(arguments.fluent_bit),
                "translation_unit": relative_file(unit["file"], arguments.fluent_bit),
                "note": (
                    "Census only. Clang is run over the recorded include and "
                    "macro options of the real build entry, whose own driver is "
                    "cc. This is not a qcore-audit run and substitutes no "
                    "compiler for verification purposes."
                ),
                "config_atomics": [
                    line
                    for line in (config.read_text().splitlines() if config.is_file() else [])
                    if "HAVE_ATOMICS_32" in line or "ENABLE_REFCNT_DEBUG" in line
                ],
            }

    # Audited file text, for line-confirmed attribution.
    audited_text: dict[str, list[str]] = {}
    for path in surface["audited_files"]:
        candidate = arguments.fluent_bit / path
        if candidate.is_file():
            audited_text[path] = candidate.read_text(errors="replace").splitlines()

    operations = []
    for module, declarations in sorted(DECLARED.items()):
        for name, arity, result in declarations:
            entry = resolve(name, functions, macros, audited_text)
            entry["module"] = module
            entry["reaches_resource_analyzer"] = module in selected
            entry["declared_arity"] = arity
            entry["declared_result"] = result
            if entry.get("actual_arity") is not None:
                entry["arity_matches"] = entry["actual_arity"] == arity
            operations.append(entry)
    report["declared_operations"] = operations

    destructors = []
    for resource, operation in sorted(DECLARED_DESTRUCTORS.items()):
        entry = resolve(operation, functions, macros, audited_text)
        entry["declared_on_resource_type"] = resource
        destructors.append(entry)
    report["declared_destructors"] = destructors

    # --- Boolean-field structural census over the audited files ---
    if functions is not None:
        candidates = []
        near_misses = {}
        histogram: dict[str, int] = {}
        examined = 0
        per_file: dict[str, int] = {}
        for name, record in sorted(functions.items()):
            file = attribute_file(name, record["line"], audited_text)
            if file is None or not record["has_body"]:
                continue
            examined += 1
            per_file[file] = per_file.get(file, 0) + 1
            reasons = boolean_shape_reasons(name, record)
            if not reasons:
                candidates.append(name)
                continue
            if len(reasons) == 1:
                near_misses[name] = reasons[0]
            for reason in reasons:
                key = reason.split(":")[0]
                histogram[key] = histogram.get(key, 0) + 1
        report["boolean_field_census"] = {
            "refusal_histogram": dict(sorted(histogram.items(), key=lambda kv: -kv[1])),
            "single_reason_near_misses": dict(sorted(near_misses.items())),
            "semantic_profiles": [
                "qcore.c_boolean_field_contract.v2",
                "qcore.c_boolean_field_contract.v3",
            ],
            "conditions": (
                "necessary structural conditions only: one record-pointer "
                "parameter, int or void result, and a body whose statements, "
                "operators and member accesses stay inside the importer's "
                "grammar with every accessed field declared int"
            ),
            "audited_files": sorted(audited_text),
            "functions_with_bodies_examined": examined,
            "functions_with_bodies_per_file": dict(sorted(per_file.items())),
            "structurally_admissible": sorted(candidates),
        }
    else:
        report["boolean_field_census"] = None

    text = json.dumps(report, indent=2, sort_keys=False) + "\n"
    if arguments.output:
        arguments.output.write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
