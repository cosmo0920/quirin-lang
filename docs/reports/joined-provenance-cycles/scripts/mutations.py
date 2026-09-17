#!/usr/bin/env python3
"""Card JOINED-PROVENANCE-CYCLES: falsify every new arm before believing it.

Each mutation breaks one rule of the cyclic extension and names the test that
must go red. A mutation no test catches is a rule nothing verifies — the
`STABILITY-REEVAL` "vacuous greenness" check applied to this card's own work.
The source file is patched in place and restored afterwards; run on a clean
tree.

Usage: python3 measure/joined-provenance-cycles/mutations.py
"""
import pathlib
import subprocess
import sys

WT = pathlib.Path(__file__).resolve().parents[2]
SRC = WT / "crates/qcore-c-gateway/src/legacy_joined_provenance.rs"

MUTATIONS = [
    (
        "the loop-created site restriction is removed (every site eligible)",
        "        if created.is_empty() || on_cycle(node) {\n            continue;\n        }\n        sites.extend(created);",
        "        if created.is_empty() {\n            continue;\n        }\n        sites.extend(created);",
        "a_site_created_inside_the_cycle_derives_nothing",
    ),
    (
        "the fixed point declines everything (wave-31 behaviour restored)",
        "        return fixed_point(cfg, &reachable);",
        "        return vec![Bindings::new(); cfg.nodes.len()];",
        "a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release",
    ),
    (
        "the meet at a join unions names instead of intersecting them",
        """                (true, true) => self
                    .bindings
                    .get(&site)
                    .into_iter()
                    .flatten()
                    .filter(|name| {
                        other
                            .bindings
                            .get(&site)
                            .is_some_and(|names| names.contains(*name))
                    })
                    .cloned()
                    .collect(),""",
        """                (true, true) => self
                    .bindings
                    .get(&site)
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect(),""",
        "(any)",
    ),
    (
        "the iteration stops after one round instead of at stability",
        "    for _ in 0..FIXED_POINT_ROUNDS {",
        "    for _ in 0..1 {",
        "a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release",
    ),
    (
        "a contracted allocator's produced site is no longer eligible",
        "            LegacyInstruction::ContractCall { produced_sites, .. } => produced_sites.clone(),\n            _ => Vec::new(),\n        };\n        if created.is_empty() || on_cycle(node) {",
        "            _ => Vec::new(),\n        };\n        if created.is_empty() || on_cycle(node) {",
        "a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release",
    ),
    (
        "the eligible-site filter no longer restricts the published bindings",
        "                    bindings.retain(|site, names| sites.contains(site) && !names.is_empty());",
        "                    bindings.retain(|_site, names| !names.is_empty());",
        "an_eligible_site_does_not_carry_an_ineligible_one_with_it",
    ),
]

# Recorded, not a gap: replacing the `None` predecessor skip with an explicit
# `Facts::default()` is caught by no test because the two are the **same**
# lattice element — an absent site is neutral at a join, which is exactly
# `QCore.CGatewayRefinements.absent_predecessor_is_neutral`. A mutation that
# cannot change a result is not evidence that a rule is unverified.


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
    code, failed = run_tests()
    if code != 0:
        print(f"the tree is not green before mutating (failures: {failed})", file=sys.stderr)
        return 2
    print("baseline: green\n")
    bad = 0
    try:
        for name, old, new, expect in MUTATIONS:
            if old not in original:
                print(f"SKIP  {name}\n      anchor not found — the mutation is stale")
                bad += 1
                continue
            SRC.write_text(original.replace(old, new, 1))
            code, failed = run_tests()
            caught = code != 0
            named = expect == "(any)" or any(expect in f for f in failed)
            ok = caught and named
            bad += 0 if ok else 1
            print(f"{'CAUGHT' if ok else 'MISSED'}  {name}")
            print(f"        expected: {expect}")
            print(f"        failures: {', '.join(failed) if failed else '(none — suite stayed green)'}")
    finally:
        SRC.write_text(original)
    code, failed = run_tests()
    print(f"\nrestored tree: {'green' if code == 0 else 'RED ' + str(failed)}")
    print(f"mutations not caught as expected: {bad}")
    return 1 if bad or code != 0 else 0


if __name__ == "__main__":
    sys.exit(main())
