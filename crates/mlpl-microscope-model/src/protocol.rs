//! Validated recording protocol values.

use serde::{Deserialize, Serialize};

/// Producer-declared retention limits.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    pub max_frames: u64,
    pub max_observations_per_frame: u64,
    pub max_values_per_observation: u64,
    pub max_total_values: u64,
}

/// One named tensor observation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub name: String,
    pub shape: Vec<u64>,
    pub values: Vec<f64>,
}

/// All observations emitted for one producer step.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub step: u64,
    pub observations: Vec<Observation>,
}

/// A completely validated retained run.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    pub(crate) schema: String,
    pub(crate) version: u64,
    pub lesson: String,
    pub budgets: Budgets,
    pub frames: Vec<Frame>,
}

impl Recording {
    /// Count all retained observations.
    #[must_use]
    pub fn observation_count(&self) -> usize {
        self.frames
            .iter()
            .map(|frame| frame.observations.len())
            .sum()
    }

    /// Count all retained numeric values.
    #[must_use]
    pub fn value_count(&self) -> usize {
        self.frames
            .iter()
            .flat_map(|frame| &frame.observations)
            .map(|observation| observation.values.len())
            .sum()
    }
}
