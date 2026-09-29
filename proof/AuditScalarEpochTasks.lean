import QCore.ScalarEpochTasks

-- Card `W48-QR-SCALAR-TYPES-R6` (`W48-QR-TYPES-EPOCH-TASKS`): fixed-width tasks
-- on the composed epoch route.

-- The derived shape reads no scalar type, so the plan–ledger agreement is the
-- `i32` program's.
#print axioms QCore.ScalarEpochTasks.clause_ignores_the_type
#print axioms QCore.ScalarEpochTasks.retains_ignores_the_type
#print axioms QCore.ScalarEpochTasks.the_derived_shape_is_type_blind
#print axioms QCore.ScalarEpochTasks.agreement_is_type_blind

-- The value statement's two error paths: a worker's raise is a join-failure
-- edge; an admitted creator exit abandons no child credit.
#print axioms QCore.ScalarEpochTasks.a_raising_worker_is_a_join_failure_edge
#print axioms QCore.ScalarEpochTasks.an_admitted_creator_exit_abandons_no_child_credit
#print axioms QCore.ScalarEpochTasks.an_exit_with_a_child_outstanding_is_refused
