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

import QCore.Closures

-- Card `W48-QR-CLOSURES`: a closure's explicit capture list is lifted into a
-- hidden named worker, whose parameter types its spawn site decides.

-- Capture admission equals the transfer judgment: the parameter a closure's
-- capture binds is the only one the judgment admits, so a closure capture is
-- admitted exactly when some hand-written twin's is, by the kind judgment
-- itself, and refused with the judgment's own reason.
#print axioms QCore.Closures.flatAdmissionNamesTheClosureKind
#print axioms QCore.Closures.flatClosureAdmissionIsTheKindJudgment
#print axioms QCore.Closures.flatClosureRefusalIsTheKindsOwn
#print axioms QCore.Closures.theClosureParameterIsTheOnlyAdmissibleOne
#print axioms QCore.Closures.aRecordCaptureAdmitsOnlyItsOwnRecordType
#print axioms QCore.Closures.aClosureCaptureIsAdmittedExactlyWhenSomeTwinIs
#print axioms QCore.Closures.aClosureCaptureIsDecidedByTheKindJudgment
#print axioms QCore.Closures.aClosureRefusalIsTheJudgmentsOwnReason
#print axioms QCore.Closures.aClosureWriteCaptureIsRefusedByName

-- No implicit capture: the body's scope is the capture binders and the
-- module's globals, independent of the enclosing scope; a body that resolves
-- uses no other outer name, and an uncaptured outer name is refused.
#print axioms QCore.Closures.resolve_none_iff
#print axioms QCore.Closures.aRefusalNamesAnUnresolvedName
#print axioms QCore.Closures.theBodyScopeIgnoresTheEnclosingScope
#print axioms QCore.Closures.theBodysFreeOuterNamesAreExactlyTheCaptureBinders
#print axioms QCore.Closures.anUncapturedOuterNameIsRefused

-- The desugaring preserves the task IR: the parameters the spawn site decides
-- are the only ones a checking twin can declare, so the lifted module lowers
-- to the twin's transfers and worker IR, and a refused closure has no
-- checking twin.
#print axioms QCore.Closures.aCheckingParameterListIsTheDiscoveredOne
#print axioms QCore.Closures.desugaringPreservesTheTaskIR
#print axioms QCore.Closures.aRefusedClosureHasNoCheckingTwin

-- The hidden name contains `#`, which no source identifier does.
#print axioms QCore.Closures.aHiddenNameIsNoSourceIdentifier

-- Non-vacuity: the end-to-end example's captures bind what its twin declares.
#print axioms QCore.Closures.theExampleCapturesBindWhatTheTwinDeclares
