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

//! Card **DEEP-AST-TRANSPORTS-AND-WALL-TIME** (gateway rules 13q, 13r): every
//! place that parses Clang's JSON AST, and the nesting each one reads.
//!
//! Rule 13o (card RDHTTP-HEADER-SUMMARY-GAP) read only the whole-unit index
//! beyond `serde_json`'s default recursion limit (128). This module is the
//! inventory the card measured, as one table ([`TRANSPORTS`], bound to
//! `QCore.DeepAstTransports.transports`), and its controls:
//!
//! * **Rule 13q.** Two more transports read a document within rule 13o's bound
//!   the same way (the default parse first; on its failure the depth scan; at
//!   most 256 levels parsed again without the limit on a dedicated 256 MiB
//!   stack; otherwise the default error): the filtered launch for a
//!   consultation of the analysed unit (`typed_c_ast::unit_declarations`:
//!   `typed_access_path::definition_ast`, `appended_declarations`), and the
//!   closed-world whole-unit `Value` (`typed_c_ast::translation_unit_ast`).
//!   Their readers' recursion is measured on documents at the bound (the
//!   tests below: the deepest admitted filtered stream and closed-world unit,
//!   their readers' stack high-water mark) and on the corpus (the analysis
//!   workers' high-water mark, `unit-timing`).
//! * **Rule 13r.** The others keep their limit, each for a stated reason: the
//!   source importers' filtered launches (`typed_c_ast::ast_declarations`,
//!   `function_ast`: their interpreters' recursion is not measured deep, and
//!   their targets are named functions of fixed shapes, so a refusal there is
//!   the fail-closed answer), the numeric property's whole-unit AST
//!   (`c_numeric_obligations::typed_ast`: no route consumes it), and the FFI
//!   layout probe (`ffi_layout`: probe declarations of one tag; its readers
//!   are not measured deep). Two readers already parse without any limit on
//!   their own 256 MiB stacks, recursing over the document there: the typed
//!   dispatch census and the bounds side pass. They are recorded, not changed.
//!
//! A stack overflow aborts the process: it is a crash, not a refusal. A
//! transport admits the band only where every reader of what it returns has
//! been measured to stay far inside the smallest stack it runs on.

/// How a transport's parse treats nesting.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the inventory is read by the tests and the Lean binding"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    /// Rule 13o/13q: the default parse, then the band up to 256 levels.
    Band,
    /// Rule 13r: the default limit only (128 levels or more refused).
    Default,
    /// No limit, on a dedicated stack where the readers run too.
    OwnStack,
}

/// One Clang JSON transport: its site, what reads what it returns, and its
/// bound. Shared with `QCore.DeepAstTransports.transports` (names and
/// bounds).
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the inventory is read by the tests and the Lean binding"
    )
)]
pub(crate) const TRANSPORTS: &[(&str, &str, Bound)] = &[
    (
        "typed_access_path::AstIndex::from_json",
        "the unit index (every typed reader of the analysis)",
        Bound::Band,
    ),
    (
        "typed_c_ast::unit_declarations",
        "definition_ast (11y fallback, retention guard), appended_declarations (12u probes)",
        Bound::Band,
    ),
    (
        "typed_c_ast::translation_unit_ast",
        "atomic_publication_source (11i closed world)",
        Bound::Band,
    ),
    (
        "typed_c_ast::ast_declarations",
        "the source importers (typed_source, boolean_source, typed_source_match, layout_extents, cast_representation)",
        Bound::Default,
    ),
    (
        "c_numeric_obligations::typed_ast",
        "the numeric property report (no route caller)",
        Bound::Default,
    ),
    (
        "ffi_layout::read_struct_layout",
        "the FFI layout probe's reader",
        Bound::Default,
    ),
    (
        "typed_dispatch_census::unit_facts",
        "its streaming seed, on the parse stack",
        Bound::OwnStack,
    ),
    (
        "index_extent::analyze_bounds_preprocessed",
        "the bounds side pass, on its analysis stack",
        Bound::OwnStack,
    ),
];

