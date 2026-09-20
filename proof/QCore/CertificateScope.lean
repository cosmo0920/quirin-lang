/-!
Card `E01-TAIL-CERT-SCOPE`: the independent Safe-certificate target
denominator.

The Rust certificate retains evidence from which its selected definition list
is reconstructed. `checked_functions` is only a compatibility index. This
model separates two questions which must not be conflated:

* `structurallyAccepts` checks coverage against the targets reconstructed from
  the certificate's source evidence;
* `acceptsFor` additionally compares with context held independently by the
  caller, so coherently replacing source evidence, inventory and digest creates
  a different certificate rather than silently changing the intended audit.

`InventoryDigest` is represented by its collision-free logical content here.
Rust serializes the same canonical rows with length prefixes and binds them by
SHA-256. Cryptographic collision resistance, faithful source extraction and
Rust–Lean correspondence remain premises, not theorems of this model.
-/
namespace QCore.CertificateScope

abbrev Name := String
abbrev InputIdentity := String

inductive Coverage where
  | analyzedSafe
  | excluded (reason : String)
  | unknown (reason : String)
  deriving DecidableEq, Repr

structure Row where
  name : Name
  coverage : Coverage
  deriving DecidableEq, Repr

def analyzedRows (targets : List Name) : List Row :=
  targets.map fun name => ⟨name, .analyzedSafe⟩

/-- Logical content of the Rust canonical inventory digest. -/
abbrev InventoryDigest := List Row

structure Certificate where
  /-- Targets reconstructed from retained source/scope evidence. -/
  evidenceTargets : List Name
  rows : List Row
  inventoryDigest : InventoryDigest
  inputIdentity : InputIdentity
  deriving DecidableEq, Repr

structure ExpectedContext where
  targets : List Name
  inventoryDigest : InventoryDigest
  inputIdentity : InputIdentity
  deriving DecidableEq, Repr

def structurallyAccepts (certificate : Certificate) : Bool :=
  certificate.evidenceTargets.Nodup &&
  certificate.rows == analyzedRows certificate.evidenceTargets &&
  certificate.inventoryDigest == certificate.rows

def acceptsFor (certificate : Certificate) (expected : ExpectedContext) : Bool :=
  structurallyAccepts certificate &&
  certificate.evidenceTargets == expected.targets &&
  certificate.inventoryDigest == expected.inventoryDigest &&
  certificate.inputIdentity == expected.inputIdentity

/-! ## General containment properties -/

/-- Structural acceptance implies exact, all-analyzed coverage of the target
list reconstructed from source evidence. -/
theorem structural_acceptance_has_exact_coverage (certificate : Certificate)
    (accepted : structurallyAccepts certificate = true) :
    certificate.rows = analyzedRows certificate.evidenceTargets := by
  simp [structurallyAccepts] at accepted
  exact accepted.2.1

/-- Structural acceptance also establishes a canonical target denominator:
duplicate names cannot inflate or ambiguously relabel coverage. -/
theorem structural_acceptance_has_no_duplicate_targets (certificate : Certificate)
    (accepted : structurallyAccepts certificate = true) :
    certificate.evidenceTargets.Nodup := by
  simp [structurallyAccepts] at accepted
  exact accepted.1

/-- The inventory digest accepted structurally is exactly the canonical
coverage rows, rather than a caller-selected independent assertion. -/
theorem structural_acceptance_binds_inventory_digest (certificate : Certificate)
    (accepted : structurallyAccepts certificate = true) :
    certificate.inventoryDigest = analyzedRows certificate.evidenceTargets := by
  simp [structurallyAccepts] at accepted
  exact accepted.2.2.trans accepted.2.1

/-- Context-bound acceptance identifies the reconstructed denominator with the
one the caller held independently. -/
theorem contextual_acceptance_binds_expected_targets
    (certificate : Certificate) (expected : ExpectedContext)
    (accepted : acceptsFor certificate expected = true) :
    certificate.evidenceTargets = expected.targets := by
  simp [acceptsFor] at accepted
  exact accepted.2.1

/-- Context-bound acceptance also identifies the effective artifact/request
identity with the caller's expected identity. -/
theorem contextual_acceptance_binds_expected_input
    (certificate : Certificate) (expected : ExpectedContext)
    (accepted : acceptsFor certificate expected = true) :
    certificate.inputIdentity = expected.inputIdentity := by
  simp [acceptsFor] at accepted
  exact accepted.2.2.2

