// SPDX-License-Identifier: Apache-2.0
//
// K4-FORMAL-CUSTODY-ALIASES scratch probe: prints the base answer of the
// three K4 rows reduced to their shapes. Not compiled from here; to rerun,
// copy it to crates/qcore-c-gateway/tests/k4_probe_scratch.rs and run
// `cargo test -p qcore-c-gateway --test k4_probe_scratch -- --nocapture`.
// Output at base 5087f9295: ../artifacts/probe-base-5087f9295.txt.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn x_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_len(s: ptr<const, u8>) -> usize\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(s);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_cat(b: ptr<mut, ptr<mut, u8>>, s: ptr<const, u8>, n: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(b);\n\
           ensures normal resource_borrowed(s); ensures normal resource_borrowed(n);\n\
           ensures normal resource_slot_moved(b, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_log(f: ptr<const, u8>, a: ptr<const, u8>, b: ptr<const, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(f);\n\
           ensures normal resource_borrowed(a); ensures normal resource_borrowed(b);\n\
           ensures normal resource_return_none(); };\n\
extern fn x_exit(c: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_none(); ensures normal resource_no_return(); };\n\
extern fn x_dlclose(h: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(h, nonnull);\n\
           ensures normal resource_kind(h, dynamic_library); ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ x_size; \
void x_free(void *p); x_size x_len(const char *s); void x_cat(char **b, const char *s, int n); \
void x_log(const char *f, const char *a, const char *b); void x_exit(int c) __attribute__((noreturn)); int x_dlclose(void *h); \
typedef struct x_hnd_s x_hnd_t; typedef char *x_sds_t;";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "k4_probe.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

fn show(label: &str, body: &str) {
    let source = format!("{DECLARATIONS}\n{body}");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    let status = match &result {
        UnrestrictedLegacyResult::Safe(_) => "Safe",
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let f = program(&result).functions.get("w").expect("w");
    eprintln!("PROBE {label}: {status}");
    for d in &f.diagnostics {
        match &d.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => {
                eprintln!("   unknown_alias:{variable} {:?}", d.line)
            }
            other => eprintln!("   {other:?}"),
        }
    }
    eprintln!("   summary: {:?}", f.summary_mode_debug());
}

trait Dbg {
    fn summary_mode_debug(&self) -> String;
}
impl<T: std::fmt::Debug> Dbg for T {
    fn summary_mode_debug(&self) -> String {
        let s = format!("{self:?}");
        s.chars().take(600).collect()
    }
}

#[test]
fn probe() {
    show(
        "metric_escape",
        "void w(char **buf, char *description, _Bool q) { int i; x_size len; \
         len = x_len(description); \
         for (i = 0; i < len; i++) { switch (description[i]) { \
           case '\\\\': x_cat(buf, \"\\\\\\\\\", 2); break; \
           case '\"': if (q) { x_cat(buf, \"x\", 2); break; } \
           default: x_cat(buf, description + i, 1); break; } } }",
    );
    show(
        "metric_escape without the arithmetic actual",
        "void w(char **buf, char *description, _Bool q) { int i; x_size len; \
         len = x_len(description); \
         for (i = 0; i < len; i++) { switch (description[i]) { \
           case '\\\\': x_cat(buf, \"\\\\\\\\\", 2); break; \
           default: x_cat(buf, \"y\", 1); break; } } }",
    );
    show(
        "metric_escape with an if instead of a switch",
        "void w(char **buf, char *description) { int i; x_size len; \
         len = x_len(description); \
         for (i = 0; i < len; i++) { if (description[i] == 'a') x_cat(buf, description + i, 1); } }",
    );
    show(
        "metric_escape, typedef'd formal, as in the source",
        r#"static void w(x_sds_t *buf, x_sds_t description, _Bool escape_quote) { int i; x_size len;
         len = x_len(description);
         for (i = 0; i < len; i++) { switch (description[i]) {
           case '\\': x_cat(buf, "\\\\", 2); break;
           case '\n': x_cat(buf, "\\n", 2); break;
           case '"': if (escape_quote) { x_cat(buf, "\\\"", 2); break; }
           default: x_cat(buf, description + i, 1); break; } } }"#,
    );
    show(
        "B1 char formal, x_sds_t *buf",
        r#"void w(x_sds_t *buf, char *description) { int i; x_size len;
         len = x_len(description);
         for (i = 0; i < len; i++) { switch (description[i]) {
           default: x_cat(buf, description + i, 1); break; } } }"#,
    );
    show(
        "B2 typedef formal, char **buf",
        r#"void w(char **buf, x_sds_t description) { int i; x_size len;
         len = x_len(description);
         for (i = 0; i < len; i++) { switch (description[i]) {
           default: x_cat(buf, description + i, 1); break; } } }"#,
    );
    show(
        "B3 typedef formal, char **buf, no arithmetic",
        r#"void w(char **buf, x_sds_t description) { int i; x_size len;
         len = x_len(description);
         for (i = 0; i < len; i++) { switch (description[i]) {
           default: x_cat(buf, "y", 1); break; } } }"#,
    );
    show(
        "B4 typedef formal, arithmetic actual, no loop",
        r#"void w(char **buf, x_sds_t description) { x_cat(buf, description + 1, 1); }"#,
    );
    show(
        "B5 char formal, arithmetic actual, no loop",
        r#"void w(char **buf, char *description) { x_cat(buf, description + 1, 1); }"#,
    );
    show(
        "B6 typedef formal, bare actual",
        r#"void w(char **buf, x_sds_t description) { x_cat(buf, description, 1); }"#,
    );
    show(
        "typedef'd formal, subscript in an if",
        "void w(x_sds_t description) { if (description[0] == 'a') x_len(description); }",
    );
    show(
        "typedef'd formal, subscript in a switch",
        "void w(x_sds_t d) { switch (d[0]) { case 'a': x_len(d); break; default: break; } }",
    );
    show(
        "plain formal, subscript in a switch",
        "void w(char *d) { switch (d[0]) { case 'a': x_len(d); break; default: break; } }",
    );
    show(
        "rd_dl_close",
        "void w(x_hnd_t *handle) { x_free((void *)handle); }",
    );
    show(
        "rd_dl_close with the dynamic_library release",
        "void w(x_hnd_t *handle) { x_dlclose((void *)handle); }",
    );
    show(
        "mk_config_print_error_msg without the exit",
        "void w(char *variable, char *path) { x_log(\"%s %s\", variable, path); x_free(path); }",
    );
    show(
        "rd_dl_close without the cast",
        "void w(char *handle) { x_free(handle); }",
    );
    show(
        "mk_config_print_error_msg",
        "void w(char *variable, char *path) { x_log(\"%s %s\", variable, path); \
         x_free(path); x_exit(1); }",
    );
}
