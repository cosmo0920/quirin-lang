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

//! Card W48-OPENSSL-CONTRACT-PLUGIN: every class rule over Clang's own AST of
//! small C definitions — each admitted shape, and each refused one (a call,
//! an assignment, an increment, a loop, a static local, a global read, a
//! store other than the contract's, an escape the contract omits).

mod support;

use quirin_audit_plugin_openssl::ast;
use quirin_audit_plugin_openssl::checker;
use quirin_audit_plugin_openssl::contracts::Store;
use serde_json::Value;

const SOURCE: &str = r#"
typedef struct ctx_st { int error; int depth; void *user; struct ctx_st *next; unsigned long options; int (*cb)(void); } CTX;
typedef struct ctx_st *CTX_PTR;
static const CTX the_table = { 1, 2, 0, 0, 0, 0 };
static CTX mutable_table;
int global_counter;
int helper(void);

void clear_all(void) { helper(); }
unsigned long peek(void) { return (unsigned long)helper(); }
int with_parameter(int x) { return x; }
CTX_PTR hidden_pointer(void) { return 0; }

const CTX *table(void) { return &the_table; }
CTX *mutable_object(void) { return &mutable_table; }
const CTX *table_with_work(void) { helper(); return &the_table; }

int get_error(const CTX *c) { return c->error; }
void *get_user(const CTX *c) { if (c == ((void *)0) || c->depth != 2) return ((void *)0); return c->user; }
int get_with_local(const CTX *c) { const CTX *n = c->next; return n == 0 ? -1 : n->depth; }
int get_with_call(const CTX *c) { return c->error + helper(); }
int get_with_assignment(CTX *c) { c->error = 0; return 1; }
int get_with_increment(CTX *c) { return c->depth++; }
int get_with_loop(const CTX *c) { int n = 0; while (c) { c = c->next; } return n; }
int get_with_static(const CTX *c) { static int seen; return seen + c->error; }
int get_global(const CTX *c) { return global_counter + c->error; }

void set_error(CTX *c, int e) { c->error = e; }
void set_user(CTX *c, void *u) { c->user = u; }
unsigned long set_options(CTX *c, unsigned long op) { return c->options |= op; }
void set_two(CTX *c, int e, void *u) { c->error = e; c->user = u; }
void set_from_call(CTX *c) { c->error = helper(); }
void set_and_call(CTX *c, int e) { c->error = e; helper(); }
void set_global(CTX *c) { global_counter = c->error; }
"#;

fn dump(name: &str) -> Vec<Value> {
    let directory = support::scratch("classes");
    std::fs::write(directory.join("classes.c"), SOURCE).unwrap();
    ast::named_declarations(&support::clang(), &directory, &[], "classes.c", name).unwrap()
}

fn definition(name: &str) -> Value {
    let nodes = dump(name);
    let definitions: Vec<&Value> = nodes
        .iter()
        .filter(|node| ast::kind(node) == "FunctionDecl" && ast::body(node).is_some())
        .collect();
    assert_eq!(definitions.len(), 1, "{name}");
    definitions[0].clone()
}

#[test]
fn no_pointer_interface_admits_only_parameterless_builtin_signatures() {
    assert!(checker::no_pointer_interface(&definition("clear_all")).is_ok());
    assert!(checker::no_pointer_interface(&definition("peek")).is_ok());
    // A parameter, or a typedef hiding a pointer result, is refused.
    assert!(checker::no_pointer_interface(&definition("with_parameter")).is_err());
    assert!(checker::no_pointer_interface(&definition("hidden_pointer")).is_err());
}

#[test]
fn static_object_return_admits_only_the_named_static_const_object() {
    let objects = dump("the_table");
    let ok = checker::static_object_return(&definition("table"), "the_table", &objects, "classes.c");
    assert!(ok.is_ok(), "{ok:?}");
    // Another name, a non-const object, another file, or more work is refused.
    assert!(
        checker::static_object_return(&definition("table"), "other", &objects, "classes.c")
            .is_err()
    );
    assert!(
        checker::static_object_return(
            &definition("mutable_object"),
            "mutable_table",
            &dump("mutable_table"),
            "classes.c"
        )
        .is_err()
    );
    assert!(
        checker::static_object_return(&definition("table"), "the_table", &objects, "other.c")
            .is_err()
    );
    assert!(
        checker::static_object_return(
            &definition("table_with_work"),
            "the_table",
            &objects,
            "classes.c"
        )
        .is_err()
    );
}

#[test]
fn pure_reader_admits_reads_and_refuses_every_effect() {
    for name in ["get_error", "get_user", "get_with_local"] {
        let result = checker::pure_reader(&definition(name));
        assert!(result.is_ok(), "{name}: {result:?}");
    }
    for (name, refused) in [
        ("get_with_call", "CallExpr"),
        ("get_with_assignment", "`=`"),
        ("get_with_increment", "`++`"),
        ("get_with_loop", "WhileStmt"),
        ("get_with_static", "VarDecl"),
        ("get_global", "global_counter"),
    ] {
        let error = checker::pure_reader(&definition(name)).unwrap_err();
        assert!(error.contains(refused), "{name}: {error}");
    }
}

fn store(member: &str, operator: &str, parameters: &[usize]) -> Store {
    Store {
        base: 0,
        member: member.to_owned(),
        operator: operator.to_owned(),
        parameters: parameters.to_vec(),
    }
}

#[test]
fn member_store_requires_exactly_the_contracts_stores_and_escapes() {
    let ok = checker::member_store(&definition("set_error"), &[store("error", "=", &[1])], &[]);
    assert!(ok.is_ok(), "{ok:?}");
    let ok = checker::member_store(&definition("set_user"), &[store("user", "=", &[1])], &[1]);
    assert!(ok.is_ok(), "{ok:?}");
    let ok = checker::member_store(
        &definition("set_options"),
        &[store("options", "|=", &[1])],
        &[],
    );
    assert!(ok.is_ok(), "{ok:?}");
    // The escape of `u` cannot be left out of the contract.
    assert!(
        checker::member_store(&definition("set_user"), &[store("user", "=", &[1])], &[])
            .unwrap_err()
            .contains("escaping")
    );
    // A store the contract does not list, or another member, is refused.
    assert!(
        checker::member_store(&definition("set_two"), &[store("error", "=", &[1])], &[])
            .is_err()
    );
    assert!(
        checker::member_store(&definition("set_error"), &[store("depth", "=", &[1])], &[])
            .is_err()
    );
    // A stored call, a further statement, or a non-member target is refused.
    assert!(checker::stores(&definition("set_from_call")).is_err());
    assert!(checker::stores(&definition("set_and_call")).is_err());
    assert!(checker::stores(&definition("set_global")).is_err());
}
