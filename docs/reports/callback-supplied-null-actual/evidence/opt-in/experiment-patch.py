"""Experiment only (never committed): read every plain `_` as supplied_nullable."""
import sys
from pathlib import Path

p = Path(sys.argv[1])
s = p.read_text()
old = """                CallbackActual::SuppliedNullable,
            ) => nonnull_at_nullable = true,
"""
new = old + """            (ResourceParameterMode::Owned { nullable: false }, CallbackActual::Supplied) => {
                nonnull_at_nullable = true
            }
"""
assert s.count(old) == 1
p.write_text(s.replace(old, new))
