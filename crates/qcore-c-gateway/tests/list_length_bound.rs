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

//! Card W45-FB-LIST-FINITENESS-PREMISE (gateway rule 48): the list-length
//! bound driven through Clang. The prelude reproduces Monkey's list header
//! (`mk_list.h`: the traversal macros and the inline primitives), so a fixture
//! reads as the slice's loops do. Each refusal the rule names has a fixture.

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::list_length_bound::ListBound;
use qcore_c_gateway::loop_measure::{
    ListSelection, LoopFact, analyze_loop_measures_c11, analyze_loop_measures_c11_with,
};
use qcore_contract::list_finiteness::parse_list_finiteness_premises;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent rule-48 test: {reason}");
            false
        }
    }
}

const PRELUDE: &str = r#"
struct mk_list { struct mk_list *prev, *next; };
#define mk_list_foreach(curr, head) for( curr = (head)->next; curr != (head); curr = curr->next )
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
#define mk_list_foreach_r(curr, head) for( curr = (head)->prev; curr != (head); curr = curr->prev )
#define container_of(ptr, type, member) ({ \
    const __typeof__( ((type *)0)->member ) *__mptr = (ptr); \
    (type *)( (char *)__mptr - __builtin_offsetof(type,member) );})
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
static inline void __mk_list_add(struct mk_list *_new, struct mk_list *prev,
                                 struct mk_list *next)
{
    next->prev = _new;
    _new->next = next;
    _new->prev = prev;
    prev->next = _new;
}
static inline void mk_list_add(struct mk_list *_new, struct mk_list *head)
{
    __mk_list_add(_new, head->prev, head);
}
static inline void mk_list_del(struct mk_list *entry)
{
    entry->prev->next = entry->next;
    entry->next->prev = entry->prev;
    entry->prev = 0;
    entry->next = 0;
}
static inline int mk_list_size(struct mk_list *head)
{
    int ret = 0;
    struct mk_list *it;
    for (it = head->next; it != head; it = it->next, ret++);
    return ret;
}
struct instance { int id; char name[16]; struct mk_list tasks; struct mk_list _head; };
struct config { int count; struct mk_list inputs; struct mk_list outputs; };
int strcmp(const char *, const char *);
void flb_log_print(int, const char *);
void flb_free(void *);
"#;

const DECLARED: &str = "quant list config.inputs |-> #finite;\n\
quant list config.outputs |-> #finite;\n\
quant list instance.tasks |-> #finite;\n";

fn loops_with(source: &str, name: &str, premises: Option<&ListSelection>) -> Vec<LoopFact> {
    let text = format!("{PRELUDE}{source}");
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    let report = analyze_loop_measures_c11_with(&unit, &[name.to_owned()], premises)
        .unwrap_or_else(|error| panic!("{source}: {error}"));
    report
        .function(name)
        .unwrap_or_else(|| panic!("no `{name}`"))
        .loops
        .clone()
}

fn declared() -> ListSelection {
    ListSelection::Declared(parse_list_finiteness_premises(DECLARED).unwrap())
}

fn outcomes(source: &str) -> Vec<Result<ListBound, (&'static str, String)>> {
    loops_with(source, "subject", Some(&declared()))
        .into_iter()
        .map(|fact| {
            let list = fact.list_bound.expect("a list outcome on every loop");
            match list.bound {
                Some(bound) => Ok(bound),
                None => Err((list.refusal.unwrap(), list.detail.unwrap_or_default())),
            }
        })
        .collect()
}

fn bound(source: &str) -> ListBound {
    let mut all = outcomes(source);
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0)
        .unwrap_or_else(|refusal| panic!("refused: {refusal:?}"))
}

fn refusal(source: &str) -> (&'static str, String) {
    let mut all = outcomes(source);
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0).expect_err("a refusal")
}

/// `flb_output_get_instance`: a read-only traversal of a declared head.
#[test]
fn a_read_only_traversal_of_a_declared_head_is_bounded() {
    if !clang_available() {
        return;
    }
    let found = bound(
        "struct instance *subject(struct config *config, int id) {\n\
           struct mk_list *head; struct instance *ins;\n\
           mk_list_foreach(head, &config->outputs) {\n\
             ins = mk_list_entry(head, struct instance, _head);\n\
             if (ins->id == id) return ins;\n\
           }\n\
           return 0; }\n",
    );
    assert_eq!(found.list, "config.outputs");
    assert_eq!(found.head, "&config->outputs");
    assert_eq!(found.cursor, "head");
    assert_eq!(found.traversal, "plain");
    assert_eq!(found.direction, "next");
    assert_eq!(found.bound, "|config->outputs| at loop entry");
    assert_eq!(found.premise, "quant list config.outputs |-> #finite");
    assert!(found.frame_functions.is_empty(), "{found:?}");
}

