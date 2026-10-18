use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceReturnMode;
use std::collections::BTreeMap;

const CONTRACT: &str = r#"module contract;
extern fn detach(parent: ptr<mut, u8>, item: ptr<mut, u8>) -> ptr<mut, u8>
contract {
 requires resource_contract_v1;
 ensures normal resource_owned(parent, nonnull);
 ensures normal resource_removed(item, parent, nullable);
 ensures normal resource_return_removed(item, nullable);
};"#;
fn load(text: &str) -> Result<qcore_schemas::contract::ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("detach.qv".to_owned(), text.to_owned())]))
}
#[test]
fn removed_result_carries_the_existing_item_not_fresh_custody() {
    let environment = load(CONTRACT).unwrap();
    assert_eq!(
        environment.contract("detach").unwrap().result,
        ResourceReturnMode::Removed {
            parameter: 1,
            nullable: true
        }
    );
}
#[test]
fn unpaired_item_bad_owner_and_nullability_mismatch_refuse_the_module() {
    for text in [
        CONTRACT.replace(
            "resource_removed(item, parent, nullable)",
            "resource_borrowed(item)",
        ),
        CONTRACT.replace(
            "resource_owned(parent, nonnull)",
            "resource_owned(parent, nullable)",
        ),
        CONTRACT.replace(
            "resource_return_removed(item, nullable)",
            "resource_return_removed(item, nonnull)",
        ),
        CONTRACT.replace(
            "resource_return_removed(item, nullable)",
            "resource_return_removed(parent, nullable)",
        ),
        CONTRACT.replace("item: ptr<mut, u8>", "item: ptr<const, u8>"),
        CONTRACT.replace("-> ptr<mut, u8>", "-> i32"),
        CONTRACT.replace("ensures normal resource_owned(parent, nonnull);", "ensures normal resource_owned(parent, nonnull); ensures normal resource_kind(parent, stream);"),
        CONTRACT.replace(
            "resource_removed(item, parent, nullable)",
            "resource_removed(item, item, nullable)",
        ),
    ] {
        assert!(load(&text).is_err(), "{text}");
    }
}
