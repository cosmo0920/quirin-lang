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

//! Card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16, decision
//! 3: "As shown above", "Migrate, one spelling", "Yes, all kinds"; contract
//! specification C09.3, grammar G05e/G05f, gateway rules 14t, 14x, 14z): one
//! member clause in three kinds, each loading to exactly the fact its retired
//! spelling did; the rejections and their diagnostics; the premise beside
//! every kind. The vectors are `QCore.NullMemberQualifier`'s
//! `qualifier_vectors_hold`.

use std::collections::BTreeMap;

use qcore_contract::bounds_contract::{
    BoundsClause, BoundsContracts, load_bounds_contract_sources,
};

const PREMISE: &str =
    "requires resource_premise(no_plugin, \"no code outside the run stores into S.next\");";

fn text(body: &str) -> String {
    format!(
        "module contract;\nextern fn f(s: ptr<mut, u8>, k: i32) -> unit\ncontract {{\n    requires bounds_contract_v1;\n    {body}\n}};\n"
    )
}

fn load(body: &str) -> Result<BoundsContracts, String> {
    load_bounds_contract_sources(&BTreeMap::from([("m.qv".to_owned(), text(body))]))
}

fn clauses(body: &str) -> Vec<BoundsClause> {
    load(body)
        .unwrap_or_else(|error| panic!("`{body}` must load: {error}"))
        .functions["f"]
        .clauses
        .iter()
        .map(|entry| entry.clause.clone())
        .collect()
}

fn rejection(body: &str) -> String {
    match load(body) {
        Ok(_) => panic!("`{body}` must reject the module"),
        Err(error) => error,
    }
}

#[test]
fn each_kind_loads_to_the_fact_of_its_retired_spelling() {
    assert_eq!(
        clauses("requires nonnull_member(S, next);"),
        vec![BoundsClause::NonnullMember {
            tag: "S".into(),
            field: "next".into(),
        }]
    );
    assert_eq!(
        clauses("requires nonnull_member(S, next) where quant #scope [[stable]];"),
        vec![BoundsClause::NonnullMemberStable {
            tag: "S".into(),
            field: "next".into(),
        }]
    );
    assert_eq!(
        clauses("requires nonnull_member(S, next, s) where quant #scope [[scope]];"),
        vec![BoundsClause::NonnullMemberScope {
            tag: "S".into(),
            field: "next".into(),
            parameter: "s".into(),
        }]
    );
    // The three kinds on one declaration are three facts; the bounds
    // consumer reads none of them.
    let all = load(
        "requires nonnull_member(S, next);\n    requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires nonnull_member(S, next, s) where quant #scope [[scope]];",
    )
    .expect("loads");
    assert_eq!(all.functions["f"].clauses.len(), 3);
    assert!(all.for_bounds().functions.is_empty());
    assert_eq!(all.null_declarations().count(), 1);
}

#[test]
fn the_clause_text_is_the_one_spelling() {
    let loaded = load(
        "requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires nonnull_member(S, next, s) where quant #scope [[scope]];",
    )
    .expect("loads");
    let texts = loaded.functions["f"]
        .clauses
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        texts,
        vec![
            "nonnull_member(S, next) where quant #scope [[stable]]",
            "nonnull_member(S, next, s) where quant #scope [[scope]]",
        ]
    );
    // The digest is over the text as written: spacing does not move it.
    let spaced = load(
        "requires nonnull_member( S , next )  where quant # scope [ [ stable ] ];\n    requires nonnull_member(S, next, s) where quant #scope [[scope]];",
    )
    .expect("loads");
    assert_eq!(
        spaced.functions["f"].text_sha256,
        loaded.functions["f"].text_sha256
    );
}

#[test]
fn the_retired_words_are_rejected_naming_the_new_form() {
    let stable = rejection("requires nonnull_member_stable(S, next);");
    assert!(
        stable.contains("nonnull-member-retired-spelling")
            && stable.contains("nonnull_member(TAG, FIELD) where quant #scope [[stable]]"),
        "{stable}"
    );
    let scope = rejection("requires nonnull_member_scope(S, next, s);");
    assert!(
        scope.contains("nonnull-member-retired-spelling")
            && scope.contains("nonnull_member(TAG, FIELD, P) where quant #scope [[scope]]"),
        "{scope}"
    );
    // No alias in any position: under `ensures`, with a qualifier, or held.
    for body in [
        "ensures normal nonnull_member_stable(S, next);",
        "requires nonnull_member_stable(S, next) where quant #scope [[stable]];",
        "requires bounds_review_pending(nonnull_member_scope(S, next, s), \"why\");",
    ] {
        assert!(
            rejection(body).contains("nonnull-member-retired-spelling"),
            "{body}"
        );
    }
}

