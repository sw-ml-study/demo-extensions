//! Generic shape-directed presentation planning.

use crate::{Observation, Recording};

/// Renderer-neutral primary representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    ScalarCard,
    VectorTable,
    MatrixHeatmap,
    TensorSlices,
    FlatTable,
}

/// Finite descriptive statistics for exact values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Summary {
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub mean: Option<f64>,
}

/// A plan that always retains exact numeric fallback data.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewPlan {
    pub kind: ViewKind,
    pub shape: Vec<u64>,
    pub exact_values: Vec<f64>,
    pub summary: Summary,
}

impl Observation {
    /// Choose a generic view from rank while preserving an exact table fallback.
    #[must_use]
    pub fn view_plan(&self) -> ViewPlan {
        let kind = match self.shape.len() {
            0 => ViewKind::ScalarCard,
            1 => ViewKind::VectorTable,
            2 => ViewKind::MatrixHeatmap,
            3 => ViewKind::TensorSlices,
            _ => ViewKind::FlatTable,
        };
        ViewPlan {
            kind,
            shape: self.shape.clone(),
            exact_values: self.values.clone(),
            summary: summarize(&self.values),
        }
    }
}

/// Build a series only from a repeated exact observation name.
#[must_use]
pub fn repeated_series(recording: &Recording, name: &str) -> Vec<(u64, Vec<f64>)> {
    recording
        .frames
        .iter()
        .filter_map(|frame| {
            frame
                .observations
                .iter()
                .find(|observation| observation.name == name)
                .map(|observation| (frame.step, observation.values.clone()))
        })
        .collect()
}

fn summarize(values: &[f64]) -> Summary {
    let Some((&first, rest)) = values.split_first() else {
        return Summary {
            minimum: None,
            maximum: None,
            mean: None,
        };
    };
    let (minimum, maximum, sum) = rest
        .iter()
        .fold((first, first, first), |(minimum, maximum, sum), value| {
            (minimum.min(*value), maximum.max(*value), sum + value)
        });
    let count = values.iter().fold(0.0, |count, _| count + 1.0);
    Summary {
        minimum: Some(minimum),
        maximum: Some(maximum),
        mean: Some(sum / count),
    }
}
