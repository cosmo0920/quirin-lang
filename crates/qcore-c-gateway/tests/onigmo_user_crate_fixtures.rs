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

//! Card ONIGMO-PROVIDER-PLUGIN: the user crate `quirin-provider-onigmo`'s
//! modules, as committed, applied by the C gateway to fixtures written
//! against Onigmo 6.2.0's prototypes (`onigmo.h:823-929` at Fluent Bit
//! `046accf6`): for every contracted function an admitted call shape and the
//! refused or accused near-misses (use after `onig_free`, double free, a
//! region freed with `free_self` 0 and leaked, a region used after free, a
//! search on a failed `onig_new`'s slot, a callback that keeps a name
//! pointer), plus the uncontracted `onig_end`.
//!
//! The gateway API applies the environment it is given: neither the pin
//! (C06.2), the version range (rule 53) nor the plugin firewall (rule 10h)
//! is evaluated here, so an admitted row is the host's `Safe`, which the
//! audit routes publish `Pending` behind the crate's checker.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent Onigmo fixture test: {reason}");
            false
        }
    }
}

const MODULES: [(&str, &str); 5] = [
    (
        "onigmo/init.qv",
        include_str!("../../../providers/quirin-provider-onigmo/contracts/onigmo/init.qv"),
    ),
    (
        "onigmo/names.qv",
        include_str!("../../../providers/quirin-provider-onigmo/contracts/onigmo/names.qv"),
    ),
    (
        "onigmo/regex.qv",
        include_str!("../../../providers/quirin-provider-onigmo/contracts/onigmo/regex.qv"),
    ),
    (
        "onigmo/region.qv",
        include_str!("../../../providers/quirin-provider-onigmo/contracts/onigmo/region.qv"),
    ),
    (
        "onigmo/search.qv",
        include_str!("../../../providers/quirin-provider-onigmo/contracts/onigmo/search.qv"),
    ),
];

/// Two callback candidates for `onig_foreach_name`: one that only reads what
/// it is handed (the identity), one that keeps the name pointer.
const CANDIDATES: &str = "module contract;

extern fn names_read(name: ptr<const, u8>, name_end: ptr<const, u8>, ngroups: i32, groups: ptr<mut, u8>, reg: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(name);
    ensures normal resource_borrowed(name_end);
    ensures normal resource_borrowed(ngroups);
    ensures normal resource_borrowed(groups);
    ensures normal resource_borrowed(reg);
    ensures normal resource_borrowed(arg);
    ensures normal resource_return_none();
};

extern fn names_keep(name: ptr<const, u8>, name_end: ptr<const, u8>, ngroups: i32, groups: ptr<mut, u8>, reg: ptr<mut, u8>, arg: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(name);
    ensures normal resource_borrowed(name_end);
    ensures normal resource_borrowed(ngroups);
    ensures normal resource_borrowed(groups);
    ensures normal resource_borrowed(reg);
    ensures normal resource_borrowed(arg);
    ensures normal resource_retained_by(name, global);
    ensures normal resource_return_none();
};
";

/// `onigmo.h`'s view at 6.2.0, reduced to what the fixtures name: the
/// typedefs `pinned/onigmo-6.2.0.json` records and the ten prototypes.
const HEADER: &str = "\
typedef unsigned char OnigUChar;
typedef __PTRDIFF_TYPE__ OnigPosition;
typedef unsigned int OnigOptionType;
typedef struct OnigEncodingTypeST { int min_enc_len; } OnigEncodingType;
typedef const OnigEncodingType* OnigEncoding;
typedef struct { unsigned int op; } OnigSyntaxType;
typedef struct re_pattern_buffer { int num_mem; } OnigRegexType;
typedef OnigRegexType* OnigRegex;
typedef struct re_registers { int allocated; int num_regs; OnigPosition* beg; OnigPosition* end; } OnigRegion;
typedef struct { OnigEncoding enc; OnigUChar* par; OnigUChar* par_end; } OnigErrorInfo;
extern const OnigEncodingType OnigEncodingUTF_8;
extern const OnigSyntaxType OnigSyntaxRuby;
int onig_init(void);
int onig_new(OnigRegex*, const OnigUChar* pattern, const OnigUChar* pattern_end, OnigOptionType option, OnigEncoding enc, const OnigSyntaxType* syntax, OnigErrorInfo* einfo);
void onig_free(OnigRegex);
OnigPosition onig_search(OnigRegex, const OnigUChar* str, const OnigUChar* end, const OnigUChar* start, const OnigUChar* range, OnigRegion* region, OnigOptionType option);
OnigPosition onig_match(OnigRegex, const OnigUChar* str, const OnigUChar* end, const OnigUChar* at, OnigRegion* region, OnigOptionType option);
OnigRegion* onig_region_new(void);
void onig_region_free(OnigRegion* region, int free_self);
int onig_name_to_backref_number(OnigRegex reg, const OnigUChar* name, const OnigUChar* name_end, const OnigRegion *region);
int onig_foreach_name(OnigRegex reg, int (*func)(const OnigUChar*, const OnigUChar*,int,int*,OnigRegex,void*), void* arg);
int onig_end(void);
int names_read(const OnigUChar* name, const OnigUChar* name_end, int ngroups, int* groups, OnigRegex reg, void* arg);
int names_keep(const OnigUChar* name, const OnigUChar* name_end, int ngroups, int* groups, OnigRegex reg, void* arg);
";

