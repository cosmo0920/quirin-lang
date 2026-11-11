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

//! Contract inclusion and contract inheritance (card **CONTRACT-INHERIT**).
//!
//! ```text
//! module contract;
//! #include "quirin-provider-libc:libc/string.qv";
//!
//! extern fn flb_memcpy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
//! contract inherit libc::memcpy;
//! ```
//!
//! # What this module does
//!
//! It resolves every `#include "crate:module";` directive against an explicit
//! [`ContractLoadPath`], resolves every `contract inherit NAME;` against the
//! declarations that are visible, applies the positional renaming, and
//! **elaborates** the module into the contract text a person would have
//! written by hand. The elaborated text is what every consumer parses and what
//! the environment retains as its trusted source assumption; the structural
//! parser keeps refusing the unresolved forms, so nothing reads an unresolved
//! inclusion (see `qcore_frontend::contract_inclusion`).
//!
//! # Publication and the load path
//!
//! A [`PublishedContractCrate`] owns `modules`, publishes a declared subset of
//! them, and names its `dependencies`. A module in crate `A` may include a
//! module `m` of crate `B` only if `B` is `A` or one of `A`'s declared
//! dependencies (no transitive visibility), and `m` is in `B`'s published set
//! — for own-crate includes too. Includes name their crate, so resolution never
//! searches and has no first-match rule. Crate dependency cycles are refused
//! when the load path is built; module include cycles during elaboration.
//!
//! # Inheritance
//!
//! A parent is written `NAME` or `NAMESPACE::NAME`. An unqualified parent is
//! looked up among the module's **own** declarations only (one that only an
//! included module declares is refused with the qualified spelling suggested);
//! a qualified parent among the modules this module includes **directly** from
//! the crate whose namespace that is ([`derive_contract_namespace`]: the crate
//! name without its `quirin-provider-` or `qcore-provider-` prefix, `-` read as
//! `_`). The include string keeps
//! the full Cargo package name — it is what the dependency rule checks — and
//! the qualifier is the namespace derived from it. Nothing is re-exported, so
//! a qualifier always names the crate that owns the contract. Exactly one
//! declaration must match. The parent must be a plain `extern fn` with a contract block, the
//! signatures must agree position by position, and the parent's clauses are
//! copied with each parent parameter word replaced by the child's parameter
//! at the same position. A child parameter name that occurs in the parent's
//! clauses as some *other* word would be captured, and is refused. Chains are
//! admitted; cycles are refused.
//!
//! Whether the renamed text *means* the same to a particular consumer is that
//! consumer's check: the resource-v1 loader compares the loaded child against
//! the loaded parent and refuses any difference
//! (`contract_loader::load_published_resource_contracts`).
//!
//! # Trust
//!
//! An inherited contract is a trusted assumption of the same strength as a
//! hand-written one, never a proof. Its provenance — the chain from child to
//! the declaration whose contract is written out, each with its crate, crate
//! version, module path and content digest — is published beside the
//! environment as [`ContractInclusionEvidence`], and the retained texts let a
//! certificate checker replay the elaboration exactly.
//!
//! Lean model: `proof/QCore/ContractInherit.lean`.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use qcore_frontend::contract_inclusion::{
    ContractIncludeDirective, ContractInclusionSyntaxErrorKind, contains_contract_inclusion_syntax,
    scan_contract_inclusion, valid_contract_crate_name, valid_contract_module_path,
};
use qcore_frontend::frontend::{
    DeclarationKind, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceModuleMode, parse_source_program,
};
use qcore_frontend::lexer::{TokenKind, lex};
use qcore_frontend::source_extension::validate_source_extension;
use qcore_schemas::contract::{
    ContractInclusionEvidence, ContractInheritanceLink, ContractModuleIdentity,
    IncludedContractModule,
};
use qcore_schemas::sha256_hex;

/// The contract loader's inclusion refusals. A loader-owned namespace on the
/// `QP***` precedent: no `Q` route code is allocated (the route space is
/// iceboxed, `Q-NAMESPACE-DECISION`). Meanings: `docs/agents/diagnostics.md`
/// and `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C15.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InclusionCode {
    /// Malformed inclusion syntax or an inheriting declaration of the wrong
    /// shape.
    Qi001,
    /// Extension clauses after `contract inherit NAME`.
    Qi002,
    /// An include target outside the `crate:module` charset.
    Qi003,
    /// A crate that is not on the load path or not reachable from the
    /// includer's crate.
    Qi004,
    /// A module the named crate does not publish.
    Qi005,
    /// An include cycle.
    Qi006,
    /// A duplicate include in one module.
    Qi007,
    /// No visible declaration has the parent's name.
    Qi008,
    /// More than one visible declaration has the parent's name.
    Qi009,
    /// A signature disagreement between child and parent.
    Qi010,
    /// An inheritance cycle.
    Qi011,
    /// A capturing renaming, or a consumer reading the renamed contract
    /// differently from the parent's.
    Qi012,
    /// Inclusion syntax at an entry point that has no load path.
    Qi013,
    /// An invalid load path.
    Qi014,
    /// A parent that is not inheritable.
    Qi015,
    /// Retained inclusion evidence that does not reproduce its environment.
    Qi016,
    /// A load-path crate whose name carries neither admitted prefix
    /// (`quirin-provider-*`, `qcore-provider-*`), whose recorded namespace is
    /// not the derived one, or whose namespace another crate also maps to.
    Qi017,
    /// An unqualified parent that only an included module declares.
    Qi018,
    /// A qualified parent whose namespace names no crate this module includes
    /// directly.
    Qi019,
    /// An inheriting declaration named like its qualified parent.
    Qi020,
}

impl InclusionCode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Qi001 => "QI001",
            Self::Qi002 => "QI002",
            Self::Qi003 => "QI003",
            Self::Qi004 => "QI004",
            Self::Qi005 => "QI005",
            Self::Qi006 => "QI006",
            Self::Qi007 => "QI007",
            Self::Qi008 => "QI008",
            Self::Qi009 => "QI009",
            Self::Qi010 => "QI010",
            Self::Qi011 => "QI011",
            Self::Qi012 => "QI012",
            Self::Qi013 => "QI013",
            Self::Qi014 => "QI014",
            Self::Qi015 => "QI015",
            Self::Qi016 => "QI016",
            Self::Qi017 => "QI017",
            Self::Qi018 => "QI018",
            Self::Qi019 => "QI019",
            Self::Qi020 => "QI020",
        }
    }
}

