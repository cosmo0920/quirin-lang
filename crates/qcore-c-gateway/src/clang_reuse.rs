//! Card **GATEWAY-ANALYSIS-PATH-OPTIMIZATION**: one translation unit's Clang
//! results reused across the analysis scopes of one worker thread.
//!
//! Each `analyze_translation_unit_under_holds` call enters a fresh source
//! scope (`typed_access_path::enter_source`), and one unit is analysed under
//! several: the analysis itself, the review-standing settle of a `Pending`
//! row, the certificate self-check of a `Safe` unit and the audit's admission
//! recheck. Each scope used to launch Clang for the whole-unit typed AST again,
//! with the same arguments and the same input, and every consultation of the
//! record-layout reader launched Clang again for the same dump. This module
//! keeps the last results of the thread so the repeated launches are answered
//! by the first one.
//!
//! It is verdict-identical by construction, not by measurement alone:
//!
//! * the key is the **whole launch identity**: the Clang resolution
//!   environment (`QCORE_CLANG` and `PATH`, the key `clang::command` resolves
//!   the executable under), the dialect, the frontend request (its complete
//!   `Debug` rendering, a superset of every flag the transport appends) and
//!   the analysed text, compared **byte for byte**, never by a digest;
//! * a launch is reused only when it reads nothing but its standard input:
//!   a text that spells an `#include`, `#import` or `#embed` directive, or
//!   `__has_include`, could read a file whose bytes the key does not hold, and
//!   is never reused (every analysed text is preprocessed, so this declines
//!   only hand-written inputs);
//! * only a **success** is kept: a failed launch is never stored, so the next
//!   consultation launches again, exactly as before;
//! * the stored value is what the first launch's own parse produced, and every
//!   lazily filled part of it (a function's line index, the unit's line
//!   tokens) is a function of that value alone.
//!
//! The executable is trusted to stay stable while in use, the boundary
//! [`crate::clang`] already rests the resolution cache and `ClangIdentity` on.
//! The cache is thread-local and holds at most [`CAPACITY`] units per kind, so
//! memory grows by at most that many retained indexes per worker thread.

use crate::legacy_c::CCompilerDialect;
use std::cell::RefCell;
use std::ffi::OsString;
use std::rc::Rc;
use std::sync::Arc;

/// Units kept per kind per thread. Two covers the measured nesting: a scope
/// opened while another unit's scope is still live (none on the measured
/// routes) evicts nothing the outer scope still asks for.
const CAPACITY: usize = 2;

/// The identity of one unit's Clang launch: equal keys launch Clang with the
/// same executable, arguments and standard input.
#[derive(Clone)]
pub(crate) struct UnitKey {
    environment: (Option<OsString>, Option<OsString>),
    dialect: CCompilerDialect,
    frontend: Arc<String>,
    source: Arc<String>,
}

impl PartialEq for UnitKey {
    fn eq(&self, other: &Self) -> bool {
        self.environment == other.environment
            && self.dialect == other.dialect
            && (Arc::ptr_eq(&self.frontend, &other.frontend) || self.frontend == other.frontend)
            && (Arc::ptr_eq(&self.source, &other.source) || self.source == other.source)
    }
}

impl UnitKey {
    /// `None` when the text may make Clang read a file (see the module
    /// documentation): such a launch is never reused.
    pub(crate) fn new(
        dialect: CCompilerDialect,
        frontend: &Arc<String>,
        source: &Arc<String>,
    ) -> Option<Self> {
        if !reads_only_standard_input(source) {
            return None;
        }
        Some(Self {
            environment: crate::clang::resolution_environment(),
            dialect,
            frontend: Arc::clone(frontend),
            source: Arc::clone(source),
        })
    }
}

