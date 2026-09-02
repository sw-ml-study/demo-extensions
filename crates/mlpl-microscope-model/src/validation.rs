//! Ordered fail-closed recording validation.
use crate::Recording;
const SCHEMA: &str = "sw-ml-study.ml-microscope-recording";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationKind {
    Schema,
    Structure,
    Numeric,
    Budget,
    Shape,
    StepOrder,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub kind: ValidationKind,
    pub message: String,
}
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ValidationError {}
/// Parse a complete recording, rejecting malformed or over-budget input.
///
/// # Errors
/// Returns the first ordered validation failure without retaining partial data.
pub fn parse_recording(input: &str) -> Result<Recording, ValidationError> {
    crate::validation_support::reject_duplicate_keys(input)?;
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|error| failure(ValidationKind::Structure, error.to_string()))?;
    validate_schema(&value)?;
    let recording: Recording = serde_json::from_value(value).map_err(|error| classify(&error))?;
    crate::validation_support::validate_structure(&recording)?;
    validate_budgets(&recording)?;
    validate_shapes(&recording)?;
    crate::validation_support::validate_steps(&recording)?;
    Ok(recording)
}
fn validate_schema(value: &serde_json::Value) -> Result<(), ValidationError> {
    if value.get("schema").and_then(serde_json::Value::as_str) != Some(SCHEMA)
        || value.get("version").and_then(serde_json::Value::as_u64) != Some(0)
    {
        Err(failure(
            ValidationKind::Schema,
            "unsupported recording schema or version",
        ))
    } else {
        Ok(())
    }
}
fn validate_budgets(recording: &Recording) -> Result<(), ValidationError> {
    let b = &recording.budgets;
    if [
        b.max_frames,
        b.max_observations_per_frame,
        b.max_values_per_observation,
        b.max_total_values,
    ]
    .contains(&0)
    {
        return Err(failure(ValidationKind::Budget, "budgets must be positive"));
    }
    if u64::try_from(recording.frames.len()).unwrap_or(u64::MAX) > b.max_frames {
        return Err(failure(ValidationKind::Budget, "frame budget exceeded"));
    }
    let mut total = 0_u64;
    for frame in &recording.frames {
        if u64::try_from(frame.observations.len()).unwrap_or(u64::MAX)
            > b.max_observations_per_frame
        {
            return Err(failure(
                ValidationKind::Budget,
                "observation budget exceeded",
            ));
        }
        for observation in &frame.observations {
            let count = u64::try_from(observation.values.len())
                .map_err(|_| failure(ValidationKind::Numeric, "value count overflow"))?;
            if count > b.max_values_per_observation {
                return Err(failure(
                    ValidationKind::Budget,
                    "observation value budget exceeded",
                ));
            }
            total = total
                .checked_add(count)
                .ok_or_else(|| failure(ValidationKind::Budget, "total value count overflow"))?;
            if total > b.max_total_values {
                return Err(failure(
                    ValidationKind::Budget,
                    "total value budget exceeded",
                ));
            }
        }
    }
    Ok(())
}
fn validate_shapes(recording: &Recording) -> Result<(), ValidationError> {
    for observation in recording
        .frames
        .iter()
        .flat_map(|frame| &frame.observations)
    {
        let product = observation
            .shape
            .iter()
            .try_fold(1_u64, |value, dimension| value.checked_mul(*dimension))
            .ok_or_else(|| failure(ValidationKind::Shape, "shape product overflow"))?;
        if product
            != u64::try_from(observation.values.len())
                .map_err(|_| failure(ValidationKind::Numeric, "value count overflow"))?
        {
            return Err(failure(
                ValidationKind::Shape,
                "shape product does not match value count",
            ));
        }
    }
    Ok(())
}
pub(crate) fn failure(kind: ValidationKind, message: impl Into<String>) -> ValidationError {
    ValidationError {
        kind,
        message: message.into(),
    }
}
fn classify(error: &serde_json::Error) -> ValidationError {
    let message = error.to_string();
    let kind = if message.contains("invalid value: integer `-")
        || message.contains("number out of range")
    {
        ValidationKind::Numeric
    } else {
        ValidationKind::Structure
    };
    failure(kind, message)
}
