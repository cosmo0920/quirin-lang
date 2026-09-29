import QCore.ScalarBufferValues

-- Card `W48-QR-SCALAR-TYPES-R5` (`W48-QR-BUFFER-ROUTE-ARITHMETIC`): the value
-- statement on the byte-buffer route, every operand known at check time.

-- Admitted exactly when the checked operation succeeds, in range and exact;
-- nothing on the route raises; a refusal is the classified raise.
#print axioms QCore.ScalarBufferValues.bufferBinary_iff_checked
#print axioms QCore.ScalarBufferValues.buffer_statement_never_raises
#print axioms QCore.ScalarBufferValues.bufferBinary_in_range
#print axioms QCore.ScalarBufferValues.bufferBinary_exact
#print axioms QCore.ScalarBufferValues.buffer_refusal_is_the_raise

-- Negation is `0 - a`.
#print axioms QCore.ScalarBufferValues.negate_is_zero_minus
#print axioms QCore.ScalarBufferValues.unsigned_negation_only_at_zero

-- Inside a loop the unrolling decides every index value, and an admitted body
-- statement never faults at run time; one faulting iteration refuses.
#print axioms QCore.ScalarBufferValues.unrolled_admits_iff
#print axioms QCore.ScalarBufferValues.admitted_body_statement_never_faults
#print axioms QCore.ScalarBufferValues.one_faulting_iteration_refuses_the_loop

-- The vectors replayed through whole programs.
#print axioms QCore.ScalarBufferValues.bufferAgreement_holds