impl fmt::Display for InclusionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One inclusion refusal. Its `Display` starts with the code, so callers that
/// carry loader errors as strings carry the code unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractInclusionError {
    pub code: InclusionCode,
    /// The module (or load-path element) the refusal is about.
    pub module: String,
    pub message: String,
}

impl ContractInclusionError {
    pub(crate) fn new(
        code: InclusionCode,
        module: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            module: module.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ContractInclusionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: {}", self.code, self.module, self.message)
    }
}

impl std::error::Error for ContractInclusionError {}

impl From<ContractInclusionError> for String {
    fn from(error: ContractInclusionError) -> Self {
        error.to_string()
    }
}

/// Refuse inclusion syntax at an entry point that has no load path (`QI013`).
/// Every such entry point in `contract_loader` calls this before parsing, so
/// an unresolved directive is refused by name rather than reaching the parser.
/// Whether a module needs a load path to be loaded: exactly the predicate on
/// which a path entry (no load path) is refused by
/// [`reject_unresolved_inclusion`]. A module for which this is false loads
/// identically as a file path and, keyed differently, as a load-path
/// reference (card **W44-QUIRIN-AUDIT-PROVIDERS**).
#[must_use]
pub fn uses_contract_inclusion(source: &str) -> bool {
    contains_contract_inclusion_syntax(source)
}

pub(crate) fn reject_unresolved_inclusion(path: &str, source: &str) -> Result<(), String> {
    if contains_contract_inclusion_syntax(source) {
        return Err(ContractInclusionError::new(
            InclusionCode::Qi013,
            path,
            "contract inclusion (`#include` or `contract inherit`) is resolved only against a \
             published contract load path; this entry point has none",
        )
        .into());
    }
    Ok(())
}

/// The package-name prefix of the data-only contract crates.
pub const CONTRACT_CRATE_PREFIX: &str = "quirin-provider-";

/// The package-name prefix of the executable provider crates, which own
/// provider contracts too (card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, the
/// user's option B).
pub const EXECUTABLE_CONTRACT_CRATE_PREFIX: &str = "qcore-provider-";

/// Exactly the package-name prefixes a crate on a load path may carry. There
/// is no other prefix and no freely recorded namespace: a namespace is always
/// derived from the package name.
pub const CONTRACT_CRATE_PREFIXES: [&str; 2] =
    [CONTRACT_CRATE_PREFIX, EXECUTABLE_CONTRACT_CRATE_PREFIX];

/// The inheritance namespace of a contract crate: its package name without
/// its admitted prefix ([`CONTRACT_CRATE_PREFIXES`]), with `-` changed to
/// `_`, so that it is an identifier (`quirin-provider-fluent-bit` →
/// `fluent_bit`, `qcore-provider-librdkafka` → `librdkafka`).
///
/// `None` for a name with neither prefix, or whose remainder is not an
/// identifier (`[a-z_][a-z0-9_]*`). The namespace names the crate that
/// **owns** a contract, never a bundle that re-exports it and never the
/// declarations' own naming prefix: nothing is re-exported, so
/// `quirin-provider-shared` contributes only its own modules, and Monkey's
/// `mk_*` contracts, which `qcore-provider-libevent` owns, are `libevent::…`.
/// Two prefixes can derive one namespace (`quirin-provider-libc` and
/// `qcore-provider-libc` both derive `libc`); the load path refuses that
/// pair like any other shared namespace (`QI017`).
#[must_use]
pub fn derive_contract_namespace(crate_name: &str) -> Option<String> {
    let rest = CONTRACT_CRATE_PREFIXES
        .iter()
        .find_map(|prefix| crate_name.strip_prefix(prefix))?;
    let namespace = rest.replace('-', "_");
    let mut chars = namespace.chars();
    let first = chars.next()?;
    ((first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
    .then_some(namespace)
}

/// One crate on a [`ContractLoadPath`]: what it owns, what it publishes, and
/// which crates it declares as dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedContractCrate {
    /// The crate's package name, e.g. `quirin-provider-libc`.
    pub name: String,
    /// The crate's recorded inheritance namespace, e.g. `libc`. It must equal
    /// [`derive_contract_namespace`] of `name`.
    pub namespace: String,
    /// The crate's version, recorded in every identity it contributes.
    pub version: String,
    /// The crates this crate declares as dependencies.
    pub dependencies: BTreeSet<String>,
    /// Every module the crate owns, by logical path, with its text.
    pub modules: BTreeMap<String, String>,
    /// The logical paths other modules may include. A subset of `modules`.
    pub published: BTreeSet<String>,
}

/// A closed, validated set of published contract crates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractLoadPath {
    crates: BTreeMap<String, PublishedContractCrate>,
}

fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
}

