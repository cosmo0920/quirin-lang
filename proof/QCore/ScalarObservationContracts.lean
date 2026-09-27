import QCore.ReachGatePosition

/-! Custody shape of W46's seven pinned Monkey/Libevent declarations.
The names and C bodies are trusted, source-pinned premises in the provider.
This model checks only the declared neutral shape consumed by the gateway:
all arguments are borrowed and every result has no custody. -/

namespace QCore.ScalarObservationContracts

open QCore.ReachGatePosition

/-- Arity in provider source order: Monkey search, two char scans, event
initialization, then three Libevent metadata queries. -/
def arities : List Nat := [4, 3, 3, 1, 1, 0, 0]

theorem every_declared_shape_is_neutral :
    arities.all (fun n => isNeutralContract (neutralContract n)) = true := by
  decide

end QCore.ScalarObservationContracts
