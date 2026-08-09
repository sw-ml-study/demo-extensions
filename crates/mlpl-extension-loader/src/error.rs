use mlpl_extension_abi::DescriptorError;

#[derive(Debug, Eq, PartialEq)]
pub enum LoadError {
    Open,
    MissingEntry,
    NullDescriptor,
    InvalidDescriptor(DescriptorError),
    MissingInvoke(String),
    DuplicateName(String),
}

#[derive(Debug, Eq, PartialEq)]
pub enum CallError {
    Inactive(String),
    UnknownFunction(String),
    WrongArity { expected: usize, actual: usize },
    UnsupportedArguments,
    InvalidResult,
    InvalidError,
    Extension(String),
    ExtensionPanicked,
}