#[test]
fn the_parameter_and_the_scope_qualifier_go_together() {
    for body in [
        "requires nonnull_member(S, next, s);",
        "requires nonnull_member(S, next, s) where quant #scope [[stable]];",
    ] {
        let error = rejection(body);
        assert!(
            error.contains("nonnull-member-parameter-needs-scope") && error.contains("[[scope]]"),
            "{body}: {error}"
        );
    }
    let error = rejection("requires nonnull_member(S, next) where quant #scope [[scope]];");
    assert!(
        error.contains("nonnull-member-scope-needs-parameter"),
        "{error}"
    );
    // P is a pointer parameter of the declaration.
    for body in [
        "requires nonnull_member(S, next, k) where quant #scope [[scope]];",
        "requires nonnull_member(S, next, q) where quant #scope [[scope]];",
    ] {
        assert!(
            rejection(body).contains("nonnull-member-scope-parameter"),
            "{body}"
        );
    }
}

#[test]
fn an_unknown_or_malformed_qualifier_is_rejected() {
    for word in ["global", "Stable", "scoped", "nullable"] {
        let error = rejection(&format!(
            "requires nonnull_member(S, next) where quant #scope [[{word}]];"
        ));
        assert!(
            error.contains("nonnull-member-qualifier-unknown") && error.contains(word),
            "{word}: {error}"
        );
    }
    for body in [
        "requires nonnull_member(S, next) where;",
        "requires nonnull_member(S, next) where quant #scope [stable];",
        "requires nonnull_member(S, next) where quant #other [[stable]];",
        "requires nonnull_member(S, next) where quant scope [[stable]];",
        "requires nonnull_member(S, next) where #scope [[stable]];",
        "requires nonnull_member(S, next) where quant #scope [[stable]] [[scope]];",
        "requires nonnull_member(S, next) where quant #scope [[stable, scope]];",
    ] {
        assert!(
            rejection(body).contains("nonnull-member-qualifier-shape"),
            "{body}"
        );
    }
}

#[test]
fn the_shape_and_position_are_checked() {
    for body in [
        // A parameter is no struct tag; the field is one word.
        "requires nonnull_member(s, next);",
        "requires nonnull_member(s, next) where quant #scope [[stable]];",
        "requires nonnull_member(S, next.a) where quant #scope [[stable]];",
        "requires nonnull_member(S);",
        "requires nonnull_member(S, next, s, k) where quant #scope [[scope]];",
    ] {
        assert!(rejection(body).contains("nonnull-member-shape"), "{body}");
    }
    for body in [
        "ensures normal nonnull_member(S, next);",
        "ensures normal nonnull_member(S, next) where quant #scope [[stable]];",
    ] {
        assert!(
            rejection(body).contains("nonnull-member-requires"),
            "{body}"
        );
    }
    // At most one per kind and member.
    assert!(
        load("requires nonnull_member(S, next) where quant #scope [[stable]];\n    requires nonnull_member(S, next) where quant #scope [[stable]];")
            .is_err()
    );
}

#[test]
fn a_premise_stands_beside_every_kind() {
    for kind in [
        "requires nonnull_member(S, next);",
        "requires nonnull_member(S, next) where quant #scope [[stable]];",
        "requires nonnull_member(S, next, s) where quant #scope [[scope]];",
    ] {
        let loaded = load(&format!("{kind}\n    {PREMISE}"))
            .unwrap_or_else(|error| panic!("{kind}: {error}"));
        assert_eq!(
            loaded.functions["f"].premises(),
            vec![("no_plugin", "no code outside the run stores into S.next")],
            "{kind}"
        );
    }
    for body in [
        PREMISE.to_owned(),
        format!("quant null-deref #s;\n    {PREMISE}"),
        format!("requires bounds_index(k, 4);\n    {PREMISE}"),
    ] {
        assert!(rejection(&body).contains("premise-beside-member"), "{body}");
    }
}
