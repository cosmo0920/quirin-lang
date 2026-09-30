//! Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the decision table and its Lean
//! binding.

use super::*;

fn binding(code: &str) -> BindingClass {
    match code {
        "absent" => BindingClass::Absent,
        "target-free" => BindingClass::TargetFree,
        "nullable" => BindingClass::Nullable,
        "unresolved" => BindingClass::Unresolved,
        "tracked" => BindingClass::Tracked,
        other => panic!("unknown binding class {other}"),
    }
}

#[test]
fn the_decision_vectors_hold() {
    for &(eligible, class, rooted, refused, expected) in DECISION_VECTORS {
        assert_eq!(
            decide(eligible, binding(class), rooted, refused).code(),
            expected,
            "{eligible} {class} {rooted} {refused}"
        );
    }
}

#[test]
fn only_an_untracked_binding_on_an_unrefused_path_holds() {
    for eligible in [false, true] {
        for class in ["absent", "target-free", "nullable", "unresolved", "tracked"] {
            for rooted in [false, true] {
                for refused in [false, true] {
                    let holds =
                        decide(eligible, binding(class), rooted, refused) == Premise::Holds;
                    assert_eq!(
                        holds,
                        eligible
                            && matches!(class, "absent" | "target-free")
                            && !rooted
                            && !refused
                    );
                }
            }
        }
    }
}

#[test]
fn a_place_rooted_at_the_root_is_recognised() {
    for place in ["r->f", "r.f", "r[0]", "*r", "(*r).f", "r->f->g"] {
        assert!(rooted_at(place, "r"), "{place}");
    }
    for place in ["r", "rk->f", "q->r", "*rk"] {
        assert!(!rooted_at(place, "r"), "{place}");
    }
}
