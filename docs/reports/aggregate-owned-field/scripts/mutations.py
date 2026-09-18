#!/usr/bin/env python3
"""Mutation harness for card AGGREGATE-OWNED-FIELD (wave 39).

Every arm the card added is shown non-vacuous: the arm is mutated by an exact
text replacement, the named test is run and must go RED, the file is restored
from its byte copy and shown sha256-equal. Run from the repository root; the
log is written to the path given as the first argument (or stdout).

    python3 docs/reports/aggregate-owned-field/scripts/mutations.py \
        docs/reports/aggregate-owned-field/mutations.log

A mutation that does not turn its test red is reported as VACUOUS and the
harness exits 1; a restore that is not byte-identical exits 2 (and stops, so
no later mutation runs over a mis-restored file).
"""
import hashlib
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(".").resolve()
OWNERSHIP = "crates/qcore-core/src/ownership.rs"
STD_BUFFER = "crates/qcore-core/src/std_buffer.rs"
EMIT = "crates/qcore-core/src/std_buffer_c_emit.rs"

UNIT = ["cargo", "test", "-q", "-p", "qcore-core", "--lib", "--"]
E2E = ["cargo", "test", "-q", "-p", "qcore-compiler", "--test", "aggregate_owned_field", "--"]

# (label, file, old, new, test command, expectation the mutation falsifies)
MUTATIONS = [
    (
        "M1-drop-through-releases-nothing",
        OWNERSHIP,
        "        for (path, resource) in releasing {\n            let entry = self",
        "        for (path, resource) in releasing.into_iter().take(0) {\n            let entry = self",
        UNIT + ["dropping_an_aggregate_releases_its_live_fields_and_tolerates_a_hole"],
        "drop-through releases the live fields (the ledger is empty after `drop p;`)",
    ),
    (
        "M2-emitter-frees-in-reverse-order",
        EMIT,
        "                for field in declared {\n                    if live.contains(&field) {",
        "                for field in declared.into_iter().rev() {\n                    if live.contains(&field) {",
        E2E + ["a_whole_record_drop_releases_the_live_fields_in_declared_order"],
        "the whole-record release frees the members in declaration order",
    ),
    (
        "M3-value-half-publishes-only-the-first-release",
        STD_BUFFER,
        "                            owned.state = OwnedFieldState::Released;\n                            released.push(format!(\"{name}.{field}\"));",
        "                            owned.state = OwnedFieldState::Released;\n                            if released.is_empty() { released.push(format!(\"{name}.{field}\")); }",
        E2E + ["a_whole_record_drop_releases_the_live_fields_in_declared_order"],
        "the certificate publishes every field the drop-through released",
    ),
    (
        "M4-construction-does-not-move",
        OWNERSHIP,
        "            self.bindings.insert(Place::name(&from), Binding::Moved);\n            self.bindings\n                .insert(Place::Field(path), Binding::Owned(resource));",
        "            self.bindings\n                .insert(Place::Field(path), Binding::Owned(resource));",
        E2E + ["using_the_buffer_after_moving_it_into_a_record_refuses_as_a_use_after_move"],
        "the moved-in binding is a use after move (the checker's own words)",
    ),
    (
        "M5-move-out-leaves-no-hole",
        OWNERSHIP,
        "        entry.owner = to.clone();\n        self.bindings.insert(from.clone(), Binding::Moved);",
        "        entry.owner = to.clone();",
        UNIT + ["moving_a_field_out_leaves_a_hole_a_second_read_refuses"],
        "a move out leaves a hole (the ledger invariant holds and a second read refuses)",
    ),
    (
        "M6-leak-rule-blind-to-fields",
        OWNERSHIP,
        "            .filter(|(_, binding)| matches!(binding, Binding::Owned(_)))\n            .map(|(place, _)| place.spelling())",
        "            .filter(|(place, binding)| matches!(binding, Binding::Owned(_)) && matches!(place, Place::Name(_)))\n            .map(|(place, _)| place.spelling())",
        UNIT + ["an_unreleased_field_is_a_normal_path_leak_named_by_its_path"],
        "an unreleased field is reported by its path",
    ),
    (
        "M7-aggregate-copied",
        OWNERSHIP,
        "            Binding::Owned(_) | Binding::Aggregate { .. } => {\n                Err(OwnershipError::CopyOfOwned(place.spelling()))\n            }",
        "            Binding::Aggregate { .. } => Ok(()),\n            Binding::Owned(_) => Err(OwnershipError::CopyOfOwned(place.spelling())),",
        UNIT + ["an_aggregate_is_never_copied"],
        "a record with an owned field is never copied",
    ),
    (
        "M8-Q055-lift-withdrawn",
        STD_BUFFER,
        "                        (Some(FieldType::Bytes), BufferOperand::Scalar(name))\n                            if self.extents.contains_key(name) =>\n                        {\n                            Ok(FieldValue::Bytes {\n                                moved_from: name.clone(),\n                            })\n                        }",
        "                        (Some(FieldType::Bytes), BufferOperand::Scalar(name))\n                            if self.extents.contains_key(name) =>\n                        {\n                            Err(BufferError::CrossesBoundary { event: index, name: name.clone() })\n                        }",
        E2E + ["a_buffer_at_a_bytes_field_is_admitted_and_at_an_i32_field_still_refused"],
        "a buffer at a bytes field is admitted (the lift)",
    ),
    (
        "M9-lend-window-gate-removed",
        STD_BUFFER,
        "    if composes_owned_field\n        && let Some(position) = program",
        "    if false\n        && let Some(position) = program",
        E2E + ["a_lend_beside_an_owned_field_and_an_owned_field_in_a_loop_body_refuse_by_name"],
        "a lend beside an owned-field record is refused by name",
    ),
    (
        "M10-loop-body-gate-removed",
        STD_BUFFER,
        "            if offending {\n                return Err(vec![BufferError::Shape {",
        "            if offending && false {\n                return Err(vec![BufferError::Shape {",
        E2E + ["a_lend_beside_an_owned_field_and_an_owned_field_in_a_loop_body_refuse_by_name"],
        "an owned-field statement in a for body is refused by name",
    ),
    (
        "M11-aggregate-readable-as-a-plain-owner",
        OWNERSHIP,
        "            Binding::Aggregate { .. } => {\n                Err(OwnershipError::AggregateAtOwnerPosition(place.spelling()))\n            }\n        }\n    }\n\n    fn write(",
        "            Binding::Aggregate { .. } => Ok(()),\n        }\n    }\n\n    fn write(",
        UNIT + ["an_aggregate_at_a_plain_owner_position_refuses_by_name"],
        "the record itself is not a plain owner a read can name",
    ),
    (
        "M12-emitter-frees-a-moved-out-member",
        EMIT,
        "                let live = records.live_fields.remove(name).unwrap_or_default();\n                for field in declared {\n                    if live.contains(&field) {",
        "                let _live = records.live_fields.remove(name).unwrap_or_default();\n                for field in declared {\n                    if true {",
        E2E + ["the_emitted_c_holds_the_pointer_in_the_struct_and_moves_by_initializer"],
        "the whole-record release skips a moved-out member (no double free)",
    ),
    (
        "M13-field-path-bounds-unchecked",
        STD_BUFFER,
        "    if index < 0 || u32::try_from(index).is_ok_and(|index| index >= extent) {\n        return Err(BufferError::OutOfBounds {\n            event,\n            name: spelling.to_owned(),",
        "    if index < 0 {\n        return Err(BufferError::OutOfBounds {\n            event,\n            name: spelling.to_owned(),",
        E2E + ["an_index_through_a_field_is_bounded_by_the_buffer_it_holds"],
        "an index through a field is bounded by the buffer the field holds",
    ),
    (
        "M14-scalar-field-at-custody-position-admitted",
        STD_BUFFER,
        "            Some(FieldType::I32) => {\n                return Err(BufferError::Record {\n                    event,\n                    refusal: RecordRefusal::FieldIsNotABuffer {",
        "            Some(FieldType::I32) if false => {\n                return Err(BufferError::Record {\n                    event,\n                    refusal: RecordRefusal::FieldIsNotABuffer {",
        E2E + ["a_custody_position_at_a_scalar_field_is_refused"],
        "a custody position naming a scalar field is refused",
    ),
]


