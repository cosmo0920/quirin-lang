use std::collections::BTreeSet;

use super::*;

/// Split like the importer's own tokenizer: identifiers, numbers and one
/// character per punctuation token, so `->` is `-` followed by `>`.
fn texts(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
        } else if byte.is_ascii_alphanumeric() || byte == b'_' {
            let start = cursor;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(&source[start..cursor]);
        } else {
            tokens.push(&source[cursor..cursor + 1]);
            cursor += 1;
        }
    }
    tokens
}

const RECORDS: &str = "struct q { int x; }; struct r { int count; char *name; \
                       struct r *next; void *data; int a[4]; struct q inner; }; \
                       typedef struct t_s { int n; char *s; } t_t; typedef struct { int n; } anon; \
                       typedef t_t t_alias;";

fn fixture(parameters: &str, body: &str) -> (TypedAccessModel, FunctionTypes) {
    let model = TypedAccessModel::read(&texts(RECORDS));
    let types = FunctionTypes::read(&model, &texts(parameters), &texts(body), &BTreeSet::new());
    (model, types)
}

fn read(parameters: &str, body: &str, left: &str) -> Result<Destination, Refusal> {
    let (model, types) = fixture(parameters, body);
    let formals = ["p", "recs", "s", "h"]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let bases = caller_bases(&texts(body), &formals, &formals);
    let roots = RecordRoots::read(&model, &texts(parameters), &texts(body), &formals);
    read_destination(&texts(left), &model, &types, &roots, &bases)
}

const PARAMETERS: &str = "struct r *p, struct r *recs, int i, char *s";

#[test]
fn each_admitted_destination_shape_names_its_member_place() {
    let body = "struct r l; struct r arr [ 2 ]; t_t * tp; t_alias ta;";
    let cases = [
        ("arr [ i ] . count", None, "arr[].count", MemberKind::Scalar, false),
        ("tp -> s", Some("tp"), "tp->s", MemberKind::Pointer, false),
        ("ta . n", None, "ta.n", MemberKind::Scalar, false),
        ("p -> count", Some("p"), "p->count", MemberKind::Scalar, true),
        ("( p ) -> name", Some("p"), "p->name", MemberKind::Pointer, true),
        ("recs [ i ] . next", Some("recs"), "recs[].next", MemberKind::Pointer, true),
        ("( recs ) [ i ] . data", Some("recs"), "recs[].data", MemberKind::Pointer, true),
        ("l . count", None, "l.count", MemberKind::Scalar, false),
        ("( & l ) -> name", None, "l.name", MemberKind::Pointer, false),
        ("( l ) . count", None, "l.count", MemberKind::Scalar, false),
    ];
    for (left, base, place, member, visible) in cases {
        let destination = read(PARAMETERS, body, left).unwrap_or_else(|refusal| {
            panic!("{left} refused: {refusal}");
        });
        assert_eq!(
            destination,
            Destination {
                base: base.map(str::to_owned),
                place: place.to_owned(),
                member,
                caller_visible: visible,
            },
            "{left}"
        );
    }
}

#[test]
fn every_other_destination_keeps_its_refusal() {
    let body = "struct r l; void *v; struct r **pp; struct r arr[2]; anon *td;";
    let refused = [
        ("( * pp ) -> count", "prefix"),
        ("( ( struct r * ) v ) -> count", "prefix"),
        ("v -> count", "root-untyped"),
        ("undeclared -> count", "root-untyped"),
        ("p -> next -> count", "chain"),
        ("p -> inner . x", "chain"),
        ("p -> a", "member"),
        ("p -> inner", "member"),
        ("p -> missing", "member"),
        ("p . count", "selector-mismatch"),
        ("l -> count", "selector-mismatch"),
        ("( & p ) -> count", "address-of-pointer"),
        ("l [ i ] . count", "subscript-of-object"),
        ("( & arr ) -> count", "address-of-pointer"),
        ("arr . count", "selector-mismatch"),
        ("td -> count", "root-untyped"),
        ("recs [ i ++ ] . count", "subscript-index"),
        ("recs [ i ] -> count", "selector-mismatch"),
        ("p -> count +", "chain"),
    ];
    for (left, reason) in refused {
        assert_eq!(read(PARAMETERS, body, left), Err(reason), "{left}");
    }
}

#[test]
fn a_rebound_or_address_taken_formal_is_not_the_callers_record() {
    for body in [
        "p = 0;",
        "p ++ ;",
        "p += 1 ;",
        "q = & p ;",
        "-- p ;",
    ] {
        let formals = BTreeSet::from(["p".to_owned()]);
        assert!(
            caller_bases(&texts(body), &formals, &formals).is_empty(),
            "{body}"
        );
    }
    for body in ["x = p ;", "if ( p == q ) { }", "f ( p ) ;", "q -> p = 0 ;", "s . p = 0 ;"] {
        let formals = BTreeSet::from(["p".to_owned()]);
        assert_eq!(
            caller_bases(&texts(body), &formals, &formals),
            formals,
            "{body}"
        );
    }
    // A formal the frame does not protect (its address is taken somewhere)
    // is never a caller base.
    let formals = BTreeSet::from(["p".to_owned()]);
    assert!(caller_bases(&[], &formals, &BTreeSet::new()).is_empty());
    // A local is never one.
    assert_eq!(
        read(PARAMETERS, "struct r * m ;", "m -> count").map(|d| d.caller_visible),
        Ok(false)
    );
}

#[test]
fn values_are_read_by_the_members_kind() {
    let (model, types) = fixture(PARAMETERS, "struct r l; int n; char *t;");
    let scalar = |right: &str| read_value(&texts(right), MemberKind::Scalar, &model, &types);
    let pointer = |right: &str| read_value(&texts(right), MemberKind::Pointer, &model, &types);
    assert_eq!(scalar("n + 1"), Ok(ValueSyntax::Scalar(Vec::new())));
    assert_eq!(scalar("p -> count"), Ok(ValueSyntax::Scalar(vec!["p".to_owned()])));
    assert_eq!(scalar("f ( n )"), Err("value-not-scalar"));
    assert_eq!(scalar("s"), Err("value-not-scalar"));
    assert_eq!(pointer("( ( void * ) 0 )"), Ok(ValueSyntax::Untracked));
    assert_eq!(pointer("0"), Ok(ValueSyntax::Untracked));
    assert_eq!(pointer("__qcore_literal__"), Ok(ValueSyntax::Untracked));
    assert_eq!(pointer("t"), Ok(ValueSyntax::Name("t".to_owned())));
    assert_eq!(pointer("( char * ) s"), Ok(ValueSyntax::Name("s".to_owned())));
    assert_eq!(pointer("n"), Err("value-scalar-into-pointer"));
    assert_eq!(pointer("undeclared"), Err("value-undeclared"));
    assert_eq!(pointer("g ( s )"), Ok(ValueSyntax::Call(4)));
    assert_eq!(pointer("( char * ) g ( s )"), Ok(ValueSyntax::Call(4)));
    assert_eq!(pointer("g ( s ) + 1"), Err("value-shape"));
    assert_eq!(pointer("p -> next"), Err("value-shape"));
    assert_eq!(pointer("& l"), Err("value-shape"));
}