impl ContractLoadPath {
    /// Validate and build a load path. Refuses (`QI014`) a duplicate crate, a
    /// malformed crate name, version or module path, a published module the
    /// crate does not own, a dependency that is absent or the crate itself,
    /// and a dependency cycle; and (`QI017`) a crate whose name is neither
    /// `quirin-provider-*` nor `qcore-provider-*`, whose recorded namespace is
    /// not the derived one, or whose namespace another crate on the load path
    /// shares.
    pub fn new(
        crates: impl IntoIterator<Item = PublishedContractCrate>,
    ) -> Result<Self, ContractInclusionError> {
        let invalid = |name: &str, message: String| {
            ContractInclusionError::new(InclusionCode::Qi014, name, message)
        };
        let mut map = BTreeMap::new();
        for krate in crates {
            if !valid_contract_crate_name(&krate.name) {
                return Err(invalid(&krate.name, "malformed crate name".into()));
            }
            match derive_contract_namespace(&krate.name) {
                None => {
                    return Err(ContractInclusionError::new(
                        InclusionCode::Qi017,
                        &krate.name,
                        format!(
                            "a contract crate's name must be `{CONTRACT_CRATE_PREFIX}NAME` or \
                             `{EXECUTABLE_CONTRACT_CRATE_PREFIX}NAME` with an identifier-shaped NAME"
                        ),
                    ));
                }
                Some(derived) if derived != krate.namespace => {
                    return Err(ContractInclusionError::new(
                        InclusionCode::Qi017,
                        &krate.name,
                        format!(
                            "recorded namespace `{}` is not the derived namespace `{derived}`",
                            krate.namespace
                        ),
                    ));
                }
                Some(_) => {}
            }
            if !valid_version(&krate.version) {
                return Err(invalid(&krate.name, "malformed crate version".into()));
            }
            if let Some(module) = krate
                .modules
                .keys()
                .find(|module| !valid_contract_module_path(module))
            {
                return Err(invalid(
                    &krate.name,
                    format!("malformed module path `{module}`"),
                ));
            }
            crate::extension_compatibility::reject_alias_collisions(
                krate.modules.keys().map(String::as_str),
            )
            .map_err(|message| invalid(&krate.name, message))?;
            for (module, source) in &krate.modules {
                crate::extension_compatibility::validate_source_name(module, source)
                    .map_err(|message| invalid(module, message))?;
            }
            if let Some(module) = krate
                .published
                .iter()
                .find(|module| !krate.modules.contains_key(*module))
            {
                return Err(invalid(
                    &krate.name,
                    format!("publishes `{module}`, which it does not own"),
                ));
            }
            if krate.dependencies.contains(&krate.name) {
                return Err(invalid(
                    &krate.name,
                    "declares itself as a dependency".into(),
                ));
            }
            let name = krate.name.clone();
            if map.insert(name.clone(), krate).is_some() {
                return Err(invalid(&name, "duplicate crate on the load path".into()));
            }
        }
        let mut namespaces: BTreeMap<&str, &str> = BTreeMap::new();
        for krate in map.values() {
            if let Some(other) = namespaces.insert(&krate.namespace, &krate.name) {
                return Err(ContractInclusionError::new(
                    InclusionCode::Qi017,
                    &krate.name,
                    format!(
                        "namespace `{}` is also the namespace of crate `{other}`",
                        krate.namespace
                    ),
                ));
            }
        }
        for krate in map.values() {
            if let Some(missing) = krate
                .dependencies
                .iter()
                .find(|dependency| !map.contains_key(*dependency))
            {
                return Err(invalid(
                    &krate.name,
                    format!("dependency `{missing}` is not on the load path"),
                ));
            }
        }
        // Dependency cycles: depth-first with a visiting stack.
        fn visit<'a>(
            map: &'a BTreeMap<String, PublishedContractCrate>,
            name: &'a str,
            visiting: &mut Vec<&'a str>,
            done: &mut BTreeSet<&'a str>,
        ) -> Result<(), String> {
            if done.contains(name) {
                return Ok(());
            }
            if let Some(position) = visiting.iter().position(|n| *n == name) {
                let mut cycle = visiting[position..].to_vec();
                cycle.push(name);
                return Err(cycle.join(" -> "));
            }
            visiting.push(name);
            for dependency in &map[name].dependencies {
                visit(map, dependency, visiting, done)?;
            }
            visiting.pop();
            done.insert(name);
            Ok(())
        }
        let mut done = BTreeSet::new();
        for name in map.keys() {
            visit(&map, name, &mut Vec::new(), &mut done)
                .map_err(|cycle| invalid(name, format!("crate dependency cycle {cycle}")))?;
        }
        Ok(Self { crates: map })
    }

    /// The crate named `name`, if it is on the load path.
    #[must_use]
    pub fn crate_named(&self, name: &str) -> Option<&PublishedContractCrate> {
        self.crates.get(name)
    }

    /// Every crate on the load path, by name (card W45-USER-CONTRACT-PLUGINS:
    /// an extending manifest joins its base's crates to its own).
    pub fn crates(&self) -> impl Iterator<Item = &PublishedContractCrate> {
        self.crates.values()
    }

    /// The identity and text of a module a crate owns (published or not).
    pub(crate) fn owned_module(
        &self,
        crate_name: &str,
        module: &str,
    ) -> Result<(ContractModuleIdentity, String), ContractInclusionError> {
        let key = format!("{crate_name}:{module}");
        let krate = self.crates.get(crate_name).ok_or_else(|| {
            ContractInclusionError::new(
                InclusionCode::Qi004,
                &key,
                format!("crate `{crate_name}` is not on the load path"),
            )
        })?;
        let text = krate.modules.get(module).ok_or_else(|| {
            ContractInclusionError::new(
                InclusionCode::Qi005,
                &key,
                format!("crate `{crate_name}` owns no module `{module}`"),
            )
        })?;
        Ok((identity_of(krate, module, text), text.clone()))
    }
}

fn identity_of(krate: &PublishedContractCrate, module: &str, text: &str) -> ContractModuleIdentity {
    ContractModuleIdentity {
        crate_name: krate.name.clone(),
        crate_version: krate.version.clone(),
        module: module.to_owned(),
        sha256: sha256_hex(text.as_bytes()),
    }
}

/// The `crate:module` key of an identity: the include spelling.
#[must_use]
pub fn module_key(identity: &ContractModuleIdentity) -> String {
    format!("{}:{}", identity.crate_name, identity.module)
}

/// How an `#include` is turned into a module.
pub(crate) trait IncludeResolver {
    fn resolve(
        &self,
        from: &ContractModuleIdentity,
        directive: &ContractIncludeDirective,
    ) -> Result<(ContractModuleIdentity, String), ContractInclusionError>;
}

/// Resolution against a published load path: the publication rule.
pub(crate) struct LoadPathResolver<'a>(pub(crate) &'a ContractLoadPath);

