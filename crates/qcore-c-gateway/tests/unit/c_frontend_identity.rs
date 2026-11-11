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

//! Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): unit tests of the dialect,
//! target, type and extension identities. The vectors marked "Lean" are the
//! ones `QCore.DialectTargetIdentity` states.

use super::*;
use crate::legacy_c::CCompilerDialect;

const ALL: [CCompilerDialect; 7] = [
    CCompilerDialect::C11,
    CCompilerDialect::Gnu99,
    CCompilerDialect::Gnu11,
    CCompilerDialect::Gnu17,
    CCompilerDialect::C17,
    CCompilerDialect::C23,
    CCompilerDialect::C2y,
];

/// Lean `dialectByte`: the baseline keeps its historical byte and nothing
/// else; the three new identities have their own.
#[test]
fn baseline_dialects_keep_their_one_historical_byte() {
    assert_eq!(dialect_identity_bytes(CCompilerDialect::C11), vec![0]);
    assert_eq!(dialect_identity_bytes(CCompilerDialect::Gnu99), vec![1]);
    assert_eq!(dialect_identity_bytes(CCompilerDialect::Gnu11), vec![2]);
    assert_eq!(dialect_identity_bytes(CCompilerDialect::Gnu17), vec![3]);
    assert_eq!(dialect_identity_bytes(CCompilerDialect::C17), vec![4]);
    assert_eq!(dialect_identity_bytes(CCompilerDialect::C23)[0], 5);
    assert_eq!(dialect_identity_bytes(CCompilerDialect::C2y)[0], 6);
}

/// Lean `distinct_dialect_distinct_bytes`.
#[test]
fn every_dialect_has_a_distinct_identity() {
    for left in ALL {
        for right in ALL {
            assert_eq!(
                left == right,
                dialect_identity_bytes(left) == dialect_identity_bytes(right),
                "{left:?} {right:?}"
            );
        }
        assert_eq!(CCompilerDialect::from_std_name(left.std_name()), Some(left));
        assert_eq!(left.clang_argument(), format!("-std={}", left.std_name()));
    }
    for alias in ["c18", "c2x", "gnu23", "gnu2x", "c99", "c89", "gnu2y", "C11"] {
        assert_eq!(CCompilerDialect::from_std_name(alias), None, "{alias}");
    }
    assert_eq!(dialect_differences_sha256(CCompilerDialect::Gnu17), None);
    assert_eq!(dialect_differences_sha256(CCompilerDialect::C17), None);
    assert_ne!(
        dialect_differences_sha256(CCompilerDialect::C23),
        dialect_differences_sha256(CCompilerDialect::C2y)
    );
}

#[test]
fn the_registry_is_refused_and_c2y_extends_c23() {
    assert!(dialect_differences(CCompilerDialect::Gnu17).is_empty());
    let c23 = dialect_differences(CCompilerDialect::C23);
    let c2y = dialect_differences(CCompilerDialect::C2y);
    assert!(c23.iter().all(|difference| c2y.contains(difference)));
    assert!(c2y.len() > c23.len());
    assert!(
        c2y.iter()
            .all(|difference| difference.standing == DifferenceStanding::Refused)
    );
    let mut keys = c2y
        .iter()
        .map(|difference| difference.key)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), c2y.len(), "keys are unique");
}

fn first(dialect: CCompilerDialect, source: &str) -> Option<&'static str> {
    let texts = crate::legacy_c::tokenize(source);
    let texts = texts
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    dialect_difference_in_tokens(dialect, &texts).map(|difference| difference.key)
}