const FIXTURES: &str = "\
int init_once(void) { return onig_init(); }

int compile_frame(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    int r = onig_new(&reg, p, pe, 0, 0, 0, &einfo);
    onig_free(reg);
    return r;
}

int compile_fluent_bit(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    int r = onig_new(&reg, p, pe, 0, (&OnigEncodingUTF_8), (&OnigSyntaxRuby), &einfo);
    onig_free(reg);
    return r;
}

int compile_leak(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    return onig_new(&reg, p, pe, 0, 0, 0, &einfo);
}

int compile_use_after_free(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    int r = onig_new(&reg, p, pe, 0, 0, 0, &einfo);
    onig_free(reg);
    if (onig_search(reg, p, pe, p, pe, 0, 0) >= 0) {
        r = 1;
    }
    return r;
}

int compile_double_free(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    int r = onig_new(&reg, p, pe, 0, 0, 0, &einfo);
    onig_free(reg);
    onig_free(reg);
    return r;
}

int search_failed_compile(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    int r = onig_new(&reg, p, pe, 0, 0, 0, &einfo);
    if (r != 0) {
        r = (int) onig_search(reg, p, pe, p, pe, 0, 0);
    }
    onig_free(reg);
    return r;
}

void destroy(OnigRegex reg) { onig_free(reg); }

int region_cycle(void)
{
    OnigRegion *region = onig_region_new();
    if (!region) {
        return -1;
    }
    onig_region_free(region, 1);
    return 0;
}

int region_leak(void)
{
    OnigRegion *region = onig_region_new();
    if (!region) {
        return -1;
    }
    return 0;
}

int region_arrays_only(void)
{
    OnigRegion *region = onig_region_new();
    if (!region) {
        return -1;
    }
    onig_region_free(region, 0);
    return 0;
}

int region_variable_self(int free_self)
{
    OnigRegion *region = onig_region_new();
    onig_region_free(region, free_self);
    return 0;
}

int region_double_free(void)
{
    OnigRegion *region = onig_region_new();
    onig_region_free(region, 1);
    onig_region_free(region, 1);
    return 0;
}

OnigPosition search_region(OnigRegex reg, const OnigUChar *s, const OnigUChar *e)
{
    OnigPosition n;
    OnigRegion *region = onig_region_new();
    if (!region) {
        return -1;
    }
    n = onig_search(reg, s, e, s, e, region, 0);
    onig_region_free(region, 1);
    return n;
}

OnigPosition search_region_after_free(OnigRegex reg, const OnigUChar *s, const OnigUChar *e)
{
    OnigRegion *region = onig_region_new();
    if (!region) {
        return -1;
    }
    onig_region_free(region, 1);
    return onig_search(reg, s, e, s, e, region, 0);
}

OnigPosition match_region(OnigRegex reg, const OnigUChar *s, const OnigUChar *e)
{
    OnigPosition n;
    OnigRegion *region = onig_region_new();
    if (!region) {
        return -1;
    }
    n = onig_match(reg, s, e, s, region, 0);
    onig_region_free(region, 1);
    return n;
}

OnigPosition match_no_region(OnigRegex reg, const OnigUChar *s, const OnigUChar *e)
{
    OnigPosition n;
    n = onig_match(reg, s, e, s, 0, 0);
    return n;
}