impl IncludeResolver for LoadPathResolver<'_> {
    fn resolve(
        &self,
        from: &ContractModuleIdentity,
        directive: &ContractIncludeDirective,
    ) -> Result<(ContractModuleIdentity, String), ContractInclusionError> {
        let includer = module_key(from);
        let target = directive.target();
        let krate = self.0.crates.get(&directive.crate_name).ok_or_else(|| {
            ContractInclusionError::new(
                InclusionCode::Qi004,
                &includer,
                format!(
                    "`#include \"{target}\"`: crate `{}` is not on the load path",
                    directive.crate_name
                ),
            )
        })?;
        let reachable = directive.crate_name == from.crate_name
            || self
                .0
                .crates
                .get(&from.crate_name)
                .is_some_and(|own| own.dependencies.contains(&directive.crate_name));
        if !reachable {
            return Err(ContractInclusionError::new(
                InclusionCode::Qi004,
                &includer,
                format!(
                    "`#include \"{target}\"`: crate `{}` is neither `{}` nor one of its declared \
                     dependencies",
                    directive.crate_name, from.crate_name
                ),
            ));
        }
        if !krate.published.contains(&directive.module) {
            let why = if krate.modules.contains_key(&directive.module) {
                "is not published by"
            } else {
                "is not a module of"
            };
            return Err(ContractInclusionError::new(
                InclusionCode::Qi005,
                &includer,
                format!(
                    "`#include \"{target}\"`: `{}` {why} crate `{}`",
                    directive.module, directive.crate_name
                ),
            ));
        }
        let text = &krate.modules[&directive.module];
        Ok((identity_of(krate, &directive.module, text), text.clone()))
    }
}

/// Resolution against retained evidence: exact lookup, used to replay an
/// elaboration without a load path. Records which retained modules it served
/// so an unused one can be refused.
pub(crate) struct RetainedResolver<'a> {
    pub(crate) included: &'a BTreeMap<String, IncludedContractModule>,
    pub(crate) used: RefCell<BTreeSet<String>>,
}

impl IncludeResolver for RetainedResolver<'_> {
    fn resolve(
        &self,
        from: &ContractModuleIdentity,
        directive: &ContractIncludeDirective,
    ) -> Result<(ContractModuleIdentity, String), ContractInclusionError> {
        let target = directive.target();
        let retained = self.included.get(&target).ok_or_else(|| {
            ContractInclusionError::new(
                InclusionCode::Qi016,
                module_key(from),
                format!("retained evidence carries no included module `{target}`"),
            )
        })?;
        if module_key(&retained.identity) != target
            || sha256_hex(retained.source.as_bytes()) != retained.identity.sha256
        {
            return Err(ContractInclusionError::new(
                InclusionCode::Qi016,
                module_key(from),
                format!("retained module `{target}` does not match its recorded identity"),
            ));
        }
        self.used.borrow_mut().insert(target);
        Ok((retained.identity.clone(), retained.source.clone()))
    }
}

/// Where an inheriting declaration's immediate parent is declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ParentSite {
    /// In the same module.
    Own(String),
    /// In a directly included module, by its `crate:module` key.
    Included(String, String),
}

/// One elaborated module.
#[derive(Debug)]
pub(crate) struct Elaborated {
    pub(crate) identity: ContractModuleIdentity,
    /// The text as written.
    pub(crate) source: String,
    /// The elaborated text: directives blanked, every `inherit NAME` replaced
    /// by the parent's clauses under the renaming.
    pub(crate) text: String,
    /// The elaborated text's declarations, in source order.
    pub(crate) declarations: Vec<SourceDeclaration>,
    /// Chain per inheriting declaration, immediate parent first.
    pub(crate) chains: BTreeMap<String, Vec<ContractInheritanceLink>>,
    /// Immediate parent per inheriting declaration.
    pub(crate) parents: BTreeMap<String, ParentSite>,
    /// Every module reached through `#include`, transitively.
    pub(crate) included: BTreeMap<String, IncludedContractModule>,
    /// Directly included modules, by key.
    pub(crate) direct: BTreeMap<String, Rc<Elaborated>>,
    /// Whether the module uses either inclusion form.
    pub(crate) uses_inclusion: bool,
}

impl Elaborated {
    pub(crate) fn declaration(&self, name: &str) -> Option<&SourceDeclaration> {
        self.declarations.iter().find(|d| d.name == name)
    }

    /// The concrete (elaborated) declaration an inheriting declaration's
    /// immediate parent resolved to.
    pub(crate) fn parent_declaration(&self, child: &str) -> Option<&SourceDeclaration> {
        match self.parents.get(child)? {
            ParentSite::Own(name) => self.declaration(name),
            ParentSite::Included(key, name) => self.direct.get(key)?.declaration(name),
        }
    }

    /// The provenance record for a selected module.
    pub(crate) fn evidence(&self) -> ContractInclusionEvidence {
        ContractInclusionEvidence {
            module: self.identity.clone(),
            source: self.source.clone(),
            included: self.included.clone(),
            inherited: self.chains.clone(),
        }
    }
}

/// The elaborator: memoized over modules, with a visiting stack for include
/// cycles.
pub(crate) struct Elaborator<'r> {
    resolver: &'r dyn IncludeResolver,
    done: BTreeMap<String, Rc<Elaborated>>,
    visiting: Vec<String>,
}

fn syntax_code(kind: ContractInclusionSyntaxErrorKind) -> InclusionCode {
    match kind {
        ContractInclusionSyntaxErrorKind::Malformed => InclusionCode::Qi001,
        ContractInclusionSyntaxErrorKind::ExtensionClauses => InclusionCode::Qi002,
        ContractInclusionSyntaxErrorKind::Target => InclusionCode::Qi003,
    }
}

fn token_kinds(source: &str) -> Option<Vec<TokenKind>> {
    lex(source).ok().map(|tokens| {
        tokens
            .into_iter()
            .map(|token| token.kind)
            .filter(|kind| *kind != TokenKind::Eof)
            .collect()
    })
}

fn selector_text(selector: &SourceContractExitSelector) -> String {
    match selector {
        SourceContractExitSelector::Normal => "normal".to_owned(),
        SourceContractExitSelector::Any => "any".to_owned(),
        SourceContractExitSelector::Error(error) => format!("error({error})"),
    }
}

/// A clause as `(introducer, expression)`, e.g. `("ensures normal", "resource_borrowed(n)")`.
type Clause = (String, String);

