//! Inspect source shapes in a retained GNU17 preprocessing snapshot. This
//! reports no custody verdict and does not authenticate a compile database.
use qcore_c_gateway::collector_identity::{
    LoopQuery, SelectorQuery, import_conditional_loop, import_selection, replay_conditional_loop,
    replay_selection,
};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect, CSourceInput};
use serde::Deserialize;
#[derive(Deserialize)]
struct Query {
    selector: SelectorQuery,
    conditional_loop: LoopQuery,
}
fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example collector_identity_audit");
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [source_path, query_path] = args.as_slice() else {
        return Err("usage: collector_identity_audit PREPROCESSED_C QUERY_JSON".into());
    };
    let source = std::fs::read_to_string(source_path).map_err(|e| e.to_string())?;
    let query: Query =
        serde_json::from_slice(&std::fs::read(query_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let unit = C11TranslationUnit {
        source: &source,
        input: CSourceInput::HistoricalStdin,
        dialect: CCompilerDialect::Gnu17,
        ..Default::default()
    };
    let selector = import_selection(&unit, &query.selector);
    if let Ok(import) = &selector {
        replay_selection(&unit, import)?;
    }
    let conditional = import_conditional_loop(&unit, &query.conditional_loop);
    if let Ok(import) = &conditional {
        replay_conditional_loop(&unit, import)?;
    }
    let report = serde_json::json!({"schema":"qcore.collector-source-audit/1","property":"source selection and constructor snapshot","scope":"retained preprocessing snapshot under GNU17/native Clang; no build-equivalence or lifetime verdict","selector":selector,"conditional_loop":conditional,"open":["list membership implies instance field","cross-call list/key stability","enduring instance/loop/flag frame","config actual equals instance config","caller lifetime composition and thread teardown"]});
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}
