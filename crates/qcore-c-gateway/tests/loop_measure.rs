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

//! Card W45-FB-ENGINE-PROGRESS-MEASURE (gateway rule 44): the loop-measure
//! fact driven through Clang. The accepting fixtures reproduce the slice's
//! three shapes (`handle_output_events` :909, `flb_input_chunk_get_direct_routes`
//! :2286, `flb_task_create` :652); each refusal shape the rule names has its
//! own fixture. Every source is self-contained, so no target header is needed.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, map_preprocessed_lines, validate_unrestricted_c11,
};
use qcore_c_gateway::loop_measure::{
    BoundKind, FunctionMeasures, LoopFact, analyze_loop_measures_c11,
    analyze_loop_measures_preprocessed,
};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent rule-44 test: {reason}");
            false
        }
    }
}

// Target-neutral widths: Clang's own `__SIZE_TYPE__`/`__UINT64_TYPE__` are
// `unsigned long` on LP64 and `unsigned long long` on LLP64 (Windows), so the
// prelude never redefines a builtin type differently from the target.
const PRELUDE: &str = "typedef __SIZE_TYPE__ size_t;\n\
typedef unsigned short uint16_t;\n\
typedef unsigned char uint8_t;\n\
typedef __UINT64_TYPE__ uint64_t;\n\
int use(int);\n\
int take(int *);\n";

fn function(source: &str, name: &str) -> FunctionMeasures {
    let text = format!("{PRELUDE}{source}");
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    let report = analyze_loop_measures_c11(&unit, &[name.to_owned()])
        .unwrap_or_else(|error| panic!("{source}: {error}"));
    assert_eq!(report.functions.len(), 1, "only the selected function");
    report
        .function(name)
        .unwrap_or_else(|| panic!("no `{name}` in {source}"))
        .clone()
}

fn only_loop(source: &str) -> LoopFact {
    let function = function(source, "subject");
    assert_eq!(function.loops.len(), 1, "{function:?}");
    function.loops[0].clone()
}

fn refusal(source: &str) -> &'static str {
    let fact = only_loop(source);
    assert!(fact.measure.is_none(), "{fact:?}");
    fact.refusal.expect("a named refusal")
}

fn measured(source: &str) -> qcore_c_gateway::loop_measure::Measure {
    let fact = only_loop(source);
    assert!(fact.refusal.is_none(), "{fact:?}");
    fact.measure.expect("a measure")
}

#[test]
fn the_output_event_batch_is_bounded_by_its_array_count() {
    if !clang_available() {
        return;
    }
    // handle_output_events, flb_engine.c:909: the macro constant is the
    // array's extent in Clang's type.
    let measure = measured(
        "#define FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE 1\n\
         int subject(size_t limit) {\n\
             uint64_t values[FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE];\n\
             size_t index;\n\
             int result = 0;\n\
             for (index = 0 ;\n\
                  index < limit &&\n\
                  index < (sizeof(values) / sizeof(values[0])) ;\n\
                  index++) {\n\
                 if (values[index] == 0) {\n\
                     break;\n\
                 }\n\
                 result = use((int) values[index]);\n\
             }\n\
             return result;\n\
         }\n",
    );
    assert_eq!(measure.bound_kind, BoundKind::Constant);
    assert_eq!(measure.bound, "1");
    assert_eq!(measure.max_iterations, 1);
    assert_eq!(measure.init, 0);
    assert_eq!(measure.counter, "index");
    assert_eq!(measure.measure, "1 - index");
    // `limit` is a stable formal too: two conjuncts admitted, the smaller kept.
    assert_eq!(measure.admitted_conjuncts, 2);
}

#[test]
fn the_stored_route_count_is_a_stable_uint16_local() {
    if !clang_available() {
        return;
    }
    // flb_input_chunk_get_direct_routes, flb_input_chunk.c:2286.
    let measure = measured(
        "int subject(const unsigned char *data, int *out) {\n\
             int index;\n\
             uint16_t stored_count;\n\
             index = 0;\n\
             stored_count = 0;\n\
             stored_count = (uint16_t) ((data[0] << 8) | data[1]);\n\
             if (stored_count == 0) {\n\
                 return 0;\n\
             }\n\
             for (index = 0; index < stored_count; index++) {\n\
                 out[index] = use(index);\n\
             }\n\
             return index;\n\
         }\n",
    );
    assert_eq!(measure.bound_kind, BoundKind::StableLocal);
    assert_eq!(measure.bound, "stored_count");
    assert_eq!(measure.max_iterations, 65_535);
    assert_eq!(measure.measure, "stored_count - index");
}

