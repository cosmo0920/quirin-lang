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

//! The fragment assembler and its one injected entry point (card
//! `C-EMIT-MAIN-UNIFY`).

use super::*;

fn fragment(section: Section, key: &str, text: &str) -> Fragment {
    Fragment::new(section, key, text)
}

fn body(text: &str) -> Entry {
    Entry::source_route(text)
}

#[test]
fn the_assembler_writes_one_entry_point_and_the_crate_spells_it_once() {
    // The single-`main` guarantee is a type: `assemble` takes one `Entry`, so
    // there is no call that produces two. This is the observable half of that —
    // the assembled text carries exactly one entry definition.
    let source = assemble(
        &[fragment(
            Section::Includes,
            "include.stdio",
            "#include <stdio.h>\n",
        )],
        &body("    /* nothing */\n"),
    );
    // Line 1 is the include, line 2 the entry's own leading blank line.
    assert_eq!(entry_definitions(&source), vec![3]);
    assert!(source.contains(ENTRY_SIGNATURE));
}

#[test]
fn the_entry_point_is_assembled_from_its_five_parts_with_no_separator_added() {
    // Every byte of the result comes from a fragment, from the entry, or from
    // the signature and the brace style. An assembler that inserted a newline
    // of its own would move every pinned program's bytes.
    let source = assemble(
        &[fragment(Section::Prelude, "p", "PRELUDE\n")],
        &Entry {
            lead: "\n".to_owned(),
            brace: BraceStyle::Allman,
            prologue: "    int q = 0;\n".to_owned(),
            body: "    q = 1;\n".to_owned(),
            epilogue: "    return q;\n}\n".to_owned(),
        },
    );
    assert_eq!(
        source,
        "PRELUDE\n\nint main(void)\n{\n    int q = 0;\n    q = 1;\n    return q;\n}\n"
    );
}

#[test]
fn both_pinned_brace_styles_are_reproduced_rather_than_normalized() {
    // Both spellings exist in emitted programs today and both are pinned by the
    // F02 emission digests. Choosing one would be a byte move for no gain.
    assert_eq!(BraceStyle::Allman.bytes(), "\n{\n");
    assert_eq!(BraceStyle::KAndR.bytes(), " {\n");
    let allman = assemble(&[], &body(""));
    assert!(allman.starts_with("\nint main(void)\n{\n"));
    let kandr = assemble(
        &[],
        &Entry {
            lead: String::new(),
            brace: BraceStyle::KAndR,
            prologue: String::new(),
            body: String::new(),
            epilogue: "}\n".to_owned(),
        },
    );
    assert_eq!(kandr, "int main(void) {\n}\n");
}

#[test]
fn two_routes_needing_one_prelude_get_one_copy() {
    // This is what makes a composed program incapable of defining a helper
    // twice: the key names the thing, so both routes spell the same key.
    let shared = fragment(Section::Prelude, "io.prelude", "#define QCORE_IO_WRITE\n");
    let route_a = [
        shared.clone(),
        fragment(Section::Prelude, "a.prelude", "A\n"),
    ];
    let route_b = [
        shared.clone(),
        fragment(Section::Prelude, "b.prelude", "B\n"),
    ];
    let merged = merge(&[&route_a, &route_b]);
    assert_eq!(
        merged.iter().filter(|f| f.key == "io.prelude").count(),
        1,
        "the shared prelude must appear once in a composed program"
    );
    let source = assemble(&merged, &body(""));
    assert_eq!(source.matches("#define QCORE_IO_WRITE").count(), 1);
    // And the composition still has one entry point.
    assert_eq!(entry_definitions(&source).len(), 1);
}