/// `flb_task_running_count`: nested traversals, the inner one over a root the
/// outer body rebinds (stable in the inner loop), with a read-only inline
/// callee and a library call.
#[test]
fn nested_traversals_with_a_read_only_callee_are_bounded() {
    if !clang_available() {
        return;
    }
    let all = outcomes(
        "int subject(struct config *config) {\n\
           int total = 0; struct mk_list *head, *t; struct instance *ins;\n\
           mk_list_foreach(head, &config->inputs) {\n\
             ins = mk_list_entry(head, struct instance, _head);\n\
             if (strcmp(ins->name, \"x\") == 0) continue;\n\
             mk_list_foreach(t, &ins->tasks) { total += mk_list_size(&ins->tasks); }\n\
           }\n\
           return total; }\n",
    );
    assert_eq!(all.len(), 2, "{all:?}");
    let outer = all[0].as_ref().unwrap();
    assert_eq!(outer.list, "config.inputs");
    assert_eq!(outer.frame_functions, ["mk_list_size"]);
    assert_eq!(outer.frame_library, ["strcmp"]);
    let inner = all[1].as_ref().unwrap();
    assert_eq!(inner.list, "instance.tasks");
    assert_eq!(inner.head, "&ins->tasks");
}

/// `mk_list_foreach_safe` and the reverse traversal are the same family.
#[test]
fn the_safe_and_reverse_forms_are_bounded() {
    if !clang_available() {
        return;
    }
    let safe = bound(
        "int subject(struct config *config) {\n\
           struct mk_list *head, *tmp; int n = 0;\n\
           mk_list_foreach_safe(head, tmp, &config->outputs) { n++; }\n\
           return n; }\n",
    );
    assert_eq!(safe.traversal, "safe");
    let reverse = bound(
        "int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach_r(head, &config->outputs) { n++; }\n\
           return n; }\n",
    );
    assert_eq!(reverse.direction, "prev");
}

#[test]
fn an_undeclared_head_is_refused() {
    if !clang_available() {
        return;
    }
    let (code, detail) = refusal(
        "int subject(struct instance *ins) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &ins->_head) { n++; }\n\
           return n; }\n",
    );
    assert_eq!(code, "premise-not-declared");
    assert!(detail.contains("instance._head"), "{detail}");
}

/// A rejected premise file declares nothing.
#[test]
fn a_rejected_premise_file_declares_nothing() {
    if !clang_available() {
        return;
    }
    let rejected = ListSelection::Rejected("line 1: not a declaration".into());
    let facts = loops_with(
        "int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) { n++; }\n\
           return n; }\n",
        "subject",
        Some(&rejected),
    );
    let list = facts[0].list_bound.clone().unwrap();
    assert_eq!(list.refusal, Some("premise-not-declared"));
    assert!(list.detail.unwrap().contains("rejected"));
}

/// Without the selection the rows are rule 44's, byte for byte.
#[test]
fn without_premises_the_rows_are_rule_44s() {
    if !clang_available() {
        return;
    }
    let source = "int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) { n++; }\n\
           for (int i = 0; i < 4; i++) n++;\n\
           return n; }\n";
    let text = format!("{PRELUDE}{source}");
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    let plain = analyze_loop_measures_c11(&unit, &["subject".to_owned()]).unwrap();
    let with = analyze_loop_measures_c11_with(&unit, &["subject".to_owned()], None).unwrap();
    assert_eq!(
        serde_json::to_string(&plain).unwrap(),
        serde_json::to_string(&with).unwrap()
    );
    assert!(
        !serde_json::to_string(&plain)
            .unwrap()
            .contains("list_bound")
    );
    // With the selection, rule 44's own fields are unchanged.
    let selected =
        analyze_loop_measures_c11_with(&unit, &["subject".to_owned()], Some(&declared())).unwrap();
    for (a, b) in plain.functions[0]
        .loops
        .iter()
        .zip(&selected.functions[0].loops)
    {
        assert_eq!(
            (a.line, a.kind, &a.measure, a.refusal),
            (b.line, b.kind, &b.measure, b.refusal)
        );
    }
    assert!(
        selected.functions[0].loops[1]
            .list_bound
            .as_ref()
            .unwrap()
            .refusal
            == Some("not-a-list-traversal")
    );
}

/// `sb_allocate_backlogs`: an insertion in the body writes links.
#[test]
fn an_insertion_in_the_frame_is_refused() {
    if !clang_available() {
        return;
    }
    let (code, detail) = refusal(
        "void subject(struct config *config, struct instance *fresh) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { mk_list_add(&fresh->_head, &config->inputs); }\n\
         }\n",
    );
    assert_eq!(code, "frame-writes-links");
    assert!(
        detail.contains("__mk_list_add") || detail.contains("mk_list_add"),
        "{detail}"
    );
}