#[test]
fn token_patterns_find_each_registered_difference() {
    use CCompilerDialect::{C2y, C23, Gnu17};
    assert_eq!(
        first(C23, "int f(void *p) { return p == nullptr; }"),
        Some("c23-nullptr")
    );
    assert_eq!(
        first(Gnu17, "int f(void *p) { return p == nullptr; }"),
        None
    );
    assert_eq!(first(C23, "int f(void) { return true; }"), Some("c23-true"));
    assert_eq!(
        first(C23, "[[nodiscard]] int f(void) { return 0; }"),
        Some("c23-attribute")
    );
    assert_eq!(first(C23, "int f(int *a) { return a[a[0]]; }"), None);
    assert_eq!(
        first(C23, "int f(void) { constexpr int k = 1; return k; }"),
        Some("c23-constexpr")
    );
    assert_eq!(
        first(C23, "int f(void) { auto k = 1; return k; }"),
        Some("c23-auto")
    );
    assert_eq!(
        first(C23, "int f(void) { int _Countof = 1; return _Countof; }"),
        None
    );
    assert_eq!(
        first(C2y, "int f(void) { return _Countof(int[2]); }"),
        Some("c2y-countof")
    );
    assert_eq!(
        first(C2y, "int f(void) { return 0o17; }"),
        Some("c2y-octal-prefix")
    );
    assert_eq!(first(C2y, "int f(void) { return 017; }"), None);
    assert_eq!(
        first(
            C2y,
            "int f(void) { o: for (;;) { for (;;) { break o; } } return 0; }"
        ),
        Some("c2y-named-jump")
    );
    assert_eq!(
        first(C2y, "int f(void) { for (;;) { break; } return 0; }"),
        None
    );
    assert_eq!(
        first(
            C2y,
            "int f(int n) { if (int x = n; x) return x; return 0; }"
        ),
        Some("c2y-condition-declaration")
    );
    assert_eq!(
        first(
            C2y,
            "int f(int n) { if (n) { for (;;) { break; } } return 0; }"
        ),
        None
    );
    assert_eq!(
        first(
            C2y,
            "int f(int n) { for (int i = 0; i < n; i++) {} return 0; }"
        ),
        None
    );
}

#[test]
fn digit_separators_refuse_only_under_c23_and_c2y() {
    let separated = "int f(void) { return 1'000; }";
    assert!(digit_separator_refusal(CCompilerDialect::C23, separated).is_some());
    assert!(digit_separator_refusal(CCompilerDialect::C2y, "int x = 0x1'f;").is_some());
    assert!(digit_separator_refusal(CCompilerDialect::Gnu17, separated).is_none());
    for clean in [
        "int f(void) { return 'a' + '0' + 1; }",
        "char *s = \"1'0\"; char c = '\\'';",
        "int a[2] = {1,'2'}; int b = x1'3';",
        "double d = 1.5e+3; int u8c = u8'a';",
    ] {
        assert!(
            digit_separator_refusal(CCompilerDialect::C23, clean).is_none(),
            "{clean}"
        );
    }
}

