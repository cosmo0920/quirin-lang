use qcore_c_gateway::legacy_c::{AnalysisStatus, ResourceContracts, analyze_legacy_c};

const ACTUAL_FRAME: &str = include_str!("fixtures/detach-lookup-membership/cjson_lookup_frame.h");

#[test]
fn production_lookup_baseline_refuses_the_unresolved_returned_cursor() {
    let mut sources = vec![("retained tracked frame".to_owned(), ACTUAL_FRAME.to_owned())];
    if let Some(path) = std::env::var_os("QCORE_LOOKUP_PRODUCTION_INPUT") {
        let source = std::fs::read_to_string(&path).expect("explicit retained production source must be readable");
        sources.push((format!("explicit retained source {path:?}"), source));
    }
    for (label, source) in sources {
        let row = analyze_legacy_c(&source, "get_array_item", &ResourceContracts::default()).unwrap();
        println!("{label}: {row:#?}");
        assert_eq!(row.status, AnalysisStatus::Unknown, "{label}: {row:?}");
        assert!(row.diagnostics.iter().any(|d| format!("{:?}", d.kind).contains("current_child")), "{label}: {row:?}");
    }
}
