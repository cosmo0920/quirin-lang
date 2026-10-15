import Lean
import Std
/-! LEAN-4-35-TOOLCHAIN: print the parser token table (reserved keywords and symbols)
of `import Lean` plus `import Std`, the widest imports the proof uses. Non-reserved
symbols (`nonReservedSymbol`, such as the def contract clause words `given`, `requires`,
`ensures`) are not tokens and do not appear here. -/
open Lean Elab Command
#eval show CommandElabM Unit from do
  let env ← getEnv
  let tt := Lean.Parser.getTokenTable env
  let toks := (tt.findPrefix "").qsort (· < ·)
  for t in toks do
    IO.println t