/// Lean `admitDataModel`: only LP64 (or no explicit selection) is admitted.
#[test]
fn targets_are_admitted_only_under_the_lp64_premise() {
    let model = |triple: &str| CTargetTriple::parse(triple).map(|t| t.data_model());
    assert_eq!(model("x86_64-pc-linux-gnu"), Ok(Some(CDataModel::Lp64)));
    assert_eq!(model("x86_64-linux-gnu"), Ok(Some(CDataModel::Lp64)));
    assert_eq!(
        model("aarch64-unknown-linux-gnu"),
        Ok(Some(CDataModel::Lp64))
    );
    assert_eq!(model("arm64-apple-macosx14.0"), Ok(Some(CDataModel::Lp64)));
    assert_eq!(
        model("riscv64-unknown-linux-gnu"),
        Ok(Some(CDataModel::Lp64))
    );
    assert_eq!(model("x86_64-pc-windows-msvc"), Ok(Some(CDataModel::Llp64)));
    assert_eq!(model("x86_64-w64-windows-gnu"), Ok(Some(CDataModel::Llp64)));
    assert_eq!(model("i686-pc-linux-gnu"), Ok(Some(CDataModel::Ilp32)));
    assert_eq!(model("x86_64-pc-linux-gnux32"), Ok(Some(CDataModel::Ilp32)));
    assert_eq!(
        model("aarch64-linux-gnu_ilp32"),
        Ok(Some(CDataModel::Ilp32))
    );
    assert_eq!(model("x86_64-unknown-unknown"), Err(
        "unsupported compilation target `x86_64-unknown-unknown`: component `unknown` is not recorded".into()
    ));
    for refused in [
        "x86_64",
        "sparc-sun-solaris",
        "x86_64-pc-linux-gnu-extra-more",
        "x86_64-pc-linux gnu",
        "x86_64-pc-$(id)",
        "",
    ] {
        assert!(CTargetTriple::parse(refused).is_err(), "{refused}");
    }
    let identity = |triple: Option<&str>, abi: Option<&str>| CTargetIdentity {
        triple: triple.map(|t| CTargetTriple::parse(t).unwrap()),
        abi: abi.map(|a| CAbiName::parse(a).unwrap()),
        architecture: None,
    };
    assert!(identity(None, None).admit().is_ok());
    assert!(identity(Some("x86_64-pc-linux-gnu"), None).admit().is_ok());
    assert!(
        identity(Some("x86_64-pc-linux-gnu"), Some("sysv"))
            .admit()
            .is_ok()
    );
    assert!(
        identity(Some("riscv64-unknown-linux-gnu"), Some("lp64d"))
            .admit()
            .is_ok()
    );
    assert!(
        identity(Some("x86_64-pc-windows-msvc"), None)
            .admit()
            .is_err()
    );
    assert!(identity(Some("i686-pc-linux-gnu"), None).admit().is_err());
    assert!(
        identity(Some("riscv64-unknown-linux-gnu"), Some("ilp32d"))
            .admit()
            .is_err()
    );
    assert!(identity(None, Some("ilp32")).admit().is_err());
    assert!(identity(None, Some("ms")).admit().is_ok());
    assert!(CAbiName::parse("lp64x").is_err());
    let arch = |architecture: &str| CTargetIdentity {
        architecture: Some(architecture.into()),
        ..Default::default()
    };
    assert!(arch("arm64").admit().is_ok());
    assert!(arch("x86_64").admit().is_ok());
    assert!(arch("i386").admit().is_err());
    assert!(arch("arm64_32").admit().is_err());
    assert!(arch("mystery").admit().is_err());
    let both = CTargetIdentity {
        architecture: Some("arm64".into()),
        ..identity(Some("aarch64-apple-darwin"), None)
    };
    assert!(both.admit().is_err());
}

/// Lean `typeId`: one C type, one identity, whatever its spelling.
#[test]
fn one_c_type_has_one_identity() {
    let id = |text: &str| CTypeId::from_canonical_spelling(text).unwrap();
    assert_eq!(id("long unsigned int"), id("unsigned long"));
    assert_eq!(id("unsigned long").spelling(), "unsigned long");
    assert_eq!(id("bool"), id("_Bool"));
    assert_eq!(id("short int"), id("short"));
    assert_eq!(id("signed"), id("int"));
    assert_eq!(id("int  const"), id("const int"));
    assert_ne!(id("char"), id("signed char"));
    assert_ne!(id("long"), id("long long"));
    assert_ne!(id("unsigned int"), id("int"));
    assert_eq!(
        CTypeId::from_specifiers(&["long", "unsigned"], 0),
        Some(id("unsigned long"))
    );
    assert_eq!(
        CTypeId::from_specifiers(&["const", "char"], 1),
        Some(id("const char *"))
    );
    assert_eq!(
        CTypeId::from_specifiers(&["struct", "cio_chunk"], 1),
        Some(id("struct cio_chunk *"))
    );
    assert_eq!(CTypeId::from_specifiers(&["mystery_t"], 0), None);
    assert_eq!(CTypeId::from_canonical_spelling("   "), None);
}