def sha(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def run(command):
    completed = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    return completed.returncode, completed.stdout + completed.stderr


def main():
    out = open(sys.argv[1], "w", encoding="utf-8") if len(sys.argv) > 1 else sys.stdout

    def log(line):
        print(line, file=out, flush=True)

    code, _ = run(UNIT + ["ownership::"])
    log(f"baseline unit: exit={code}")
    code, _ = run(E2E)
    log(f"baseline e2e: exit={code}")
    vacuous = 0
    for label, file, old, new, command, expectation in MUTATIONS:
        path = ROOT / file
        original = path.read_bytes()
        before = hashlib.sha256(original).hexdigest()
        text = original.decode("utf-8")
        if text.count(old) != 1:
            log(f"{label}: ANCHOR NOT UNIQUE ({text.count(old)} occurrences) - harness defect, stopping")
            sys.exit(3)
        path.write_text(text.replace(old, new), encoding="utf-8")
        try:
            code, output = run(command)
        finally:
            path.write_bytes(original)
        after = sha(path)
        verdict = "RED (arm is driven)" if code != 0 else "GREEN - VACUOUS"
        if code == 0:
            vacuous += 1
        log(f"{label}: exit={code} {verdict}")
        log(f"  falsifies: {expectation}")
        for line in output.splitlines():
            if "panicked at" in line or "error[" in line or "test result" in line:
                log(f"  {line.strip()}")
                break
        if before != after:
            log(f"  RESTORE FAILED for {file}: {before} != {after}")
            sys.exit(2)
        log(f"  restored: sha256 EQUAL ({before[:12]})")
    code, _ = run(E2E)
    log(f"restored e2e: exit={code}")
    code, _ = run(UNIT + ["ownership::"])
    log(f"restored unit: exit={code}")
    log(f"vacuous: {vacuous} of {len(MUTATIONS)}")
    sys.exit(1 if vacuous else 0)


if __name__ == "__main__":
    main()
