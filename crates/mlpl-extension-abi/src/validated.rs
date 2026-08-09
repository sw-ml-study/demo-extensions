#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedFunction {
    name: String,
    arity: u32,
}

impl ValidatedFunction {
    pub(crate) const fn new(name: String, arity: u32) -> Self {
        Self { name, arity }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn arity(&self) -> u32 {
        self.arity
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ValidatedExtension {
    name: String,
    version: String,
    functions: Vec<ValidatedFunction>,
}

impl ValidatedExtension {
    pub(crate) const fn new(
        name: String,
        version: String,
        functions: Vec<ValidatedFunction>,
    ) -> Self {
        Self {
            name,
            version,
            functions,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub fn functions(&self) -> &[ValidatedFunction] {
        &self.functions
    }
}
