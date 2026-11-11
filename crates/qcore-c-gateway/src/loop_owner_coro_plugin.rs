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

//! Three premises of the loop-owner census derived from the program (card
//! **W47-PREMISES-CORO-PLUGIN**, gateway rule 10p's amendment).
//!
//! Rule 10p's earlier cards stated three premises the census's discharges
//! rest on. This module derives each from the census's own lexical reading
//! and the typed member facts of the recorded build, and says what is left:
//!
//! 1. **P-CORO-HANDLE.** An input coroutine's handle (`struct flb_coro *`)
//!    and the collector it runs are confined to a closed set of functions:
//!    the constructor, the hand-off, the entry, the resume wrapper and the
//!    destruction. Every typed access of the members holding them
//!    (`struct flb_input_coro::coro`, `struct flb_libco_in_params::coro` and
//!    `::coll`) stands in one of those functions; inside them a value read
//!    from such a member, or a local or parameter holding one, is only
//!    dereferenced, tested, stored back into such a member, or passed to a
//!    listed parameter of a listed function; `input_coro_list` is only
//!    initialised and appended to (never read through). What is left is the
//!    running coroutine's handle of itself (the thread-local current
//!    coroutine `flb_coro_resume` sets, read by `flb_coro_get()`, and `co_active()`),
//!    named **P-CURRENT-HANDLE**: no thread resumes a coroutine while it runs.
//! 2. **P-PLUGIN-DIR.** (a) A `struct flb_input_plugin`'s `flags` is written
//!    at run time only into a fresh allocation, before the allocation is
//!    named anywhere but in its own member stores; the instance's coroutine
//!    flag is written by the instance constructor alone, and every
//!    coroutine construction stands in the then-arm of a test of that flag.
//!    (b) Per collector registrant registered in `plugins/D`: every
//!    registration's instance is a parameter chained, through direct calls
//!    passing unassigned parameters, to a callback stored only in `D`'s
//!    descriptor initializers (P-CALLBACK-INSTANCE derived for those fields,
//!    the instance's plugin written only by its constructor or by installing
//!    a copy of the instance's own plugin into which no plugin function is
//!    stored) or to a collector callback of `D` so registered. A registrant
//!    for which (b) is not derived keeps the coroutine route.
//! 3. **The immutable threaded flag.** Every write of the instance's
//!    threaded flag is in the instance constructor, in a function writing an
//!    instance it created itself (from an allocator or the constructor), or
//!    in the property setter's arm for the `threaded` key. A setter call whose
//!    key the typed reading reads as one other literal cannot reach that arm.
//!    What is left, named **P-THREADED-SET-BEFORE-START**: the listed setter
//!    calls and creation writes happen before the instance they write starts
//!    (its collectors are attached to a loop by the flag's value then).
//!
//! Each premise is derived only when the typed reading recorded the members
//! it reads; a violation, or an access the reading could not classify,
//! leaves it underived, and the census then withholds every discharge that
//! rests on it (the coroutine entry becomes unresolved and the route is
//! never dropped; a directory's flag is not used; every mode-fixing edge's
//! callee becomes unresolved). A reading without typed facts derives none
//! of them and keeps the earlier cards' statements (no measured reading is
//! untyped).

use std::collections::{BTreeMap, BTreeSet};

use super::loop_owner_instance::{address_taken, assignments, call_arguments, ordered_parameters};
use super::{Definition, HopTables, Keyed, StepB};
use crate::legacy_c::is_identifier;
use crate::typed_dispatch_census::{ArgumentCall, TypedFacts, member_key};

/// A member holding an input coroutine's handle or collector (P-CORO-HANDLE).
#[derive(Clone, Copy, Debug)]
pub struct HandleMember {
    pub record: &'static str,
    pub field: &'static str,
}

/// One function of the closed set: the parameters that carry a handle.
#[derive(Clone, Copy, Debug)]
pub struct HandleFunction {
    pub name: &'static str,
    pub handle_parameters: &'static [usize],
}

/// Fluent Bit's names for this card's derivations, as read at the `90978ac6`
/// export.
#[derive(Clone, Copy, Debug)]
pub struct CoroPluginTables {
    /// `include/fluent-bit/flb_input.h:588` (`struct flb_input_coro::coro`),
    /// `:629` (`struct flb_libco_in_params`).
    pub handle_members: &'static [HandleMember],
    /// The functions the handle is confined to.
    pub handle_functions: &'static [HandleFunction],
    /// Calls a handle may be passed to without being kept (logging, the
    /// allocator's release, the stack-size macro).
    pub inert_callees: &'static [&'static str],
    /// Functions of the set that create an input coroutine or switch into
    /// one: called only from inside the set.
    pub confined_calls: &'static [&'static str],
    /// `input_coro_list` of the instance and of its worker.
    pub list_members: &'static [HandleMember],
    /// The only uses of a list member that do not read through it.
    pub list_uses: &'static [&'static str],
    /// Calls that return the running coroutine's handle (P-CURRENT-HANDLE).
    pub current_handle_calls: &'static [&'static str],
    /// Allocators whose result is a fresh object.
    pub allocators: &'static [&'static str],
    /// The instance's threaded flag (`struct flb_input_instance::is_threaded`).
    pub threaded_field: &'static str,
    /// The property setter, its key argument, the key that selects the
    /// threaded arm and the comparison that selects an arm.
    pub property_setter: (&'static str, usize),
    pub threaded_key: &'static str,
    pub key_check: &'static str,
    /// The instance's link into the loop: a collector registration's
    /// constructor, which attaches a collector to the loop the flag selects.
    pub collector_constructor: &'static str,
}

/// Fluent Bit at the `90978ac6` export: `include/fluent-bit/flb_input.h`
/// (`flb_input_coro_create` :597, `input_params_set` :639,
/// `input_pre_cb_collect` :663, `flb_input_coro_resume` :687,
/// `flb_input_coro_collect` :700), `include/fluent-bit/flb_coro.h`
/// (`flb_coro_destroy` :93, `flb_coro_resume` :118, `flb_coro_create` :125),
/// `src/flb_input.c` (`flb_input_coro_destroy` :723, `flb_input_new` :326,
/// `flb_input_set_property`'s `threaded` arm :994-1003, `prop_key_check`
/// :689), `src/flb_plugin_proxy.c:390` (the proxy's fresh descriptor).
pub const FLUENT_BIT_CORO_PLUGIN: CoroPluginTables = CoroPluginTables {
    handle_members: &[
        HandleMember {
            record: "struct flb_input_coro",
            field: "coro",
        },
        HandleMember {
            record: "struct flb_libco_in_params",
            field: "coro",
        },
        HandleMember {
            record: "struct flb_libco_in_params",
            field: "coll",
        },
    ],
    handle_functions: &[
        HandleFunction {
            name: "flb_input_coro_create",
            handle_parameters: &[],
        },
        HandleFunction {
            name: "flb_input_coro_collect",
            handle_parameters: &[],
        },
        HandleFunction {
            name: "input_params_set",
            handle_parameters: &[0, 1],
        },
        HandleFunction {
            name: "input_pre_cb_collect",
            handle_parameters: &[],
        },
        HandleFunction {
            name: "flb_input_coro_resume",
            handle_parameters: &[],
        },
        HandleFunction {
            name: "flb_coro_resume",
            handle_parameters: &[0],
        },
        HandleFunction {
            name: "flb_input_coro_destroy",
            handle_parameters: &[],
        },
        HandleFunction {
            name: "flb_coro_destroy",
            handle_parameters: &[0],
        },
    ],
    inert_callees: &["flb_free", "flb_trace", "FLB_CORO_DATA", "flb_coro_set"],
    confined_calls: &["flb_input_coro_create", "input_params_set"],
    list_members: &[
        HandleMember {
            record: "struct flb_input_instance",
            field: "input_coro_list",
        },
        HandleMember {
            record: "struct flb_input_thread_instance",
            field: "input_coro_list",
        },
    ],
    list_uses: &["arg:mk_list_init", "arg:mk_list_add"],
    current_handle_calls: &["flb_coro_get", "co_active"],
    allocators: &["flb_calloc", "flb_malloc", "calloc", "malloc"],
    threaded_field: "is_threaded",
    property_setter: ("flb_input_set_property", 1),
    threaded_key: "threaded",
    key_check: "prop_key_check",
    collector_constructor: "collector_create",
};