#[test]
fn deduplication_is_idempotent_and_keeps_the_first_of_each_key() {
    let fragments = [
        fragment(Section::Prelude, "p", "first\n"),
        fragment(Section::Prelude, "q", "q\n"),
        fragment(Section::Prelude, "p", "first\n"),
    ];
    let once: Vec<Fragment> = deduplicate(&fragments).into_iter().cloned().collect();
    assert_eq!(once.len(), 2);
    assert_eq!(once[0].key, "p");
    assert_eq!(once[1].key, "q");
    let twice: Vec<Fragment> = deduplicate(&once).into_iter().cloned().collect();
    assert_eq!(once, twice, "deduplication is idempotent");
}

#[test]
fn deduplication_is_on_the_key_alone_and_crosses_sections() {
    // A key names a thing, and a thing lives in one place. A route that
    // contributed one prelude into two sections gets one copy, in the first
    // position it asked for.
    let fragments = [
        fragment(Section::Includes, "thing", "THING\n"),
        fragment(Section::Prelude, "thing", "THING\n"),
    ];
    assert_eq!(deduplicate(&fragments).len(), 1);
    assert_eq!(deduplicate(&fragments)[0].section, Section::Includes);
}

#[test]
fn a_key_whose_two_contributors_disagree_is_reported() {
    // First-wins would otherwise discard the second route's text silently.
    // The procedure decides that question; the backends do not call it.
    let agreeing = [
        fragment(Section::Prelude, "p", "same\n"),
        fragment(Section::Prelude, "p", "same\n"),
    ];
    assert_eq!(conflicting_keys(&agreeing), Vec::<String>::new());
    let disagreeing = [
        fragment(Section::Prelude, "p", "one\n"),
        fragment(Section::Prelude, "q", "q\n"),
        fragment(Section::Prelude, "p", "other\n"),
    ];
    assert_eq!(conflicting_keys(&disagreeing), vec!["p".to_owned()]);
}

#[test]
fn assembly_preserves_the_contributed_order_and_does_not_sort() {
    // Measured, not preferred: sorting by section would lift the task
    // backend's late `#include <stdio.h>` above the logger prelude, which is a
    // byte move in a digest-pinned program. Insertion order reproduces every
    // backend's existing bytes exactly.
    let fragments = [
        fragment(Section::Prelude, "diagnostic", "DIAG\n"),
        fragment(Section::Includes, "include.stdio", "#include <stdio.h>\n"),
    ];
    let source = assemble(&fragments, &body(""));
    assert!(
        source.starts_with("DIAG\n#include <stdio.h>\n"),
        "assemble must not reorder a single route's contribution"
    );
}

#[test]
fn a_section_inversion_is_reported_rather_than_hidden() {
    let ordered = [
        fragment(Section::Compat, "compat", "C\n"),
        fragment(Section::Includes, "i", "I\n"),
        fragment(Section::Prelude, "p", "P\n"),
    ];
    assert_eq!(out_of_order_fragments(&ordered), Vec::<usize>::new());
    let inverted = [
        fragment(Section::Prelude, "p", "P\n"),
        fragment(Section::Includes, "i", "I\n"),
    ];
    assert_eq!(out_of_order_fragments(&inverted), vec![1]);
}

#[test]
fn merging_orders_by_section_because_two_routes_have_no_relative_order() {
    // The composition path does sort: C requires a compatibility header before
    // an include and an include before the definitions that read its types, and
    // two independently written routes have no insertion order of their own.
    let route_a = [
        fragment(Section::Prelude, "a.prelude", "AP\n"),
        fragment(Section::Includes, "a.include", "AI\n"),
    ];
    let route_b = [
        fragment(Section::Compat, "b.compat", "BC\n"),
        fragment(Section::Prelude, "b.prelude", "BP\n"),
    ];
    let merged = merge(&[&route_a, &route_b]);
    let keys: Vec<&str> = merged.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        vec!["b.compat", "a.include", "a.prelude", "b.prelude"],
        "sections order the routes; contribution order breaks ties inside one"
    );
    assert_eq!(out_of_order_fragments(&merged), Vec::<usize>::new());
}