fn clauses_of(declaration: &SourceDeclaration) -> Vec<Clause> {
    declaration
        .function_contract
        .as_ref()
        .map(|contract| {
            contract
                .clauses
                .iter()
                .map(|clause| {
                    let introducer = match &clause.value.kind {
                        SourceContractClauseKind::Requires => "requires".to_owned(),
                        SourceContractClauseKind::Ensures(selector) => {
                            format!("ensures {}", selector_text(selector))
                        }
                        SourceContractClauseKind::Quant(selector) => {
                            format!("quant {}", selector_text(selector))
                        }
                        // Card QUANT-INDEX-CHECKS: rendered as written, so a
                        // re-rendered clause parses back to the same kind.
                        SourceContractClauseKind::QuantEffect(selector) => {
                            format!("quant effect {}", selector_text(selector))
                        }
                        SourceContractClauseKind::QuantProposition => "quant proposition".into(),
                        SourceContractClauseKind::QuantCondition(selector) => {
                            format!("quant condition {selector}")
                        }
                        SourceContractClauseKind::QuantTheorem => "quant theorem".into(),
                        SourceContractClauseKind::QuantCheck => "quant checks".to_owned(),
                        // Card NULL-SAFETY-PROPERTY: likewise.
                        SourceContractClauseKind::QuantNullDeref => "quant null-deref".to_owned(),
                        // Card NULL-DEREF-QTT-GROUNDING: likewise.
                        SourceContractClauseKind::QuantDeref => "quant deref".to_owned(),
                        SourceContractClauseKind::QuantLive => "quant live".to_owned(),
                        // Card OUTER-CONTRACT: the expression starts at
                        // `#outer`, so `quant` re-renders it as written.
                        SourceContractClauseKind::QuantOuter => "quant".to_owned(),
                        SourceContractClauseKind::QuantConditionBinding => "quant".into(),
                        SourceContractClauseKind::QuantConditionBlock { negated, .. } => {
                            if *negated {
                                "quant not".into()
                            } else {
                                "quant".into()
                            }
                        }
                        SourceContractClauseKind::QuantRegistered => "quant registered".into(),
                        SourceContractClauseKind::QuantCell => "quant cell".into(),
                    };
                    (introducer, clause.value.expression_source.clone())
                })
                .collect()
        })
        .unwrap_or_default()
}

