use mlpl_system_layout::{Layout, ValidationError};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../../fixtures/system-layout-v1.json");
const SCHEMA: &str = include_str!("../../../schemas/system-layout-v1.schema.json");

#[test]
fn fixture_satisfies_json_schema_and_semantic_validation() {
    let instance: Value = serde_json::from_str(FIXTURE).unwrap();
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(schema["properties"]["schema"]["const"], instance["schema"]);
    assert_eq!(
        schema["properties"]["version"]["const"],
        instance["version"]
    );

    let layout = Layout::parse(FIXTURE).unwrap();
    assert_eq!(layout.spaces().len(), 2);
    assert_eq!(layout.regions().len(), 4);
    assert_eq!(layout.relationships().len(), 2);
    assert_eq!(layout.spaces()[0].block_size(), Some(512));
    assert_eq!(layout.regions()[0].purpose(), "header");
    assert_eq!(layout.relationships()[0].kind(), "describes");
    assert_eq!(layout.composition().unwrap().format(), "C24IMG");
}

#[test]
fn validation_order_is_stable() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["schema"] = json!("wrong");
    value["version"] = json!(99);
    value["spaces"] = json!([]);
    assert_eq!(
        Layout::parse(&value.to_string()),
        Err(ValidationError::UnsupportedSchema)
    );

    value["schema"] = json!("sw-ml-study.system-layout");
    assert_eq!(
        Layout::parse(&value.to_string()),
        Err(ValidationError::UnsupportedVersion(99))
    );

    value["version"] = json!(1);
    assert_eq!(
        Layout::parse(&value.to_string()),
        Err(ValidationError::Budget("spaces"))
    );
}

#[test]
fn rejects_unknown_references_overflow_and_unknown_fields() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["regions"][0]["space_id"] = json!("missing");
    assert_eq!(
        Layout::parse(&value.to_string()),
        Err(ValidationError::UnknownSpace("missing".into()))
    );

    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["regions"][0]["address"] = json!(u64::MAX);
    assert_eq!(
        Layout::parse(&value.to_string()),
        Err(ValidationError::InvalidRegion("region-a".into()))
    );

    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["surprise"] = json!(true);
    assert_eq!(
        Layout::parse(&value.to_string()),
        Err(ValidationError::Malformed)
    );
}