/-- Any denominator disagreement rejects, for arbitrary certificates and
contexts rather than only the concrete mutation witnesses below. -/
theorem unequal_expected_denominator_rejects
    (certificate : Certificate) (expected : ExpectedContext)
    (different : certificate.evidenceTargets ≠ expected.targets) :
    acceptsFor certificate expected = false := by
  simp [acceptsFor, different]

/-- Any effective input disagreement likewise rejects independently of the
coverage rows. -/
theorem unequal_expected_input_rejects
    (certificate : Certificate) (expected : ExpectedContext)
    (different : certificate.inputIdentity ≠ expected.inputIdentity) :
    acceptsFor certificate expected = false := by
  simp [acceptsFor, different]

/-- Removing a target row while retaining the source evidence cannot pass the
structural replay, even if the compatibility checked set was reduced with it. -/
theorem scope_reduction_is_rejected :
    structurallyAccepts
      { evidenceTargets := ["alpha", "beta"]
        rows := analyzedRows ["alpha"]
        inventoryDigest := analyzedRows ["alpha"]
        inputIdentity := "input-a" } = false := by
  decide

/-- Duplicate target names are not a larger denominator; canonical replay
refuses them before considering their digest. -/
theorem duplicate_targets_are_rejected :
    structurallyAccepts
      { evidenceTargets := ["alpha", "alpha"]
        rows := analyzedRows ["alpha", "alpha"]
        inventoryDigest := analyzedRows ["alpha", "alpha"]
        inputIdentity := "input-a" } = false := by
  decide

/-- An excluded or Unknown definition cannot be hidden inside a Safe
certificate's target inventory. -/
theorem unresolved_coverage_is_rejected :
    structurallyAccepts
      { evidenceTargets := ["alpha"]
        rows := [{ name := "alpha", coverage := .unknown "not projected" }]
        inventoryDigest := [{ name := "alpha", coverage := .unknown "not projected" }]
        inputIdentity := "input-a" } = false := by
  decide

/-- A smaller artifact can be structurally valid for its own evidence. This is
why structural self-consistency is not authentication of the caller's target. -/
theorem a_coherent_smaller_artifact_is_structurally_distinct :
    let smaller : Certificate :=
      { evidenceTargets := ["alpha"]
        rows := analyzedRows ["alpha"]
        inventoryDigest := analyzedRows ["alpha"]
        inputIdentity := "input-b" }
    structurallyAccepts smaller = true := by
  decide

/-- The caller-held context rejects that coherent smaller artifact as a
replacement for an audit whose intended denominator had two definitions. -/
theorem expected_context_rejects_coherent_scope_replacement :
    let smaller : Certificate :=
      { evidenceTargets := ["alpha"]
        rows := analyzedRows ["alpha"]
        inventoryDigest := analyzedRows ["alpha"]
        inputIdentity := "input-b" }
    let intended : ExpectedContext :=
      { targets := ["alpha", "beta"]
        inventoryDigest := analyzedRows ["alpha", "beta"]
        inputIdentity := "input-a" }
    acceptsFor smaller intended = false := by
  decide

/-- Even where an explicit partial scope keeps the same denominator, a changed
header/effective input identity is a different audit context. -/
theorem changed_effective_input_is_rejected :
    let certificate : Certificate :=
      { evidenceTargets := ["alpha"]
        rows := analyzedRows ["alpha"]
        inventoryDigest := analyzedRows ["alpha"]
        inputIdentity := "header-v1" }
    let changedHeader : ExpectedContext :=
      { targets := ["alpha"]
        inventoryDigest := analyzedRows ["alpha"]
        inputIdentity := "header-v2" }
    acceptsFor certificate changedHeader = false := by
  decide

/-- The unmodified complete certificate remains accepted for its independently
derived context, proving the gate is not a blanket refusal. -/
theorem complete_matching_scope_is_accepted :
    let certificate : Certificate :=
      { evidenceTargets := ["alpha", "beta"]
        rows := analyzedRows ["alpha", "beta"]
        inventoryDigest := analyzedRows ["alpha", "beta"]
        inputIdentity := "input-a" }
    let expected : ExpectedContext :=
      { targets := ["alpha", "beta"]
        inventoryDigest := analyzedRows ["alpha", "beta"]
        inputIdentity := "input-a" }
    acceptsFor certificate expected = true := by
  decide

end QCore.CertificateScope