#[test]
fn the_abi_reader_meets_the_same_identity() {
    let source = "typedef long unsigned int size_t; typedef size_t my_size; \
                  typedef struct cio_chunk *handle; typedef volatile int vint;";
    let tokens = crate::legacy_c::tokenize(source);
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let reader = |words: &[&str], stars| {
        crate::legacy_c::declared_abi::declared_type_id(&texts, words, stars)
    };
    let id = |text: &str| CTypeId::from_canonical_spelling(text);
    assert_eq!(reader(&["my_size"], 0), id("unsigned long"));
    assert_eq!(reader(&["size_t"], 1), id("unsigned long *"));
    assert_eq!(reader(&["handle"], 0), id("struct cio_chunk *"));
    assert_eq!(reader(&["vint"], 0), None);
    assert_eq!(reader(&["undeclared_t"], 0), None);
}

/// Clang's canonical spellings, measured (Clang 21.1.8): G, T and the ABI
/// reader key one type identically.
#[test]
fn routes_g_and_t_read_the_identity_clang_prints() {
    if !crate::test_support::clang_available() {
        return;
    }
    // The lexical ABI reader sees source tokens, not Clang's predefined macro expansion.
    let size_type = if cfg!(windows) {
        "unsigned long long"
    } else {
        "unsigned long"
    };
    let source = format!(
        "typedef {size_type} size_t; typedef size_t my_size;\n\
         my_size subject(my_size a, {size_type} b, const char *c) {{ return a + b + (c != 0); }}\n"
    );
    let unit = crate::legacy_c::C11TranslationUnit {
        source: &source,
        ..Default::default()
    };
    let declarations =
        crate::typed_c_ast::ast_declarations(&source, &unit, "subject").expect("AST");
    let function = declarations
        .iter()
        .find(|node| node["kind"] == "FunctionDecl" && node["name"] == "subject")
        .expect("subject");
    let parameters = function["inner"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "ParmVarDecl")
        .collect::<Vec<_>>();
    let g = parameters
        .iter()
        .map(|node| crate::typed_access_path::type_id(node))
        .collect::<Vec<_>>();
    let t = parameters
        .iter()
        .map(|node| crate::typed_source::canonical_type_id(node))
        .collect::<Vec<_>>();
    assert_eq!(g, t);
    let tokens = crate::legacy_c::tokenize(&source);
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    let abi = crate::legacy_c::declared_abi::declared_type_id(&texts, &["my_size"], 0);
    assert_eq!(g[0], abi);
    assert_eq!(g[0], g[1]);
    assert_eq!(g[2], CTypeId::from_specifiers(&["const", "char"], 1));
}

/// Clang 21.1.8's spellings of each extension family (probe recorded in the
/// card report).
#[test]
fn extension_spellings_are_classified() {
    let family = |text: &str| CTypeId::from_canonical_spelling(text).unwrap().extension();
    assert_eq!(family("_Complex double"), Some(CExtensionFamily::Complex));
    assert_eq!(
        family("__attribute__((__vector_size__(4 * sizeof(int)))) int"),
        Some(CExtensionFamily::Vector)
    );
    assert_eq!(
        family("float __attribute__((ext_vector_type(4)))"),
        Some(CExtensionFamily::Vector)
    );
    assert_eq!(family("_BitInt(7)"), Some(CExtensionFamily::BitPrecise));
    assert_eq!(
        family("unsigned _BitInt(9)"),
        Some(CExtensionFamily::BitPrecise)
    );
    assert_eq!(family("short _Fract"), Some(CExtensionFamily::FixedPoint));
    assert_eq!(family("_Sat _Accum"), Some(CExtensionFamily::FixedPoint));
    assert_eq!(family("__SVInt8_t"), Some(CExtensionFamily::Vector));
    for plain in [
        "unsigned long",
        "struct complex_number *",
        "my_vector_size_t",
        "__int128",
    ] {
        assert_eq!(family(plain), None, "{plain}");
    }
    assert!(unit_spells_extension(&["x", "=", "2.0i"]));
    assert!(unit_spells_extension(&["_BitInt"]));
    assert!(!unit_spells_extension(&[
        "x", "=", "0x1f", "+", "1e3", "+", "10UL"
    ]));
}