#[test]
fn merging_is_idempotent_on_an_already_merged_list() {
    let route = [
        fragment(Section::Includes, "i", "I\n"),
        fragment(Section::Prelude, "p", "P\n"),
    ];
    let once = merge(&[&route]);
    let twice = merge(&[once.as_slice()]);
    assert_eq!(once, twice);
}

#[test]
fn the_entry_scan_counts_every_definition_it_is_shown() {
    // Non-vacuity in both directions: the scan finds a second entry point if a
    // fragment carries one, which is why the tests assert one per program
    // rather than trusting the assembler alone.
    assert_eq!(entry_definitions("int main(void)\n{\n}\n"), vec![1]);
    assert_eq!(
        entry_definitions("int main(void) {}\nint main(int c, char **v) {}\n"),
        vec![1, 2]
    );
    assert_eq!(
        entry_definitions("void other(void) {}\n"),
        Vec::<usize>::new()
    );
}

#[test]
fn a_fragment_carrying_its_own_entry_point_is_visible_to_the_scan() {
    // The assembler guarantees it writes one; it does not guarantee a
    // contributor did not smuggle another in. That is the stated boundary, and
    // it is the reason the scan exists beside the type.
    let source = assemble(
        &[fragment(
            Section::FileScope,
            "smuggled",
            "int main(void) { return 1; }\n",
        )],
        &body(""),
    );
    assert_eq!(entry_definitions(&source).len(), 2);
}

#[test]
fn the_declared_sections_are_in_the_order_c_requires() {
    let mut sections = [
        Section::FileScope,
        Section::Prelude,
        Section::Declarations,
        Section::Includes,
        Section::Compat,
        Section::Config,
        Section::Banner,
    ];
    sections.sort();
    assert_eq!(
        sections,
        [
            Section::Banner,
            Section::Config,
            Section::Compat,
            Section::Includes,
            Section::Declarations,
            Section::Prelude,
            Section::FileScope,
        ]
    );
}

// ---------------------------------------------------------------------------
// The real backends' contributions
// ---------------------------------------------------------------------------

/// Every route's preamble, under the name the report's architecture map uses.
///
/// The three program-dependent ones are omitted: `source.string.v1`,
/// `source.buffer.v1` and the task route build their fragments from a checked
/// program, and the properties below are about the fragments' *keys and
/// sections*, which those routes share with the constant preambles they sit
/// beside. Their own emission is covered by their own suites.
fn every_constant_preamble() -> Vec<(&'static str, Vec<Fragment>)> {
    vec![
        ("sequential", crate::c_emit::preamble(None)),
        (
            "sequential+adapter",
            crate::c_emit::preamble(Some(crate::std_io::Level::Info)),
        ),
        ("std-io", crate::std_io_c_emit::preamble()),
        ("socket", crate::std_socket_c_emit::preamble()),
        ("mutex", crate::mutex_c_emit::preamble()),
        (
            "task",
            crate::task_c_emit::preamble(1, 1, None, /* writes_lines */ true),
        ),
        (
            "task+adapter",
            crate::task_c_emit::preamble(1, 1, Some(crate::std_io::Level::Info), true),
        ),
        (
            "task-silent",
            crate::task_c_emit::preamble(1, 1, None, false),
        ),
    ]
}

#[test]
fn no_two_routes_disagree_about_a_fragment_they_share() {
    // The shared fragments — the IO prelude, the typed logger, the threads
    // compatibility header — are contributed by several routes under one key.
    // First-wins deduplication would silently discard a second route's text if
    // the two ever disagreed, so this is the check that they do not, taken over
    // every route's contribution at once as a composition would see them.
    let all: Vec<Fragment> = every_constant_preamble()
        .into_iter()
        .flat_map(|(_, fragments)| fragments)
        .collect();
    assert_eq!(
        conflicting_keys(&all),
        Vec::<String>::new(),
        "two routes contribute one key with different text"
    );
    // Non-vacuous: keys really are shared across routes.
    let shared = crate::io_emit::IO_PRELUDE_KEY;
    assert!(
        all.iter().filter(|f| f.key == shared).count() >= 2,
        "the IO prelude must be contributed by more than one route"
    );
    // And a composition of all of them installs it once.
    assert_eq!(merge(&[&all]).iter().filter(|f| f.key == shared).count(), 1);
}

