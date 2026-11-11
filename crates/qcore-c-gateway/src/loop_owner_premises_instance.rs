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

//! The list-head instance's premises, derived from the pinned source (card
//! **W47-PREMISES-INSTANCE**, gateway rule 10p's amendment).
//!
//! Step B's list-head derivation (card W47-LOOP-OWNER-INSTANCE-FLOW) and the
//! owner obligation rested on five stated premises. In the premised census
//! ([`super::read_program_premised`] with [`PremiseInputs`]) each is derived
//! from the program's own text and typed reading, or narrowed to a named
//! residual:
//!
//! 1. **P-FRESH-CONTEXT**: the context constructor stores the one config the
//!    config initializer returns in its context's `config`; the initializer
//!    returns its one fresh allocation (an allocator whose every result is a
//!    null pointer or the C library's `calloc`) after initialising the input
//!    list and the registered-plugin list at its top level with a list
//!    initialiser whose body leaves an empty list.
//! 2. **P-REGISTERED-DESCRIPTORS**: the build's generated registration
//!    function, resolved by every compile command of the initializer's file
//!    through its `-I` order, is read: one allocate / `memcpy(V, &OBJ, …)` /
//!    `mk_list_add(&V->_head, &CFG->in_plugins)` block per descriptor, called
//!    once by the initializer after the list's initialisation; every
//!    registered descriptor's name is read, and the literal a creation
//!    passes matches a registered one. Any other generated function naming an
//!    input list, any other caller of the registration, any unlink or re-link
//!    of an input instance (`struct flb_input_instance::_head` used other
//!    than by the constructor's append) and any mutation of a listed member
//!    in a function the census does not read are hazards.
//! 3. **P-HEAD-INSTANCE**, narrowed: the constructor returns a non-null
//!    instance only after appending that allocation to the config's input
//!    list (its one allocation, every other return a null pointer, no jump
//!    between the allocation and the append). What stays stated is
//!    **P-CREATIONS-SUCCEED**: the context constructor and the one creation's
//!    constructor return non-null, which the test does not check before its
//!    read (acutest's `TEST_CHECK` does not return).
//! 4. **P-INSTANCE**, per obligated collector operation: derived when the
//!    operation's instance argument is its function's unassigned first
//!    parameter, every edge into that function is a loop dispatch whose
//!    dispatch passes `C->instance` of the collector `C` it calls through, and
//!    a collector's `instance` is written only by the collector constructor
//!    (member writes alone, P-INSTANCE-FIELDS-BY-MEMBER for the collector).
//!    A refuted part makes the site unresolved; an operation whose instance
//!    is not a parameter (`ctx->ins`), or a function reached otherwise, keeps
//!    the premise stated for that site (named per site).
//! 5. **P-INSTANCE-FIELDS-BY-MEMBER**: the typed reading's record views of
//!    `struct flb_input_instance` and `struct flb_input_collector`: a whole
//!    assignment, a copy-initialised or by-value object, an initializer list,
//!    a byte function over one, or a pointer to one converted to a character
//!    or another object type is a violation (the flag is underived and the
//!    collector-dependent sites refuted). What stays stated is that a pointer
//!    converted to `void *` or an integer (counted) is not written through
//!    bytewise, and that no offset from another member's address reaches the
//!    plugin, flag or instance member.
//!
//! Anything else leaves the flag underived (the route kept) or the site
//! unresolved; nothing here is `Unsafe`.

use std::collections::{BTreeMap, BTreeSet};

use super::loop_owner_instance::{
    address_taken, assignments, call_arguments, loops, ordered_parameters,
};
use super::{CensusReading, Definition, ReadHop, StepB};
use crate::legacy_c::{is_identifier, tokenize};
use crate::typed_dispatch_census::{TypedFacts, member_key};

/// A file of the build's generated root that the census does not read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeneratedFile {
    /// Its path under the generated root.
    pub path: String,
    pub text: String,
    /// The read files every compile command of which resolves one of their
    /// `#include <…>` lines to this file through its `-I` order.
    pub included_by: BTreeSet<String>,
}

/// What the premised census reads besides the program's files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PremiseInputs {
    pub generated: Vec<GeneratedFile>,
}

/// How P-INSTANCE stands at one obligated call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SitePremise {
    /// Derived from the program, with what it rests on.
    Derived(String),
    /// Not derived here: the stated premise, narrowed as named.
    Stated(String),
    /// A part of the derivation is refuted: the site is not discharged.
    Refuted(String),
}

/// The premised census's report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PremiseReport {
    /// `PATH:LINE CALLEE` of each listed collector operation → P-INSTANCE.
    pub instance_sites: BTreeMap<String, SitePremise>,
    /// P-INSTANCE-FIELDS-BY-MEMBER over the typed reading.
    pub fields_by_member: FieldsByMember,
}

/// The record views the typed reading recorded, classified.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldsByMember {
    /// Whether every listed record's views were recorded.
    pub read: bool,
    /// `RECORD FILE:LINE FUNCTION KIND`: non-member writes or byte views.
    pub violations: Vec<String>,
    /// Pointers converted to `void *` or an integer (the stated residual).
    pub escapes: Vec<String>,
    /// Pointers converted to another non-character object type: under C's
    /// effective-type rule (stated since W47-LOOP-OWNER-TYPED-DISPATCH) an
    /// instance or collector is not accessed through them.
    pub other_types: Vec<String>,
    /// Pointers passed to a release function, or compared.
    pub harmless: usize,
}

/// Fluent Bit's names for these derivations, as read at the `90978ac6`
/// export.
#[derive(Clone, Copy, Debug)]
pub struct InstancePremiseTables {
    /// `src/flb_config.c:281` (`flb_config_init`).
    pub config_initializer: &'static str,
    /// `include/fluent-bit/flb_mem.h:84` (`flb_calloc`) and its primitive.
    pub allocator: &'static str,
    pub allocator_primitive: &'static str,
    /// `lib/monkey/include/monkey/mk_core/mk_list.h:48`.
    pub list_init: &'static str,
    pub list_add: &'static str,
    pub list_next: &'static str,
    pub list_prev: &'static str,
    /// The generated `flb_plugins_register` (`@FLB_IN_PLUGINS_ADD@` of
    /// `include/fluent-bit/flb_plugins.h.in`), its allocator and copy.
    pub registration: &'static str,
    pub registration_allocator: &'static str,
    pub registration_copy: &'static str,
    /// The instance's link on its config's input list.
    pub instance_link: &'static str,
    /// `(operation, instance argument)`: the obligated collector operations
    /// (`include/fluent-bit/flb_input.h`).
    pub operations: &'static [(&'static str, usize)],
    /// The records whose non-member uses are read.
    pub records: &'static [&'static str],
    /// Calls a pointer may be passed to whole without a byte write.
    pub release_functions: &'static [&'static str],
    /// Calls that write or copy bytes through a pointer argument.
    pub byte_functions: &'static [&'static str],
}