OnigPosition search_no_region(OnigRegex reg, const OnigUChar *s, const OnigUChar *e)
{
    OnigPosition n;
    n = onig_search(reg, s, e, s, e, ((void *) 0), 0);
    return n;
}

int backref(OnigRegex reg, const OnigUChar *n, const OnigUChar *ne, const OnigRegion *region)
{
    int number = onig_name_to_backref_number(reg, n, ne, region);
    return number;
}

int names_each(OnigRegex reg, void *arg)
{
    int r = onig_foreach_name(reg, names_read, arg);
    return r;
}

int names_kept(OnigRegex reg, void *arg)
{
    int r = onig_foreach_name(reg, names_keep, arg);
    return r;
}

int names_undeclared(OnigRegex reg, void *arg,
                     int (*f)(const OnigUChar*, const OnigUChar*,int,int*,OnigRegex,void*))
{
    int r = onig_foreach_name(reg, f, arg);
    return r;
}

int end_library(void) { return onig_end(); }

struct holder { void *regex; };

OnigPosition search_member(struct holder *h, const OnigUChar *s, const OnigUChar *e)
{
    OnigPosition n = onig_search(h->regex, s, e, s, e, 0, 0);
    return n;
}

OnigPosition search_null(const OnigUChar *s, const OnigUChar *e)
{
    OnigPosition n = onig_search(0, s, e, s, e, 0, 0);
    return n;
}

int search_after_compile(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    OnigPosition n;
    int r = onig_new(&reg, p, pe, 0, 0, 0, &einfo);
    if (reg != 0) {
        n = onig_search(reg, p, pe, p, pe, 0, 0);
        if (n >= 0) {
            r = 1;
        }
    }
    onig_free(reg);
    return r;
}

int search_after_null_exit(const OnigUChar *p, const OnigUChar *pe)
{
    OnigRegex reg;
    OnigErrorInfo einfo;
    OnigPosition n;
    int r = onig_new(&reg, p, pe, 0, 0, 0, &einfo);
    if (!reg) {
        return r;
    }
    n = onig_search(reg, p, pe, p, pe, 0, 0);
    onig_free(reg);
    return (int) n;
}
";

fn contracts() -> ResourceContracts {
    let mut sources = MODULES
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect::<BTreeMap<_, _>>();
    sources.insert("candidates.qv".to_owned(), CANDIDATES.to_owned());
    load_resource_contract_sources(&sources).expect("the crate's modules load")
}

/// A function's row: its status and every diagnostic, as text.
fn row(name: &str, contracts: &ResourceContracts) -> (AnalysisStatus, Vec<String>) {
    let text = format!("{HEADER}{FIXTURES}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        name,
        contracts,
    )
    .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let row = &summary.functions[name];
    let leaves = row
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect();
    (row.status, leaves)
}

fn has(leaves: &[String], word: &str) -> bool {
    leaves.iter().any(|leaf| leaf.contains(word))
}

/// The modules load: every declaration but `onig_new` and
/// `onig_foreach_name` is in the function map; those two are withheld with
/// the retaining declarations (C04.14, C04.19), and `onig_end` is not
/// declared at all.
#[test]
fn the_crate_modules_load_as_declared() {
    let contracts = contracts();
    for name in [
        "onig_free",
        "onig_search",
        "onig_match",
        "onig_region_new",
        "onig_region_free",
        "onig_name_to_backref_number",
        "onig_init",
    ] {
        assert!(contracts.contract(name).is_some(), "{name} is declared");
    }
    for name in ["onig_new", "onig_foreach_name"] {
        assert!(contracts.contract(name).is_none(), "{name} is withheld");
        assert!(
            contracts.retaining_contract(name).is_some(),
            "{name} is a retaining declaration"
        );
    }
    assert!(contracts.contract("onig_end").is_none());
    assert!(
        contracts.retaining_contract("onig_end").is_none(),
        "onig_end is not declared"
    );
}

/// One admitted shape per contracted function.
#[test]
fn admitted_shapes() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    for name in [
        // onig_init
        "init_once",
        // onig_new and onig_free: the slot's object released on every path;
        // the retained pattern is a caller's object and the `einfo` holder an
        // automatic one only named as `&einfo` (rule 12t: outlives); `enc`
        // and `syntax` null constants (no object).
        "compile_frame",
        // onig_free of a caller's object.
        "destroy",
        // onig_region_new, onig_region_free(_, 1).
        "region_cycle",
        // onig_search and onig_match with a live region.
        "search_region",
        "match_region",
        "backref",
        // onig_foreach_name with a candidate whose contract is the identity.
        "names_each",
    ] {
        let (status, leaves) = row(name, &contracts);
        assert_eq!(status, AnalysisStatus::Safe, "{name}: {leaves:?}");
    }
}