#[cfg(test)]
pub(crate) mod stack {
    //! The stack high-water mark of a closure, measured on a fresh thread.
    //!
    //! The thread's stack is far larger than anything measured (so nothing
    //! overflows while measuring) and larger than the C library's 40 MiB
    //! stack cache (so it is a fresh mapping, never a reused one). Its pages
    //! are mapped on first touch; after the closure, the lowest page present
    //! in `/proc/self/pagemap` below the thread's first frame is the deepest
    //! point the closure's frames (and their drops) reached.

    #[cfg(target_os = "linux")]
    use std::io::{Read, Seek, SeekFrom};

    const SIZE: usize = 64 << 20;
    #[cfg(target_os = "linux")]
    const PAGE: usize = 4096;

    #[inline(never)]
    fn frame_address() -> usize {
        let marker = 0u8;
        std::hint::black_box(&marker) as *const u8 as usize
    }

    /// The deepest touched page at or below `top`, as KiB below `top`;
    /// `None` when pagemap cannot be read (the measurement is unavailable).
    #[cfg(target_os = "linux")]
    fn touched_below(top: usize) -> Option<usize> {
        let high = top / PAGE;
        // The thread's first frame lies below its stack's top (the C
        // library's thread block and the start frames), so the scan stops
        // 1 MiB short of the mapping's far end: it never reads the guard page
        // or a neighbouring mapping.
        let low = (top - SIZE + (1 << 20)) / PAGE;
        let mut map = std::fs::File::open("/proc/self/pagemap").ok()?;
        map.seek(SeekFrom::Start((low * 8) as u64)).ok()?;
        let mut entries = vec![0u8; (high - low + 1) * 8];
        map.read_exact(&mut entries).ok()?;
        let lowest = entries.as_chunks::<8>().0.iter().position(|entry| {
            let bits = u64::from_le_bytes(*entry);
            bits & (0b11 << 62) != 0
        })?;
        Some((high - (low + lowest)) * PAGE / 1024)
    }

    #[cfg(not(target_os = "linux"))]
    fn touched_below(_top: usize) -> Option<usize> {
        None
    }

