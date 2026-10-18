# Checked same-item detach

Work in progress from `e8b9ff07defe4c6375b1547ce2016c68a8ded98c`.
The user chose checked same-item transfer on 2026-10-18.
[The frozen prediction](PREDICTION.md) precedes implementation.

The C body does not establish membership merely by accepting a non-NULL item.
Checked containment, physical list representation, identity-preserving return
and the no-transfer NULL path are separate obligations. Measurements and gates
are pending; no verdict or coverage change is claimed by this preparation.