/// Fluent Bit at the `90978ac6` export.
pub const FLUENT_BIT_INSTANCE_PREMISES: InstancePremiseTables = InstancePremiseTables {
    config_initializer: "flb_config_init",
    allocator: "flb_calloc",
    allocator_primitive: "calloc",
    list_init: "mk_list_init",
    list_add: "mk_list_add",
    list_next: "next",
    list_prev: "prev",
    registration: "flb_plugins_register",
    registration_allocator: "flb_malloc",
    registration_copy: "memcpy",
    instance_link: "_head",
    operations: &[
        ("flb_input_collector_running", 1),
        ("flb_input_collector_pause", 1),
        ("flb_input_collector_resume", 1),
        ("flb_input_collector_delete", 1),
    ],
    records: &["struct flb_input_instance", "struct flb_input_collector"],
    release_functions: &["flb_free", "free"],
    byte_functions: &[
        "memcpy",
        "memmove",
        "memset",
        "memccpy",
        "bzero",
        "explicit_bzero",
        "strcpy",
        "strncpy",
        "read",
        "fread",
        "recv",
        "__builtin_memcpy",
        "__builtin_memmove",
        "__builtin_memset",
        "__builtin___memcpy_chk",
        "__builtin___memmove_chk",
        "__builtin___memset_chk",
    ],
};

/// The typed accesses of a listed member: `(file, line, function, use)`.
fn member_sites<'t>(
    typed: &'t TypedFacts,
    record: &str,
    field: &str,
) -> Vec<(&'t str, usize, &'t str, &'t str)> {
    typed
        .member_accesses
        .get(&member_key(record, field))
        .into_iter()
        .flatten()
        .filter_map(|site| {
            let mut parts = site.splitn(4, '|');
            Some((
                parts.next()?,
                parts.next()?.parse().ok()?,
                parts.next()?,
                parts.next()?,
            ))
        })
        .collect()
}

fn reads_only(use_label: &str) -> bool {
    matches!(use_label, "read" | "address-read")
}

/// Brace depth of each token of `range` relative to its opening `{` (the
/// body's own statements are at depth 1).
fn depths(texts: &[&str], range: (usize, usize)) -> BTreeMap<usize, usize> {
    let mut depth = 0usize;
    let mut out = BTreeMap::new();
    for (index, text) in texts.iter().enumerate().take(range.1 + 1).skip(range.0) {
        if *text == "}" {
            depth = depth.saturating_sub(1);
        }
        out.insert(index, depth);
        if *text == "{" {
            depth += 1;
        }
    }
    out
}

/// The positions of `name (` calls (not members) in `range`.
fn calls_of(texts: &[&str], range: (usize, usize), name: &str) -> Vec<usize> {
    (range.0..=range.1)
        .filter(|&k| {
            texts[k] == name && texts.get(k + 1) == Some(&"(") && !matches!(texts[k - 1], "." | ">")
        })
        .collect()
}

/// The `return` statements of `range`, as their token slices.
fn returns<'t>(texts: &[&'t str], range: (usize, usize)) -> Vec<(usize, Vec<&'t str>)> {
    (range.0..=range.1)
        .filter(|&k| texts[k] == "return")
        .map(|k| {
            let end = super::loop_owner_instance::semicolon(texts, k + 1, range.1);
            (k, texts[k + 1..end].to_vec())
        })
        .collect()
}

fn is_null(tokens: &[&str]) -> bool {
    matches!(tokens, ["NULL"] | ["0"])
}