impl CoroPluginTables {
    /// The members whose accesses the typed reading records for this card.
    #[must_use]
    pub fn member_fields(&self, hops: &HopTables) -> Vec<(String, String)> {
        self.handle_members
            .iter()
            .chain(self.list_members)
            .map(|m| (m.record.to_owned(), m.field.to_owned()))
            .chain([
                (
                    format!("struct {}", hops.plugin_descriptor),
                    hops.plugin_flags_field.to_owned(),
                ),
                (
                    "struct flb_input_instance".to_owned(),
                    self.threaded_field.to_owned(),
                ),
            ])
            .collect()
    }

    /// The setter's key literal.
    #[must_use]
    pub fn literal_calls(&self) -> Vec<(String, usize)> {
        vec![(self.property_setter.0.to_owned(), self.property_setter.1)]
    }

    /// The setter's arm comparisons.
    #[must_use]
    pub fn argument_calls(&self) -> Vec<ArgumentCall> {
        vec![ArgumentCall {
            callee: self.key_check.to_owned(),
            first: 0,
            caller: Some(self.property_setter.0.to_owned()),
        }]
    }
}

/// What this card derived: per premise, `None` when there is no typed
/// reading (the premise stays stated), else the derivation's notes or why it
/// is not derived.
#[derive(Clone, Debug, Default)]
pub struct PremiseReading {
    pub coro_handle: Option<Result<Vec<String>, String>>,
    pub descriptor_flags: Option<Result<Vec<String>, String>>,
    pub threaded_flag: Option<Result<Vec<String>, String>>,
    /// Per collector registrant registered only in `plugins/`: whether its
    /// registrations are derived on its own plugins' instances.
    pub registrations: BTreeMap<usize, Result<String, String>>,
    /// Named residual premises and the sites each stands for.
    pub residuals: BTreeMap<String, Vec<String>>,
}

impl PremiseReading {
    /// Whether the coroutine route may be dropped for an input derived to run
    /// no coroutine.
    #[must_use]
    pub fn route_may_be_dropped(&self) -> bool {
        !matches!(self.coro_handle, Some(Err(_))) && !matches!(self.descriptor_flags, Some(Err(_)))
    }

    /// The report lines of each premise.
    #[must_use]
    pub fn report(&self, functions: &[String]) -> BTreeMap<String, Vec<String>> {
        let mut report = BTreeMap::new();
        let line = |derivation: &Option<Result<Vec<String>, String>>| match derivation {
            None => vec!["stated: no typed reading".to_owned()],
            Some(Ok(notes)) => std::iter::once("derived".to_owned())
                .chain(notes.iter().cloned())
                .collect(),
            Some(Err(why)) => vec![format!("not derived: {why}")],
        };
        report.insert("P-CORO-HANDLE".to_owned(), line(&self.coro_handle));
        report.insert(
            "P-PLUGIN-DIR descriptor flags".to_owned(),
            line(&self.descriptor_flags),
        );
        report.insert(
            "immutable threaded flag".to_owned(),
            line(&self.threaded_flag),
        );
        report.insert(
            "P-PLUGIN-DIR registrations".to_owned(),
            self.registrations
                .iter()
                .map(|(id, result)| match result {
                    Ok(why) => format!("{}: derived ({why})", functions[*id]),
                    Err(why) => format!("{}: not derived ({why})", functions[*id]),
                })
                .collect(),
        );
        for (name, sites) in &self.residuals {
            report.insert(format!("residual {name}"), sites.clone());
        }
        report
    }
}

/// A typed member access: file, line, function, use.
struct Access {
    file: String,
    line: usize,
    function: String,
    use_label: String,
}

impl Access {
    fn at(&self) -> String {
        format!(
            "{}:{} in `{}` ({})",
            self.file, self.line, self.function, self.use_label
        )
    }
}

fn accesses(typed: &TypedFacts, record: &str, field: &str) -> Vec<Access> {
    typed
        .member_accesses
        .get(&member_key(record, field))
        .into_iter()
        .flatten()
        .filter_map(|site| {
            let mut parts = site.splitn(4, '|');
            Some(Access {
                file: parts.next()?.to_owned(),
                line: parts.next()?.parse().ok()?,
                function: parts.next()?.to_owned(),
                use_label: parts.next()?.to_owned(),
            })
        })
        .collect()
}

/// Whether `texts[k]` begins an assignment operator's left operand end:
/// `=` (not `==`), a compound assignment, or an increment/decrement.
fn assigned_after(texts: &[&str], k: usize) -> bool {
    let at = |i: usize| texts.get(i).copied().unwrap_or("");
    match (at(k), at(k + 1)) {
        ("=", next) => next != "=",
        ("|" | "&" | "^" | "+" | "-" | "*" | "/" | "%", "=") => true,
        ("+", "+") | ("-", "-") => true,
        ("<", "<") | (">", ">") => at(k + 2) == "=",
        _ => false,
    }
}

/// The member tokens `B -> F` / `B . F` of `field` on `line` inside `range`.
fn member_tokens(
    texts: &[&str],
    lines: &[usize],
    range: (usize, usize),
    line: usize,
    field: &str,
) -> Vec<usize> {
    (range.0..=range.1)
        .filter(|&k| {
            lines[k] == line
                && texts[k] == field
                && (texts[k - 1] == "." || (texts[k - 1] == ">" && texts[k - 2] == "-"))
        })
        .collect()
}

/// The object a member token `k` goes through, when it is one identifier
/// (`B -> F`, `B . F`).
fn member_base<'a>(texts: &[&'a str], k: usize) -> Option<&'a str> {
    let base = if texts[k - 1] == "." { k - 2 } else { k - 3 };
    let name = texts.get(base)?;
    (is_identifier(name) && !matches!(texts.get(base.wrapping_sub(1)), Some(&"." | &">")))
        .then_some(*name)
}