#[test]
fn an_address_taken_int_bound_is_refused() {
    if !clang_available() {
        return;
    }
    // flb_task_create, flb_task.c:652: `direct_output_count`'s address is
    // passed to a callee before the loop, and the body calls others.
    assert_eq!(
        refusal(
            "int subject(void) {\n\
                 int direct_output_count = 0;\n\
                 int direct_output_index;\n\
                 if (take(&direct_output_count) == 0 && direct_output_count > 0) {\n\
                     for (direct_output_index = 0;\n\
                          direct_output_index < direct_output_count;\n\
                          direct_output_index++) {\n\
                         if (use(direct_output_index) != 0) {\n\
                             break;\n\
                         }\n\
                     }\n\
                 }\n\
                 return 0;\n\
             }\n"
        ),
        "bound-address-taken"
    );
}

#[test]
fn an_address_taken_narrow_bound_is_bounded_by_its_type() {
    if !clang_available() {
        return;
    }
    let measure = measured(
        "int take16(uint16_t *);\n\
         int subject(void) {\n\
             uint16_t count = 0;\n\
             int i;\n\
             take16(&count);\n\
             for (i = 0; i < count; i++) { use(i); }\n\
             return 0;\n\
         }\n",
    );
    assert_eq!(measure.bound_kind, BoundKind::TypeRange);
    assert_eq!(measure.max_iterations, 65_535);
}

#[test]
fn constants_through_clang() {
    if !clang_available() {
        return;
    }
    let macro_bound = measured(
        "#define N 10\nint subject(void) { int s = 0; for (int i = 2; i < N; i++) s += use(i); return s; }\n",
    );
    assert_eq!(
        (macro_bound.bound_kind, macro_bound.max_iterations),
        (BoundKind::Constant, 8)
    );
    assert_eq!(macro_bound.init, 2);
    let enumerator = measured(
        "enum { PASSES = 2 };\nint subject(void) { int p; for (p = 0; p < PASSES; ++p) use(p); return 0; }\n",
    );
    assert_eq!(
        (enumerator.bound.as_str(), enumerator.max_iterations),
        ("2", 2)
    );
    // c0 >= B: the measure is 0.
    let empty =
        measured("int subject(void) { int i; for (i = 5; i < 3; i++) use(i); return 0; }\n");
    assert_eq!(empty.max_iterations, 0);
}

#[test]
fn a_narrow_field_bounds_by_its_type() {
    if !clang_available() {
        return;
    }
    let measure = measured(
        "struct view { uint16_t count; };\n\
         int subject(struct view *v) { int i; for (i = 0; i < v->count; i++) use(i); return 0; }\n",
    );
    assert_eq!(measure.bound_kind, BoundKind::TypeRange);
    assert_eq!(measure.max_iterations, 65_535);
}

#[test]
fn early_exits_and_continue_are_admitted() {
    if !clang_available() {
        return;
    }
    let measure = measured(
        "int subject(int n) {\n\
             int i;\n\
             for (i = 0; i < n; i++) {\n\
                 if (use(i) == 1) continue;\n\
                 if (use(i) == 2) break;\n\
                 if (use(i) == 3) goto out;\n\
                 if (use(i) == 4) return 1;\n\
             }\n\
         out:\n\
             return 0;\n\
         }\n",
    );
    assert_eq!(measure.bound_kind, BoundKind::StableLocal);
    assert_eq!(measure.max_iterations, 2_147_483_647);
}

#[test]
fn a_wrapping_counter_is_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        refusal(
            "int subject(void) { unsigned char i; for (i = 0; i < 300; i++) use(i); return 0; }\n"
        ),
        "counter-may-wrap"
    );
    assert_eq!(
        refusal(
            "int subject(uint16_t n) { uint8_t i; for (i = 0; i < n; i++) use(i); return 0; }\n"
        ),
        "counter-may-wrap"
    );
}

#[test]
fn a_written_bound_is_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        refusal("int subject(int n) { int i; for (i = 0; i < n; i++) n = use(n); return 0; }\n"),
        "bound-written-in-loop"
    );
}

#[test]
fn an_address_taken_counter_is_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        refusal(
            "int subject(void) { int i; take(&i); for (i = 0; i < 10; i++) use(i); return 0; }\n"
        ),
        "counter-address-taken"
    );
}

#[test]
fn a_double_increment_is_refused() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        refusal(
            "int subject(void) { int i; for (i = 0; i < 10; i++) { use(i); i++; } return 0; }\n"
        ),
        "counter-written-in-loop"
    );
    assert_eq!(
        refusal("int subject(void) { int i; for (i = 0; i < 10; i += 2) use(i); return 0; }\n"),
        "step-not-increment"
    );
}

