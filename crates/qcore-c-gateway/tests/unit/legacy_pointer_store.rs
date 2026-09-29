//! Card CG11-POINTER-VALUED-STORES (rule 10v): the flow helpers.

use super::*;

#[test]
fn a_place_rooted_at_the_root_is_not_the_root() {
    assert!(rooted_at("s->p", "s"));
    assert!(rooted_at("s.in.p", "s"));
    assert!(rooted_at("s[i]", "s"));
    assert!(rooted_at("*s", "s"));
    assert!(rooted_at("(*s).p", "s"));
    assert!(!rooted_at("*sp", "s"));
    assert!(!rooted_at("s", "s"));
    assert!(!rooted_at("sp->p", "s"));
    assert!(!rooted_at("t->p", "s"));
}
