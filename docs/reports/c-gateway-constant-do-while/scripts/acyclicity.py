#!/usr/bin/env python3
"""Card C-GATEWAY-CONSTANT-DO-WHILE: the interaction with JOINED-PROVENANCE-CYCLES.

`legacy_joined_provenance::derive` topologically sorts the CFG and, when the
sort fails — that is, when the CFG has a cycle — **returns no conditional
provenance facts at all** for that whole function. Removing a back edge can
therefore flip a function from "no facts" to "facts derived", which is a real
interaction with wave 35's second arbiter and is measured here rather than
argued.

The probe is inserted into `legacy_c.rs` (this card's own file) rather than
into `legacy_joined_provenance.rs`, which this card stays out of. It runs the
same Kahn sort `derive` runs, over the same CFG, and appends one line per
analysed function to `$QCORE_CDW_ACYCLIC`.

Two instrumented binaries are built — one from `HEAD` (the back edge always
present) and one from this card's tree — and each is run over the pinned
corpus. Both instrumentations are reverted on every exit path; neither is part
of the card's change.
"""
import collections
import os
import pathlib
import subprocess
import sys

M = pathlib.Path(__file__).resolve().parent
WT = M.parents[1]
TARGET = WT / "crates/qcore-c-gateway/src/legacy_c.rs"

ANCHOR = "    let joined = joined_provenance::derive(&cfg);"

PROBE = '''    let joined = joined_provenance::derive(&cfg);
    if let Ok(path) = std::env::var("QCORE_CDW_ACYCLIC") {
        // Card C-GATEWAY-CONSTANT-DO-WHILE measurement probe (not shipped):
        // the same Kahn sort `joined_provenance::derive` runs, so a line here
        // says whether that function got any conditional provenance at all.
        let mut reachable = std::collections::BTreeSet::new();
        let mut pending = vec![cfg.entry];
        while let Some(node) = pending.pop() {
            if reachable.insert(node) {
                pending.extend(&cfg.nodes[node].successors);
            }
        }
        let mut degrees = vec![0usize; cfg.nodes.len()];
        for &node in &reachable {
            for &next in &cfg.nodes[node].successors {
                degrees[next] += 1;
            }
        }
        let mut ready = reachable
            .iter()
            .copied()
            .filter(|node| degrees[*node] == 0)
            .collect::<std::collections::VecDeque<_>>();
        let mut sorted = 0usize;
        while let Some(node) = ready.pop_front() {
            sorted += 1;
            for &next in &cfg.nodes[node].successors {
                degrees[next] -= 1;
                if degrees[next] == 0 {
                    ready.push_back(next);
                }
            }
        }
        let acyclic = sorted == reachable.len();
        use std::io::Write as _;
        if let Ok(mut fh) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(fh, "{}\\t{}", cfg.function, u8::from(acyclic));
        }
    }'''


def patch(source_text):
    assert source_text.count(ANCHOR) == 1, "probe anchor is not unique"
    return source_text.replace(ANCHOR, PROBE)


def build(tag):
    subprocess.run(
        [
            "cargo", "build", "--release", "-p", "qcore-compiler",
            "--example", "cycle19_fluent_bit_libraries_audit",
        ],
        cwd=WT, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT,
    )
    binary = M / "bin" / f"audit-probe-{tag}"
    binary.write_bytes(
        (WT / "target/release/examples/cycle19_fluent_bit_libraries_audit").read_bytes()
    )
    binary.chmod(0o755)
    return binary


def audit(tag, binary):
    log = M / f"acyclic-{tag}.tsv"
    log.unlink(missing_ok=True)
    env = dict(os.environ)
    env["QCORE_CDW_ACYCLIC"] = str(log)
    subprocess.run(
        [sys.executable, "-B", str(M / "run_audit.py"), f"probe-{tag}", str(binary)],
        cwd=WT, check=True, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.STDOUT,
    )
    return log


def read(log):
    table = {}
    for line in log.read_text().splitlines():
        name, flag = line.rsplit("\t", 1)
        # A name can be analysed more than once (re-analysis validates a
        # summary); acyclicity is a property of the CFG, so any disagreement
        # is reported rather than collapsed.
        table.setdefault(name, set()).add(flag == "1")
    return table


def main():
    original = TARGET.read_text()
    mine = original
    head = subprocess.run(
        ["git", "show", "HEAD:crates/qcore-c-gateway/src/legacy_c.rs"],
        cwd=WT, check=True, stdout=subprocess.PIPE,
    ).stdout.decode()
    try:
        results = {}
        for tag, text in (("head", head), ("card", mine)):
            TARGET.write_text(patch(text))
            binary = build(tag)
            results[tag] = read(audit(tag, binary))
            print(f"{tag}: {len(results[tag])} analysed function names")
    finally:
        TARGET.write_text(original)
    print(f"source restored: {TARGET.read_text() == original}")

    before, after = results["head"], results["card"]
    shared = sorted(set(before) & set(after))
    flipped = [n for n in shared if before[n] == {False} and after[n] == {True}]
    unflipped_cyclic = [n for n in shared if before[n] == {False} and after[n] == {False}]
    acyclic_both = [n for n in shared if before[n] == {True} and after[n] == {True}]
    regressed = [n for n in shared if True in before[n] and after[n] == {False}]
    mixed = [n for n in shared if len(before[n]) > 1 or len(after[n]) > 1]

    print(f"\nfunctions seen in both runs: {len(shared)}")
    print(f"  acyclic in both:                        {len(acyclic_both)}")
    print(f"  cyclic in both:                         {len(unflipped_cyclic)}")
    print(f"  CYCLIC -> ACYCLIC (this card's effect): {len(flipped)}")
    print(f"  acyclic -> cyclic (must be 0):          {len(regressed)}")
    print(f"  mixed within a run (re-analysis):       {len(mixed)}")
    (M / "acyclicity.txt").write_text(
        "\n".join(
            [
                f"functions seen in both runs: {len(shared)}",
                f"acyclic in both: {len(acyclic_both)}",
                f"cyclic in both: {len(unflipped_cyclic)}",
                f"cyclic -> acyclic: {len(flipped)}",
                f"acyclic -> cyclic: {len(regressed)}",
                f"mixed within a run: {len(mixed)}",
                "",
                "functions this card makes acyclic:",
                *(f"  {n}" for n in flipped),
            ]
        )
        + "\n"
    )
    print(f"\nwrote {M / 'acyclicity.txt'}")
    return 1 if regressed else 0


if __name__ == "__main__":
    sys.exit(main())
