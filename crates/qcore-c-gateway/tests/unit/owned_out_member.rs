// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Unit tests of rule 72's pure half: the decision table, the slot reader and
//! the caller-supplied roots, each bound to the vectors the Lean model
//! `QCore.OwnedOutMemberSlot` shares.

use std::collections::BTreeSet;

use super::*;

fn owner(code: &str) -> OwnerClass {
    match code {
        "untracked" => OwnerClass::Untracked,
        "tracked" => OwnerClass::Tracked,
        "unresolved" => OwnerClass::Unresolved,
        other => panic!("unknown owner {other}"),
    }
}

#[test]
fn decision_vectors() {
    for (code, decision) in DECISION_VECTORS {
        assert_eq!(decide(owner(code)).code(), *decision, "{code}");
    }
    // Only an untracked owner admits.
    for code in ["tracked", "unresolved"] {
        assert_ne!(decide(owner(code)), Decision::Admit);
    }
}

#[test]
fn slot_vectors() {
    for (actual, expected) in ROOT_VECTORS {
        let tokens = actual.split(' ').collect::<Vec<_>>();
        let read = read_slot(&tokens);
        match expected {
            None => assert_eq!(read, None, "{actual}"),
            Some((root, links, member)) => {
                let slot = read.unwrap_or_else(|| panic!("{actual} is a slot"));
                assert_eq!(slot.root, *root, "{actual}");
                assert_eq!(slot.links, *links, "{actual}");
                assert_eq!(slot.member, *member, "{actual}");
            }
        }
    }
    let slot = read_slot(&["&", "c", "->", "span", "->", "status", ".", "message"]).unwrap();
    assert_eq!(slot.place(), "c->span->status.message");
    let chain = slot.chain(true, false);
    assert!(chain.dereferenced_root && chain.root_never_bound);
    assert_eq!(chain.links, ["span"]);
}

fn roots(body: &str, formals: &[&str]) -> BTreeSet<String> {
    let tokens = body.split_whitespace().collect::<Vec<_>>();
    caller_roots(
        &tokens,
        &formals.iter().map(|name| (*name).to_owned()).collect(),
    )
}

#[test]
fn caller_roots_follow_casts_and_member_reads_of_unrebound_formals() {
    // The cmetrics shape.
    let found = roots(
        "struct cmt_map * map ; struct cmt_opts * opts ; map = ( struct cmt_map * ) context ; \
         opts = map -> opts ; return f ( reader , & opts -> ns ) ;",
        &["reader", "index", "context"],
    );
    assert!(found.contains("map") && found.contains("opts") && found.contains("context"));
    // The ctraces shape: a declarator's initialiser.
    let found = roots(
        "struct ctx * context = ctx ; return f ( reader , & context -> span -> name ) ;",
        &["reader", "index", "ctx"],
    );
    assert!(found.contains("context"));
}

#[test]
fn a_root_with_another_write_is_not_a_caller_root() {
    // A rebound formal.
    assert!(!roots("s = t ; f ( & s -> p ) ;", &["s", "t"]).contains("s"));
    // A local assigned an allocation, or also a caller root.
    assert!(!roots("o = malloc ( 8 ) ; f ( & o -> p ) ;", &["s"]).contains("o"));
    assert!(!roots("o = s ; o = malloc ( 8 ) ;", &["s"]).contains("o"));
    // An address of this frame's own object.
    assert!(!roots("struct v x ; p = & x ; f ( & p -> q ) ;", &["s"]).contains("p"));
    // An address-taken local, a compound write, a union cast.
    assert!(!roots("p = s ; g ( & p ) ;", &["s"]).contains("p"));
    assert!(!roots("p = s ; p += 1 ;", &["s"]).contains("p"));
    assert!(!roots("p = ( union u * ) s ;", &["s"]).contains("p"));
    // An address-taken formal.
    assert!(!roots("g ( & s ) ;", &["s"]).contains("s"));
}

#[test]
fn the_minted_contract_reads_only_a_neutral_slot_contract() {
    use qcore_contract::contract_loader::load_resource_contract_sources;
    let text = "module contract;
extern fn slot_rc(reader: ptr<mut, u8>, out: ptr<mut, ptr<mut, u8>>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(reader);
    ensures normal resource_on_return(eq, 0, resource_out(out, nonnull));
    ensures normal resource_on_return(ne, 0, resource_borrowed(out));
    ensures normal resource_return_none();
};
extern fn slot_plain(out: ptr<mut, ptr<mut, u8>>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_out(out, nullable);
    ensures normal resource_return_none();
};
extern fn slot_and_release(out: ptr<mut, ptr<mut, u8>>, p: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_out(out, nullable);
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};
extern fn two_slots(a: ptr<mut, ptr<mut, u8>>, b: ptr<mut, ptr<mut, u8>>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_out(a, nullable);
    ensures normal resource_out(b, nullable);
    ensures normal resource_return_none();
};
extern fn borrowed(p: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
";
    let contracts = load_resource_contract_sources(&std::collections::BTreeMap::from([(
        "t.qv".to_owned(),
        text.to_owned(),
    )]))
    .unwrap();
    let (minted, slot) = minted_contract(&contracts, "slot_rc").expect("a disposition slot");
    assert_eq!(slot, 1);
    assert!(
        minted
            .parameters
            .iter()
            .all(|mode| *mode == ResourceParameterMode::Borrowed)
    );
    assert_eq!(
        minted_contract(&contracts, "slot_plain").map(|(_, s)| s),
        Some(0)
    );
    assert!(minted_contract(&contracts, "slot_and_release").is_none());
    assert!(minted_contract(&contracts, "two_slots").is_none());
    assert!(minted_contract(&contracts, "borrowed").is_none());
    assert!(minted_contract(&contracts, "undeclared").is_none());
}