#[test]
fn the_task_routes_late_stdio_include_is_the_only_section_inversion() {
    // Measured, and reproduced rather than repaired (card `C-EMIT-MAIN-UNIFY`).
    // The task backend places `#include <stdio.h>` among its preludes instead
    // of beside its other includes. Lifting it would be inert in C — the logger
    // prelude uses no stdio — but it is a byte move in a program the two F02
    // emission digests pin, so `assemble` preserves insertion order and this
    // test names the one case rather than letting it hide.
    for (name, fragments) in every_constant_preamble() {
        let inversions = out_of_order_fragments(&fragments);
        if name.starts_with("task") && !inversions.is_empty() {
            assert_eq!(inversions.len(), 1, "{name}: one inversion, not more");
            assert_eq!(
                fragments[inversions[0]].key,
                crate::io_emit::STDIO_INCLUDE_KEY,
                "{name}: the inversion must be the stdio include and nothing else"
            );
        } else {
            assert_eq!(
                inversions,
                Vec::<usize>::new(),
                "{name} contributes its fragments out of section order"
            );
        }
    }
    // Non-vacuous: the inversion really is there on the route that writes.
    assert_eq!(
        out_of_order_fragments(&crate::task_c_emit::preamble(1, 1, None, true)).len(),
        1
    );
    // And merging repairs it, which is what a composition would get.
    assert_eq!(
        out_of_order_fragments(&merge(&[&crate::task_c_emit::preamble(1, 1, None, true)])),
        Vec::<usize>::new()
    );
}

#[test]
fn no_routes_preamble_defines_an_entry_point_of_its_own() {
    // The assembler guarantees it writes one; this is the other half — that no
    // contributor smuggles a second in. Together they are the single-`main`
    // property for every route that has a constant preamble.
    for (name, fragments) in every_constant_preamble() {
        let source = assemble(&fragments, &body("    ;\n"));
        assert_eq!(
            entry_definitions(&source).len(),
            1,
            "{name} assembles to a program without exactly one entry point"
        );
    }
}

/// The two-sided domain tie with `QCore.CProgram`, in the style of
/// `QCore.SingleEmissionRoute`'s.
///
/// Both sides walk the same fragment list and compute the same two numbers
/// independently: how many fragments survive deduplication, and how many entry
/// definitions the assembled text carries. Agreement is evidence that the Lean
/// model and the procedure the backends run decide the same thing on this
/// domain.
#[test]
fn the_domain_outcomes_agree_with_the_lean_model() {
    let domain: Vec<Vec<Fragment>> = vec![
        vec![],
        vec![fragment(Section::Prelude, "a", "A\n")],
        vec![
            fragment(Section::Prelude, "a", "A\n"),
            fragment(Section::Prelude, "a", "A\n"),
        ],
        vec![
            fragment(Section::Prelude, "a", "A\n"),
            fragment(Section::Prelude, "b", "B\n"),
        ],
        vec![
            fragment(Section::Includes, "a", "A\n"),
            fragment(Section::Prelude, "a", "A\n"),
            fragment(Section::Prelude, "b", "B\n"),
        ],
        vec![fragment(
            Section::FileScope,
            "s",
            "int main(void) { return 1; }\n",
        )],
    ];
    let kept: Vec<usize> = domain
        .iter()
        .map(|fragments| deduplicate(fragments).len())
        .collect();
    let entries: Vec<usize> = domain
        .iter()
        .map(|fragments| entry_definitions(&assemble(fragments, &body(""))).len())
        .collect();
    assert_eq!(kept, vec![0, 1, 1, 2, 2, 1]);
    assert_eq!(entries, vec![1, 1, 1, 1, 1, 2]);
}