/// The adversarial near-misses: each is refused (`Unknown`) or accused
/// (provisional `Unsafe`), never `Safe` and never `Pending`.
#[test]
fn refused_and_accused_near_misses() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    for (name, expected) in [
        // `enc`/`syntax` are file-scope objects' addresses, Fluent Bit's
        // `ONIG_ENCODING_UTF8`/`ONIG_SYNTAX_RUBY`: the `global` holder admits
        // static storage only as a string literal (rule 12t).
        ("compile_fluent_bit", AnalysisStatus::Unknown),
        // The slot's fresh object is never released.
        ("compile_leak", AnalysisStatus::Unsafe),
        // A use and a second release of the object `onig_free` released.
        ("compile_use_after_free", AnalysisStatus::Unknown),
        ("compile_double_free", AnalysisStatus::Unsafe),
        // A search on the slot a failed `onig_new` left (NULL by the body,
        // `regcomp.c:5883`, `:5892`): `reg` is owned and non-null.
        ("search_failed_compile", AnalysisStatus::Unknown),
        ("search_null", AnalysisStatus::Unknown),
        // No region: the null pointer constant at `resource_owned(region,
        // nullable)` is not admitted by the gateway (a precision limit,
        // ONIGMO-NULL-REGION-ACTUAL; refusing).
        ("search_no_region", AnalysisStatus::Unknown),
        ("match_no_region", AnalysisStatus::Unknown),
        // The same search after a null test of the slot's variable: the test
        // does not refine an out slot's nullable identity of a withheld
        // (retaining) declaration, so the non-null requirement is not shown
        // (a precision limit, ONIGMO-OUT-SLOT-NULL-REFINEMENT; refusing).
        ("search_after_compile", AnalysisStatus::Unknown),
        ("search_after_null_exit", AnalysisStatus::Unknown),
        // A regex read from a member: no custody this body can show.
        ("search_member", AnalysisStatus::Unknown),
        // A region leaked, freed with `free_self` 0 (the undeclared arm), with
        // a variable `free_self`, freed twice, used after free.
        ("region_leak", AnalysisStatus::Unsafe),
        ("region_arrays_only", AnalysisStatus::Unknown),
        ("region_variable_self", AnalysisStatus::Unknown),
        ("region_double_free", AnalysisStatus::Unsafe),
        ("search_region_after_free", AnalysisStatus::Unknown),
        // A callback that keeps the name pointer past its run, and an open
        // candidate set.
        ("names_kept", AnalysisStatus::Unknown),
        ("names_undeclared", AnalysisStatus::Unknown),
        // onig_end has no declaration.
        ("end_library", AnalysisStatus::Unknown),
    ] {
        let (status, leaves) = row(name, &contracts);
        assert_eq!(status, expected, "{name}: {leaves:?}");
    }
}

/// The named reasons of the refusals and accusations.
#[test]
fn refusal_reasons() {
    if !clang_available() {
        return;
    }
    let contracts = contracts();
    let reason = |name: &str, word: &str| {
        let (_, leaves) = row(name, &contracts);
        assert!(has(&leaves, word), "{name}: {word}: {leaves:?}");
    };
    reason(
        "compile_fluent_bit",
        "retains `& OnigEncodingUTF_8` in `global`",
    );
    reason("compile_fluent_bit", "retention-undecided");
    reason("compile_leak", "Leak");
    reason("compile_double_free", "DoubleRelease");
    reason("region_leak", "Leak");
    reason("region_double_free", "DoubleRelease");
    for name in [
        "compile_use_after_free",
        "search_failed_compile",
        "search_null",
        "search_after_compile",
        "search_after_null_exit",
        "search_no_region",
    ] {
        reason(
            name,
            "arguments do not match resource contract for `onig_search`",
        );
    }
    for name in ["region_arrays_only", "region_variable_self"] {
        reason(
            name,
            "call to `onig_region_free` outside its declared literal-argument premise",
        );
    }
    reason("search_region_after_free", "UnknownAlias");
    reason("names_kept", "callback `names_keep`");
    reason("names_undeclared", "callback-undecided");
    reason("end_library", "uncontracted function `onig_end`");
}
