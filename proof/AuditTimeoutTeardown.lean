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
