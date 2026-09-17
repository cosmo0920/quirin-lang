#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CALL-FRAMES: falsify every new arm before believing it.

Each mutation breaks one rule of the call-frame preservation and names the test
that must go red. A mutation no test catches is a rule nothing verifies — the
`STABILITY-REEVAL` "vacuous greenness" check applied to this card's own work.
The source file is patched in place with `write_text`, which advances the
mtime, so cargo cannot skip the rebuild and report a stale binary's result
(`MUTATION-HARNESS-MTIME-RESTORE`). Run on a clean tree.

Usage: python3 measure/joined-provenance-call-frames/mutations.py
"""
import pathlib
import subprocess
import sys

WT = pathlib.Path(__file__).resolve().parents[2]
SRC = WT / "crates/qcore-c-gateway/src/legacy_joined_provenance.rs"
# `rebound_names` lives in the gateway's main module; two mutations reach it,
# because the call arm relies on it having already forgotten what the call
# writes.
LEGACY_C = WT / "crates/qcore-c-gateway/src/legacy_c.rs"

KEEP = """                for names in self.bindings.values_mut() {
                    names.retain(|name| preserved.contains(name));
                }"""

MUTATIONS = [
    (
        "the call clears every relation again (pre-card behaviour restored)",
        KEEP,
        "                self.bindings.clear();",
        "a_call_no_longer_forgets_a_relation_it_cannot_write",
    ),
    (
        "the call clears every relation again — the end-to-end route",
        KEEP,
        "                self.bindings.clear();",
        "an_acquisition_survives_an_intervening_call_end_to_end",
    ),
    (
        "the frame test is dropped: a call preserves every name unconditionally",
        KEEP,
        "                let _ = preserved;",
        "a_call_forgets_a_name_the_frame_list_does_not_carry",
    ),
    (
        "the reallocated parameter is no longer forgotten",
        """                if let ResourceReturnMode::Reallocated { parameter, .. } = contract.result
                    && let Some(Some(name)) = arguments.get(parameter)
                {
                    self.forget(name);
                }""",
        "",
        "a_call_still_forgets_a_reallocated_parameter",
    ),
    (
        "the rebound forget is skipped, so a call keeps its own destination",
        "        for name in rebound_names(instruction) {\n            self.forget(name);\n        }",
        "",
        "a_call_still_forgets_its_own_destination",
    ),
    (
        # The bare `matches!` spelling occurs eight times in `legacy_c.rs`, so
        # the anchor carries `rebound_names`' whole `filter_map` to be unique;
        # a non-unique anchor mutates an unrelated rule and reports a green
        # suite as a coverage gap.
        "the out-slot forget is bypassed: `rebound_names` stops claiming them",
        """                    matches!(mode, ResourceParameterMode::Out { .. })
                        .then_some(argument.as_deref()?)""",
        """                    false
                        .then_some(argument.as_deref()?)""",
        "a_call_still_forgets_an_out_slot",
        LEGACY_C,
    ),
]

# NOT mutated, and recorded rather than hidden: the *construction* of the frame
# (`cfg.conditional_names = unique_local_names ∩ frame_protected − formals`,
# cards AOM-BINDING-POISON and C-GATEWAY-WAVE) is pre-existing code this card
# reads and does not write. Its exclusions — static duration, file-scope,
# address-taken, shadowed, formal — are this rule's load-bearing premise but
# are not this card's arms; see `CALL-FRAMES-FRAME-READER-PREMISE` in the
# report.


def run_tests():
    proc = subprocess.run(
        ["cargo", "test", "-p", "qcore-c-gateway", "--lib"],
        cwd=WT,
        capture_output=True,
        text=True,
    )
    failed = [
        line.split()[1]
        for line in proc.stdout.splitlines()
        if line.startswith("test ") and line.rstrip().endswith("FAILED")
    ]
    return proc.returncode, failed


def main():
    original = SRC.read_text()
    legacy_c_original = LEGACY_C.read_text()
    code, failed = run_tests()
    if code != 0:
        print(f"the tree is not green before mutating (failures: {failed})", file=sys.stderr)
        return 2
    print("baseline: green\n")
    bad = 0
    try:
        for mutation in MUTATIONS:
            name, old, new, expect = mutation[:4]
            target = mutation[4] if len(mutation) > 4 else SRC
            source = original if target == SRC else target.read_text()
            if old not in source:
                print(f"SKIP  {name}\n      anchor not found — the mutation is stale")
                bad += 1
                continue
            target.write_text(source.replace(old, new, 1))
            code, failed = run_tests()
            caught = code != 0
            named = expect == "(any)" or any(expect in f for f in failed)
            ok = caught and named
            bad += 0 if ok else 1
            print(f"{'CAUGHT' if ok else 'MISSED'}  {name}")
            print(f"        expected: {expect}")
            print(
                f"        failures: {', '.join(failed) if failed else '(none — suite stayed green)'}"
            )
            target.write_text(source)
    finally:
        SRC.write_text(original)
        LEGACY_C.write_text(legacy_c_original)
    code, failed = run_tests()
    print(f"\nrestored tree: {'green' if code == 0 else 'RED ' + str(failed)}")
    print(f"mutations not caught as expected: {bad}")
    return 1 if bad or code != 0 else 0


if __name__ == "__main__":
    sys.exit(main())
