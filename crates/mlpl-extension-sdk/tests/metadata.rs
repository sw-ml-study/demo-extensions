use mlpl_extension_sdk::{ExtensionMetadata, MetadataError};

const VALID: &str = r#"
[[functions]]
name = "greet"
documentation = "Return a greeting count."
returns = "i64"

[[functions.arguments]]
name = "name"
type = "string"
default = "world"

[[functions.arguments]]
name = "excited"
type = "bool"
default = false

[[types]]
name = "Greeting"
documentation = "An opaque greeting resource."
"#;

#[test]
fn valid_metadata_renders_stable_help() {
    let metadata = ExtensionMetadata::parse(VALID).unwrap();
    metadata.validate_exports(&[("greet", 2)]).unwrap();
    assert_eq!(
        metadata.help("hello.greet").unwrap(),
        "hello.greet(name: string = \"world\", excited: bool = false) -> i64\nReturn a greeting count."
    );
    assert_eq!(
        metadata.type_help("Greeting").unwrap(),
        "type Greeting\nAn opaque greeting resource."
    );
}

#[test]
fn malformed_and_duplicate_metadata_fail_deterministically() {
    assert_eq!(
        ExtensionMetadata::parse("not = [valid"),
        Err(MetadataError::Malformed)
    );

    let duplicate_function = format!("{VALID}\n{VALID}");
    assert_eq!(
        ExtensionMetadata::parse(&duplicate_function),
        Err(MetadataError::DuplicateFunction("greet".into()))
    );

    let duplicate_argument = VALID.replace("name = \"excited\"", "name = \"name\"");
    assert_eq!(
        ExtensionMetadata::parse(&duplicate_argument),
        Err(MetadataError::DuplicateArgument {
            function: "greet".into(),
            argument: "name".into(),
        })
    );

    let duplicate_type =
        format!("{VALID}\n[[types]]\nname = \"Greeting\"\ndocumentation = \"duplicate\"\n");
    assert_eq!(
        ExtensionMetadata::parse(&duplicate_type),
        Err(MetadataError::DuplicateType("Greeting".into()))
    );
}

#[test]
fn incompatible_defaults_and_exports_fail_closed() {
    let incompatible = VALID.replace("default = false", "default = \"no\"");
    assert_eq!(
        ExtensionMetadata::parse(&incompatible),
        Err(MetadataError::IncompatibleDefault {
            function: "greet".into(),
            argument: "excited".into(),
            expected: "bool".into(),
        })
    );

    let metadata = ExtensionMetadata::parse(VALID).unwrap();
    assert_eq!(
        metadata.validate_exports(&[("greet", 1)]),
        Err(MetadataError::ArityMismatch {
            function: "greet".into(),
            descriptor: 1,
            metadata: 2,
        })
    );
    assert_eq!(
        metadata.validate_exports(&[("other", 2)]),
        Err(MetadataError::ExportMismatch {
            descriptor: vec!["other".into()],
            metadata: vec!["greet".into()],
        })
    );
}