fn render_clauses(clauses: &[Clause]) -> String {
    clauses
        .iter()
        .map(|(introducer, expression)| format!("{introducer} {expression};"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a declaration is exactly `extern fn N(params) -> R contract { BODY };`
/// with no `unsafe`, usage annotation, effects row or other signature syntax.
fn canonical_shape(declaration: &SourceDeclaration, body: &str) -> bool {
    if declaration.kind != DeclarationKind::ExternFunction
        || declaration.function_contract.is_none()
    {
        return false;
    }
    let Some(result) = declaration.result_type.as_ref() else {
        return false;
    };
    let parameters = declaration
        .parameters
        .iter()
        .map(|p| format!("{}: {}", p.name.value, p.type_source))
        .collect::<Vec<_>>()
        .join(", ");
    let expected = format!(
        "extern fn {}({parameters}) -> {} contract {{ {body} }};",
        declaration.name, result.source
    );
    token_kinds(&expected).as_ref() == Some(&declaration.tokens)
}

fn parameter_names(declaration: &SourceDeclaration) -> Vec<String> {
    declaration
        .parameters
        .iter()
        .map(|p| p.name.value.clone())
        .collect()
}

/// Signature agreement: same arity, same type token sequence per position,
/// same result type.
fn signatures_agree(child: &SourceDeclaration, parent: &SourceDeclaration) -> Result<(), String> {
    if child.parameters.len() != parent.parameters.len() {
        return Err(format!(
            "`{}` takes {} parameter(s) and its parent `{}` takes {}",
            child.name,
            child.parameters.len(),
            parent.name,
            parent.parameters.len()
        ));
    }
    for (index, (c, p)) in child.parameters.iter().zip(&parent.parameters).enumerate() {
        if token_kinds(&c.type_source) != token_kinds(&p.type_source) {
            return Err(format!(
                "parameter {} of `{}` is `{}` and of its parent `{}` is `{}`",
                index + 1,
                child.name,
                c.type_source,
                parent.name,
                p.type_source
            ));
        }
    }
    let result = |d: &SourceDeclaration| d.result_type.as_ref().map(|r| r.tokens.clone());
    if result(child) != result(parent) {
        return Err(format!(
            "`{}` returns `{}` and its parent `{}` returns `{}`",
            child.name,
            child.result_type.as_ref().map_or("", |r| r.source.as_str()),
            parent.name,
            parent
                .result_type
                .as_ref()
                .map_or("", |r| r.source.as_str())
        ));
    }
    Ok(())
}

/// The positional renaming σ: parent parameter `pᵢ` ↦ child parameter `cᵢ`,
/// every other word unchanged, applied to every word token of every clause
/// expression. Refuses a capturing renaming: a child parameter name occurring
/// in the parent's clauses (expressions or selectors) as a word that is not a
/// parent parameter.
fn rename_clauses(
    parent_parameters: &[String],
    child_parameters: &[String],
    clauses: &[Clause],
) -> Result<Vec<Clause>, String> {
    let position = |word: &str| parent_parameters.iter().position(|p| p == word);
    let mut renamed = Vec::with_capacity(clauses.len());
    for (introducer, expression) in clauses {
        // Graph IDs and named-premise labels are declaration-local symbols,
        // not formal references. Opaque statements also stay literal. Future
        // typed subjects need a separate capture-aware substitution rule.
        if crate::hypothesis::is_clause(expression)
            || crate::premise_flow::is_clause(expression)
            || crate::contract_lifecycle::is_clause(expression)
            || crate::contract_loader::premise_clause(expression)?.is_some()
        {
            renamed.push((introducer.clone(), expression.clone()));
            continue;
        }
        for token in lex(introducer).map_err(|e| e.to_string())? {
            if let TokenKind::Word(word) = &token.kind
                && (child_parameters.contains(word) || parent_parameters.contains(word))
            {
                return Err(format!(
                    "the clause selector word `{word}` coincides with a parameter name"
                ));
            }
        }
        let tokens = lex(expression).map_err(|e| e.to_string())?;
        let mut out = String::with_capacity(expression.len());
        let mut last = 0;
        for token in &tokens {
            let TokenKind::Word(word) = &token.kind else {
                continue;
            };
            out.push_str(&expression[last..token.span.start]);
            match position(word) {
                Some(index) => out.push_str(&child_parameters[index]),
                None => {
                    if child_parameters.contains(word) {
                        return Err(format!(
                            "child parameter `{word}` would be captured by the parent's clause \
                             `{introducer} {expression};`"
                        ));
                    }
                    out.push_str(word);
                }
            }
            last = token.span.end;
        }
        out.push_str(&expression[last..]);
        renamed.push((introducer.clone(), out));
    }
    Ok(renamed)
}

/// Replace every byte of `span` except newlines by a space, preserving length.
fn blank(text: &mut [u8], start: usize, end: usize) {
    for byte in &mut text[start..end] {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}

impl<'r> Elaborator<'r> {
    pub(crate) fn new(resolver: &'r dyn IncludeResolver) -> Self {
        Self {
            resolver,
            done: BTreeMap::new(),
            visiting: Vec::new(),
        }
    }

    pub(crate) fn elaborate(
        &mut self,
        identity: ContractModuleIdentity,
        source: &str,
    ) -> Result<Rc<Elaborated>, ContractInclusionError> {
        let key = module_key(&identity);
        if let Some(position) = self.visiting.iter().position(|k| *k == key) {
            let mut cycle = self.visiting[position..].to_vec();
            cycle.push(key.clone());
            return Err(ContractInclusionError::new(
                InclusionCode::Qi006,
                &key,
                format!("include cycle {}", cycle.join(" -> ")),
            ));
        }
        if let Some(done) = self.done.get(&key) {
            return Ok(Rc::clone(done));
        }
        self.visiting.push(key.clone());
        let result = self.elaborate_fresh(&key, identity, source);
        self.visiting.pop();
        let elaborated = Rc::new(result?);
        self.done.insert(key, Rc::clone(&elaborated));
        Ok(elaborated)
    }

    fn elaborate_fresh(
        &mut self,
        key: &str,
        identity: ContractModuleIdentity,
        source: &str,
    ) -> Result<Elaborated, ContractInclusionError> {
        let error = |code, message: String| ContractInclusionError::new(code, key, message);
        let syntax = scan_contract_inclusion(source)
            .map_err(|e| error(syntax_code(e.kind), e.to_string()))?;
        if syntax.is_empty() {
            let parsed = parse_source_program(source).map_err(|e| {
                error(
                    InclusionCode::Qi015,
                    format!("module does not parse as a contract module: {e}"),
                )
            })?;
            validate_source_extension(key, parsed.mode)
                .map_err(|message| error(InclusionCode::Qi015, message))?;
            if parsed.mode != SourceModuleMode::Contract {
                return Err(error(
                    InclusionCode::Qi015,
                    "module is not a `module contract;` module".into(),
                ));
            }
            return Ok(Elaborated {
                identity,
                source: source.to_owned(),
                text: source.to_owned(),
                declarations: parsed.declarations.into_iter().map(|d| d.value).collect(),
                chains: BTreeMap::new(),
                parents: BTreeMap::new(),
                included: BTreeMap::new(),
                direct: BTreeMap::new(),
                uses_inclusion: false,
            });
        }

        // Includes: resolve, elaborate first, collect the transitive closure.
        let mut included = BTreeMap::new();
        let mut direct: BTreeMap<String, Rc<Elaborated>> = BTreeMap::new();
        for directive in &syntax.includes {
            let target = directive.target();
            if direct.contains_key(&target) {
                return Err(error(
                    InclusionCode::Qi007,
                    format!("`{target}` is included more than once"),
                ));
            }
            let (child_identity, child_source) = self.resolver.resolve(&identity, directive)?;
            let child = self.elaborate(child_identity.clone(), &child_source)?;
            included.insert(
                target.clone(),
                IncludedContractModule {
                    identity: child_identity,
                    source: child_source,
                },
            );
            for (reached, module) in &child.included {
                included.insert(reached.clone(), module.clone());
            }
            direct.insert(target, child);
        }

        // Probe: the module with directives blanked and each `inherit NAME`
        // replaced by an empty block of the same length, so byte offsets agree
        // with the written text.
        let mut probe = source.as_bytes().to_vec();
        for directive in &syntax.includes {
            blank(&mut probe, directive.span.start, directive.span.end);
        }
        for site in &syntax.inherits {
            blank(&mut probe, site.span.start, site.span.end);
            probe[site.span.start] = b'{';
            probe[site.span.end - 1] = b'}';
        }
        let probe =
            String::from_utf8(probe).map_err(|e| error(InclusionCode::Qi001, e.to_string()))?;
        let parsed = parse_source_program(&probe).map_err(|e| {
            error(
                InclusionCode::Qi001,
                format!("module does not parse once its inclusion forms are set aside: {e}"),
            )
        })?;
        validate_source_extension(key, parsed.mode)
            .map_err(|message| error(InclusionCode::Qi001, message))?;
        if parsed.mode != SourceModuleMode::Contract {
            return Err(error(
                InclusionCode::Qi001,
                "module is not a `module contract;` module".into(),
            ));
        }
        let own: Vec<SourceDeclaration> =
            parsed.declarations.into_iter().map(|d| d.value).collect();
        let own_spans: Vec<_> = own
            .iter()
            .map(|d| d.function_contract.as_ref().map(|c| c.span.start))
            .collect();

        // Map each site to its declaration and check the child's shape.
        let mut sites: BTreeMap<usize, (Option<String>, String)> = BTreeMap::new();
        for site in &syntax.inherits {
            let index = own_spans
                .iter()
                .position(|start| *start == Some(site.contract_span.start))
                .ok_or_else(|| {
                    error(
                        InclusionCode::Qi001,
                        "`contract inherit` is not a declaration's contract".into(),
                    )
                })?;
            let child = &own[index];
            // `unsafe extern fn` carries no contract; a defined `fn` has a body.
            let written_extern = child.tokens.first() == Some(&TokenKind::Word("extern".into()));
            if !written_extern || !canonical_shape(child, "") {
                return Err(error(
                    InclusionCode::Qi001,
                    format!(
                        "`contract inherit` on `{}` requires a plain \
                         `extern fn NAME(PARAMS) -> TYPE contract inherit PARENT;` declaration",
                        child.name
                    ),
                ));
            }
            sites.insert(index, (site.namespace.clone(), site.parent.clone()));
        }

        let mut resolution = Resolution {
            key,
            identity: &identity,
            own: &own,
            sites: &sites,
            direct: &direct,
            resolved: BTreeMap::new(),
            in_progress: BTreeSet::new(),
        };
        for index in sites.keys() {
            resolution.resolve(*index)?;
        }
        let resolved = resolution.resolved;

        // Elaborated text: directives blanked; each `inherit NAME` replaced by
        // the renamed clause block. Replacements are applied back to front.
        let mut text = source.as_bytes().to_vec();
        for directive in &syntax.includes {
            blank(&mut text, directive.span.start, directive.span.end);
        }
        let mut text =
            String::from_utf8(text).map_err(|e| error(InclusionCode::Qi001, e.to_string()))?;
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        for site in &syntax.inherits {
            let index = own_spans
                .iter()
                .position(|start| *start == Some(site.contract_span.start))
                .expect("mapped above");
            let (clauses, _, _) = &resolved[&index];
            replacements.push((
                site.span.start,
                site.span.end,
                format!("{{ {} }}", render_clauses(clauses)),
            ));
        }
        replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0));
        for (start, end, block) in replacements {
            text.replace_range(start..end, &block);
        }
        let elaborated = parse_source_program(&text).map_err(|e| {
            error(
                InclusionCode::Qi012,
                format!("the elaborated module does not parse: {e}"),
            )
        })?;
        let declarations: Vec<SourceDeclaration> = elaborated
            .declarations
            .into_iter()
            .map(|d| d.value)
            .collect();
        if declarations.iter().map(|d| &d.name).collect::<Vec<_>>()
            != own.iter().map(|d| &d.name).collect::<Vec<_>>()
        {
            return Err(error(
                InclusionCode::Qi012,
                "elaboration changed the module's declarations".into(),
            ));
        }
        let mut chains = BTreeMap::new();
        let mut parents = BTreeMap::new();
        for (index, (_, chain, parent)) in resolved {
            if sites.contains_key(&index) {
                chains.insert(own[index].name.clone(), chain);
                parents.insert(own[index].name.clone(), parent);
            }
        }
        Ok(Elaborated {
            identity,
            source: source.to_owned(),
            text,
            declarations,
            chains,
            parents,
            included,
            direct,
            uses_inclusion: true,
        })
    }
}

/// In-module parent resolution with memoization and a cycle check.
struct Resolution<'a> {
    key: &'a str,
    identity: &'a ContractModuleIdentity,
    own: &'a [SourceDeclaration],
    sites: &'a BTreeMap<usize, (Option<String>, String)>,
    direct: &'a BTreeMap<String, Rc<Elaborated>>,
    /// Per own declaration index: its concrete clauses (in its own parameter
    /// names), its chain, and (for an inheriting one) its immediate parent.
    resolved: BTreeMap<usize, (Vec<Clause>, Vec<ContractInheritanceLink>, ParentSite)>,
    in_progress: BTreeSet<usize>,
}

enum Candidate<'a> {
    Own(usize),
    Included(&'a str, &'a Elaborated, &'a SourceDeclaration),
}

impl Resolution<'_> {
    fn error(&self, code: InclusionCode, message: String) -> ContractInclusionError {
        ContractInclusionError::new(code, self.key, message)
    }

    fn resolve(&mut self, index: usize) -> Result<(), ContractInclusionError> {
        if self.resolved.contains_key(&index) {
            return Ok(());
        }
        let child = &self.own[index];
        let (namespace, parent_name) = self.sites[&index].clone();
        if !self.in_progress.insert(index) {
            return Err(self.error(
                InclusionCode::Qi011,
                format!("inheritance cycle through `{}`", child.name),
            ));
        }
        // The scope: an unqualified parent is looked up among this module's own
        // declarations only; `NAMESPACE::NAME` among the directly included
        // modules of the crate whose namespace that is.
        let written = match &namespace {
            Some(namespace) => format!("{namespace}::{parent_name}"),
            None => parent_name.clone(),
        };
        let mut candidates: Vec<Candidate<'_>> = Vec::new();
        match &namespace {
            None => {
                candidates.extend(
                    self.own
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| d.name == parent_name)
                        .map(|(i, _)| Candidate::Own(i)),
                );
                if candidates.is_empty() {
                    let qualified: BTreeSet<String> = self
                        .direct
                        .values()
                        .filter(|module| module.declaration(&parent_name).is_some())
                        .map(|module| {
                            derive_contract_namespace(&module.identity.crate_name).map_or_else(
                                || module.identity.crate_name.clone(),
                                |ns| format!("{ns}::{parent_name}"),
                            )
                        })
                        .collect();
                    if !qualified.is_empty() {
                        return Err(self.error(
                            InclusionCode::Qi018,
                            format!(
                                "`{}` inherits `{parent_name}` unqualified, but only an included \
                                 module declares it; write `contract inherit {}`",
                                child.name,
                                qualified.into_iter().collect::<Vec<_>>().join("` or `")
                            ),
                        ));
                    }
                }
            }
            Some(namespace) => {
                let modules: Vec<(&String, &Rc<Elaborated>)> = self
                    .direct
                    .iter()
                    .filter(|(_, module)| {
                        derive_contract_namespace(&module.identity.crate_name).as_deref()
                            == Some(namespace.as_str())
                    })
                    .collect();
                if modules.is_empty() {
                    return Err(self.error(
                        InclusionCode::Qi019,
                        format!(
                            "`{}` inherits `{written}`, but namespace `{namespace}` names no \
                             crate this module includes directly",
                            child.name
                        ),
                    ));
                }
                if child.name == parent_name {
                    return Err(self.error(
                        InclusionCode::Qi020,
                        format!(
                            "`{}` is named like its parent `{written}`; the two would be one \
                             function with two contracts in one environment",
                            child.name
                        ),
                    ));
                }
                for (key, module) in modules {
                    for declaration in &module.declarations {
                        if declaration.name == parent_name {
                            candidates.push(Candidate::Included(key, module, declaration));
                        }
                    }
                }
            }
        }
        let candidate = match candidates.len() {
            0 => {
                return Err(self.error(
                    InclusionCode::Qi008,
                    format!(
                        "`{}` inherits `{written}`, which no declaration in its scope declares",
                        child.name
                    ),
                ));
            }
            1 => candidates.pop().expect("one candidate"),
            count => {
                return Err(self.error(
                    InclusionCode::Qi009,
                    format!(
                        "`{}` inherits `{written}`, which {count} declarations in its scope declare",
                        child.name
                    ),
                ));
            }
        };
        let (parent_declaration, parent_clauses, parent_chain, parent_identity, site) =
            match candidate {
                Candidate::Own(parent_index) => {
                    let parent = &self.own[parent_index];
                    if self.sites.contains_key(&parent_index) {
                        // An inheriting parent in this module: resolve it first.
                        self.resolve(parent_index)?;
                        let (clauses, chain, _) = &self.resolved[&parent_index];
                        (
                            parent,
                            clauses.clone(),
                            chain.clone(),
                            self.identity.clone(),
                            ParentSite::Own(parent.name.clone()),
                        )
                    } else {
                        let clauses = clauses_of(parent);
                        if !canonical_shape(parent, &render_clauses(&clauses))
                            || parent.tokens.first() != Some(&TokenKind::Word("extern".into()))
                        {
                            return Err(self.error(
                                InclusionCode::Qi015,
                                format!(
                                    "`{parent_name}` is not a plain `extern fn … contract {{ … }};` \
                                     declaration and cannot be inherited"
                                ),
                            ));
                        }
                        (
                            parent,
                            clauses,
                            Vec::new(),
                            self.identity.clone(),
                            ParentSite::Own(parent.name.clone()),
                        )
                    }
                }
                Candidate::Included(key, module, parent) => {
                    let clauses = clauses_of(parent);
                    if !canonical_shape(parent, &render_clauses(&clauses))
                        || parent.tokens.first() != Some(&TokenKind::Word("extern".into()))
                    {
                        return Err(self.error(
                            InclusionCode::Qi015,
                            format!(
                                "`{parent_name}` in `{key}` is not a plain \
                                 `extern fn … contract {{ … }};` declaration and cannot be inherited"
                            ),
                        ));
                    }
                    (
                        parent,
                        clauses,
                        module.chains.get(&parent.name).cloned().unwrap_or_default(),
                        module.identity.clone(),
                        ParentSite::Included(key.to_owned(), parent.name.clone()),
                    )
                }
            };
        signatures_agree(child, parent_declaration)
            .map_err(|message| self.error(InclusionCode::Qi010, message))?;
        // Card W45-CONTRACT-REVISION-MODE: a revision mode is not inherited.
        let clauses = rename_clauses(
            &parameter_names(parent_declaration),
            &parameter_names(child),
            &crate::contract_revision::strip_revision_markers(&parent_clauses),
        )
        .map_err(|message| self.error(InclusionCode::Qi012, message))?;
        let mut chain = vec![ContractInheritanceLink {
            declaration: parent_declaration.name.clone(),
            module: parent_identity,
        }];
        chain.extend(parent_chain);
        self.in_progress.remove(&index);
        self.resolved.insert(index, (clauses, chain, site));
        Ok(())
    }
}