    /// `work`'s result and the KiB of stack it used below its caller's
    /// frame, or `None` for the figure where pagemap is unreadable.
    pub(crate) fn high_water<T: Send>(work: impl FnOnce() -> T + Send) -> (T, Option<usize>) {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(SIZE)
                .spawn_scoped(scope, || {
                    let top = frame_address();
                    let result = work();
                    (result, touched_below(top))
                })
                .expect("a measuring thread")
                .join()
                .expect("the measuring thread")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::stack::high_water;
    use super::*;
    use crate::deep_unit_ast::{ADMITTED_DEPTH, DEFAULT_LIMIT, depth, values_from_slice};
    use crate::legacy_c::{C11TranslationUnit, CCompilerDialect};
    use serde_json::Value;

    /// The budget a reader of an admitted document must stay within, in the
    /// debug test build: half the 2 MiB stack of an analysis worker (whose
    /// release frames are smaller still).
    #[cfg(target_os = "linux")]
    const BUDGET_KIB: usize = 1024;

    // Behavior controls run everywhere; pagemap observations are Linux-only.
    // Failure to measure on Linux remains a failure, not a skipped assertion.
    fn assert_stack_budget(kib: Option<usize>, context: &str) {
        #[cfg(target_os = "linux")]
        {
            let kib = kib.expect("Linux /proc/self/pagemap must be readable");
            eprintln!("{context}: {kib} KiB");
            assert!(kib <= BUDGET_KIB, "{context}: {kib} KiB");
        }
        #[cfg(not(target_os = "linux"))]
        {
            assert_eq!(kib, None);
            eprintln!(
                "{context}: stack measurement unavailable (requires Linux /proc/self/pagemap)"
            );
        }
    }

    const LEAN: &str = include_str!("../../../proof/QCore/DeepAstTransports.lean");

    /// `QCore.DeepAstTransports.transports` is [`TRANSPORTS`]: the same sites
    /// in the same order with the same bounds.
    #[test]
    fn lean_transports_are_the_rust_transports() {
        let header = "def transports : List (String × Bound) := [";
        let start = LEAN.find(header).expect("the Lean list exists") + header.len();
        let body = &LEAN[start..];
        let body = &body[..body.find("\n]").expect("the Lean list closes")];
        let rows = body
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
            .map(|line| line.trim_end_matches(',').to_owned())
            .collect::<Vec<_>>();
        let expected = TRANSPORTS
            .iter()
            .map(|(site, _, bound)| {
                let bound = match bound {
                    Bound::Band => ".band",
                    Bound::Default => ".default",
                    Bound::OwnStack => ".ownStack",
                };
                format!("(\"{site}\", {bound})")
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, expected);
    }

    fn nested(levels: usize) -> String {
        format!("{}{}", "[".repeat(levels), "]".repeat(levels))
    }

    /// Rule 13q on the filtered stream: the stream's depth is its deepest
    /// value's; the default reading is unchanged, the band is read, beyond it
    /// and on a malformed stream the default error stands.
    #[test]
    fn a_filtered_stream_is_read_within_the_band() {
        let shallow = format!("{} {}", nested(3), nested(5));
        let deep = format!("{} {}", nested(3), nested(200));
        let edge = format!("{} {}", nested(ADMITTED_DEPTH), nested(1));
        let beyond = format!("{} {}", nested(1), nested(ADMITTED_DEPTH + 1));
        let malformed = format!("{} {}x", nested(1), nested(200));
        assert_eq!(depth(deep.as_bytes()), 200);
        let default = |text: &str| crate::deep_unit_ast::default_values::<Value>(text.as_bytes());
        let band = |text: &str| values_from_slice::<Value>(text.as_bytes());
        assert_eq!(band(&shallow).unwrap(), default(&shallow).unwrap());
        assert_eq!(band(&shallow).unwrap().len(), 2);
        assert!(default(&deep).is_err() && default(&edge).is_err());
        assert_eq!(band(&deep).unwrap().len(), 2);
        assert_eq!(band(&edge).unwrap().len(), 2);
        for refused in [&beyond, &malformed] {
            assert_eq!(
                band(refused).unwrap_err().to_string(),
                default(refused).unwrap_err().to_string()
            );
        }
        assert!(default(&nested(DEFAULT_LIMIT - 1)).is_ok());
    }

    fn clang_available() -> bool {
        match crate::clang::command() {
            Ok(_) => true,
            Err(reason) => {
                assert!(
                    std::env::var_os("QCORE_CLANG").is_none(),
                    "explicit QCORE_CLANG configuration is invalid: {reason}"
                );
                eprintln!("skipping Clang-dependent DEEP-AST-TRANSPORTS test: {reason}");
                false
            }
        }
    }

    /// A definition whose `else if` chain has `arms` arms; Clang nests each
    /// arm's `IfStmt` two JSON levels inside the previous one.
    fn chain(arms: usize) -> String {
        let mut body = String::from(
            "int chain(int x, const char *p) {\n    int shadow = x;\n    if (x == 0) return 0;\n",
        );
        for arm in 1..arms {
            body.push_str(&format!("    else if (x == {arm}) return {arm};\n"));
        }
        body.push_str("    return shadow;\n}\n");
        body
    }

    /// A definition whose value is a nested conditional expression of `arms`
    /// arms (the shape of `curl/typecheck-gcc.h`'s expansion).
    fn conditional(arms: usize) -> String {
        let mut expression = String::from("-1");
        for arm in (0..arms).rev() {
            expression = format!("(x == {arm} ? {arm} : {expression})");
        }
        format!(
            "int chain(int x, const char *p) {{\n    int shadow = x;\n    return {expression};\n}}\n"
        )
    }

    /// The filtered stream of `chain` in `source`, with its depth.
    fn filtered_depth(source: &str) -> usize {
        let unit = C11TranslationUnit {
            source,
            ..C11TranslationUnit::default()
        };
        let mut command = crate::clang::command().expect("Clang");
        unit.append_frontend_flags(&mut command);
        let output = command
            .args([
                "-x",
                "c",
                unit.dialect.clang_argument(),
                "-fsyntax-only",
                "-Xclang",
                "-ast-dump=json",
                "-Xclang",
                "-ast-dump-filter",
                "-Xclang",
                "chain",
                "-",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .expect("stdin")
                    .write_all(source.as_bytes())?;
                child.wait_with_output()
            })
            .expect("Clang ran");
        depth(&output.stdout)
    }

    /// The deepest `else if` chain whose filtered stream is within the band.
    fn deepest_admitted(shape: fn(usize) -> String, from: usize) -> (usize, usize) {
        let mut arms = from;
        while filtered_depth(&shape(arms + 1)) <= ADMITTED_DEPTH {
            arms += 1;
        }
        (arms, filtered_depth(&shape(arms)))
    }

    fn self_init_chain(arms: usize) -> String {
        let mut source = String::from(
            "struct Link { struct Link *next; };\nvoid reset(struct Link *p) { p->next = p; }\nvoid chain(struct Link *p) {\n",
        );
        for _ in 0..arms {
            source.push_str("if (p == 0) {} else ");
        }
        source.push_str("{}\nreset(p);\n}\n");
        source
    }

    #[test]
    fn tracked_initializer_embedded_source_readers_fit_at_the_admitted_bound() {
        if !clang_available() {
            return;
        }
        let shape = |arms| {
            self_init_chain(arms)
                .replace(
                    "void chain(struct Link *p)",
                    "struct Owner { struct Link link; }; void chain(struct Owner *p)",
                )
                .replace("reset(p);", "reset(&p->link);")
        };
        let (arms, _) = deepest_admitted(shape, 50);
        let mut measured = false;
        for backoff in 0..=3 {
            let source = shape(arms - backoff);
            let nesting = filtered_depth(&source);
            let (answer, kib) = high_water(move || {
                let _scope =
                    crate::typed_access_path::enter_source(&source, CCompilerDialect::Gnu17, None);
                crate::legacy_c::tracked_self_init::exercise_source_readers(&source)
            });
            if answer == Some(true) {
                assert!(nesting > DEFAULT_LIMIT);
                assert_stack_budget(
                    kib,
                    &format!("rule 36b embedded initializer source readers, depth {nesting}"),
                );
                measured = true;
                break;
            }
            assert_eq!(
                answer, None,
                "a parsed deep caller must admit the checked embedded target"
            );
        }
        assert!(
            measured,
            "must exercise a positive embedded target in the deep band"
        );
    }

    #[test]
    fn initializer_child_slot_frame_readers_fit_at_the_admitted_bound() {
        if !clang_available() {
            return;
        }
        let shape = |arms| {
            self_init_chain(arms)
            .replace("void chain(struct Link *p)", "struct Item { int n; }; struct Owner { struct Link head; struct Item *payload; }; void chain(struct Owner *p, struct Item *c)")
            .replace("reset(p);", "p->payload=c; reset(&p->head);")
        };
        let (arms, _) = deepest_admitted(shape, 50);
        let mut measured = false;
        for backoff in 0..=3 {
            let source = shape(arms - backoff);
            let nesting = filtered_depth(&source);
            let (answer, kib) = high_water(move || {
                let _scope =
                    crate::typed_access_path::enter_source(&source, CCompilerDialect::Gnu17, None);
                crate::legacy_c::tracked_self_init::exercise_frame_source_readers(&source)
            });
            if answer == Some(true) {
                assert!(nesting > DEFAULT_LIMIT);
                assert_stack_budget(
                    kib,
                    &format!("initializer child-slot source/layout readers, depth {nesting}"),
                );
                measured = true;
                break;
            }
            assert_eq!(
                answer, None,
                "a parsed deep source must exercise a positive child-slot proof"
            );
        }
        assert!(
            measured,
            "must reach the optional frame proof in the deep band"
        );
    }

    #[test]
    fn checked_initializer_source_readers_fit_at_the_admitted_bound() {
        if !clang_available() {
            return;
        }
        let (arms, _) = deepest_admitted(self_init_chain, 50);
        let mut measured = false;
        for backoff in 0..=3 {
            let source = self_init_chain(arms - backoff);
            let nesting = filtered_depth(&source);
            let (answer, kib) = high_water(move || {
                let _scope =
                    crate::typed_access_path::enter_source(&source, CCompilerDialect::Gnu17, None);
                crate::legacy_c::self_link_init::exercise_source_readers(&source)
            });
            if answer == Some(true) {
                assert!(nesting > DEFAULT_LIMIT);
                assert_stack_budget(
                    kib,
                    &format!("rule 36a checked initializer source readers, depth {nesting}"),
                );
                measured = true;
                break;
            }
            assert_eq!(
                answer, None,
                "an admitted index must check the complete source profile"
            );
        }
        assert!(
            measured,
            "must exercise a positive source profile in the deep band"
        );
    }

    /// The analysed-unit consultations (rule 13q) and their readers on the
    /// deepest admitted definition: the definition is read, the rule-11y
    /// extension reader and the retention guard's shadow reader answer as on
    /// a shallow definition, and every reader (the cached stream's clone,
    /// the walks, the drop) stays within the budget. One arm deeper, the
    /// stream is beyond the bound and nothing is read (the refusing answer).
    #[test]
    fn the_analysed_unit_readers_fit_at_the_bound() {
        if !clang_available() {
            return;
        }
        for (shape, from) in [(chain as fn(usize) -> String, 100), (conditional, 50)] {
            let (arms, deepest) = deepest_admitted(shape, from);
            assert!(
                deepest > DEFAULT_LIMIT && deepest <= ADMITTED_DEPTH,
                "{deepest}"
            );
            let read = |source: String| {
                high_water(move || {
                    let _scope = crate::typed_access_path::enter_source(
                        &source,
                        CCompilerDialect::Gnu17,
                        None,
                    );
                    let ast = crate::typed_access_path::definition_ast("chain");
                    let extension = ast.as_ref().map(|ast| {
                        crate::c_frontend_identity::extension_in_ast(
                            ast,
                            &crate::typed_access_path::type_id,
                        )
                        .is_some()
                    });
                    // An unused formal forces the read-frame reader through the
                    // entire deepest admitted tree, without an early refusal.
                    let formal_frame = ast
                        .as_ref()
                        .map(|ast| crate::index_traversal::read_only_formal(ast, "p"));
                    let shadowed = crate::legacy_c::retention_guard::shadowed_in("chain", "shadow");
                    let unshadowed =
                        crate::legacy_c::retention_guard::shadowed_in("chain", "other");
                    // The default-limit importer transport refuses it.
                    let importer = crate::typed_c_ast::ast_declarations(
                        &source,
                        &C11TranslationUnit {
                            source: &source,
                            ..C11TranslationUnit::default()
                        },
                        "chain",
                    )
                    .is_ok();
                    (
                        ast.is_some(),
                        extension,
                        formal_frame,
                        shadowed,
                        unshadowed,
                        importer,
                    )
                })
            };
            let (admitted, kib) = read(shape(arms));
            assert_eq!(
                admitted,
                (true, Some(false), Some(true), true, false, false),
                "{arms} arms"
            );
            assert_stack_budget(
                kib,
                &format!("rule 13q readers, {arms} arms, depth {deepest}"),
            );
            // Rule 36 adds readers of this band. Measure their actual walks
            // at the deepest index this shape admits (the whole-unit index
            // can add outer JSON levels to the filtered definition).
            let read_formal = |source: String| {
                high_water(move || {
                    let _scope = crate::typed_access_path::enter_source(
                        &source,
                        CCompilerDialect::Gnu17,
                        None,
                    );
                    crate::legacy_c::exact_formal_return::exercise_source_readers()
                })
            };
            let mut measured = false;
            for backoff in 0..=3 {
                let source = shape(arms - backoff);
                let source_depth = filtered_depth(&source);
                let (answer, formal_kib) = read_formal(source);
                if answer == Some(true) {
                    assert!(source_depth > DEFAULT_LIMIT);
                    assert_stack_budget(
                        formal_kib,
                        &format!(
                            "rule 36 source readers, {} arms, filtered depth {source_depth}",
                            arms - backoff
                        ),
                    );
                    measured = true;
                    break;
                }
                assert_eq!(answer, None);
            }
            assert!(measured, "rule 36 must exercise a deep admitted index");
            assert_eq!(read_formal(shape(arms + 1)).0, None);
            let (beyond, _) = read(shape(arms + 1));
            // Beyond the bound nothing is read: the shadow reader refuses
            // (`true` for any name), as before the card.
            assert_eq!(
                beyond,
                (false, None, None, true, true, false),
                "{} arms",
                arms + 1
            );
        }
    }

    const ATOMIC: &str = r#"
#include <stdatomic.h>
static _Atomic int done = 0;
static int payload;
void publish(void) {
    payload = 7;
    atomic_store_explicit(&done, 1, memory_order_release);
}
int consume(void) {
    int seen = atomic_load_explicit(&done, memory_order_acquire);
    if (seen != 1) return 0;
    return payload;
}
"#;

    fn atomic(
        source: &str,
    ) -> Result<
        crate::atomic_publication_source::AtomicPublicationSourceCertificate,
        crate::atomic_publication_source::AtomicPublicationSourceRefusal,
    > {
        crate::atomic_publication_source::extract_atomic_publication_source(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "fixture-revision",
            &crate::atomic_publication_source::AtomicPublicationSourceProfile {
                publisher: "publish",
                consumer: "consume",
                flag: "done",
                payload: "payload",
            },
            crate::atomic_publication_source::AtomicPublicationExecutionBounds {
                publisher_invocations: 1,
                successful_consumer_invocations: 1,
            },
        )
    }

    /// Rule 13q on the closed world: a unit holding a definition at the band's
    /// edge is read whole, and its judgment is the shallow unit's (the same
    /// values, orders and scope); its readers stay within the budget. One arm
    /// deeper the unit is refused with the default parse's error, as before.
    #[test]
    fn the_closed_world_readers_fit_at_the_bound() {
        if !clang_available() {
            return;
        }
        if !crate::clang_version().is_ok_and(|version| version.supports_cg19_atomic_ast()) {
            return;
        }
        let shallow = atomic(ATOMIC).expect("the shallow unit is certified");
        // The whole unit nests two levels deeper than the filtered stream.
        let (arms, _) = deepest_admitted(chain, 100);
        let arms = arms - 1;
        let source = format!("{ATOMIC}{}", chain(arms));
        let (deep, kib) = high_water(move || atomic(&source));
        let deep = deep.expect("the deep unit is certified");
        assert_eq!(
            (
                deep.initial_value,
                deep.published_value,
                deep.successful_observation
            ),
            (
                shallow.initial_value,
                shallow.published_value,
                shallow.successful_observation
            )
        );
        assert_eq!(
            (deep.store_order, deep.load_order),
            (shallow.store_order, shallow.load_order)
        );
        assert_eq!(deep.scope, shallow.scope);
        assert_stack_budget(kib, &format!("rule 13q closed-world readers, {arms} arms"));
        let beyond = format!("{ATOMIC}{}", chain(arms + 3));
        let (refused, _) = high_water(move || atomic(&beyond));
        match refused {
            Err(
                crate::atomic_publication_source::AtomicPublicationSourceRefusal::InvalidSource(
                    reason,
                ),
            ) => {
                assert!(reason.contains("recursion limit exceeded"), "{reason}")
            }
            other => panic!("expected the default parse's refusal: {other:?}"),
        }
    }

    /// The nesting of a value as `deep_unit_ast::depth` counts its JSON text.
    fn value_depth(value: &Value) -> usize {
        match value {
            Value::Array(items) => 1 + items.iter().map(value_depth).max().unwrap_or(0),
            Value::Object(map) => 1 + map.values().map(value_depth).max().unwrap_or(0),
            _ => 0,
        }
    }

    /// Measurement only, run with `--ignored`: the analysed-unit consultation
    /// (rule 13q) and its readers on the real definitions of an external
    /// preprocessed unit. `DAT_DEEP_SOURCE` names the preprocessed text (read
    /// as the analysed source, `gnu17`, no frontend flags), `DAT_DEEP_FUNCTIONS`
    /// a comma list of definitions. Each line printed: the definition, its
    /// filtered value's nesting, whether it was read, the two readers'
    /// answers, and the readers' stack high-water mark in KiB.
    #[test]
    #[ignore = "measurement on an external preprocessed unit"]
    fn measure_the_readers_on_a_preprocessed_unit() {
        let (Some(path), Ok(names)) = (
            std::env::var_os("DAT_DEEP_SOURCE"),
            std::env::var("DAT_DEEP_FUNCTIONS"),
        ) else {
            return;
        };
        let source = std::fs::read_to_string(path).expect("the preprocessed unit");
        for name in names.split(',') {
            let (text, function) = (source.clone(), name.to_owned());
            let ((read, extension, shadowed), kib) = high_water(move || {
                let _scope =
                    crate::typed_access_path::enter_source(&text, CCompilerDialect::Gnu17, None);
                let ast = crate::typed_access_path::definition_ast(&function);
                let extension = ast.as_ref().map(|ast| {
                    crate::c_frontend_identity::extension_in_ast(
                        ast,
                        &crate::typed_access_path::type_id,
                    )
                    .map(|(_, spelling)| spelling)
                });
                let shadowed =
                    crate::legacy_c::retention_guard::shadowed_in(&function, "qcore_no_such_name");
                (ast.is_some(), extension, shadowed)
            });
            let (text, function) = (source.clone(), name.to_owned());
            let (nesting, _) = high_water(move || {
                let _scope =
                    crate::typed_access_path::enter_source(&text, CCompilerDialect::Gnu17, None);
                crate::typed_access_path::definition_ast(&function)
                    .as_ref()
                    .map(value_depth)
            });
            println!(
                "{name} nesting {nesting:?} read {read} extension {extension:?} shadowed {shadowed} stack {kib:?} KiB"
            );
        }
    }

    /// The measurement itself: a closure that recurses deeper uses more
    /// stack, and the figure is in the right range.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "stack high-water measurement requires Linux /proc/self/pagemap"
    )]
    fn the_high_water_mark_measures_recursion() {
        #[inline(never)]
        fn recurse(levels: usize) -> usize {
            let pad = std::hint::black_box([0u8; 1024]);
            if levels == 0 {
                pad[0] as usize
            } else {
                recurse(levels - 1) + pad[1] as usize
            }
        }
        let (_, small) = high_water(|| recurse(16));
        let (_, large) = high_water(|| recurse(512));
        let (small, large) = (small.expect("pagemap"), large.expect("pagemap"));
        assert!(large >= small + 400, "{small} KiB, {large} KiB");
        assert!(large >= 512, "{large} KiB");
    }
}
