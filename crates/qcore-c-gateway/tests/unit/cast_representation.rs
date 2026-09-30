//! Unit tests of card CG08-CASTS-AND-TYPE-QUERIES (gateway rule 11b): the
//! cast table, the type-query operand classes, and the mechanical binding to
//! `proof/QCore/CastRepresentation.lean`.

use super::*;

#[test]
fn variably_modified_reads_every_extent() {
    assert!(!variably_modified("int"));
    assert!(!variably_modified("int[4]"));
    assert!(!variably_modified("char[16][2]"));
    assert!(!variably_modified("int[]"));
    assert!(variably_modified("int[n]"));
    assert!(variably_modified("int (*)[n]"));
    assert!(variably_modified("int[4][n + 1]"));
    assert!(variably_modified("int[4"));
}