impl StepB<'_> {
    fn definitions_named(&self, name: &str) -> Vec<&Definition> {
        self.definitions.iter().filter(|d| d.name == name).collect()
    }

    fn lines(&self, file: usize) -> Vec<usize> {
        self.scans[file].tokens.iter().map(|t| t.line).collect()
    }

    /// Whether a definition is in the program without a typed reading:
    /// `Some(true)` when Clang read it; `Some(false)` when no unit compiled it
    /// (a `.c` file the typed reading read without it, or one no command
    /// names; a header no unit Clang could not read may include); `None`
    /// when a unit Clang could not read may compile it.
    fn typed_read(
        &self,
        typed: &TypedFacts,
        failed: &BTreeSet<usize>,
        definition: &Definition,
    ) -> Option<bool> {
        let path = &self.files[definition.file].path;
        if typed
            .functions
            .contains(&format!("{path}|{}", definition.name))
        {
            return Some(true);
        }
        (!failed.contains(&definition.file)).then_some(false)
    }

    /// The files a unit Clang could not read may compile: the unit and,
    /// through `#include`s matched by file name over the read files, every
    /// header it may reach.
    fn failed_units_files(&self, typed: &TypedFacts) -> BTreeSet<usize> {
        let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (file, source) in self.files.iter().enumerate() {
            let name = source.path.rsplit('/').next().unwrap_or(&source.path);
            by_name.entry(name).or_default().push(file);
        }
        let mut seen = BTreeSet::new();
        let mut stack = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| typed.failures.contains_key(&f.path))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        while let Some(file) = stack.pop() {
            if !seen.insert(file) {
                continue;
            }
            for line in self.files[file].text.lines() {
                let Some(rest) = line.trim_start().strip_prefix('#') else {
                    continue;
                };
                let Some(rest) = rest.trim_start().strip_prefix("include") else {
                    continue;
                };
                let name = rest
                    .trim()
                    .trim_start_matches(['"', '<'])
                    .split(['"', '>'])
                    .next()
                    .unwrap_or("");
                let base = name.rsplit('/').next().unwrap_or(name);
                stack.extend(by_name.get(base).into_iter().flatten().copied());
            }
        }
        seen
    }

    /// A function the typed reading did not read, which a unit it could not
    /// read may compile, that names `field` as a member (written, when
    /// `writes_only`).
    fn untyped_member(&self, typed: &TypedFacts, field: &str, writes_only: bool) -> Option<String> {
        let failed = self.failed_units_files(typed);
        for definition in self.definitions {
            if self.typed_read(typed, &failed, definition).is_some() {
                continue;
            }
            let texts = self.texts(definition.file);
            let found = (definition.body.0..=definition.body.1).find(|&k| {
                texts[k] == field
                    && (texts[k - 1] == "." || (texts[k - 1] == ">" && texts[k - 2] == "-"))
                    && (!writes_only || assigned_after(&texts, k + 1))
            });
            if let Some(k) = found {
                return Some(format!(
                    "`{}` names `{field}` at {}:{} and has no typed reading",
                    self.functions[self.index[&definition.census_name]],
                    self.files[definition.file].path,
                    self.scans[definition.file].tokens[k].line
                ));
            }
        }
        None
    }

    /// Every function reachable from `roots` over the census graph.
    fn closure(&self, roots: &[usize]) -> BTreeSet<usize> {
        let mut seen = BTreeSet::new();
        let mut stack = roots.to_vec();
        while let Some(at) = stack.pop() {
            if seen.insert(at) {
                stack.extend(self.reach.next.get(&at).into_iter().flatten().copied());
            }
        }
        seen
    }

    /// The census ids of the calls between tokens `from` and `to` of `g`.
    fn calls_between(&self, g: &Definition, from: usize, to: usize) -> Vec<usize> {
        let texts = self.texts(g.file);
        let mut ids = Vec::new();
        for k in from..to {
            if is_identifier(texts[k])
                && texts.get(k + 1) == Some(&"(")
                && !matches!(texts[k - 1], "." | ">")
            {
                ids.extend(self.resolve_name(g.file, texts[k]));
            }
        }
        ids
    }

    /// Whether a call between two points of `g` may reach a collector
    /// registration: its closure over the census graph holds the collector
    /// constructor, a function the typed reading could not read, or an
    /// unmodelled indirect call while some escaped function reaches the
    /// constructor.
    fn may_register_between(
        &self,
        typed: &TypedFacts,
        tables: &CoroPluginTables,
        g: &Definition,
        from: usize,
        to: usize,
    ) -> Option<String> {
        let calls = self.calls_between(g, from, to);
        let closure = self.closure(&calls);
        let constructors = self
            .definitions_named(tables.collector_constructor)
            .iter()
            .map(|d| self.index[&d.census_name])
            .collect::<BTreeSet<_>>();
        if let Some(hit) = closure.intersection(&constructors).next() {
            return Some(format!(
                "a call before it reaches `{}`",
                self.functions[*hit]
            ));
        }
        for id in &closure {
            if self.reach.unstructured.contains(id) {
                return Some(format!(
                    "a call before it reaches `{}`, in a file this reader cannot structure",
                    self.functions[*id]
                ));
            }
        }
        let indirect = closure.iter().any(|id| {
            let name = &self.functions[*id];
            let plain = name.rsplit_once("::").map_or(name.as_str(), |(_, p)| p);
            self.definitions
                .iter()
                .filter(|d| self.index[&d.census_name] == *id)
                .any(|d| {
                    let prefix = format!("{}|{plain}|", self.files[d.file].path);
                    typed
                        .indirect_calls
                        .range(prefix.clone()..)
                        .next()
                        .is_some_and(|key| key.starts_with(&prefix))
                })
        });
        if indirect {
            let wild = self.reach.wild.keys().copied().collect::<Vec<_>>();
            if !self.closure(&wild).is_disjoint(&constructors) {
                return Some(
                    "a call before it makes an unmodelled indirect call, and an escaped function \
                     reaches a collector registration"
                        .to_owned(),
                );
            }
        }
        None
    }

    /// Derive this card's premises.
    pub(super) fn coro_plugin_premises(&self, tables: &CoroPluginTables) -> PremiseReading {
        let mut reading = PremiseReading::default();
        let Some(typed) = self.typed else {
            return reading;
        };
        let mut residuals: BTreeMap<String, Vec<String>> = BTreeMap::new();
        reading.coro_handle = Some(self.coro_handle(typed, tables, &mut residuals));
        reading.descriptor_flags = Some(self.descriptor_flags(typed, tables));
        reading.threaded_flag = Some(self.threaded_flag(typed, tables, &mut residuals));
        reading.registrations = self.registration_instances(typed);
        reading.residuals = residuals;
        reading
    }

    // ---- P-CORO-HANDLE ---------------------------------------------------

    fn coro_handle(
        &self,
        typed: &TypedFacts,
        tables: &CoroPluginTables,
        residuals: &mut BTreeMap<String, Vec<String>>,
    ) -> Result<Vec<String>, String> {
        let closed = tables
            .handle_functions
            .iter()
            .map(|f| f.name)
            .collect::<BTreeSet<_>>();
        let mut notes = Vec::new();
        for member in tables.handle_members.iter().chain(tables.list_members) {
            if !typed
                .member_fields_read
                .contains(&member_key(member.record, member.field))
            {
                return Err(format!(
                    "the typed reading did not record `{}::{}`",
                    member.record, member.field
                ));
            }
        }
        for member in tables.handle_members {
            let found = accesses(typed, member.record, member.field);
            for access in &found {
                if !closed.contains(access.function.as_str()) {
                    return Err(format!(
                        "`{}::{}` is accessed at {}, outside the coroutine's own functions",
                        member.record,
                        member.field,
                        access.at()
                    ));
                }
            }
            notes.push(format!(
                "`{}::{}` accessed only in {}",
                member.record,
                member.field,
                found
                    .iter()
                    .map(|a| format!("`{}` ({})", a.function, a.use_label))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for member in tables.list_members {
            for access in accesses(typed, member.record, member.field) {
                if !tables.list_uses.contains(&access.use_label.as_str()) {
                    return Err(format!(
                        "`{}::{}` is used at {}: a coroutine may be taken from it",
                        member.record,
                        member.field,
                        access.at()
                    ));
                }
            }
        }
        notes.push("`input_coro_list` is only initialised and appended to".to_owned());
        let fields = tables
            .handle_members
            .iter()
            .chain(tables.list_members)
            .map(|m| m.field)
            .collect::<BTreeSet<_>>();
        for field in &fields {
            if let Some(access) = accesses(typed, "?", field).first() {
                return Err(format!(
                    "a `{field}` member of an unmatched record is accessed at {}",
                    access.at()
                ));
            }
            if let Some(why) = self.untyped_member(typed, field, false) {
                return Err(why);
            }
        }
        // The set's creation and hand-off are called from inside the set only.
        for name in tables.confined_calls {
            let ids = self
                .definitions_named(name)
                .iter()
                .map(|d| self.index[&d.census_name])
                .collect::<BTreeSet<_>>();
            if ids.iter().any(|id| self.taken.contains(id)) {
                return Err(format!("`{name}` is named other than by a call"));
            }
            for g in self.definitions {
                if closed.contains(g.name.as_str()) {
                    continue;
                }
                let texts = self.texts(g.file);
                for k in g.body.0..=g.body.1 {
                    if texts[k] == *name
                        && texts.get(k + 1) == Some(&"(")
                        && !matches!(texts[k - 1], "." | ">")
                        && !g.parameters.contains(*name)
                        && self
                            .resolve_name(g.file, name)
                            .iter()
                            .any(|id| ids.contains(id))
                    {
                        return Err(format!(
                            "`{name}` is called at {}:{} from `{}`, outside the coroutine's own \
                             functions",
                            self.files[g.file].path, self.scans[g.file].tokens[k].line, g.name
                        ));
                    }
                }
            }
        }
        notes.push(format!(
            "{} are called only inside the set",
            tables
                .confined_calls
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(" and ")
        ));
        // Inside the closed set: a handle value is only dereferenced, tested,
        // stored into a handle member, or passed to a listed parameter.
        for function in tables.handle_functions {
            for definition in self.definitions_named(function.name) {
                self.confined(definition, function, tables)?;
            }
        }
        notes.push(format!(
            "inside {} a handle is only dereferenced, tested, stored into a handle member or \
             passed on within the set",
            closed
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        // P-CURRENT-HANDLE: the running coroutine's handle of itself, at
        // every call in the program.
        let failed = self.failed_units_files(typed);
        let mut current = Vec::new();
        for definition in self.definitions {
            if self.typed_read(typed, &failed, definition) == Some(false) {
                continue;
            }
            let texts = self.texts(definition.file);
            for k in definition.body.0..=definition.body.1 {
                if tables.current_handle_calls.contains(&texts[k])
                    && texts.get(k + 1) == Some(&"(")
                    && !matches!(texts[k - 1], "." | ">")
                {
                    current.push(format!(
                        "{}:{} `{}`",
                        self.files[definition.file].path,
                        self.scans[definition.file].tokens[k].line,
                        texts[k]
                    ));
                }
            }
        }
        current.sort();
        current.dedup();
        residuals.insert("P-CURRENT-HANDLE".to_owned(), current);
        Ok(notes)
    }

    /// The handle stays inside `definition` or the closed set.
    fn confined(
        &self,
        definition: &Definition,
        function: &HandleFunction,
        tables: &CoroPluginTables,
    ) -> Result<(), String> {
        let texts = self.texts(definition.file);
        let body = definition.body;
        let path = &self.files[definition.file].path;
        let line = |k: usize| self.scans[definition.file].tokens[k].line;
        let parameters = ordered_parameters(&texts, definition.header);
        let mut handles = function
            .handle_parameters
            .iter()
            .filter_map(|i| parameters.get(*i).cloned())
            .collect::<BTreeSet<_>>();
        let member_fields = tables
            .handle_members
            .iter()
            .map(|m| m.field)
            .collect::<BTreeSet<_>>();
        // Locals assigned from a handle member or a coroutine creation, to a
        // fixed point over local copies.
        loop {
            let before = handles.len();
            for k in body.0..=body.1 {
                if !is_identifier(texts[k])
                    || matches!(texts[k - 1], "." | ">")
                    || texts.get(k + 1) != Some(&"=")
                    || texts.get(k + 2) == Some(&"=")
                {
                    continue;
                }
                let end = (k + 2..=body.1)
                    .find(|i| texts[*i] == ";")
                    .unwrap_or(body.1);
                let rhs = &texts[k + 2..end];
                let from_member = rhs.len() >= 3
                    && member_fields.contains(rhs[rhs.len() - 1])
                    && matches!(rhs[rhs.len() - 2], "." | ">");
                let from_handle = matches!(rhs, [one] if handles.contains(*one));
                let from_creation = rhs.contains(&"flb_coro_create");
                if from_member || from_handle || from_creation {
                    handles.insert(texts[k].to_owned());
                }
            }
            if handles.len() == before {
                break;
            }
        }
        let callee_of = |k: usize| -> Option<(&str, usize)> {
            // The call whose argument list holds position k, and the index.
            let mut depth = 0isize;
            let mut index = 0usize;
            for i in (body.0..k).rev() {
                match texts[i] {
                    ")" | "]" | "}" => depth += 1,
                    "[" | "{" => {
                        if depth == 0 {
                            return None;
                        }
                        depth -= 1;
                    }
                    "(" => {
                        if depth == 0 {
                            let name = texts[i - 1];
                            return is_identifier(name).then_some((name, index));
                        }
                        depth -= 1;
                    }
                    "," if depth == 0 => index += 1,
                    ";" if depth == 0 => return None,
                    _ => {}
                }
            }
            None
        };
        let listed_parameter = |callee: &str, index: usize| {
            tables
                .handle_functions
                .iter()
                .any(|f| f.name == callee && f.handle_parameters.contains(&index))
                || tables.inert_callees.contains(&callee)
        };
        for k in body.0..=body.1 {
            // A handle expression ends at k: a handle local, or a handle
            // member read `X -> F`.
            let is_local = handles.contains(texts[k]) && !matches!(texts[k - 1], "." | ">");
            let is_member = member_fields.contains(texts[k])
                && (texts[k - 1] == "." || (texts[k - 1] == ">" && texts[k - 2] == "-"));
            if !is_local && !is_member {
                continue;
            }
            // A declaration's name or an assignment's target.
            if is_local && texts.get(k + 1) == Some(&"=") && texts.get(k + 2) != Some(&"=") {
                continue;
            }
            if is_local
                && matches!(texts.get(k + 1), Some(&";" | &","))
                && (texts[k - 1] == "*" || is_identifier(texts[k - 1]))
                && texts[k - 1] != "return"
            {
                continue;
            }
            let next = texts.get(k + 1).copied().unwrap_or("");
            let after = texts.get(k + 2).copied().unwrap_or("");
            // Dereferenced.
            if (next == "-" && after == ">") || next == "." {
                continue;
            }
            // A member store's target (`X -> coro = …`).
            if is_member && next == "=" && after != "=" {
                continue;
            }
            // Tested.
            if texts[k - 1] == "!"
                || (next == "=" && after == "=")
                || (next == "!" && after == "=")
                || (texts[k - 1] == "(" && next == ")" && matches!(texts[k - 2], "if" | "while"))
            {
                continue;
            }
            // The whole right-hand side of a store into a handle member or a
            // handle local.
            if next == ";" {
                let statement_start = (body.0..k)
                    .rev()
                    .find(|i| matches!(texts[*i], ";" | "{" | "}"))
                    .map_or(body.0, |i| i + 1);
                let statement = &texts[statement_start..=k];
                let equals = statement
                    .iter()
                    .enumerate()
                    .filter(|(i, t)| {
                        **t == "="
                            && statement.get(i + 1) != Some(&"=")
                            && !matches!(
                                statement.get(i.wrapping_sub(1)),
                                Some(&"=" | &"!" | &"<" | &">")
                            )
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                if let [e] = equals[..] {
                    let target = &statement[..e];
                    let value = &statement[e + 1..];
                    let whole = value.len() == 1
                        || (value.len() >= 3
                            && is_identifier(value[0])
                            && value[1..value.len() - 1]
                                .chunks(2)
                                .all(|c| matches!(c, ["-", ">"] | [".", _]))
                            && value[1..].len() % 2 == 1);
                    let into_member = target.len() >= 3
                        && member_fields.contains(target[target.len() - 1])
                        && matches!(target[target.len() - 2], "." | ">");
                    let into_local = matches!(target.last(), Some(t) if handles.contains(*t));
                    if whole && (into_member || into_local) {
                        continue;
                    }
                }
            }
            // Passed to a listed parameter.
            if matches!(next, "," | ")")
                && let Some((callee, index)) = callee_of(k)
                && listed_parameter(callee, index)
            {
                continue;
            }
            return Err(format!(
                "an input coroutine's handle leaves `{}` at {path}:{}",
                function.name,
                line(k)
            ));
        }
        Ok(())
    }

    // ---- P-PLUGIN-DIR (a): descriptor flags ------------------------------

    fn descriptor_flags(
        &self,
        typed: &TypedFacts,
        tables: &CoroPluginTables,
    ) -> Result<Vec<String>, String> {
        let hops = self.hops;
        let record = format!("struct {}", hops.plugin_descriptor);
        let field = hops.plugin_flags_field;
        if !typed
            .member_fields_read
            .contains(&member_key(&record, field))
        {
            return Err(format!(
                "the typed reading did not record `{record}::{field}`"
            ));
        }
        let mut notes = Vec::new();
        for access in accesses(typed, &record, field) {
            match access.use_label.as_str() {
                "read" | "address-read" => {}
                "write" => {
                    let fresh = self.fresh_write(tables, &access, field).map_err(|why| {
                        format!("`{record}::{field}` is written at {}: {why}", access.at())
                    })?;
                    notes.push(fresh);
                }
                _ => {
                    return Err(format!(
                        "`{record}::{field}`'s address is taken at {}",
                        access.at()
                    ));
                }
            }
        }
        for access in accesses(typed, "?", field) {
            if access.use_label != "read" && access.use_label != "address-read" {
                return Err(format!(
                    "a `{field}` member of an unmatched record is written at {}",
                    access.at()
                ));
            }
        }
        // A function the typed reading did not read that writes a `flags`
        // member of a record the census cannot name.
        if let Some(why) = self.untyped_member(typed, field, true) {
            return Err(why);
        }
        // The instance's coroutine flag: the constructor alone.
        let constructor = hops.instance_constructor.0;
        let writers = self
            .flag_writers
            .iter()
            .map(|w| w.map(|id| self.functions[id].as_str()))
            .collect::<BTreeSet<_>>();
        if writers != BTreeSet::from([Some(constructor)]) {
            return Err(format!(
                "`{}` is written other than by `{constructor}` alone",
                hops.instance_flag_field
            ));
        }
        notes.push(format!(
            "`{}` is written by `{constructor}` alone",
            hops.instance_flag_field
        ));
        // Every coroutine construction tests the flag first.
        for coroutine in hops.coroutines {
            for definition in self.definitions {
                let texts = self.texts(definition.file);
                for k in definition.body.0..=definition.body.1 {
                    if texts[k] != coroutine.constructor
                        || texts.get(k + 1) != Some(&"(")
                        || matches!(texts[k - 1], "." | ">")
                    {
                        continue;
                    }
                    if !self.under_flag_test(definition, k) {
                        return Err(format!(
                            "`{}` is called at {}:{} outside a test of `{}`",
                            coroutine.constructor,
                            self.files[definition.file].path,
                            self.scans[definition.file].tokens[k].line,
                            hops.instance_flag_field
                        ));
                    }
                }
            }
        }
        notes.push(
            "rests on P-REGISTERED-DESCRIPTORS (the registered plugins are the build's \
             descriptors) and P-DESCRIPTOR-FLAGS-BY-MEMBER (a descriptor's flags are written only \
             through member accesses)"
                .to_owned(),
        );
        notes.push(format!(
            "every `{}` call stands in the then-arm of a test of its collector's `{}`",
            hops.coroutines
                .iter()
                .map(|c| c.constructor)
                .collect::<Vec<_>>()
                .join("`, `"),
            hops.instance_flag_field
        ));
        Ok(notes)
    }

    /// Whether the constructor call at token `k` of `definition` stands in
    /// the then-block of `if (C -> instance -> FLAG)`, `C` its own collector
    /// argument, assigned nowhere between the test and the call.
    fn under_flag_test(&self, definition: &Definition, k: usize) -> bool {
        let texts = self.texts(definition.file);
        let flag = self.hops.instance_flag_field;
        let instance = self.flow.collector_instance_field;
        let Some(collector) = call_arguments(&texts, k + 1).and_then(|(arguments, _)| {
            arguments
                .first()
                .filter(|(s, e)| e - s == 1 && is_identifier(texts[*s]))
                .map(|(s, _)| texts[*s])
        }) else {
            return false;
        };
        (definition.body.0..k).any(|i| {
            if texts[i] != "if" || texts.get(i + 1) != Some(&"(") {
                return false;
            }
            let Some((_, close)) = call_arguments(&texts, i + 1) else {
                return false;
            };
            let condition = &texts[i + 2..close];
            if condition != [collector, "-", ">", instance, "-", ">", flag].as_slice()
                || texts.get(close + 1) != Some(&"{")
            {
                return false;
            }
            let Some(end) = matching_brace(&texts, close + 1) else {
                return false;
            };
            close + 1 < k && k < end && assignments(&texts, (close, k), collector).is_empty()
        })
    }

    /// A write `B -> F op …` at `access` into a fresh allocation: `B` a local
    /// assigned once, from an allocator (behind at most a cast), and named
    /// between that assignment and the write only in its own member stores
    /// and null tests.
    fn fresh_write(
        &self,
        tables: &CoroPluginTables,
        access: &Access,
        field: &str,
    ) -> Result<String, String> {
        let definition = self
            .definitions
            .iter()
            .find(|d| d.name == access.function && self.files[d.file].path == access.file)
            .ok_or("the writing function is not read")?;
        let texts = self.texts(definition.file);
        let lines = self.lines(definition.file);
        let writes = member_tokens(&texts, &lines, definition.body, access.line, field)
            .into_iter()
            .filter(|k| assigned_after(&texts, k + 1))
            .collect::<Vec<_>>();
        let [w] = writes[..] else {
            return Err("the write is not one member store".to_owned());
        };
        let base = member_base(&texts, w).ok_or("the written object is not a local")?;
        if definition.parameters.contains(base) {
            return Err(format!("the written object `{base}` is a parameter"));
        }
        let assigned = creations(&texts, definition.body, base);
        let [a] = assigned[..] else {
            return Err(format!("`{base}` is not assigned exactly once"));
        };
        if a > w || address_taken(&texts, definition.body, base) {
            return Err(format!("`{base}` is not a fresh object at the write"));
        }
        let mut rhs = a + 2;
        if texts[rhs] == "(" {
            // A cast `( T * )`.
            let (_, close) = call_arguments(&texts, rhs).ok_or("an unreadable cast")?;
            rhs = close + 1;
        }
        if !tables.allocators.contains(&texts[rhs]) || texts.get(rhs + 1) != Some(&"(") {
            return Err(format!("`{base}` is not assigned from an allocator"));
        }
        let (_, close) = call_arguments(&texts, rhs + 1).ok_or("an unreadable allocation")?;
        for k in close + 1..w {
            if texts[k] != base || matches!(texts[k - 1], "." | ">") {
                continue;
            }
            let member_store = texts.get(k + 1) == Some(&"-")
                && texts.get(k + 2) == Some(&">")
                && texts.get(k + 3).is_some_and(|f| is_identifier(f))
                && assigned_after(&texts, k + 4);
            let tested = texts[k - 1] == "!"
                || (texts.get(k + 1) == Some(&"=") && texts.get(k + 2) == Some(&"="));
            if !(member_store || tested) {
                return Err(format!(
                    "`{base}` is used at {}:{} before the write",
                    self.files[definition.file].path, lines[k]
                ));
            }
        }
        Ok(format!(
            "`{}` writes `{field}` into its fresh allocation `{base}` at {}:{} before naming it \
             anywhere else",
            access.function, access.file, access.line
        ))
    }

    // ---- P-PLUGIN-DIR (b): registrations on the plugin's own instances ----

    fn registration_instances(
        &self,
        typed: &TypedFacts,
    ) -> BTreeMap<usize, Result<String, String>> {
        let hops = self.hops;
        let mut by_registrant: BTreeMap<usize, Vec<Registration<'_>>> = BTreeMap::new();
        for (registrant, function, instance, file) in self.collector_registrations {
            by_registrant.entry(*registrant).or_default().push((
                *function,
                instance.as_deref(),
                *file,
            ));
        }
        let p_writers = self.plugin_field_writers(typed);
        // Every call by name, by the name called: (definition, token).
        let mut calls: CallIndex = BTreeMap::new();
        for (d, g) in self.definitions.iter().enumerate() {
            let texts = self.texts(g.file);
            for k in g.body.0..=g.body.1 {
                if is_identifier(texts[k])
                    && texts.get(k + 1) == Some(&"(")
                    && !matches!(texts[k - 1], "." | ">" | "&")
                    && !g.parameters.contains(texts[k])
                {
                    calls.entry(texts[k].to_owned()).or_default().push((d, k));
                }
            }
        }
        let mut results = BTreeMap::new();
        for (registrant, registrations) in &by_registrant {
            let dirs = registrations
                .iter()
                .map(|(_, _, file)| plugin_dir(hops, &self.files[*file].path))
                .collect::<BTreeSet<_>>();
            let [Some(dir)] = dirs.iter().collect::<Vec<_>>()[..] else {
                continue;
            };
            let result = p_writers.clone().and_then(|()| {
                let mut visiting = BTreeSet::new();
                for (function, instance, _) in registrations {
                    self.registration_on_own_instance(
                        *function,
                        *instance,
                        dir,
                        &calls,
                        &mut visiting,
                    )?;
                }
                Ok(format!(
                    "every registration's instance comes from `plugins/{dir}`'s descriptor \
                     callbacks"
                ))
            });
            results.insert(*registrant, result);
        }
        results
    }

    fn registration_on_own_instance(
        &self,
        function: usize,
        instance: Option<&str>,
        dir: &str,
        calls: &CallIndex,
        visiting: &mut BTreeSet<(usize, usize)>,
    ) -> Result<(), String> {
        let name = &self.functions[function];
        let definition = self.single_definition(function)?;
        let Some(instance) = instance else {
            return Err(format!(
                "the registration in `{name}` does not pass a variable"
            ));
        };
        let texts = self.texts(definition.file);
        let parameters = ordered_parameters(&texts, definition.header);
        let Some(index) = parameters.iter().position(|p| p == instance) else {
            return Err(format!(
                "the registration in `{name}` is on `{instance}`, not a parameter"
            ));
        };
        self.instance_parameter(function, index, dir, calls, visiting)
    }

    fn single_definition(&self, function: usize) -> Result<&Definition, String> {
        let found = self
            .definitions
            .iter()
            .filter(|d| self.index[&d.census_name] == function)
            .collect::<Vec<_>>();
        match found[..] {
            [one] => Ok(one),
            _ => Err(format!(
                "`{}` is not defined once",
                self.functions[function]
            )),
        }
    }

    /// Every value of `function`'s `index`-th parameter is an instance created
    /// from a descriptor of `plugins/dir`.
    fn instance_parameter(
        &self,
        function: usize,
        index: usize,
        dir: &str,
        calls: &CallIndex,
        visiting: &mut BTreeSet<(usize, usize)>,
    ) -> Result<(), String> {
        if !visiting.insert((function, index)) {
            return Ok(());
        }
        let hops = self.hops;
        let name = self.functions[function].clone();
        let definition = self.single_definition(function)?;
        let texts = self.texts(definition.file);
        let parameters = ordered_parameters(&texts, definition.header);
        let Some(parameter) = parameters.get(index) else {
            return Err(format!("`{name}` has no parameter {index}"));
        };
        if !assignments(&texts, definition.body, parameter).is_empty()
            || address_taken(&texts, definition.body, parameter)
        {
            return Err(format!(
                "`{name}` reassigns its instance parameter `{parameter}`"
            ));
        }
        if self.taken.contains(&function) || self.spawns.iter().any(|(_, e)| *e == function) {
            return Err(format!(
                "`{name}` is named other than by a call or a descriptor"
            ));
        }
        let stores = self
            .field_stores
            .iter()
            .filter(|(id, ..)| *id == function)
            .collect::<Vec<_>>();
        let registered = self
            .collector_registrations
            .iter()
            .filter(|(r, ..)| *r == function)
            .collect::<Vec<_>>();
        let mut entry = false;
        let spans = if stores.is_empty() {
            Vec::new()
        } else {
            self.descriptor_spans(dir)
        };
        for (_, field, file, position) in &stores {
            let inside = spans
                .iter()
                .any(|(f, open, close)| f == file && open < position && position < close);
            if !inside {
                return Err(format!(
                    "`{name}` is stored in `{field}` outside `plugins/{dir}`'s descriptors"
                ));
            }
        }
        let fields = stores
            .iter()
            .map(|(_, f, ..)| f.clone())
            .filter(|f| *f != hops.collector_field)
            .collect::<BTreeSet<_>>();
        if !fields.is_empty() {
            if index != 0 {
                return Err(format!(
                    "`{name}` is a callback whose instance is not its first parameter"
                ));
            }
            let (keys, any_key) = self.registrant_keys(function);
            self.callback_instance(function, &fields, &keys, any_key)?;
            entry = true;
        }
        let collector_store = stores.iter().any(|(_, f, ..)| *f == hops.collector_field);
        if collector_store && registered.is_empty() {
            return Err(format!(
                "`{name}` is stored in a descriptor's `{}` and never registered",
                hops.collector_field
            ));
        }
        if !registered.is_empty() {
            if index != 0 {
                return Err(format!(
                    "`{name}` is a collector whose instance is not its first parameter"
                ));
            }
            for (_, registering, instance, file) in &registered {
                if plugin_dir(hops, &self.files[*file].path).as_deref() != Some(dir) {
                    return Err(format!("`{name}` is registered outside `plugins/{dir}`"));
                }
                self.registration_on_own_instance(
                    *registering,
                    instance.as_deref(),
                    dir,
                    calls,
                    visiting,
                )?;
            }
            self.collector_dispatch_passes_its_instance(function)?;
            let (keys, any_key) = self.registrant_keys(function);
            self.callback_instance(function, &BTreeSet::new(), &keys, any_key)?;
            entry = true;
        }
        // Direct callers pass one of their own unassigned parameters.
        let mut callers = 0usize;
        for (d, k) in calls.get(&definition.name).into_iter().flatten() {
            let g = &self.definitions[*d];
            let k = *k;
            let g_texts = self.texts(g.file);
            {
                if !self.resolve_name(g.file, g_texts[k]).contains(&function) {
                    continue;
                }
                callers += 1;
                let caller = self.index[&g.census_name];
                let at = format!(
                    "{}:{}",
                    self.files[g.file].path, self.scans[g.file].tokens[k].line
                );
                let (arguments, _) = call_arguments(&g_texts, k + 1)
                    .ok_or_else(|| format!("the call of `{name}` at {at} is not read"))?;
                let passed = arguments
                    .get(index)
                    .filter(|(s, e)| e - s == 1)
                    .map(|(s, _)| g_texts[*s])
                    .ok_or_else(|| format!("the call of `{name}` at {at} passes no variable"))?;
                let g_parameters = ordered_parameters(&g_texts, g.header);
                let j = g_parameters
                    .iter()
                    .position(|p| p == passed)
                    .ok_or_else(|| {
                        format!("the call of `{name}` at {at} passes `{passed}`, not a parameter")
                    })?;
                self.instance_parameter(caller, j, dir, calls, visiting)?;
            }
        }
        if callers == 0 && !entry {
            return Err(format!("no caller passes `{name}` an instance"));
        }
        Ok(())
    }

    fn registrant_keys(&self, function: usize) -> (BTreeSet<String>, bool) {
        let mut keys = BTreeSet::new();
        let mut any = false;
        for set in self.registrants.values() {
            for key in set.get(&function).into_iter().flatten() {
                match key {
                    Keyed::Any => any = true,
                    Keyed::Record(record) => {
                        keys.insert(record.clone());
                    }
                }
            }
        }
        (keys, any)
    }

    /// Every collector-field dispatch that may reach `function` passes its
    /// own collector's instance: `C -> cb_collect ( C -> instance , …)`.
    fn collector_dispatch_passes_its_instance(&self, function: usize) -> Result<(), String> {
        let hops = self.hops;
        let (keys, any_key) = self.registrant_keys(function);
        for site in self.dispatch_sites {
            if site.field != hops.collector_field {
                continue;
            }
            let reaches = match &site.records {
                None => true,
                Some(records) => any_key || !records.is_disjoint(&keys),
            };
            if !reaches {
                continue;
            }
            let texts = self.texts(site.file);
            let k = site.position;
            let at = format!(
                "{}:{}",
                self.files[site.file].path, self.scans[site.file].tokens[k].line
            );
            let object = (k >= 3 && texts[k - 1] == ">" && texts[k - 2] == "-")
                .then(|| texts[k - 3])
                .filter(|o| is_identifier(o) && !matches!(texts[k - 4], "." | ">"));
            let first = call_arguments(&texts, k + 1)
                .and_then(|(arguments, _)| arguments.first().map(|(s, e)| texts[*s..*e].to_vec()));
            let passes = matches!((object, first.as_deref()),
                (Some(o), Some([c, "-", ">", f])) if *c == o && *f == "instance");
            if !passes {
                return Err(format!(
                    "the collector dispatch at {at} does not pass its own collector's instance"
                ));
            }
        }
        Ok(())
    }

    /// The token spans (file, `{`, `}`) of `plugins/dir`'s descriptor
    /// initializers.
    fn descriptor_spans(&self, dir: &str) -> Vec<(usize, usize, usize)> {
        let hops = self.hops;
        let mut spans = Vec::new();
        for (file, source) in self.files.iter().enumerate() {
            if plugin_dir(hops, &source.path).as_deref() != Some(dir) {
                continue;
            }
            let texts = self.texts(file);
            for k in 0..texts.len() {
                if texts[k] == "struct"
                    && texts.get(k + 1) == Some(&hops.plugin_descriptor)
                    && texts.get(k + 2).is_some_and(|t| is_identifier(t))
                    && texts.get(k + 3) == Some(&"=")
                    && texts.get(k + 4) == Some(&"{")
                    && let Some(close) = matching_brace(&texts, k + 4)
                {
                    spans.push((file, k + 4, close));
                }
            }
        }
        spans
    }

    /// The instance's plugin field is written only by the instance
    /// constructor, or by installing a copy of the same instance's plugin
    /// (`H.M = *X->p;` then `X->p = &H.M;`) into which no function defined
    /// under the plugin root is stored.
    fn plugin_field_writers(&self, typed: &TypedFacts) -> Result<(), String> {
        let hops = self.hops;
        let record = self.flow.instance_record;
        let field = hops.instance_plugin_field;
        if !typed
            .member_fields_read
            .contains(&member_key(record, field))
        {
            return Err(format!(
                "the typed reading did not record `{record}::{field}`"
            ));
        }
        for access in accesses(typed, record, field) {
            match access.use_label.as_str() {
                "read" | "address-read" => {}
                "write" if access.function == hops.instance_constructor.0 => {}
                "write" => self.copy_installation(&access)?,
                _ => {
                    return Err(format!(
                        "`{record}::{field}`'s address is taken at {}",
                        access.at()
                    ));
                }
            }
        }
        for access in accesses(typed, "?", field) {
            if access.use_label != "read" && access.use_label != "address-read" {
                return Err(format!(
                    "a `{field}` member of an unmatched record is written at {}",
                    access.at()
                ));
            }
        }
        Ok(())
    }

    fn copy_installation(&self, access: &Access) -> Result<(), String> {
        let hops = self.hops;
        let field = hops.instance_plugin_field;
        let not = || {
            Err(format!(
                "`{field}` is written at {} other than by installing a copy of the instance's \
                 own plugin",
                access.at()
            ))
        };
        let Some(definition) = self
            .definitions
            .iter()
            .find(|d| d.name == access.function && self.files[d.file].path == access.file)
        else {
            return not();
        };
        let texts = self.texts(definition.file);
        let lines = self.lines(definition.file);
        let writes = member_tokens(&texts, &lines, definition.body, access.line, field)
            .into_iter()
            .filter(|k| texts.get(k + 1) == Some(&"=") && texts.get(k + 2) != Some(&"="))
            .collect::<Vec<_>>();
        let [w] = writes[..] else {
            return not();
        };
        // `X -> p = & H . M ;`
        let Some(x) = member_base(&texts, w) else {
            return not();
        };
        let (h, m) = match texts.get(w + 2..w + 7) {
            Some(["&", h, ".", m, ";"]) if is_identifier(h) && is_identifier(m) => (*h, *m),
            _ => return not(),
        };
        // `H . M = * X -> p ;` earlier.
        let copied = (definition.body.0..w).any(|k| {
            texts.get(k..k + 9) == Some(&[h, ".", m, "=", "*", x, "-", ">", field][..])
                && texts.get(k + 9) == Some(&";")
        });
        if !copied {
            return not();
        }
        // No plugin function stored into the copy.
        for k in definition.body.0..=definition.body.1 {
            if texts.get(k..k + 3) == Some(&[h, ".", m][..])
                && texts.get(k + 3) == Some(&".")
                && texts.get(k + 5) == Some(&"=")
                && let Some(value) = texts.get(k + 6)
            {
                let stored = self.resolve_name(definition.file, value);
                if stored.iter().any(|id| {
                    self.definitions.iter().any(|d| {
                        self.index[&d.census_name] == *id
                            && self.files[d.file].path.starts_with(hops.plugin_root)
                    })
                }) {
                    return Err(format!(
                        "a plugin's function `{value}` is stored into the copy installed at {}",
                        access.at()
                    ));
                }
            }
        }
        Ok(())
    }

    // ---- The immutable threaded flag -------------------------------------

    fn threaded_flag(
        &self,
        typed: &TypedFacts,
        tables: &CoroPluginTables,
        residuals: &mut BTreeMap<String, Vec<String>>,
    ) -> Result<Vec<String>, String> {
        let record = self.flow.instance_record;
        let field = tables.threaded_field;
        if !typed
            .member_fields_read
            .contains(&member_key(record, field))
        {
            return Err(format!(
                "the typed reading did not record `{record}::{field}`"
            ));
        }
        let (setter, key_argument) = tables.property_setter;
        let mut notes = Vec::new();
        let mut residual = Vec::new();
        let mut setter_writes = false;
        for access in accesses(typed, record, field) {
            match access.use_label.as_str() {
                "read" | "address-read" => continue,
                "write" => {}
                _ => {
                    return Err(format!(
                        "`{record}::{field}`'s address is taken at {}",
                        access.at()
                    ));
                }
            }
            if access.function == setter {
                self.threaded_arm(typed, tables, &access)?;
                setter_writes = true;
                notes.push(format!(
                    "`{setter}` writes `{field}` only in its `{}` arm ({}:{})",
                    tables.threaded_key, access.file, access.line
                ));
                continue;
            }
            let created = self
                .created_write(typed, tables, &access, field)
                .map_err(|why| {
                    format!("`{record}::{field}` is written at {}: {why}", access.at())
                })?;
            residual.push(format!(
                "{}:{} `{}` ({created})",
                access.file, access.line, access.function
            ));
        }
        for access in accesses(typed, "?", field) {
            if access.use_label != "read" && access.use_label != "address-read" {
                return Err(format!(
                    "a `{field}` member of an unmatched record is written at {}",
                    access.at()
                ));
            }
        }
        if let Some(why) = self.untyped_member(typed, field, true) {
            return Err(why);
        }
        if setter_writes {
            // Every call of the setter: a key read as one literal other than
            // the threaded key cannot reach the arm.
            let setters = self.definitions_named(setter);
            for definition in &setters {
                let texts = self.texts(definition.file);
                let parameters = ordered_parameters(&texts, definition.header);
                let key = parameters
                    .get(key_argument)
                    .ok_or_else(|| format!("`{setter}` has no key parameter"))?;
                if !assignments(&texts, definition.body, key).is_empty()
                    || address_taken(&texts, definition.body, key)
                {
                    return Err(format!("`{setter}` reassigns its key parameter `{key}`"));
                }
            }
            let ids = setters
                .iter()
                .map(|d| self.index[&d.census_name])
                .collect::<BTreeSet<_>>();
            if ids.iter().any(|id| self.taken.contains(id)) {
                return Err(format!("`{setter}` is named other than by a call"));
            }
            let mut other_keys = 0usize;
            for g in self.definitions {
                let texts = self.texts(g.file);
                let path = &self.files[g.file].path;
                let lines = self.lines(g.file);
                for k in g.body.0..=g.body.1 {
                    if texts[k] != setter
                        || texts.get(k + 1) != Some(&"(")
                        || matches!(texts[k - 1], "." | ">")
                        || self.resolve_name(g.file, setter).is_disjoint_with(&ids)
                    {
                        continue;
                    }
                    let line = lines[k];
                    let same_line = (g.body.0..=g.body.1)
                        .filter(|&i| lines[i] == line && texts[i] == setter)
                        .count();
                    let literal = typed
                        .literal_calls
                        .get(&crate::typed_dispatch_census::literal_call_key(
                            path, line, setter,
                        ))
                        .filter(|set| set.len() == 1 && same_line == 1)
                        .and_then(|set| set.iter().next());
                    match literal {
                        Some(key) if !key.eq_ignore_ascii_case(tables.threaded_key) => {
                            other_keys += 1;
                        }
                        Some(key) => residual.push(format!(
                            "{path}:{line} `{}` (key \"{key}\")",
                            self.functions[self.index[&g.census_name]]
                        )),
                        None => residual.push(format!(
                            "{path}:{line} `{}` (key argument {key_argument} not one literal)",
                            self.functions[self.index[&g.census_name]]
                        )),
                    }
                }
            }
            notes.push(format!(
                "{other_keys} `{setter}` calls pass another literal key and cannot write it"
            ));
        }
        residual.sort();
        residual.dedup();
        residuals.insert("P-THREADED-SET-BEFORE-START".to_owned(), residual);
        Ok(notes)
    }

    /// The setter's write stands in the then-block of an `if` whose
    /// condition's one comparison call passes the threaded key literal.
    fn threaded_arm(
        &self,
        typed: &TypedFacts,
        tables: &CoroPluginTables,
        access: &Access,
    ) -> Result<(), String> {
        let not = |why: &str| {
            Err(format!(
                "`{}` writes the flag at {}:{} {why}",
                access.function, access.file, access.line
            ))
        };
        let Some(definition) = self
            .definitions
            .iter()
            .find(|d| d.name == access.function && self.files[d.file].path == access.file)
        else {
            return not("in a function the census does not read");
        };
        let texts = self.texts(definition.file);
        let lines = self.lines(definition.file);
        let writes = member_tokens(
            &texts,
            &lines,
            definition.body,
            access.line,
            tables.threaded_field,
        );
        let [w] = writes[..] else {
            return not("in more than one member store");
        };
        // The innermost `if (…) {` whose block holds the write.
        let mut arm = None;
        for i in definition.body.0..w {
            if texts[i] != "if" || texts.get(i + 1) != Some(&"(") {
                continue;
            }
            let Some((_, close)) = call_arguments(&texts, i + 1) else {
                continue;
            };
            if texts.get(close + 1) != Some(&"{") {
                continue;
            }
            if let Some(end) = matching_brace(&texts, close + 1)
                && close + 1 < w
                && w < end
            {
                arm = Some((i, close));
            }
        }
        let Some((i, close)) = arm else {
            return not("outside a keyed arm");
        };
        let checks = (i..close)
            .filter(|k| texts[*k] == tables.key_check && texts.get(k + 1) == Some(&"("))
            .collect::<Vec<_>>();
        let [check] = checks[..] else {
            return not("in an arm without one key comparison");
        };
        let key = typed
            .argument_lists
            .get(&format!(
                "{}|{}|{}",
                access.file, lines[check], tables.key_check
            ))
            .filter(|lists| lists.len() == 1)
            .and_then(|lists| lists.iter().next())
            .and_then(|list| list.first())
            .and_then(|first| first.strip_prefix("s:"));
        match key {
            Some(key) if key == tables.threaded_key => Ok(()),
            _ => not("in an arm the typed reading does not key by the threaded key"),
        }
    }

    /// A write into an instance the writing function created itself: the
    /// base is a local assigned once from an allocator or the instance
    /// constructor (behind at most a cast); when no call between the creation
    /// and the write may register a collector, that much is derived.
    fn created_write(
        &self,
        typed: &TypedFacts,
        tables: &CoroPluginTables,
        access: &Access,
        field: &str,
    ) -> Result<String, String> {
        let definition = self
            .definitions
            .iter()
            .find(|d| d.name == access.function && self.files[d.file].path == access.file)
            .ok_or("the writing function is not read")?;
        let texts = self.texts(definition.file);
        let lines = self.lines(definition.file);
        let writes = member_tokens(&texts, &lines, definition.body, access.line, field)
            .into_iter()
            .filter(|k| assigned_after(&texts, k + 1))
            .collect::<Vec<_>>();
        let [w] = writes[..] else {
            return Err("the write is not one member store".to_owned());
        };
        let base = member_base(&texts, w).ok_or("the written object is not a local")?;
        if definition.parameters.contains(base) {
            return Err(format!(
                "the written instance `{base}` is a parameter: a run-time write"
            ));
        }
        let assigned = creations(&texts, definition.body, base);
        let [a] = assigned[..] else {
            return Err(format!(
                "the written instance `{base}` is not assigned exactly once"
            ));
        };
        if a > w || address_taken(&texts, definition.body, base) {
            return Err(format!(
                "the written instance `{base}` is not created before the write"
            ));
        }
        let mut rhs = a + 2;
        if texts[rhs] == "(" {
            let (_, close) = call_arguments(&texts, rhs).ok_or("an unreadable cast")?;
            rhs = close + 1;
        }
        let constructor = self.hops.instance_constructor.0;
        let creator = texts[rhs];
        if !(tables.allocators.contains(&creator) || creator == constructor)
            || texts.get(rhs + 1) != Some(&"(")
        {
            return Err(format!(
                "the written instance `{base}` is not created in `{}`",
                access.function
            ));
        }
        let (_, close) = call_arguments(&texts, rhs + 1).ok_or("an unreadable creation")?;
        Ok(
            match self.may_register_between(typed, tables, definition, close + 1, w) {
                None => format!(
                    "a fresh instance from `{creator}`, no collector registered before the write"
                ),
                Some(why) => format!("a fresh instance from `{creator}`; {why}"),
            },
        )
    }
}

/// One registration: the registering function, its instance argument, the
/// file.
type Registration<'a> = (usize, Option<&'a str>, usize);

/// Every call by name: the name called → (definition index, token).
type CallIndex = BTreeMap<String, Vec<(usize, usize)>>;

trait Disjoint {
    fn is_disjoint_with(&self, other: &BTreeSet<usize>) -> bool;
}

impl Disjoint for Vec<usize> {
    fn is_disjoint_with(&self, other: &BTreeSet<usize>) -> bool {
        !self.iter().any(|id| other.contains(id))
    }
}

/// The assignments of `name` in `range` other than a null initialisation
/// (`= NULL;`, `= 0;`).
fn creations(texts: &[&str], range: (usize, usize), name: &str) -> Vec<usize> {
    assignments(texts, range, name)
        .into_iter()
        .filter(|&k| {
            !(matches!(texts.get(k + 2), Some(&"NULL" | &"0")) && texts.get(k + 3) == Some(&";"))
        })
        .collect()
}

fn plugin_dir(hops: &HopTables, path: &str) -> Option<String> {
    path.strip_prefix(hops.plugin_root)
        .and_then(|rest| rest.split_once('/'))
        .map(|(dir, _)| dir.to_owned())
}

fn matching_brace(texts: &[&str], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, text) in texts.iter().enumerate().skip(open) {
        match *text {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
