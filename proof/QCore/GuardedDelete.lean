namespace QCore.GuardedDelete

structure Result where
  syscallCalls : Nat
  returned : Int
  diagnosticCalls : Nat
  deriving DecidableEq

def eligible (descriptorActive mappingActive : Bool) : Bool :=
  !descriptorActive && !mappingActive

def finish (descriptorActive mappingActive success : Bool) : Result :=
  if eligible descriptorActive mappingActive then
    if success then ⟨1, 0, 0⟩ else ⟨1, -1, 1⟩
  else ⟨0, -1, 0⟩

theorem active_descriptor_blocks_delete (mapping success : Bool) :
    finish true mapping success = ⟨0, -1, 0⟩ := by simp [finish,eligible]

theorem active_mapping_blocks_delete (descriptor success : Bool) :
    finish descriptor true success = ⟨0, -1, 0⟩ := by simp [finish,eligible]

theorem eligible_failure_is_reported : finish false false false = ⟨1, -1, 1⟩ := rfl

theorem eligible_success_returns_zero : finish false false true = ⟨1, 0, 0⟩ := rfl

-- Returning a deletion result cannot consume C allocation custody. That would
-- require a separate release transition and owned-pointer provenance.
structure Record where
  recordLive : Bool
  pathLive : Bool
  descriptorActive : Bool
  mappingActive : Bool
  deriving DecidableEq

def delete (record : Record) (success : Bool) : Record × Result :=
  (record, finish record.descriptorActive record.mappingActive success)

theorem delete_preserves_record_and_path (record : Record) (success : Bool) :
    (delete record success).1 = record := rfl

end QCore.GuardedDelete
