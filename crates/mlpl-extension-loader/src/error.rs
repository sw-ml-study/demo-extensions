use mlpl_extension_abi::DescriptorError;
use mlpl_extension_sdk::MetadataError;
use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum LoadError {
    Open,
    MissingEntry,
    NullDescriptor,
    InvalidDescriptor(DescriptorError),
    InvalidMetadata(MetadataError),
    MissingInvoke(String),
    DuplicateName(String),
    NamespaceMismatch { expected: String, actual: String },
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

#[derive(Debug, Eq, PartialEq)]
pub enum PackageError {
    ManifestRead,
    InvalidManifest,
    InvalidName(String),
    NamespaceMismatch {
        expected: String,
        actual: String,
    },
    UnsupportedAbi(u32),
    UnsafePath(String),
    MissingModule(String),
    MissingArtifact(String),
    EscapedPackage(String),
    UnsupportedPlatform {
        requested: String,
        available: Vec<String>,
    },
    DuplicatePlatform(String),
    DuplicateName(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManifestRead => write!(formatter, "could not read extension manifest"),
            Self::InvalidManifest => write!(formatter, "invalid extension manifest"),
            Self::InvalidName(name) => write!(formatter, "invalid extension name '{name}'"),
            Self::NamespaceMismatch { expected, actual } => write!(
                formatter,
                "native namespace must be '{expected}', received '{actual}'"
            ),
            Self::UnsupportedAbi(abi) => write!(formatter, "unsupported extension ABI {abi}"),
            Self::UnsafePath(path) => write!(formatter, "unsafe package path '{path}'"),
            Self::MissingModule(path) => write!(formatter, "missing MLPL module '{path}'"),
            Self::MissingArtifact(path) => write!(formatter, "missing native artifact '{path}'"),
            Self::EscapedPackage(path) => write!(formatter, "package path escapes root '{path}'"),
            Self::UnsupportedPlatform {
                requested,
                available,
            } => write!(
                formatter,
                "unsupported platform '{requested}'; available: {}",
                available.join(", ")
            ),
            Self::DuplicatePlatform(target) => {
                write!(formatter, "duplicate native platform '{target}'")
            }
            Self::DuplicateName(name) => write!(formatter, "duplicate extension name '{name}'"),
        }
    }
}

impl std::error::Error for PackageError {}
