use qcore_schemas::chunk_summary::{BooleanFieldExpression, BooleanFieldStatement, BooleanFieldSourceInput, ChunkSourceField, BOOLEAN_FIELD_SOURCE_INPUT_VERSION};
fn main() {
    let input = BooleanFieldSourceInput {
        schema_version: BOOLEAN_FIELD_SOURCE_INPUT_VERSION,
        function: "example".into(), source_revision: "declared".into(),
        field: ChunkSourceField::Locked,
        program: BooleanFieldStatement::Return(BooleanFieldExpression::Field),
        dependencies: Default::default(), profile_assumptions: Default::default(),
    };
    let json = serde_json::to_value(&input).unwrap();
    assert_eq!(json["schema_version"], 2);
    assert!(json.get("summary").is_none());
    let roundtrip: BooleanFieldSourceInput = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(roundtrip, input);
    let mut old_shape = json;
    old_shape["summary"] = serde_json::json!({});
    assert!(serde_json::from_value::<BooleanFieldSourceInput>(old_shape).is_err());
    println!("schema 1.0 consumer: statement input roundtrip and old-shape rejection pass");
}
