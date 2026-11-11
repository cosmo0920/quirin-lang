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