#[test]
fn a_non_less_than_test_is_refused() {
    if !clang_available() {
        return;
    }
    for test in ["i <= 10", "i != 10", "10 > i", "use(i)"] {
        assert_eq!(
            refusal(&format!(
                "int subject(void) {{ int i; for (i = 0; {test}; i++) use(i); return 0; }}\n"
            )),
            "test-not-less-than",
            "{test}"
        );
    }
}

#[test]
fn the_other_refusal_shapes() {
    if !clang_available() {
        return;
    }
    let cases = [
        // A converting comparison: `int` against `size_t`.
        (
            "int subject(size_t n) { int i; for (i = 0; i < n; i++) use(i); return 0; }\n",
            "comparison-converts",
        ),
        // A static counter.
        (
            "int subject(void) { static int i; for (i = 0; i < 10; i++) use(i); return 0; }\n",
            "counter-not-local-integer",
        ),
        // A field bound of a wide type.
        (
            "struct c { int size; };\nint subject(struct c *c) { int i; for (i = 0; i < c->size; i++) use(i); return 0; }\n",
            "bound-not-stable",
        ),
        (
            "int subject(int n) { int i; for (i = n; i < 10; i++) use(i); return 0; }\n",
            "init-not-constant",
        ),
        (
            "int subject(void) { int i; for (i = 0; i < 10; i++) { again: if (use(i)) goto again; } return 0; }\n",
            "label-in-loop",
        ),
        (
            "int vfork(void);\nint subject(void) { int i; vfork(); for (i = 0; i < 10; i++) use(i); return 0; }\n",
            "opaque-control",
        ),
        // A `case` of an outer switch inside the loop (Duff's device).
        (
            "int subject(int k) { int i = 0; switch (k) { case 0: for (i = 0; i < 4; i++) { case 1: use(i); } } return i; }\n",
            "label-in-loop",
        ),
    ];
    for (source, code) in cases {
        assert_eq!(refusal(source), code, "{source}");
    }
}

#[test]
fn the_slice_refusing_controls_have_no_counter() {
    if !clang_available() {
        return;
    }
    // cb_queue_chunks (sb.c:611), ipow (flb_scheduler.c:237) and
    // random_uniform (flb_scheduler.c:106): no counted `for` loop.
    let ipow = "int subject(int base, int exp) {\n\
                    int result = 1;\n\
                    while (exp) {\n\
                        if (exp & 1) result *= base;\n\
                        exp >>= 1;\n\
                        base *= base;\n\
                    }\n\
                    return result;\n\
                }\n";
    let random = "int rand(void);\n\
                  int subject(int min, int max) {\n\
                      int range = max - min + 1;\n\
                      int copies = 2147483647 / range;\n\
                      int limit = range * copies;\n\
                      int value;\n\
                      do { value = rand(); } while (value < 0 || value >= limit);\n\
                      return min + value / copies;\n\
                  }\n";
    let queue = "struct node { struct node *next; };\n\
                 int subject(struct node *head) {\n\
                     while (head != 0) { if (use(1) < 0) continue; head = head->next; }\n\
                     return 0;\n\
                 }\n";
    for source in [ipow, random, queue] {
        let fact = only_loop(source);
        assert_eq!(fact.refusal, Some("no-counter"), "{source}");
        assert!(fact.measure.is_none());
    }
}

#[test]
fn a_macro_loop_is_placed_at_its_use() {
    if !clang_available() {
        return;
    }
    let text = format!(
        "{PRELUDE}#define EACH(i, n) for (i = 0; i < n; i++)\n\
         int subject(void) {{\n\
             int i;\n\
             EACH(i, 4) use(i);\n\
             while (use(0)) {{}}\n\
             return 0;\n\
         }}\n"
    );
    let unit = C11TranslationUnit {
        source: &text,
        ..Default::default()
    };
    let preprocessed = validate_unrestricted_c11(&unit).expect("preprocessed");
    let report = analyze_loop_measures_preprocessed(&preprocessed, &unit, &["subject".to_owned()])
        .expect("facts");
    let function = report.function("subject").expect("subject");
    // A fact's line indexes the preprocessed text; the gateway's line map
    // presents it at the original line: the prelude is six lines, the macro
    // use is on line 10 and the `while` on line 11.
    let map = map_preprocessed_lines(&unit, &preprocessed);
    let lines = function
        .loops
        .iter()
        .map(|fact| {
            let line = usize::try_from(fact.line.expect("a line")).expect("a line");
            (map.present(line, "input"), fact.kind)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![
            (Some("input:10".to_owned()), "for"),
            (Some("input:11".to_owned()), "while")
        ]
    );
    assert_eq!(
        function.loops[0].measure.as_ref().map(|m| m.max_iterations),
        Some(4)
    );
}