/// An unlink in the body, and a direct link write.
#[test]
fn an_unlink_or_a_link_write_is_refused() {
    if !clang_available() {
        return;
    }
    let (code, _) = refusal(
        "void subject(struct config *config) {\n\
           struct mk_list *head, *tmp;\n\
           mk_list_foreach_safe(head, tmp, &config->outputs) { mk_list_del(head); }\n\
         }\n",
    );
    assert_eq!(code, "frame-writes-links");
    let (code, detail) = refusal(
        "void subject(struct config *config, struct instance *e) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { e->_head.next = head; }\n\
         }\n",
    );
    assert_eq!(code, "frame-writes-links");
    assert!(detail.contains("list pointer"), "{detail}");
    // A character write may alias a link.
    let (code, _) = refusal(
        "void subject(struct config *config, struct instance *e) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { e->name[0] = 0; }\n\
         }\n",
    );
    assert_eq!(code, "frame-writes-links");
    // A record assignment may overwrite a node.
    let (code, _) = refusal(
        "void subject(struct config *config, struct instance *e, struct instance *f) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { *e = *f; }\n\
         }\n",
    );
    assert_eq!(code, "frame-writes-links");
}

/// `flb_log_print` is defined in another unit.
#[test]
fn an_undefined_callee_is_refused() {
    if !clang_available() {
        return;
    }
    let (code, detail) = refusal(
        "void subject(struct config *config) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { flb_log_print(1, \"x\"); }\n\
         }\n",
    );
    assert_eq!(code, "frame-callee-undefined");
    assert!(detail.contains("flb_log_print"), "{detail}");
    // Transitively, through a callee the unit defines.
    let (code, detail) = refusal(
        "static void report(void) { flb_free(0); }\n\
         void subject(struct config *config) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { report(); }\n\
         }\n",
    );
    assert_eq!(code, "frame-callee-undefined");
    assert!(detail.contains("flb_free"), "{detail}");
}

/// A callee defined after its caller still establishes the frame.
#[test]
fn a_callee_defined_later_is_read() {
    if !clang_available() {
        return;
    }
    let found = bound(
        "static int weight(struct instance *e);\n\
         int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) {\n\
             n += weight(mk_list_entry(head, struct instance, _head)); }\n\
           return n; }\n\
         static int weight(struct instance *e) { int w = e->id; w++; return w; }\n",
    );
    assert_eq!(found.frame_functions, ["weight"]);
}

#[test]
fn an_indirect_call_is_refused() {
    if !clang_available() {
        return;
    }
    let (code, _) = refusal(
        "void subject(struct config *config, void (*cb)(void)) {\n\
           struct mk_list *head;\n\
           mk_list_foreach(head, &config->outputs) { cb(); }\n\
         }\n",
    );
    assert_eq!(code, "frame-indirect-call");
}

#[test]
fn shape_refusals_are_named() {
    if !clang_available() {
        return;
    }
    // A formal list pointer is not a declared member.
    let (code, _) = refusal(
        "int subject(struct mk_list *tasks) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, tasks) { n++; }\n\
           return n; }\n",
    );
    assert_eq!(code, "head-not-member");
    // The root rebound in the body.
    let (code, _) = refusal(
        "int subject(struct config *config, struct config *other) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) { config = other; }\n\
           return n; }\n",
    );
    assert_eq!(code, "head-root-not-stable");
    // The cursor written in the body.
    let (code, _) = refusal(
        "int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) { head = head->next; }\n\
           return n; }\n",
    );
    assert_eq!(code, "cursor-written");
    // A while loop over a list is not of the family.
    let (code, _) = refusal(
        "int subject(struct config *config) {\n\
           struct mk_list *head = config->outputs.next; int n = 0;\n\
           while (head != &config->outputs) { head = head->next; n++; }\n\
           return n; }\n",
    );
    assert_eq!(code, "not-a-list-traversal");
    // A label in the loop.
    let (code, _) = refusal(
        "int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) { again: n++; if (n < 2) goto again; }\n\
           return n; }\n",
    );
    assert_eq!(code, "label-in-loop");
    // Inline assembly in the frame.
    let (code, _) = refusal(
        "static void fence(void) { __asm__ volatile(\"\" ::: \"memory\"); }\n\
         int subject(struct config *config) {\n\
           struct mk_list *head; int n = 0;\n\
           mk_list_foreach(head, &config->outputs) { fence(); }\n\
           return n; }\n",
    );
    assert_eq!(code, "opaque-control");
}
