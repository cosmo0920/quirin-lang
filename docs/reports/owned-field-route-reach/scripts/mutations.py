#!/usr/bin/env python3
"""Falsification harness for card `OWNED-FIELD-ROUTE-REACH` (wave 41).

Every arm this card adds or makes reachable is shown **non-vacuous by
mutation**: the arm's own text is replaced by one that breaks exactly the rule
it carries, the named test must go **red**, and the file is then restored from
a byte copy taken before the edit and shown sha256-equal. A mutation whose test
stays green is reported VACUOUS and fails the harness — a green mutation means
the test was not measuring the arm.

Restoration is by **rewrite plus `touch`**, never by an mtime-preserving copy
(`MUTATION-HARNESS-STALE-ARTIFACTS`): cargo keys on mtime, and a restore that
preserves it leaves the next build reading a stale artifact.

The baseline is proved green **before and after** the whole run.

    python3 docs/reports/owned-field-route-reach/scripts/mutations.py . \
        --log measure/mutations.log
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
import time
from pathlib import Path

RUST_TESTS = [
    ("qcore-compiler", "owned_field_route_reach"),
]

#: (id, file, needle, replacement, package, test target, test name, what it falsifies)
MUTATIONS: list[tuple[str, str, str, str, str, str, str, str]] = [
    (
        "M1",
        "crates/qcore-core/src/std_buffer.rs",
        """                        self.operations
                            .push(OwnershipOp::Copy(Self::owner(renames, source)));""",
        """                        let _ = source;""",
        "qcore-compiler",
        "owned_field_route_reach",
        "copying_a_record_with_an_owned_field_refuses_as_the_checkers_own_copy_rule",
        "the custody walk asks the IR about a ledger-held name",
    ),
    (
        "M2",
        "crates/qcore-frontend/src/source_std_buffer.rs",
        """        return Some(BufferEvent::CopyBind {
            binding: binding.to_owned(),
            source: source.to_owned(),
        });""",
        """        let _ = source;""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_record_of_scalars_is_copyable_and_the_copy_is_by_value",
        "the reader recognises `let NAME = NAME;` at all",
    ),
    (
        "M3",
        "crates/qcore-core/src/std_buffer.rs",
        """                    if !owns {
                        self.records.insert(binding.clone(), value);
                    }""",
        """                    let _ = owns;""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_record_of_scalars_is_copyable_and_the_copy_is_by_value",
        "the value half binds the copied record value",
    ),
    (
        "M4",
        "crates/qcore-core/src/std_buffer.rs",
        """                if let Some(value) = self.scalars.get(source).copied() {
                    self.scalars.insert(binding.clone(), value);
                } else if let Some(value) = self.records.get(source).cloned() {""",
        """                if false {
                } else if let Some(value) = self.records.get(source).cloned() {""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_scalar_is_copyable_and_the_copy_carries_its_value",
        "the value half carries a copied scalar's value",
    ),
    (
        "M5",
        "crates/qcore-core/src/std_buffer_c_emit.rs",
        """                    "{indent}struct {} {emitted} = {};",
                    record_type_tag(&type_name),
                    record_binding_name(from)""",
        """                    "{indent}struct {} {emitted} = {{ 0 }};{}",
                    record_type_tag(&type_name),
                    record_binding_name(from)""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_record_of_scalars_is_copyable_and_the_copy_is_by_value",
        "the emitted C copies the record rather than zeroing it",
    ),
    (
        "M6",
        "crates/qcore-core/src/std_buffer_c_emit.rs",
        """            format!("int32_t {emitted} = {from};")""",
        """            format!("int32_t {emitted} = INT32_C(0); (void) {from};")""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_scalar_is_copyable_and_the_copy_carries_its_value",
        "the emitted C copies the scalar rather than zeroing it",
    ),
    (
        "M7",
        "crates/qcore-core/src/source_record.rs",
        """    if spelling == VIEW_FIELD_SPELLING {
        return RecordRefusal::ViewFieldType {""",
        """    if false {
        return RecordRefusal::ViewFieldType {""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_view_field_refuses_by_its_own_name",
        "`AGGREGATE-VIEW-FIELD` refuses by its own name",
    ),
    (
        "M8",
        "crates/qcore-core/src/source_record.rs",
        """    if declared_records.contains(spelling) {
        return RecordRefusal::NestedFieldType {""",
        """    if false && declared_records.contains(spelling) {
        return RecordRefusal::NestedFieldType {""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_nested_record_field_refuses_by_its_own_name_in_both_declaration_orders",
        "`AGGREGATE-NESTED` refuses by its own name",
    ),
    (
        "M9",
        "crates/qcore-frontend/src/source_record.rs",
        """    let declared_records: std::collections::BTreeSet<String> = program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Record)
        .map(|declaration| declaration.value.name.clone())
        .collect();""",
        """    let declared_records: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_nested_record_field_refuses_by_its_own_name_in_both_declaration_orders",
        "the nested classification sees the module's declared record names",
    ),
    (
        "M10",
        "crates/qcore-frontend/src/source_tasks.rs",
        """        if declaration.value.kind == DeclarationKind::Record {""",
        """        if false && declaration.value.kind == DeclarationKind::Record {""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_record_in_a_task_module_refuses_by_its_own_name",
        "`AGGREGATE-TASK-CAPTURE` refuses by its own name",
    ),
    (
        "M11",
        "crates/qcore-core/src/std_buffer.rs",
        """                        if self.acquired.contains(source) || self.loans.contains_key(source) {
                            operations.push(OwnershipOp::Copy(source.clone()));
                        }
                        // A copied **scalar** carries its value forward""",
        """                        // A copied **scalar** carries its value forward""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_loan_at_a_copy_position_refuses_once_from_the_ownership_ir",
        "the lend-structured projection asks the IR about a copy too",
    ),
    (
        "M12",
        "crates/qcore-core/src/std_buffer.rs",
        """                | BufferEvent::CopyBind { binding, .. } => {
                    names.insert(binding.clone());
                }""",
        """                | BufferEvent::CopyBind { binding, .. } => {
                    let _ = binding;
                }""",
        "qcore-compiler",
        "owned_field_route_reach",
        "a_copy_bind_in_a_body_that_shadows_an_outer_name_refuses",
        "a copy bind's binding counts as a name the body binds",
    ),
]

#: Lean mutations: (id, file, needle, replacement, theorem, what it falsifies)
LEAN_MUTATIONS: list[tuple[str, str, str, str, str, str]] = [
    (
        "L1",
        "proof/QCore/OwnedFieldCopyReach.lean",
        """  | .aggregate _ => .error (.copyOfOwned n)""",
        """  | .aggregate _ => .ok ()""",
        "copy_refuses_every_bound_place",
        "every bound place refuses, the aggregate arm included",
    ),
    (
        "L2",
        "proof/QCore/OwnedFieldCopyReach.lean",
        """  | .dropped => .error (.useAfterDrop n)""",
        """  | .dropped => .ok ()""",
        "copy_refuses_every_bound_place",
        "every bound place refuses, the dropped arm included",
    ),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command: list[str], cwd: Path, log) -> int:
    log.write(f"\n$ {' '.join(command)}\n")
    log.flush()
    return subprocess.call(command, cwd=str(cwd), stdout=log, stderr=log)


def cargo_test(root: Path, package: str, target: str, name: str | None, log) -> int:
    command = ["cargo", "test", "-p", package, "--test", target]
    if name:
        command += [name, "--", "--exact"]
    return run(command, root, log)


def lake_build(root: Path, log) -> int:
    return run(["lake", "build", "QCore.OwnedFieldCopyReach"], root / "proof", log)


def restore(path: Path, original: bytes, log) -> bool:
    """Rewrite the file and bump its mtime — never an mtime-preserving copy."""
    path.write_bytes(original)
    now = time.time()
    import os

    os.utime(path, (now, now))
    digest = sha256(path)
    want = hashlib.sha256(original).hexdigest()
    log.write(f"restore {path}: sha256 {digest} {'==' if digest == want else '!='} {want}\n")
    return digest == want


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("--log", default="measure/mutations.log")
    args = ap.parse_args()
    root = Path(args.root).resolve()
    log_path = Path(args.log)
    if not log_path.is_absolute():
        log_path = root / log_path
    log_path.parent.mkdir(parents=True, exist_ok=True)

    vacuous: list[str] = []
    broken: list[str] = []
    with log_path.open("w", encoding="utf-8") as log:
        log.write("=== baseline before ===\n")
        for package, target in RUST_TESTS:
            if cargo_test(root, package, target, None, log) != 0:
                print("baseline is not green; refusing to mutate", file=sys.stderr)
                return 2
        if lake_build(root, log) != 0:
            print("Lean baseline is not green; refusing to mutate", file=sys.stderr)
            return 2

        for (
            mid,
            rel,
            needle,
            replacement,
            package,
            target,
            name,
            falsifies,
        ) in MUTATIONS:
            path = root / rel
            original = path.read_bytes()
            text = original.decode("utf-8")
            if text.count(needle) != 1:
                log.write(f"\n{mid}: needle occurs {text.count(needle)} times in {rel}\n")
                broken.append(f"{mid}: needle not unique in {rel}")
                continue
            log.write(f"\n=== {mid}: {rel} — falsifies: {falsifies} ===\n")
            path.write_text(text.replace(needle, replacement), encoding="utf-8")
            code = cargo_test(root, package, target, name, log)
            log.write(f"{mid}: `{name}` exit {code} ({'RED' if code != 0 else 'GREEN'})\n")
            if code == 0:
                vacuous.append(f"{mid} ({name})")
            if not restore(path, original, log):
                broken.append(f"{mid}: restore mismatch")

        for mid, rel, needle, replacement, theorem, falsifies in LEAN_MUTATIONS:
            path = root / rel
            original = path.read_bytes()
            text = original.decode("utf-8")
            if text.count(needle) != 1:
                broken.append(f"{mid}: needle not unique in {rel}")
                continue
            log.write(f"\n=== {mid}: {rel} — falsifies: {falsifies} ({theorem}) ===\n")
            path.write_text(text.replace(needle, replacement), encoding="utf-8")
            code = lake_build(root, log)
            log.write(f"{mid}: `lake build` exit {code} ({'RED' if code != 0 else 'GREEN'})\n")
            if code == 0:
                vacuous.append(f"{mid} ({theorem})")
            if not restore(path, original, log):
                broken.append(f"{mid}: restore mismatch")

        log.write("\n=== baseline after ===\n")
        after_ok = True
        for package, target in RUST_TESTS:
            if cargo_test(root, package, target, None, log) != 0:
                after_ok = False
        if lake_build(root, log) != 0:
            after_ok = False
        log.write(f"baseline after: {'GREEN' if after_ok else 'RED'}\n")

        total = len(MUTATIONS) + len(LEAN_MUTATIONS)
        log.write(
            f"\nSUMMARY: {total - len(vacuous)} of {total} red, "
            f"{len(vacuous)} vacuous, {len(broken)} broken\n"
        )
        for entry in vacuous:
            log.write(f"  VACUOUS {entry}\n")
        for entry in broken:
            log.write(f"  BROKEN  {entry}\n")

    print(f"mutations: {len(MUTATIONS) + len(LEAN_MUTATIONS)} run, "
          f"{len(vacuous)} vacuous, {len(broken)} broken; log {log_path}")
    return 0 if not vacuous and not broken and after_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
