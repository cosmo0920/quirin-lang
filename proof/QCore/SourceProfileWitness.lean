-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

namespace QCore.SourceProfileWitness

/-! Reduced model of the shared source-profile gate. Path/digest syntax,
byte hashing, substring evaluation and revision comparison are explicit input
premises. This proves inventory and whole-profile admission properties, not
Rust/parser refinement, cryptographic authenticity or library semantics. -/

structure AuditedSource where
  path : String
  shapeValid : Bool
  present : Bool
  digestMatches : Bool
  deriving DecidableEq, Repr

structure Witness where
  name : String
  file : String
  shapeValid : Bool
  passes : Bool
  grants : List Nat
  deriving DecidableEq, Repr

structure Profile where
  audited : List AuditedSource
  witnesses : List Witness
  deriving DecidableEq, Repr

def Profile.Valid (profile : Profile) : Prop :=
  (profile.audited.map AuditedSource.path).Nodup ∧
  (profile.witnesses.map Witness.name).Nodup ∧
  (∀ file ∈ profile.audited, file.shapeValid = true) ∧
  (∀ witness ∈ profile.witnesses, witness.shapeValid = true ∧
    ∃ file ∈ profile.audited, file.path = witness.file)

instance (profile : Profile) : Decidable profile.Valid := by
  unfold Profile.Valid
  infer_instance

def Profile.Checked (profile : Profile) : Prop :=
  (∀ file ∈ profile.audited, file.present = true ∧ file.digestMatches = true) ∧
  (∀ witness ∈ profile.witnesses, witness.passes = true)

instance (profile : Profile) : Decidable profile.Checked := by
  unfold Profile.Checked
  infer_instance

def derive (profile : Profile) (revisionMatches : Bool) : Option (List Nat) :=
  if profile.Valid ∧ revisionMatches = true ∧ profile.Checked then
    some (profile.witnesses.flatMap Witness.grants)
  else none

theorem accepted_profile_is_valid {profile : Profile} {revision : Bool}
    {grants : List Nat} (accepted : derive profile revision = some grants) :
    profile.Valid ∧ revision = true ∧ profile.Checked := by
  unfold derive at accepted
  split at accepted
  · assumption
  · contradiction

theorem accepted_witness_has_checked_file {profile : Profile} {revision : Bool}
    {grants : List Nat} (accepted : derive profile revision = some grants)
    {witness : Witness} (member : witness ∈ profile.witnesses) :
    witness.shapeValid = true ∧ witness.passes = true ∧
    ∃ file ∈ profile.audited, file.path = witness.file ∧
      file.present = true ∧ file.digestMatches = true := by
  obtain ⟨valid, _, checked⟩ := accepted_profile_is_valid accepted
  obtain ⟨shape, file, fileMember, path⟩ := valid.2.2.2 witness member
  exact ⟨shape, checked.2 witness member, file, fileMember, path,
    checked.1 file fileMember⟩

theorem accepted_grant_has_checked_witness {profile : Profile} {revision : Bool}
    {grants : List Nat} (accepted : derive profile revision = some grants)
    {mechanism : Nat} (member : mechanism ∈ grants) :
    ∃ witness ∈ profile.witnesses, mechanism ∈ witness.grants ∧
      witness.passes = true ∧ ∃ file ∈ profile.audited,
        file.path = witness.file ∧ file.present = true ∧ file.digestMatches = true := by
  have gate := accepted_profile_is_valid accepted
  have output : profile.witnesses.flatMap Witness.grants = grants := by
    simpa [derive, gate.1, gate.2.1, gate.2.2] using accepted
  rw [← output, List.mem_flatMap] at member
  obtain ⟨witness, witnessMember, grantMember⟩ := member
  obtain ⟨_, passes, source⟩ := accepted_witness_has_checked_file accepted witnessMember
  exact ⟨witness, witnessMember, grantMember, passes, source⟩

theorem unaudited_witness_rejects (profile : Profile) (revision : Bool)
    {witness : Witness} (member : witness ∈ profile.witnesses)
    (missing : ¬ ∃ file ∈ profile.audited, file.path = witness.file) :
    derive profile revision = none := by
  have invalid : ¬ profile.Valid := fun valid => missing (valid.2.2.2 witness member).2
  simp [derive, invalid]

theorem missing_audited_file_rejects (profile : Profile) (revision : Bool)
    {file : AuditedSource} (member : file ∈ profile.audited)
    (missing : file.present = false) : derive profile revision = none := by
  have unchecked : ¬ profile.Checked := by
    intro checked
    have present := (checked.1 file member).1
    simp [missing] at present
  simp [derive, unchecked]

theorem malformed_witness_rejects (profile : Profile) (revision : Bool)
    {witness : Witness} (member : witness ∈ profile.witnesses)
    (malformed : witness.shapeValid = false) : derive profile revision = none := by
  have invalid : ¬ profile.Valid := by
    intro valid
    have shape := (valid.2.2.2 witness member).1
    simp [malformed] at shape
  simp [derive, invalid]

theorem failed_witness_rejects_all_grants (profile : Profile) (revision : Bool)
    {witness : Witness} (member : witness ∈ profile.witnesses)
    (failed : witness.passes = false) : derive profile revision = none := by
  have unchecked : ¬ profile.Checked := by
    intro checked
    have passes := checked.2 witness member
    simp [failed] at passes
  simp [derive, unchecked]

theorem duplicate_witness_names_reject (profile : Profile) (revision : Bool)
    (duplicate : ¬ (profile.witnesses.map Witness.name).Nodup) :
    derive profile revision = none := by
  have invalid : ¬ profile.Valid := fun valid => duplicate valid.2.1
  simp [derive, invalid]

theorem empty_profile_grants_nothing : derive ⟨[], []⟩ true = some [] := by
  simp [derive, Profile.Valid, Profile.Checked]

end QCore.SourceProfileWitness
