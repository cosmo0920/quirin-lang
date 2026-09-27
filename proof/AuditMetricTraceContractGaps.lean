import QCore.MetricTraceContractGaps

/-! Audited custody-vector parity for W46-METRIC-TRACE-CONTRACT-GAPS.
The body/ABI readings at the two source pins are external premises. -/

#print axioms QCore.MetricTraceContractGaps.selected_count
#print axioms QCore.MetricTraceContractGaps.selected_contracts_have_no_custody_transfer
#print axioms QCore.MetricTraceContractGaps.borrowed_does_not_discharge_owned
#print axioms QCore.MetricTraceContractGaps.releasing_a_selected_formal_contradicts
#print axioms QCore.MetricTraceContractGaps.missing_body_evidence_keeps_premise
