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

import QCore.RemovedSelection

/-! Card RULE33-PILOT: rule 33 as a switchable package.

Rule 33 is consulted at two host points. Each CFG build asks its caller
expansion for the analysed body: a checked frame's own body, or its
decimal-constant caller statements, become the frame's constituent calls;
switched off, the body stays verbatim, so every use of a frame keeps the
host's reading of an uncontracted call (its refusal). The custody derivation
asks its entry row: a checked frame whose own run under the selection entry
validates is installed and its row is that run; switched off, no frame is
installed and the row is the ordinary, contract-free analysis. These theorems
state that the switch on is exactly rule 33's model (`QCore.RemovedSelection`),
that off is the gateway before rule 33, and that off never makes a row `Safe`
that is not `Safe` with rule 33 on. The Rust/model correspondence, the host's
frame reading and the analyzer's run observations remain premises; so does the
older routes' own soundness, which off hands every frame name back to. -/
namespace QCore.RemovedSelectionSwitch

open QCore.RemovedSelection

/-- The package's validation of a frame row's run (Rust `validates`): clean,
some exit, selecting on some path, every exit accepted. -/
def validatesRun (d : Disposition) (clean : Bool) (exits : List ExitFacts) : Bool :=
  clean && !exits.isEmpty && exits.any (fun f => f.selected != .absent) &&
    exits.all (acceptExit d)

/-- The entry row's admission with rule 33 on or off: a disabled package is
not asked, which counts as a refusal. -/
def entryAdmits (on : Bool) (d : Disposition) (clean : Bool) (exits : List ExitFacts) :
    Bool :=
  on && validatesRun d clean exits

/-- The caller expansion with rule 33 on or off (`none`: the body stays
verbatim). -/
def expandUnder {Body : Type} (on : Bool) (expansion : Option Body) : Option Body :=
  if on then expansion else none

theorem the_switch_on_reads_rule_33 (d : Disposition) (clean : Bool)
    (exits : List ExitFacts) : entryAdmits true d clean exits = validatesRun d clean exits := by
  simp [entryAdmits]

theorem the_switch_on_is_the_expansion {Body : Type} (expansion : Option Body) :
    expandUnder true expansion = expansion := by
  simp [expandUnder]

theorem the_switch_off_installs_no_frame (d : Disposition) (clean : Bool)
    (exits : List ExitFacts) : entryAdmits false d clean exits = false := by
  simp [entryAdmits]

theorem the_switch_off_expands_nothing {Body : Type} (expansion : Option Body) :
    expandUnder false expansion = none := by
  simp [expandUnder]

/-- An admitted row is clean, selects on some path and accepts every exit:
the switch adds no admission of its own. -/
theorem an_admitted_row_validates (on : Bool) (d : Disposition) (clean : Bool)
    (exits : List ExitFacts) (h : entryAdmits on d clean exits = true) :
    clean = true ∧ (∃ f ∈ exits, f.selected ≠ .absent) ∧ ∀ f ∈ exits, acceptExit d f = true := by
  simp only [entryAdmits, validatesRun, Bool.and_eq_true, List.any_eq_true,
    List.all_eq_true, bne_iff_ne, ne_eq] at h
  exact ⟨h.2.1.1.1, h.2.1.2, h.2.2⟩

/-- A frame's own row is `Safe` with rule 33 on or off: an installed frame's
row is its validated run (clean, hence `Safe`); any other row is the ordinary
analysis, `fallbackSafe`. -/
def frameRowSafe (on : Bool) (d : Disposition) (clean : Bool) (exits : List ExitFacts)
    (fallbackSafe : Bool) : Bool :=
  if entryAdmits on d clean exits then clean else fallbackSafe

theorem the_switch_off_row_is_the_ordinary_analysis (d : Disposition) (clean : Bool)
    (exits : List ExitFacts) (fallbackSafe : Bool) :
    frameRowSafe false d clean exits fallbackSafe = fallbackSafe := by
  simp [frameRowSafe, entryAdmits]

/-- Off never widens a frame's own row. -/
theorem the_switch_off_never_widens_a_frame_row (d : Disposition) (clean : Bool)
    (exits : List ExitFacts) (fallbackSafe : Bool)
    (h : frameRowSafe false d clean exits fallbackSafe = true) :
    frameRowSafe true d clean exits fallbackSafe = true := by
  rw [the_switch_off_row_is_the_ordinary_analysis] at h
  unfold frameRowSafe
  by_cases admits : entryAdmits true d clean exits = true
  · simp only [admits, ite_true]
    exact (an_admitted_row_validates true d clean exits admits).1
  · simp only [admits]
    exact h

/-- A caller statement with rule 33 on or off: an expanded statement is
decided by the ordinary analysis of its constituent calls (`analysedSafe`);
a statement left verbatim calls an uncontracted frame and is refused. -/
def callerSafe {Body : Type} (on : Bool) (expansion : Option Body)
    (analysedSafe : Body → Bool) : Bool :=
  match expandUnder on expansion with
  | some body => analysedSafe body
  | none => false

theorem the_switch_off_refuses_every_frame_caller {Body : Type} (expansion : Option Body)
    (analysedSafe : Body → Bool) : callerSafe false expansion analysedSafe = false := by
  simp [callerSafe, expandUnder]

/-- Off never widens a caller. -/
theorem the_switch_off_never_widens_a_caller {Body : Type} (expansion : Option Body)
    (analysedSafe : Body → Bool) (h : callerSafe false expansion analysedSafe = true) :
    callerSafe true expansion analysedSafe = true := by
  simp [the_switch_off_refuses_every_frame_caller] at h

/-- The retained cJSON frames: the release frame's validated run (the
selection released on the selecting path, nothing on the `W < 0` arm) is
admitted with the switch on and not installed with it off. -/
theorem the_switch_decides_the_retained_release_frame :
    entryAdmits true .released true [⟨true, true, .released, .nothing⟩,
      ⟨true, true, .absent, .nothing⟩] = true ∧
    entryAdmits false .released true [⟨true, true, .released, .nothing⟩,
      ⟨true, true, .absent, .nothing⟩] = false := by
  decide

end QCore.RemovedSelectionSwitch
