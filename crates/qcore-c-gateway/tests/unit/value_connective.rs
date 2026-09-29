//! Unit tests of card CG06's value-region reading (gateway rule 10z) and the
//! mechanical binding to `proof/QCore/ValueConnectives.lean`.

use super::*;

#[test]
fn cg06_admission_table() {
    for (siblings, decision, inert, middle, admitted) in ADMISSION_VECTORS {
        assert_eq!(
            admit(siblings, decision, inert, middle).is_ok(),
            admitted,
            "{siblings} {decision} {inert} {middle:?}"
        );
    }
}
