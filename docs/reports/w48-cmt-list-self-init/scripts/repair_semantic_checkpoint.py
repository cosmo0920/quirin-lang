from pathlib import Path
p=Path('crates/qcore-contract/tests/production_symbolic_adoption.rs');s=p.read_text()
start=s.index('    // Rule 37 intentionally replaces these two inheritance declarations.');end=s.index('    const CJSON:',start)
block=s[start:end];a=block.index('        let before =');b=block.index('        assert!(current.starts_with(after)');pins=block[a:b]
helper='''const RK_MEMORY: &str = "providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv";
fn rule37_checkpoint_prefixes() -> (&'static str, &'static str) {
'''+''.join(line[4:]+'\n' for line in pins.splitlines())+'''    (before, after)
}
// The symbolic-adoption control uses current semantic contracts on both sides.
// Apply only the exact pinned rule37 change to the frozen pre-adoption bytes;
// the inverse byte-erasure check independently preserves the frozen checkpoint.
fn current_semantic_checkpoint(path: &str, frozen: &str) -> String {
    if path != RK_MEMORY {
        return frozen.into();
    }
    let (before, after) = rule37_checkpoint_prefixes();
    assert!(frozen.starts_with(before));
    assert_eq!(frozen.matches(before).count(), 1);
    frozen.replacen(before, after, 1)
}
'''
replace='''    // Rule37's exact semantic replacement is separate from symbolic metadata.
    if path == RK_MEMORY {
        let (before, after) = rule37_checkpoint_prefixes();
        assert!(current.starts_with(after), "rule37 source prefix changed");
        assert_eq!(current.matches(after).count(), 1);
        return current.replacen(after, before, 1);
    }
'''
s=s[:start]+replace+s[end:];needle='// Only the historical byte-erasure seam uses this recovered checkpoint.\n';assert s.count(needle)==1;s=s.replace(needle,helper+needle)
needle='        let old = sources(path, &before);\n';assert s.count(needle)==1;s=s.replace(needle,'        let semantic_before = current_semantic_checkpoint(path, &before);\n        let old = sources(path, &semantic_before);\n');p.write_text(s)
print('Rebased only the known rule37 semantic prefix for graph/mode comparisons; raw frozen-byte comparison remains exact.')
