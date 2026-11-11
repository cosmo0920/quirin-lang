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

import QCore.TimeoutTeardown

open QCore.EventDriven
open QCore.TimeoutTeardown

example : legalLifecycle .running .free .freed = false := by decide
example : legalLifecycle .running .deleteBlocking .running = false := by decide
example : legalLifecycle .running .deleteCurrentCallback .running = true := by decide
example : legalLifecycle .running .freeCurrentCallback .selfTeardown = true := by decide
example : legalLifecycle .selfTeardown .callbackExitSelfTeardown .freed = true := by decide
example : legalLifecycle .selfTeardown .free .freed = false := by decide
example : legalLifecycle .selfTeardown .activate .active = false := by decide

def accepted : Evidence := {
  mapWriteDescriptor := 8
  closedDescriptor := 8
  callbackRunning := true
  sameDispatcher := true
  currentCallbackDeleted := true
  eventDescriptorAbsent := true
  eventFreedOnce := true
  mapFreedOnce := true
  writeEndpointRetired := true
  peerEndpointHandledByCaller := true
  callbackExited := true
  trace := [1, 2, 3, 4, 5, 6]
  expectedTrace := [1, 2, 3, 4, 5, 6]
}

example : (check accepted).isSome = true := by decide
example : (check { accepted with sameDispatcher := false }).isNone = true := by decide
example : (check { accepted with currentCallbackDeleted := false }).isNone = true := by decide
example : (check { accepted with eventDescriptorAbsent := false }).isNone = true := by decide
example : (check { accepted with mapFreedOnce := false }).isNone = true := by decide
example : (check { accepted with writeEndpointRetired := false }).isNone = true := by decide
example : (check { accepted with peerEndpointHandledByCaller := false }).isNone = true := by decide
example : (check { accepted with callbackExited := false }).isNone = true := by decide
example : (check { accepted with trace := [1, 3, 2, 4, 5, 6] }).isNone = true := by decide

example : (check { accepted with closedDescriptor := 9 }).isNone = true := by decide
