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

    pub(crate) fn clear(&self) {
        self.entries.borrow_mut().clear();
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

/// Launches kept per thread by a [`LaunchCache`].
const LAUNCH_CAPACITY: usize = 32;

/// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the last [`LAUNCH_CAPACITY`]
/// successful results of one Clang transport on this thread, keyed by the
/// launch itself: the resolution environment, the executable, every argument
/// and the standard input, all compared exactly. The same rules as
/// [`UnitCache`]: successes only, never a launch whose input may read a file
/// or whose command carries its own environment or working directory.
pub(crate) struct LaunchCache<T> {
    entries: RefCell<Vec<(LaunchKey, Rc<T>)>>,
}

struct LaunchKey {
    environment: (Option<OsString>, Option<OsString>),
    program: OsString,
    arguments: Vec<OsString>,
    input: String,
}

impl LaunchKey {
    fn matches(
        &self,
        environment: &(Option<OsString>, Option<OsString>),
        command: &std::process::Command,
        input: &str,
    ) -> bool {
        self.environment == *environment
            && self.program == command.get_program()
            && self
                .arguments
                .iter()
                .map(OsString::as_os_str)
                .eq(command.get_args())
            && self.input == input
    }
}

impl<T: Clone> LaunchCache<T> {
    pub(crate) const fn new() -> Self {
        Self {
            entries: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn clear(&self) {
        self.entries.borrow_mut().clear();
    }

    /// `launch(command)`'s result, or the stored result of an identical
    /// earlier launch on this thread. Only a success is stored.
    pub(crate) fn get_or_launch(
        &self,
        command: std::process::Command,
        input: &str,
        launch: impl FnOnce(std::process::Command) -> Result<T, String>,
    ) -> Result<T, String> {
        if !reads_only_standard_input(input)
            || command.get_envs().next().is_some()
            || command.get_current_dir().is_some()
        {
            return launch(command);
        }
        let environment = crate::clang::resolution_environment();
        if let Some((_, value)) = self
            .entries
            .borrow()
            .iter()
            .find(|(key, _)| key.matches(&environment, &command, input))
        {
            return Ok(value.as_ref().clone());
        }
        let key = LaunchKey {
            environment,
            program: command.get_program().to_owned(),
            arguments: command.get_args().map(ToOwned::to_owned).collect(),
            input: input.to_owned(),
        };
        let value = launch(command)?;
        let mut entries = self.entries.borrow_mut();
        if entries.len() == LAUNCH_CAPACITY {
            entries.remove(0);
        }
        entries.push((key, Rc::new(value.clone())));
        Ok(value)
    }
}

/// Bound optional Clang reuse to one audit unit, including its certificate
/// rechecks. Active source/statement readers own their own Rc/Arc leases;
/// clearing a reuse table cannot invalidate them. Nested boundaries may cause
/// fresh launches but cannot substitute weaker evidence. Cleanup also runs on
/// errors and unwinding. This affects only the calling worker thread.
pub fn with_audit_unit_lifetimes<T>(work: impl FnOnce() -> T) -> T {
    fn release() {
        crate::typed_access_path::release_reuse_caches();
        crate::typed_c_ast::release_reuse_caches();
    }
    struct Release;
    impl Drop for Release {
        fn drop(&mut self) {
            release();
        }
    }
    release();
    let _release = Release;
    work()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_is_reused_only_for_the_same_program_arguments_and_input() {
        let cache = LaunchCache::<usize>::new();
        let command = |argument: &str| {
            let mut command = std::process::Command::new("clang");
            command.args(["-x", "c", argument, "-"]);
            command
        };
        let launches = std::cell::Cell::new(0);
        let launch = |_: std::process::Command| {
            launches.set(launches.get() + 1);
            Ok(launches.get())
        };
        assert_eq!(cache.get_or_launch(command("a"), "int x;\n", launch), Ok(1));
        assert_eq!(cache.get_or_launch(command("a"), "int x;\n", launch), Ok(1));
        assert_eq!(cache.get_or_launch(command("b"), "int x;\n", launch), Ok(2));
        assert_eq!(cache.get_or_launch(command("a"), "int y;\n", launch), Ok(3));
        // A failure is never kept, and a text that may read a file is never
        // reused.
        let failed = LaunchCache::<usize>::new();
        assert!(
            failed
                .get_or_launch(command("a"), "int x;\n", |_| Err("no".into()))
                .is_err()
        );
        assert_eq!(
            failed.get_or_launch(command("a"), "int x;\n", launch),
            Ok(4)
        );
        let include = "#include <a.h>\n";
        assert_eq!(cache.get_or_launch(command("a"), include, launch), Ok(5));
        assert_eq!(cache.get_or_launch(command("a"), include, launch), Ok(6));
        // A command carrying its own environment is never reused.
        let mut with_env = command("a");
        with_env.env("X", "1");
        assert_eq!(cache.get_or_launch(with_env, "int x;\n", launch), Ok(7));
    }

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
