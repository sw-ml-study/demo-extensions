use crate::InvokeFnV1;

#[derive(Debug)]
pub struct ValidatedFunction {
    name: String,
    arity: u32,
    invoke: Option<InvokeFnV1>,
}

impl PartialEq for ValidatedFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.arity == other.arity
            && self.invoke.is_some() == other.invoke.is_some()
    }
}

impl Eq for ValidatedFunction {}

impl ValidatedFunction {
    pub(crate) const fn new(name: String, arity: u32, invoke: Option<InvokeFnV1>) -> Self {
        Self {
            name,
            arity,
            invoke,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn arity(&self) -> u32 {
        self.arity
    }

    #[must_use]
    pub const fn invoke(&self) -> Option<InvokeFnV1> {
        self.invoke
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
