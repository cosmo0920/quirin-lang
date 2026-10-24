import Lean
import QCore

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  let out := (← IO.getEnv "DUMP_OUT").getD "decls.txt"
  let mut all : Array String := #[]
  let mut audited : Array String := #[]
  for (name, info) in env.constants.toList do
    let text := name.toString
    let kind := match info with
      | .thmInfo _ => "thm" | .defnInfo _ => "def" | .axiomInfo _ => "axiom"
      | .opaqueInfo _ => "opaque" | .inductInfo _ => "induct" | .ctorInfo _ => "ctor"
      | .recInfo _ => "rec" | .quotInfo _ => "quot"
    all := all.push s!"{kind} {text}"
    if text.startsWith "QCore." || text.startsWith "_private.QCore." then
      audited := audited.push s!"{kind} {text}"
  let allSorted := all.qsort (· < ·)
  let auditedSorted := audited.qsort (· < ·)
  IO.FS.writeFile (out ++ ".audited") (String.intercalate "\n" auditedSorted.toList ++ "\n")
  IO.FS.writeFile (out ++ ".all") (String.intercalate "\n" allSorted.toList ++ "\n")
  logInfo m!"audited {audited.size} all {all.size}"