/// The file-scope function definitions of a token list: `(name, name
/// position, body)`.
fn definitions_of(texts: &[&str]) -> Vec<(String, usize, (usize, usize))> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut k = 0;
    while k < texts.len() {
        match texts[k] {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            name if depth == 0 && is_identifier(name) && texts.get(k + 1) == Some(&"(") => {
                if let Some((_, close)) = call_arguments(texts, k + 1)
                    && texts.get(close + 1) == Some(&"{")
                {
                    let open = close + 1;
                    let mut inner = 0usize;
                    let mut end = None;
                    for (index, text) in texts.iter().enumerate().skip(open) {
                        match *text {
                            "{" => inner += 1,
                            "}" => {
                                inner -= 1;
                                if inner == 0 {
                                    end = Some(index);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(end) = end {
                        out.push((name.to_owned(), k, (open, end)));
                        k = end + 1;
                        continue;
                    }
                }
            }
            _ => {}
        }
        k += 1;
    }
    out
}

/// One registration block of the generated function, from `at` (the
/// allocation's variable): `V = ALLOC ( sizeof ( struct T ) ) ; if ( ! V ) {
/// CALL ( ) ; … return - 1 ; } COPY ( V , & OBJ , sizeof ( struct T ) ) ;
/// ADD ( & V -> _head , & CFG -> LIST ) ;`, returning `(T, OBJ, LIST, end)`.
fn registration_block(
    texts: &[&str],
    at: usize,
    tables: &InstancePremiseTables,
    cfg: &str,
) -> Option<(String, String, String, usize)> {
    let v = texts[at];
    let head = [
        v,
        "=",
        tables.registration_allocator,
        "(",
        "sizeof",
        "(",
        "struct",
    ];
    if texts.get(at..at + head.len())? != head {
        return None;
    }
    let record = *texts.get(at + head.len())?;
    let mut k = at + head.len() + 1;
    if texts.get(k..k + 3)? != [")", ")", ";"] {
        return None;
    }
    k += 3;
    if texts.get(k..k + 6)? != ["if", "(", "!", v, ")", "{"] {
        return None;
    }
    k += 6;
    // `CALL ( ) ;`* then `return - 1 ; }`.
    loop {
        match texts.get(k..k + 4)? {
            ["return", "-", "1", ";"] => {
                k += 4;
                break;
            }
            [call, "(", ")", ";"] if is_identifier(call) => k += 4,
            _ => return None,
        }
    }
    if texts.get(k)? != &"}" {
        return None;
    }
    k += 1;
    let copy = [tables.registration_copy, "(", v, ",", "&"];
    if texts.get(k..k + copy.len())? != copy {
        return None;
    }
    k += copy.len();
    let object = *texts.get(k)?;
    k += 1;
    if texts.get(k..k + 8)? != [",", "sizeof", "(", "struct", record, ")", ")", ";"] {
        return None;
    }
    k += 8;
    let add = [
        tables.list_add,
        "(",
        "&",
        v,
        "-",
        ">",
        tables.instance_link,
        ",",
        "&",
        cfg,
        "-",
        ">",
    ];
    if texts.get(k..k + add.len())? != add {
        return None;
    }
    k += add.len();
    let list = *texts.get(k)?;
    if texts.get(k + 1..k + 3)? != [")", ";"] {
        return None;
    }
    Some((
        format!("struct {record}"),
        object.to_owned(),
        list.to_owned(),
        k + 3,
    ))
}

/// The generated file defining the registration, its tokens, the
/// definition's body and its name token.
type RegistrationSource<'a> = (&'a GeneratedFile, Vec<String>, (usize, usize), usize);

impl StepB<'_> {
    fn premise_tables(&self) -> &InstancePremiseTables {
        &FLUENT_BIT_INSTANCE_PREMISES
    }

    /// The one census definition named `name`.
    fn one_definition(&self, name: &str) -> Result<&Definition, String> {
        let found = self
            .definitions
            .iter()
            .filter(|d| d.name == name)
            .collect::<Vec<_>>();
        match found[..] {
            [one] => Ok(one),
            [] => Err(format!("`{name}` is not read")),
            _ => Err(format!("`{name}` is defined more than once")),
        }
    }

    /// **P-FRESH-CONTEXT**: the context the test reads is a fresh one whose
    /// input and registered-plugin lists are empty when the constructor
    /// returns.
    fn fresh_context(&self) -> Result<String, String> {
        let tables = self.premise_tables();
        let flow = self.flow;
        let underived = |why: String| Err(format!("P-FRESH-CONTEXT is not derived: {why}"));
        // The list initialiser leaves an empty list: `L->next = L; L->prev =
        // L;` and nothing else.
        let init = match self.one_definition(tables.list_init) {
            Ok(init) => init,
            Err(why) => return underived(why),
        };
        let texts = self.texts(init.file);
        let body = &texts[init.body.0 + 1..init.body.1];
        let parameters = ordered_parameters(&texts, init.header);
        let [parameter] = &parameters[..] else {
            return underived(format!("`{}` has not one parameter", tables.list_init));
        };
        let p = parameter.as_str();
        let empty_a = [
            p,
            "-",
            ">",
            tables.list_next,
            "=",
            p,
            ";",
            p,
            "-",
            ">",
            tables.list_prev,
            "=",
            p,
            ";",
        ];
        let empty_b = [
            p,
            "-",
            ">",
            tables.list_prev,
            "=",
            p,
            ";",
            p,
            "-",
            ">",
            tables.list_next,
            "=",
            p,
            ";",
        ];
        if body != empty_a && body != empty_b {
            return underived(format!(
                "`{}`'s body is not `{p}->next = {p}; {p}->prev = {p};`",
                tables.list_init
            ));
        }
        // The allocator returns only a null pointer or the primitive's result.
        let allocator = match self.one_definition(tables.allocator) {
            Ok(allocator) => allocator,
            Err(why) => return underived(why),
        };
        let a_texts = self.texts(allocator.file);
        for (_, value) in returns(&a_texts, allocator.body) {
            let fresh = value.first() == Some(&tables.allocator_primitive)
                && value.get(1) == Some(&"(")
                && call_arguments(&value, 1).is_some_and(|(_, close)| close + 1 == value.len());
            if !is_null(&value) && !fresh {
                return underived(format!(
                    "`{}` returns `{}`, not a null pointer or `{}(…)`",
                    tables.allocator,
                    value.join(" "),
                    tables.allocator_primitive
                ));
            }
        }
        // The initializer: one fresh allocation, both lists initialised at
        // its top level before every return of it.
        let initializer = match self.one_definition(tables.config_initializer) {
            Ok(initializer) => initializer,
            Err(why) => return underived(why),
        };
        let texts = self.texts(initializer.file);
        let body = initializer.body;
        let (_, jumps) = loops(&texts, body);
        if jumps {
            return underived(format!("`{}` has a jump", tables.config_initializer));
        }
        let depth = depths(&texts, body);
        let mut local = None;
        for (_, value) in returns(&texts, body) {
            match value[..] {
                [name] if is_identifier(name) && name != "NULL" => {
                    if local.is_some_and(|l| l != name) {
                        return underived(format!(
                            "`{}` returns two locals",
                            tables.config_initializer
                        ));
                    }
                    local = Some(name);
                }
                _ if is_null(&value) => {}
                _ => {
                    return underived(format!(
                        "`{}` returns `{}`",
                        tables.config_initializer,
                        value.join(" ")
                    ));
                }
            }
        }
        let Some(config) = local else {
            return underived(format!("`{}` returns no config", tables.config_initializer));
        };
        let assigned = assignments(&texts, body, config);
        let [at] = assigned[..] else {
            return underived(format!("`{config}` is not assigned once"));
        };
        if texts.get(at + 2) != Some(&tables.allocator)
            || texts.get(at + 3) != Some(&"(")
            || depth.get(&at) != Some(&1)
            || address_taken(&texts, body, config)
        {
            return underived(format!(
                "`{config}` is not one top-level allocation by `{}` whose address stays \
                 untaken",
                tables.allocator
            ));
        }
        let last_return = returns(&texts, body)
            .iter()
            .filter(|(_, value)| value[..] == [config])
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();
        for list in [flow.list_field, flow.registrations_field] {
            let init_at = calls_of(&texts, body, tables.list_init)
                .into_iter()
                .filter(|&k| texts.get(k + 2..k + 7) == Some(&["&", config, "-", ">", list][..]))
                .collect::<Vec<_>>();
            let [k] = init_at[..] else {
                return underived(format!(
                    "`{}(&{config}->{list})` is not called once",
                    tables.list_init
                ));
            };
            if depth.get(&k) != Some(&1) || k < at || last_return.iter().any(|r| *r < k) {
                return underived(format!(
                    "`{}(&{config}->{list})` is not at the top level of `{}` between the \
                     allocation and every return of `{config}`",
                    tables.list_init, tables.config_initializer
                ));
            }
            // Nothing else in the initializer names the list.
            let named = (body.0..=body.1)
                .filter(|&j| texts[j] == list && matches!(texts[j - 1], "." | ">"))
                .count();
            if named != 1 {
                return underived(format!(
                    "`{}` names `{list}` other than to initialise it",
                    tables.config_initializer
                ));
            }
        }
        // The constructor stores the initializer's one result in the context
        // it returns.
        let constructor = match self.one_definition(flow.context_constructor) {
            Ok(constructor) => constructor,
            Err(why) => return underived(why),
        };
        let c_texts = self.texts(constructor.file);
        let c_body = constructor.body;
        let c_depth = depths(&c_texts, c_body);
        let (_, c_jumps) = loops(&c_texts, c_body);
        if c_jumps {
            return underived(format!("`{}` has a jump", flow.context_constructor));
        }
        let mut holder = None;
        for (_, value) in returns(&c_texts, c_body) {
            match value[..] {
                [name] if is_identifier(name) && name != "NULL" => {
                    if holder.is_some_and(|h| h != name) {
                        return underived(format!(
                            "`{}` returns two locals",
                            flow.context_constructor
                        ));
                    }
                    holder = Some(name);
                }
                _ if is_null(&value) => {}
                _ => {
                    return underived(format!(
                        "`{}` returns `{}`",
                        flow.context_constructor,
                        value.join(" ")
                    ));
                }
            }
        }
        let Some(holder) = holder else {
            return underived(format!("`{}` returns no context", flow.context_constructor));
        };
        let init_calls = calls_of(&c_texts, c_body, tables.config_initializer);
        let [call] = init_calls[..] else {
            return underived(format!(
                "`{}` does not call `{}` once",
                flow.context_constructor, tables.config_initializer
            ));
        };
        if c_depth.get(&call) != Some(&1) {
            return underived(format!(
                "`{}`'s call of `{}` is not at its top level",
                flow.context_constructor, tables.config_initializer
            ));
        }
        let field = flow.context_config_field;
        let stores = (c_body.0..=c_body.1)
            .filter(|&k| {
                c_texts[k] == holder
                    && c_texts.get(k + 1..k + 4) == Some(&["-", ">", field][..])
                    && c_texts.get(k + 4) == Some(&"=")
                    && c_texts.get(k + 5) != Some(&"=")
            })
            .collect::<Vec<_>>();
        let [store] = stores[..] else {
            return underived(format!(
                "`{holder}->{field}` is not stored once in `{}`",
                flow.context_constructor
            ));
        };
        let value_end = super::loop_owner_instance::semicolon(&c_texts, store + 5, c_body.1);
        let value = &c_texts[store + 5..value_end];
        let direct = value == [tables.config_initializer, "(", ")"] && store + 5 == call;
        let through_local = match value {
            [local] if is_identifier(local) => {
                let local_assigned = assignments(&c_texts, c_body, local);
                matches!(local_assigned[..], [k] if k + 2 == call
                    && c_texts.get(call + 1..call + 4) == Some(&["(", ")", ";"][..]))
                    && !address_taken(&c_texts, c_body, local)
                    && store > call
            }
            _ => false,
        };
        if !(direct || through_local) || c_depth.get(&store) != Some(&1) {
            return underived(format!(
                "`{holder}->{field}` is not the one `{}()` result, stored at the top level of \
                 `{}`",
                tables.config_initializer, flow.context_constructor
            ));
        }
        let holder_made = assignments(&c_texts, c_body, holder);
        let [made] = holder_made[..] else {
            return underived(format!("`{holder}` is not assigned once"));
        };
        if c_texts.get(made + 2) != Some(&tables.allocator) || made > store {
            return underived(format!(
                "`{holder}` is not a fresh `{}` allocation before its `{field}` store",
                tables.allocator
            ));
        }
        Ok(format!(
            "P-FRESH-CONTEXT derived: `{}` stores the one `{}()` result in the `{}` context it \
             returns; `{}` returns its one `{}` allocation after `{}(&{config}->{})` and \
             `{}(&{config}->{})` at its top level, each leaving an empty list",
            flow.context_constructor,
            tables.config_initializer,
            tables.allocator,
            tables.config_initializer,
            tables.allocator,
            tables.list_init,
            flow.list_field,
            tables.list_init,
            flow.registrations_field
        ))
    }

    /// The generated file defining the registration, and its definition.
    fn registration_source(&self) -> Result<RegistrationSource<'_>, String> {
        let tables = self.premise_tables();
        let Some(premises) = self.premises else {
            return Err("no generated source was read".to_owned());
        };
        if self
            .definitions
            .iter()
            .any(|d| d.name == tables.registration)
        {
            return Err(format!(
                "a read file also defines `{}`",
                tables.registration
            ));
        }
        let mut found = Vec::new();
        for file in &premises.generated {
            let tokens = tokenize(&file.text);
            let texts = tokens.iter().map(|t| t.text.clone()).collect::<Vec<_>>();
            let refs = texts.iter().map(String::as_str).collect::<Vec<_>>();
            for (name, header, body) in definitions_of(&refs) {
                if name == tables.registration {
                    found.push((file, texts.clone(), body, header));
                }
            }
        }
        match found.len() {
            1 => Ok(found.remove(0)),
            0 => Err(format!(
                "no generated file defines `{}`",
                tables.registration
            )),
            _ => Err(format!(
                "`{}` is defined more than once among the generated files",
                tables.registration
            )),
        }
    }

    /// **P-REGISTERED-DESCRIPTORS**: the descriptor objects the generated
    /// registration appends to the registered-plugin list, in order, with
    /// their names.
    pub(super) fn registered_descriptors(&self) -> Result<(Vec<(String, String)>, String), String> {
        let tables = self.premise_tables();
        let flow = self.flow;
        let underived =
            |why: String| Err(format!("P-REGISTERED-DESCRIPTORS is not derived: {why}"));
        let Some(typed) = self.typed else {
            return underived("no typed reading names the descriptors".to_owned());
        };
        let initializer = match self.one_definition(tables.config_initializer) {
            Ok(initializer) => initializer,
            Err(why) => return underived(why),
        };
        let (file, texts, body, header) = match self.registration_source() {
            Ok(found) => found,
            Err(why) => return underived(why),
        };
        let init_path = &self.files[initializer.file].path;
        if !file.included_by.contains(init_path) {
            return underived(format!(
                "not every compile command of {init_path} resolves its include to the \
                 generated {}",
                file.path
            ));
        }
        // The initializer calls it once, at its top level, on its config,
        // after initialising the registered-plugin list.
        let i_texts = self.texts(initializer.file);
        let depth = depths(&i_texts, initializer.body);
        let calls = calls_of(&i_texts, initializer.body, tables.registration);
        let [call] = calls[..] else {
            return underived(format!(
                "`{}` does not call `{}` once",
                tables.config_initializer, tables.registration
            ));
        };
        let init_list = calls_of(&i_texts, initializer.body, tables.list_init)
            .into_iter()
            .find(|&k| {
                i_texts.get(k + 6) == Some(&flow.registrations_field)
                    && i_texts.get(k + 2) == Some(&"&")
            });
        let config = i_texts.get(call + 2).copied().unwrap_or("");
        if depth.get(&call) != Some(&1)
            || init_list.is_none_or(|k| k > call)
            || i_texts.get(call + 3) != Some(&")")
            || i_texts.get(init_list.unwrap_or(0) + 3) != Some(&config)
        {
            return underived(format!(
                "`{}` is not called once at the top level of `{}` on its config, after \
                 `{}(&config->{})`",
                tables.registration,
                tables.config_initializer,
                tables.list_init,
                flow.registrations_field
            ));
        }
        // The body: declarations `struct T * V ;`, registration blocks, then
        // `return 0 ;`.
        let refs = texts.iter().map(String::as_str).collect::<Vec<_>>();
        let parameters = ordered_parameters(&refs, header);
        let Some(cfg) = parameters.first().cloned() else {
            return underived(format!("`{}` has no parameter", tables.registration));
        };
        let mut locals: BTreeMap<&str, String> = BTreeMap::new();
        let mut k = body.0 + 1;
        while refs.get(k) == Some(&"struct")
            && refs.get(k + 2) == Some(&"*")
            && refs.get(k + 4) == Some(&";")
        {
            locals.insert(refs[k + 3], format!("struct {}", refs[k + 1]));
            k += 5;
        }
        let mut registered = Vec::new();
        let input_record = format!("struct {}", self.hops.plugin_descriptor);
        while k < body.1 {
            if refs.get(k..k + 4) == Some(&["return", "0", ";", "}"][..]) && k + 3 == body.1 {
                k = body.1;
                break;
            }
            let Some((record, object, list, end)) = registration_block(&refs, k, tables, &cfg)
            else {
                return underived(format!(
                    "{}:{} is not a registration block this reading reads",
                    file.path,
                    tokenize(&file.text).get(k).map_or(0, |token| token.line)
                ));
            };
            if locals.get(refs[k]) != Some(&record) {
                return underived(format!("`{}` is not declared as a `{record} *`", refs[k]));
            }
            let input = record == input_record;
            if input != (list == flow.registrations_field) || list == flow.list_field {
                return underived(format!(
                    "a `{record}` is appended to `{list}` in {}",
                    file.path
                ));
            }
            if input {
                registered.push(object);
            }
            k = end;
        }
        if k != body.1 {
            return underived(format!(
                "{}'s registration does not end in `return 0;`",
                file.path
            ));
        }
        // The calls it makes reach no hazard of the census.
        let hazards = self.hazards(typed);
        let reaching = self.reach_reaching(&hazards.keys().copied().collect::<BTreeSet<_>>());
        for name in [
            tables.registration_allocator,
            tables.registration_copy,
            tables.list_add,
            "flb_errno",
        ] {
            for id in self.resolve_name(initializer.file, name) {
                if reaching.contains(&id) {
                    return underived(format!(
                        "the registration's call of `{name}` reaches a hazard"
                    ));
                }
            }
        }
        // Every registered descriptor's name is read.
        let prefix = format!(
            "struct {}|{}|",
            self.hops.plugin_descriptor, self.hops.plugin_name_field
        );
        let mut named = Vec::new();
        for object in registered {
            let names = typed
                .named_objects
                .iter()
                .filter(|(_, objects)| objects.contains(&object))
                .filter_map(|(key, _)| key.strip_prefix(&prefix))
                .collect::<Vec<_>>();
            let [name] = names[..] else {
                return underived(format!(
                    "the registered descriptor `{object}` has not one read name"
                ));
            };
            named.push((object, name.to_owned()));
        }
        Ok((named, file.path.clone()))
    }

    /// In the premised census, the literal a creation passes must match a
    /// registered descriptor (ignoring ASCII case, as `check_protocol`
    /// compares): the creation's name lookup cannot fail, and the list holds
    /// only copies of registered descriptors.
    pub(super) fn registered_match(&self, literal: &str) -> Result<String, String> {
        let (registered, path) = self.registered_descriptors()?;
        let matching = registered
            .iter()
            .filter(|(_, name)| name.eq_ignore_ascii_case(literal))
            .map(|(object, _)| object.as_str())
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(format!(
                "P-REGISTERED-DESCRIPTORS: no registered descriptor matches \"{literal}\""
            ));
        }
        Ok(format!(
            "P-REGISTERED-DESCRIPTORS derived: the generated `{}` ({path}) registers {} \
             descriptors, one heap copy each, once, after the list's initialisation; \"{literal}\" \
             matches {} of them ({})",
            self.premise_tables().registration,
            registered.len(),
            matching.len(),
            matching.join(", ")
        ))
    }

    /// Every function from which one of `targets` is reachable.
    fn reach_reaching(&self, targets: &BTreeSet<usize>) -> BTreeSet<usize> {
        let mut back: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (from, tos) in &self.reach.next {
            for to in tos {
                back.entry(*to).or_default().push(*from);
            }
        }
        let mut seen = targets.clone();
        let mut stack = targets.iter().copied().collect::<Vec<_>>();
        while let Some(at) = stack.pop() {
            for from in back.get(&at).into_iter().flatten() {
                if seen.insert(*from) {
                    stack.push(*from);
                }
            }
        }
        seen
    }

    /// The premised census's hazards beside Step B's: a call of a generated
    /// function that names an input list (the derived registration from the
    /// config initializer excepted), and an unlink or re-link of an input
    /// instance. Only in the premised census.
    pub(super) fn premise_hazards(
        &self,
        typed: &TypedFacts,
        hazards: &mut BTreeMap<usize, String>,
    ) {
        let Some(premises) = self.premises else {
            return;
        };
        let tables = self.premise_tables();
        let flow = self.flow;
        let mut generated: BTreeMap<String, String> = BTreeMap::new();
        for file in &premises.generated {
            let tokens = tokenize(&file.text);
            let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
            for (name, _, body) in definitions_of(&texts) {
                let names = (body.0..=body.1).any(|k| {
                    matches!(texts[k - 1], "." | ">")
                        && (texts[k] == flow.list_field || texts[k] == flow.registrations_field)
                });
                if names {
                    generated.entry(name).or_insert_with(|| file.path.clone());
                }
            }
        }
        for definition in self.definitions {
            let texts = self.texts(definition.file);
            for (name, path) in &generated {
                if name == tables.registration && definition.name == tables.config_initializer {
                    continue;
                }
                if !calls_of(&texts, definition.body, name).is_empty() {
                    hazards
                        .entry(self.index[&definition.census_name])
                        .or_insert_with(|| {
                            format!(
                                "calls `{name}`, a generated function ({path}) naming an input list"
                            )
                        });
                }
            }
        }
        for (file, line, function, use_label) in
            member_sites(typed, flow.instance_record, tables.instance_link)
        {
            if reads_only(use_label)
                || (function == self.hops.instance_constructor.0
                    && use_label == format!("arg:{}", tables.list_add))
            {
                continue;
            }
            for definition in self
                .definitions
                .iter()
                .filter(|d| d.name == function && self.files[d.file].path == file)
            {
                hazards
                    .entry(self.index[&definition.census_name])
                    .or_insert_with(|| {
                        format!(
                            "unlinks or re-links an input instance at {file}:{line} ({use_label})"
                        )
                    });
            }
        }
    }

    /// Mutations of a listed member in a function the census does not read
    /// (nothing in the census graph can reach or exclude them).
    fn unread_mutations(&self, typed: &TypedFacts) -> Vec<String> {
        let tables = self.premise_tables();
        let flow = self.flow;
        let mut out = Vec::new();
        let listed = [
            (flow.config_record, flow.list_field),
            (flow.config_record, flow.registrations_field),
            (flow.instance_record, self.hops.instance_plugin_field),
            (flow.instance_record, self.hops.instance_flag_field),
            (flow.instance_record, tables.instance_link),
            (self.hops.collector_record, flow.collector_instance_field),
        ];
        for (record, field) in listed {
            for (file, line, function, use_label) in member_sites(typed, record, field) {
                if reads_only(use_label) {
                    continue;
                }
                let read = self
                    .definitions
                    .iter()
                    .any(|d| d.name == function && self.files[d.file].path == file);
                if !read {
                    out.push(format!(
                        "`{record}::{field}` is used ({use_label}) at {file}:{line} in `{function}`, \
                         a function the census does not read"
                    ));
                }
            }
        }
        out
    }

    /// **P-HEAD-INSTANCE**, narrowed: the constructor returns a non-null
    /// instance only after appending its one allocation to the config's input
    /// list.
    fn creation_appends(&self) -> Result<String, String> {
        let tables = self.premise_tables();
        let flow = self.flow;
        let constructor = self.hops.instance_constructor.0;
        let underived = |why: String| {
            Err(format!(
                "P-HEAD-INSTANCE's appended creation is not derived: {why}"
            ))
        };
        let definition = match self.one_definition(constructor) {
            Ok(definition) => definition,
            Err(why) => return underived(why),
        };
        let texts = self.texts(definition.file);
        let body = definition.body;
        let (_, jumps) = loops(&texts, body);
        if jumps {
            return underived(format!("`{constructor}` has a `goto` or a label"));
        }
        let all_returns = returns(&texts, body);
        let non_null = all_returns
            .iter()
            .filter(|(_, value)| !is_null(value))
            .collect::<Vec<_>>();
        let [(last, value)] = non_null[..] else {
            return underived(format!("`{constructor}` has not one non-null return"));
        };
        let [instance] = value[..] else {
            return underived(format!("`{constructor}` returns `{}`", value.join(" ")));
        };
        if texts.get(last + 3) != Some(&"}") || last + 3 != body.1 {
            return underived(format!(
                "`{constructor}`'s `return {instance};` is not its last statement"
            ));
        }
        if address_taken(&texts, body, instance) {
            return underived(format!("the address of `{instance}` is taken"));
        }
        // `instance` is a local initialised to a null pointer (or not at all)
        // and assigned once from the allocator.
        let assigned = assignments(&texts, body, instance);
        let allocations = assigned
            .iter()
            .filter(|&&k| texts.get(k + 2) == Some(&tables.allocator))
            .copied()
            .collect::<Vec<_>>();
        let others = assigned
            .iter()
            .filter(|&&k| {
                texts.get(k + 2) != Some(&tables.allocator)
                    && !(matches!(texts.get(k + 2), Some(&"NULL" | &"0"))
                        && texts.get(k + 3) == Some(&";")
                        && matches!(texts.get(k - 1), Some(&"*")))
            })
            .count();
        let [allocation] = allocations[..] else {
            return underived(format!(
                "`{instance}` is not allocated once by `{}`",
                tables.allocator
            ));
        };
        if others != 0 {
            return underived(format!(
                "`{instance}` is assigned other than by its null declaration and its allocation"
            ));
        }
        let config = definition.first_parameter.as_deref().unwrap_or("");
        if !assignments(&texts, body, config).is_empty() {
            return underived(format!("`{config}` is reassigned"));
        }
        let append = [
            tables.list_add,
            "(",
            "&",
            instance,
            "-",
            ">",
            tables.instance_link,
            ",",
            "&",
            config,
            "-",
            ">",
            flow.list_field,
            ")",
            ";",
        ];
        let appends = calls_of(&texts, body, tables.list_add)
            .into_iter()
            .filter(|&k| texts.get(k..k + append.len()) == Some(&append[..]))
            .collect::<Vec<_>>();
        let [add] = appends[..] else {
            return underived(format!(
                "`{}(&{instance}->{}, &{config}->{})` is not called once",
                tables.list_add, tables.instance_link, flow.list_field
            ));
        };
        let depth = depths(&texts, body);
        let same_block = depth.get(&add) == depth.get(&allocation) && {
            let d = depth[&allocation];
            (allocation..add).all(|k| depth[&k] >= d)
        };
        let escapes = (allocation..add).any(|k| {
            matches!(texts[k], "break" | "continue" | "goto")
                || (texts[k] == "return" && !is_null(&texts[k + 1..k + 2]))
        });
        if add < allocation || !same_block || escapes {
            return underived(format!(
                "the append at {}:{} is not on every path from `{instance}`'s allocation that \
                 does not return a null pointer",
                self.files[definition.file].path, self.scans[definition.file].tokens[add].line
            ));
        }
        Ok(format!(
            "P-HEAD-INSTANCE narrowed: `{constructor}` returns a non-null instance only after \
             appending its one `{}` allocation to `{config}->{}` ({}:{}); stated: \
             P-CREATIONS-SUCCEED (`{}` and the one creation's `{constructor}` return non-null; \
             the test checks neither before its read)",
            tables.allocator,
            flow.list_field,
            self.files[definition.file].path,
            self.scans[definition.file].tokens[add].line,
            flow.context_constructor
        ))
    }

    /// **P-INSTANCE-FIELDS-BY-MEMBER**: the typed reading's record views.
    pub(super) fn fields_by_member(&self) -> FieldsByMember {
        let tables = self.premise_tables();
        let Some(typed) = self.typed else {
            return FieldsByMember::default();
        };
        let mut out = FieldsByMember {
            read: tables
                .records
                .iter()
                .all(|record| typed.record_views_read.contains(*record)),
            ..FieldsByMember::default()
        };
        for record in tables.records {
            for site in typed.record_views.get(*record).into_iter().flatten() {
                let mut parts = site.splitn(4, '|');
                let (Some(file), Some(line), Some(function), Some(kind)) =
                    (parts.next(), parts.next(), parts.next(), parts.next())
                else {
                    out.violations.push(format!("{record} {site}: unreadable"));
                    continue;
                };
                let label = format!("{record} {file}:{line} {function} {kind}");
                let Some(conversion) = kind.strip_prefix("conv:") else {
                    out.violations.push(label);
                    continue;
                };
                let (destination, context) =
                    conversion.split_once('|').unwrap_or((conversion, "value"));
                let callee = context.strip_prefix("arg:");
                let byte_view = matches!(
                    destination,
                    "char*" | "unsigned char*" | "signed char*" | "uint8_t*" | "int8_t*"
                );
                if callee.is_some_and(|c| tables.byte_functions.contains(&c)) || byte_view {
                    out.violations.push(label);
                } else if destination == "void*" || !destination.ends_with('*') {
                    if callee.is_some_and(|c| tables.release_functions.contains(&c))
                        || context == "compare"
                    {
                        out.harmless += 1;
                    } else {
                        out.escapes.push(label);
                    }
                } else {
                    out.other_types.push(label);
                }
            }
        }
        out
    }

    /// The premised census's checks at the end of the list-head derivation:
    /// the notes of what derived, or why the head is not the one creation's.
    pub(super) fn head_premises(&self) -> Result<Vec<String>, String> {
        let Some(typed) = self.typed else {
            return Err("the premises need the typed reading".to_owned());
        };
        let mut notes = vec![self.fresh_context()?];
        let unread = self.unread_mutations(typed);
        if !unread.is_empty() {
            return Err(format!(
                "P-REGISTERED-DESCRIPTORS is not derived: {}",
                unread.join("; ")
            ));
        }
        if !typed.member_fields_read.contains(&member_key(
            self.flow.instance_record,
            self.premise_tables().instance_link,
        )) {
            return Err(
                "P-HEAD-INSTANCE is not derived: the typed reading did not record an input \
                 instance's links"
                    .to_owned(),
            );
        }
        notes.push(self.creation_appends()?);
        let fields = self.fields_by_member();
        if !fields.read {
            return Err(
                "P-INSTANCE-FIELDS-BY-MEMBER is not derived: the typed reading did not record \
                 the records' non-member uses"
                    .to_owned(),
            );
        }
        if !fields.violations.is_empty() {
            return Err(format!(
                "P-INSTANCE-FIELDS-BY-MEMBER is refuted: {} non-member write(s) or byte view(s) \
                 ({})",
                fields.violations.len(),
                fields.violations.join("; ")
            ));
        }
        notes.push(format!(
            "P-INSTANCE-FIELDS-BY-MEMBER narrowed: no whole assignment, copy, initializer list, \
             byte function or character view of an instance or collector; stated: the {} \
             pointer(s) converted to `void *` or an integer are not written through bytewise, the \
             {} converted to another object type are not used to access one (C's effective-type \
             rule), and no offset from another member's address reaches the plugin, flag or \
             instance member",
            fields.escapes.len(),
            fields.other_types.len()
        ));
        Ok(notes)
    }

    /// **P-INSTANCE** at every listed collector operation of the reading.
    pub(super) fn instance_premises(&self, reading: &CensusReading) -> PremiseReport {
        let tables = self.premise_tables();
        let fields = self.fields_by_member();
        let mut report = PremiseReport {
            fields_by_member: fields.clone(),
            ..PremiseReport::default()
        };
        // A collector's instance is written by its constructor alone, from
        // its instance parameter, through member writes only.
        let collector_fixed = self.collector_instance_fixed(&fields);
        let mut into: BTreeMap<usize, Vec<(usize, ReadHop)>> = BTreeMap::new();
        for edge in &reading.edges {
            into.entry(edge.callee)
                .or_default()
                .push((edge.caller, edge.hop));
        }
        for call in &reading.calls {
            let Some((_, argument)) = tables.operations.iter().find(|(o, _)| *o == call.callee)
            else {
                continue;
            };
            let key = format!("{}:{} {}", call.file, call.line, call.callee);
            let premise = self.site_premise(reading, call, *argument, &into, &collector_fixed);
            report.instance_sites.insert(key, premise);
        }
        report
    }

    fn collector_instance_fixed(&self, fields: &FieldsByMember) -> Result<(), String> {
        let Some(typed) = self.typed else {
            return Err("no typed reading".to_owned());
        };
        let flow = self.flow;
        let collector = self.hops.collector_record;
        let field = flow.collector_instance_field;
        if !typed
            .member_fields_read
            .contains(&member_key(collector, field))
        {
            return Err(format!(
                "the typed reading did not record `{field}`'s writers"
            ));
        }
        for (file, line, function, use_label) in
            member_sites(typed, collector, field).into_iter().chain(
                member_sites(typed, "?", field)
                    .into_iter()
                    .filter(|site| site.3 == "write"),
            )
        {
            if reads_only(use_label) {
                continue;
            }
            if function != flow.collector_constructor || use_label != "write" {
                return Err(format!(
                    "a collector's `{field}` is written at {file}:{line} in `{function}` ({use_label})"
                ));
            }
        }
        if !typed.parameter_stores.iter().any(|store| {
            store.function == flow.collector_constructor
                && store.record == collector
                && store.field == field
        }) {
            return Err(format!(
                "`{}` does not store a parameter in `{field}`",
                flow.collector_constructor
            ));
        }
        let violations = fields
            .violations
            .iter()
            .filter(|v| v.starts_with(collector))
            .collect::<Vec<_>>();
        if !fields.read || !violations.is_empty() {
            return Err(format!(
                "a collector is written other than through its members ({})",
                violations
                    .iter()
                    .map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        Ok(())
    }

    fn site_premise(
        &self,
        reading: &CensusReading,
        call: &super::ReadCall,
        argument: usize,
        into: &BTreeMap<usize, Vec<(usize, ReadHop)>>,
        collector_fixed: &Result<(), String>,
    ) -> SitePremise {
        let Some(definition) = self.definitions.iter().find(|d| {
            self.index[&d.census_name] == call.caller && self.files[d.file].path == call.file
        }) else {
            return SitePremise::Stated("the calling function is not read".to_owned());
        };
        let texts = self.texts(definition.file);
        let tokens = &self.scans[definition.file].tokens;
        let at = calls_of(&texts, definition.body, &call.callee)
            .into_iter()
            .find(|&k| tokens[k].line == call.line);
        let Some(at) = at else {
            return SitePremise::Stated("the call is not read".to_owned());
        };
        let Some((arguments, _)) = call_arguments(&texts, at + 1) else {
            return SitePremise::Stated("the call's arguments are not read".to_owned());
        };
        let Some(&(start, end)) = arguments.get(argument) else {
            return SitePremise::Stated("the call has no instance argument".to_owned());
        };
        let spelled = texts[start..end].join(" ");
        let first = definition.first_parameter.as_deref();
        if end != start + 1 || first != Some(texts[start]) {
            let form = match &texts[start..end] {
                [_, "-", ">", "ins"] => {
                    "P-INSTANCE-CONTEXT: a plugin context's `ins` is the \
                                         instance whose context it is"
                }
                _ => "P-INSTANCE: the argument is the dispatching instance",
            };
            return SitePremise::Stated(format!(
                "the instance argument `{spelled}` is not the function's first parameter; \
                 stated: {form}"
            ));
        }
        let parameter = texts[start];
        if !assignments(&texts, definition.body, parameter).is_empty()
            || address_taken(&texts, definition.body, parameter)
        {
            return SitePremise::Refuted(format!(
                "P-INSTANCE is refuted: the instance parameter `{parameter}` of `{}` is \
                 reassigned or its address taken",
                definition.name
            ));
        }
        if let Err(why) = collector_fixed {
            return SitePremise::Refuted(format!("P-INSTANCE is refuted: {why}"));
        }
        let edges = into.get(&call.caller).cloned().unwrap_or_default();
        if edges.is_empty() {
            return SitePremise::Stated("no edge reaches the function".to_owned());
        }
        let field = self.hops.collector_field;
        let instance = self.flow.collector_instance_field;
        let mut stated = Vec::new();
        let mut dispatchers = BTreeSet::new();
        for (from, hop) in edges {
            // A dispatch from a coroutine entry passes the collector the
            // coroutine was made with (P-CORO-HANDLE's, not derived here).
            let from_entry = reading
                .coroutine_bodies
                .iter()
                .any(|(entry, body)| *entry == from && *body == call.caller);
            match hop {
                ReadHop::CollectorRegistry if from_entry => stated.push(format!(
                    "through the coroutine entry `{}` (the collector the coroutine was made \
                     with, P-CORO-HANDLE)",
                    self.functions[from]
                )),
                ReadHop::CollectorRegistry => {
                    let sites = self
                        .dispatch_sites
                        .iter()
                        .filter(|site| site.caller == from && site.field == field)
                        .collect::<Vec<_>>();
                    if sites.is_empty() {
                        return SitePremise::Stated(format!(
                            "the dispatch of `{}` is not read",
                            self.functions[from]
                        ));
                    }
                    for site in sites {
                        let d_texts = self.texts(site.file);
                        let k = site.position;
                        let line = self.scans[site.file].tokens[k].line;
                        let path = &self.files[site.file].path;
                        let object = (k >= 3
                            && d_texts[k - 1] == ">"
                            && d_texts[k - 2] == "-"
                            && is_identifier(d_texts[k - 3])
                            && !matches!(d_texts.get(k - 4), Some(&"." | &">")))
                        .then(|| d_texts[k - 3]);
                        let passes = object.is_some_and(|c| {
                            call_arguments(&d_texts, k + 1).is_some_and(|(arguments, _)| {
                                arguments.first().is_some_and(|(s, e)| {
                                    d_texts[*s..*e] == [c, "-", ">", instance]
                                })
                            })
                        });
                        if !passes {
                            return SitePremise::Refuted(format!(
                                "P-INSTANCE is refuted: the dispatch at {path}:{line} does not \
                                 pass `C->{instance}` of the collector `C` it calls through"
                            ));
                        }
                        dispatchers.insert(format!("{path}:{line}"));
                    }
                }
                ReadHop::Coroutine => stated.push(format!(
                    "through the coroutine entry `{}` (the collector the coroutine was made \
                     with, P-CORO-HANDLE)",
                    self.functions[from]
                )),
                ReadHop::Direct => stated.push(format!("called by `{}`", self.functions[from])),
                ReadHop::Vtable => {
                    stated.push(format!(
                        "dispatched from `{}` through a field",
                        self.functions[from]
                    ));
                }
            }
        }
        if !stated.is_empty() {
            return SitePremise::Stated(format!(
                "the instance is `{}`'s first parameter, but the function is also reached {}; \
                 stated: P-INSTANCE",
                definition.name,
                stated.join(", ")
            ));
        }
        SitePremise::Derived(format!(
            "P-INSTANCE derived: `{parameter}` is `{}`'s unassigned first parameter, every edge \
             into it is a loop dispatch passing `C->{instance}` ({}), and a collector's \
             `{instance}` is written only by `{}` from its instance parameter",
            definition.name,
            dispatchers.into_iter().collect::<Vec<_>>().join(", "),
            self.flow.collector_constructor
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{definitions_of, depths};

    #[test]
    fn definitions_and_depths() {
        let texts = [
            "int", "f", "(", "int", "a", ")", "{", "if", "(", "a", ")", "{", "}", "}",
        ];
        let found = definitions_of(&texts);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "f");
        assert_eq!(found[0].2, (6, 13));
        let depth = depths(&texts, (6, 13));
        assert_eq!(depth[&7], 1);
        assert_eq!(depth[&12], 1);
    }
}
