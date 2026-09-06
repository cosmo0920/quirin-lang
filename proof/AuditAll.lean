import Lean
import QCore

/-! Audit every declaration in the compiled QCore namespace, including private
helpers. Per-cycle print audits remain useful evidence, but do not enumerate
all project declarations. Only the standard logical axioms are allowed here. -/

open Lean Elab Command in
run_elab do
  let allowed := #[`propext, `Classical.choice, `Quot.sound]
  let environment ← getEnv
  let mut checked : Nat := 0
  for (name, _) in environment.constants.toList do
    let text := name.toString
    if text.startsWith "QCore." || text.startsWith "_private.QCore." then
      checked := checked + 1
      for dependency in (← collectAxioms name) do
        unless allowed.contains dependency do
          throwError "{name} depends on forbidden axiom {dependency}"
  if checked == 0 then
    throwError "QCore axiom audit checked no declarations"
  logInfo m!"QCore axiom audit: {checked} declarations checked; only standard logical axioms"
