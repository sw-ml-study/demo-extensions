use std::collections::HashSet;

use serde::Deserialize;

#[derive(Debug, Eq, PartialEq)]
pub enum MetadataError {
    Malformed,
    DuplicateFunction(String),
    DuplicateArgument {
        function: String,
        argument: String,
    },
    DuplicateType(String),
    IncompatibleDefault {
        function: String,
        argument: String,
        expected: String,
    },
    ArityMismatch {
        function: String,
        descriptor: usize,
        metadata: usize,
    },
    ExportMismatch {
        descriptor: Vec<String>,
        metadata: Vec<String>,
    },
    UnknownFunction(String),
    UnknownType(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ExtensionMetadata {
    #[serde(default)]
    functions: Vec<FunctionMetadata>,
    #[serde(default)]
    types: Vec<NativeTypeMetadata>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct FunctionMetadata {
    name: String,
    documentation: String,
    returns: String,
    #[serde(default)]
    arguments: Vec<ArgumentMetadata>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ArgumentMetadata {
    name: String,
    #[serde(rename = "type")]
    value_type: String,
    #[serde(default)]
    default: Option<toml::Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct NativeTypeMetadata {
    name: String,
    documentation: String,
}

impl ExtensionMetadata {
    /// Parses and validates deterministic extension documentation metadata.
    ///
    /// # Errors
    ///
    /// Rejects malformed TOML, duplicate names, and typed defaults that do not
    /// agree with their declared argument type.
    pub fn parse(source: &str) -> Result<Self, MetadataError> {
        let metadata: Self = toml::from_str(source).map_err(|_| MetadataError::Malformed)?;
        metadata.validate()?;
        Ok(metadata)
    }

    /// Confirms metadata describes exactly the exported names and arities.
    ///
    /// # Errors
    ///
    /// Returns a deterministic mismatch with sorted export-name lists, or the
    /// first metadata-order arity mismatch.
    pub fn validate_exports(&self, exports: &[(&str, usize)]) -> Result<(), MetadataError> {
        let mut descriptor: Vec<_> = exports.iter().map(|(name, _)| (*name).to_owned()).collect();
        let mut metadata: Vec<_> = self
            .functions
            .iter()
            .map(|item| item.name.clone())
            .collect();
        descriptor.sort();
        metadata.sort();
        if descriptor != metadata {
            return Err(MetadataError::ExportMismatch {
                descriptor,
                metadata,
            });
        }
        for function in &self.functions {
            let Some(descriptor_arity) = exports
                .iter()
                .find_map(|(name, arity)| (*name == function.name).then_some(*arity))
            else {
                return Err(MetadataError::ExportMismatch {
                    descriptor: descriptor.clone(),
                    metadata: metadata.clone(),
                });
            };
            if descriptor_arity != function.arguments.len() {
                return Err(MetadataError::ArityMismatch {
                    function: function.name.clone(),
                    descriptor: descriptor_arity,
                    metadata: function.arguments.len(),
                });
            }
        }
        Ok(())
    }

    /// Renders one stable function signature and its documentation.
    ///
    /// # Errors
    ///
    /// Returns `UnknownFunction` when the qualified name's final component is
    /// absent from this metadata document.
    pub fn help(&self, qualified_name: &str) -> Result<String, MetadataError> {
        let local_name = qualified_name.rsplit('.').next().unwrap_or(qualified_name);
        let function = self
            .functions
            .iter()
            .find(|item| item.name == local_name)
            .ok_or_else(|| MetadataError::UnknownFunction(qualified_name.to_owned()))?;
        let arguments = function
            .arguments
            .iter()
            .map(format_argument)
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "{qualified_name}({arguments}) -> {}\n{}",
            function.returns, function.documentation
        ))
    }

    /// Renders stable documentation for an extension-defined native type.
    ///
    /// # Errors
    ///
    /// Returns `UnknownType` when the named type is not declared.
    pub fn type_help(&self, name: &str) -> Result<String, MetadataError> {
        let native_type = self
            .types
            .iter()
            .find(|item| item.name == name)
            .ok_or_else(|| MetadataError::UnknownType(name.to_owned()))?;
        Ok(format!(
            "type {}\n{}",
            native_type.name, native_type.documentation
        ))
    }

    fn validate(&self) -> Result<(), MetadataError> {
        let mut functions = HashSet::new();
        for function in &self.functions {
            if !functions.insert(&function.name) {
                return Err(MetadataError::DuplicateFunction(function.name.clone()));
            }
            let mut arguments = HashSet::new();
            for argument in &function.arguments {
                if !arguments.insert(&argument.name) {
                    return Err(MetadataError::DuplicateArgument {
                        function: function.name.clone(),
                        argument: argument.name.clone(),
                    });
                }
                if argument
                    .default
                    .as_ref()
                    .is_some_and(|value| !default_matches(&argument.value_type, value))
                {
                    return Err(MetadataError::IncompatibleDefault {
                        function: function.name.clone(),
                        argument: argument.name.clone(),
                        expected: argument.value_type.clone(),
                    });
                }
            }
        }
        let mut types = HashSet::new();
        for native_type in &self.types {
            if !types.insert(&native_type.name) {
                return Err(MetadataError::DuplicateType(native_type.name.clone()));
            }
        }
        Ok(())
    }
}

fn default_matches(value_type: &str, value: &toml::Value) -> bool {
    matches!(
        (value_type, value),
        ("bool", toml::Value::Boolean(_))
            | ("i64", toml::Value::Integer(_))
            | ("f64", toml::Value::Float(_) | toml::Value::Integer(_))
            | ("string", toml::Value::String(_))
    )
}

fn format_argument(argument: &ArgumentMetadata) -> String {
    let mut rendered = format!("{}: {}", argument.name, argument.value_type);
    if let Some(default) = &argument.default {
        rendered.push_str(" = ");
        rendered.push_str(&default.to_string());
    }
    rendered
}