/// One module of a published load path, elaborated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElaboratedContractModule {
    /// `crate:module`.
    pub key: String,
    /// The elaborated text. For a module that uses no inclusion, the text as
    /// written, byte for byte.
    pub text: String,
    /// The module's provenance, if it uses inclusion.
    pub evidence: Option<ContractInclusionEvidence>,
}

/// Elaborate one module owned by a crate on `load_path`, for any consumer.
///
/// This is the interpreter-independent half of the feature: resolution, the
/// publication rule, the positional renaming and capture refusal. It does
/// **not** check that the renamed text means the same thing to a particular
/// consumer; a consumer that reads some clause positions structurally rather
/// than by parameter lookup must compare its reading of every inherited
/// declaration with its reading of the parent, as the resource-v1 loader does
/// (`contract_loader::load_published_resource_contracts`). No other consumer
/// is wired in this card.
pub fn elaborate_contract_module(
    load_path: &ContractLoadPath,
    crate_name: &str,
    module: &str,
) -> Result<ElaboratedContractModule, ContractInclusionError> {
    let (identity, text) = load_path.owned_module(crate_name, module)?;
    let resolver = LoadPathResolver(load_path);
    let elaborated = Elaborator::new(&resolver).elaborate(identity, &text)?;
    Ok(ElaboratedContractModule {
        key: module_key(&elaborated.identity),
        text: elaborated.text.clone(),
        evidence: elaborated.uses_inclusion.then(|| elaborated.evidence()),
    })
}