/// Whether Clang, given `text` on its standard input, reads no other file:
/// no line is an `#include`, `#import` or `#embed` directive and the text never
/// spells `__has_include` (or `__has_embed`). Fail-closed: a directive spelled
/// in any way this reader does not split (a comment between `#` and the name,
/// a line splice) still contains the name, so it only ever answers `false`
/// for more texts, never fewer.
pub(crate) fn reads_only_standard_input(text: &str) -> bool {
    if text.contains("__has_include") || text.contains("__has_embed") {
        return false;
    }
    !text.lines().any(|line| {
        let Some(rest) = line.trim_start().strip_prefix('#') else {
            return false;
        };
        rest.contains("include") || rest.contains("import") || rest.contains("embed")
    })
}

/// The last [`CAPACITY`] successful results of one kind on this thread.
pub(crate) struct UnitCache<T> {
    entries: RefCell<Vec<(UnitKey, Rc<T>)>>,
}

impl<T> UnitCache<T> {
    pub(crate) const fn new() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
        }
    }

    /// The stored result for `key`, or `launch`'s, stored only when it
    /// succeeded. Without a key (an unreusable launch) this is `launch()`.
    pub(crate) fn get_or_launch(
        &self,
        key: Option<UnitKey>,
        launch: impl FnOnce() -> Option<T>,
    ) -> Option<Rc<T>> {
        let Some(key) = key else {
            return launch().map(Rc::new);
        };
        if let Some((_, value)) = self
            .entries
            .borrow()
            .iter()
            .find(|(stored, _)| *stored == key)
        {
            return Some(Rc::clone(value));
        }
        let value = Rc::new(launch()?);
        let mut entries = self.entries.borrow_mut();
        if entries.len() == CAPACITY {
            entries.remove(0);
        }
        entries.push((key, Rc::clone(&value)));
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_that_may_read_a_file_is_never_reused() {
        for text in [
            "#include <stdio.h>\nint x;",
            "  #  include \"a.h\"\n",
            "#import \"a.h\"\n",
            "#embed \"a.bin\"\n",
            "#if __has_include(<a.h>)\n#endif\n",
            "# /* c */ include \"a.h\"\n",
        ] {
            assert!(!reads_only_standard_input(text), "{text}");
        }
        for text in ["int x;\n", "#pragma once\nint f(void) { return 1; }\n"] {
            assert!(reads_only_standard_input(text), "{text}");
        }
    }

    #[test]
    fn only_successes_are_kept_and_keys_compare_whole_texts() {
        let cache = UnitCache::<usize>::new();
        let frontend = Arc::new(String::from("request"));
        let source = Arc::new(String::from("int x;\n"));
        let key = || UnitKey::new(CCompilerDialect::C11, &frontend, &source);
        let mut launches = 0;
        assert!(
            cache
                .get_or_launch(key(), || {
                    launches += 1;
                    None
                })
                .is_none()
        );
        assert_eq!(
            *cache
                .get_or_launch(key(), || {
                    launches += 1;
                    Some(7)
                })
                .unwrap(),
            7
        );
        assert_eq!(
            *cache
                .get_or_launch(key(), || {
                    launches += 1;
                    Some(8)
                })
                .unwrap(),
            7
        );
        assert_eq!(launches, 2);
        // The same text in a different allocation is the same key; a
        // different text, dialect or request is not.
        let copy = Arc::new(String::from("int x;\n"));
        let same = UnitKey::new(CCompilerDialect::C11, &frontend, &copy);
        assert_eq!(*cache.get_or_launch(same, || Some(9)).unwrap(), 7);
        let other = Arc::new(String::from("int y;\n"));
        let different = UnitKey::new(CCompilerDialect::C11, &frontend, &other);
        assert_eq!(*cache.get_or_launch(different, || Some(10)).unwrap(), 10);
        let gnu = UnitKey::new(CCompilerDialect::Gnu11, &frontend, &source);
        assert_eq!(*cache.get_or_launch(gnu, || Some(11)).unwrap(), 11);
        // Capacity two: the oldest unit (C11 `int x;`) was evicted.
        assert_eq!(*cache.get_or_launch(key(), || Some(12)).unwrap(), 12);
        // No key: always launched, never stored.
        assert_eq!(*cache.get_or_launch(None, || Some(13)).unwrap(), 13);
        assert_eq!(*cache.get_or_launch(None, || Some(14)).unwrap(), 14);
    }
}