/// Replay one module's retained evidence and compare it with the elaborated
/// source assumption it accompanies (`QI016` on any difference).
pub(crate) fn replay_evidence(
    key: &str,
    evidence: &ContractInclusionEvidence,
    elaborated_source: &str,
) -> Result<Rc<Elaborated>, ContractInclusionError> {
    let refuse = |message: String| ContractInclusionError::new(InclusionCode::Qi016, key, message);
    if module_key(&evidence.module) != key {
        return Err(refuse(format!(
            "evidence names module `{}`",
            module_key(&evidence.module)
        )));
    }
    if sha256_hex(evidence.source.as_bytes()) != evidence.module.sha256 {
        return Err(refuse(
            "retained source does not match its recorded digest".into(),
        ));
    }
    let resolver = RetainedResolver {
        included: &evidence.included,
        used: RefCell::new(BTreeSet::new()),
    };
    let elaborated = Elaborator::new(&resolver)
        .elaborate(evidence.module.clone(), &evidence.source)
        .map_err(|e| refuse(format!("replay refused: {e}")))?;
    if !elaborated.uses_inclusion {
        return Err(refuse(
            "evidence accompanies a module that uses no inclusion".into(),
        ));
    }
    if elaborated.text != elaborated_source {
        return Err(refuse(
            "replaying the retained texts does not reproduce the elaborated source assumption"
                .into(),
        ));
    }
    if elaborated.chains != evidence.inherited {
        return Err(refuse(
            "replayed inheritance chains differ from the evidence".into(),
        ));
    }
    if *resolver.used.borrow() != evidence.included.keys().cloned().collect::<BTreeSet<_>>()
        || elaborated.included != evidence.included
    {
        return Err(refuse(
            "retained included modules differ from those the replay reaches".into(),
        ));
    }
    Ok(elaborated)
}
